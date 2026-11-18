//! ffprobe/ffmpeg 로 입력을 읽어 RGBA 프레임을 내보낸다.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub const MAX_FPS: f64 = 30.0;

const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "ppm", "pgm", "tif", "tiff"];
const IMAGE_FORMATS: &[&str] = &[
    "image2", "png_pipe", "jpeg_pipe", "webp_pipe", "bmp_pipe", "ppm_pipe", "pgm_pipe", "tiff_pipe",
];

#[derive(Debug, Clone)]
pub struct MediaInfo {
    pub path: PathBuf,
    pub width: usize,
    pub height: usize,
    pub fps: f64,
    pub duration: Option<f64>,
    pub is_image: bool,
}

impl MediaInfo {
    pub fn name(&self) -> String {
        self.path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
    }
}

fn which(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|d| d.join(name).is_file()))
        .unwrap_or(false)
}

pub fn require_ffmpeg() -> Result<(), String> {
    let missing: Vec<&str> = ["ffmpeg", "ffprobe"].into_iter().filter(|n| !which(n)).collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} 를 찾을 수 없습니다. macOS 라면 'brew install ffmpeg' 로 설치하세요.",
            missing.join(", ")
        ))
    }
}

fn parse_rate(v: Option<&serde_json::Value>) -> f64 {
    let Some(s) = v.and_then(|v| v.as_str()) else { return 0.0 };
    match s.split_once('/') {
        Some((a, b)) => {
            let (a, b) = (a.parse::<f64>().unwrap_or(0.0), b.parse::<f64>().unwrap_or(0.0));
            if b > 0.0 { a / b } else { 0.0 }
        }
        None => s.parse().unwrap_or(0.0),
    }
}

pub fn probe(path: &Path) -> Result<MediaInfo, String> {
    if !path.exists() {
        return Err(format!("파일이 없습니다: {}", path.display()));
    }
    if path.is_dir() {
        return Err(format!("폴더가 아니라 파일을 넣어 주세요: {}", path.display()));
    }
    require_ffmpeg()?;
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries"])
        .arg("stream=width,height,avg_frame_rate,r_frame_rate:stream_side_data=rotation:format=duration,format_name")
        .args(["-of", "json"])
        .arg(path)
        .output()
        .map_err(|e| format!("ffprobe 실행 실패: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let last = err.trim().lines().last().unwrap_or("알 수 없는 오류");
        let prefix = format!("{}: ", path.display());
        let last = last.strip_prefix(&prefix).unwrap_or(last);
        return Err(format!("읽을 수 없는 파일입니다 ({last}): {}", path.display()));
    }
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("ffprobe 출력 해석 실패: {e}"))?;
    let st = v["streams"].get(0).ok_or_else(|| format!("영상/이미지 스트림이 없는 파일입니다: {}", path.display()))?;
    let mut w = st["width"].as_u64().unwrap_or(0) as usize;
    let mut h = st["height"].as_u64().unwrap_or(0) as usize;
    if w == 0 || h == 0 {
        return Err(format!("크기를 알 수 없는 파일입니다: {}", path.display()));
    }
    // 휴대폰 영상의 회전 정보: ffmpeg 가 자동 회전하므로 가로세로를 맞춰 둔다.
    if let Some(list) = st["side_data_list"].as_array() {
        for sd in list {
            let rot = sd["rotation"].as_f64().or_else(|| sd["rotation"].as_str().and_then(|s| s.parse().ok()));
            if let Some(r) = rot {
                if (r.abs() as i64) % 180 == 90 {
                    std::mem::swap(&mut w, &mut h);
                }
            }
        }
    }
    let fmt = v["format"]["format_name"].as_str().unwrap_or("");
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let is_image = IMAGE_EXTS.contains(&ext.as_str()) || fmt.split(',').any(|f| IMAGE_FORMATS.contains(&f));
    let mut fps = parse_rate(st.get("avg_frame_rate"));
    if !(fps > 0.0 && fps <= 240.0) {
        fps = parse_rate(st.get("r_frame_rate"));
    }
    if !(fps > 0.0 && fps <= 240.0) {
        fps = 10.0;
    }
    let duration = v["format"]["duration"].as_str().and_then(|s| s.parse().ok());
    Ok(MediaInfo { path: path.to_path_buf(), width: w, height: h, fps, duration, is_image })
}

/// ffmpeg 를 파이프로 띄우고, 읽기 스레드가 프레임을 채널로 넘긴다.
pub struct Decoder {
    child: Child,
    pub frames: Receiver<Vec<u8>>,
    reader: Option<thread::JoinHandle<()>>,
}

impl Decoder {
    pub fn spawn(info: &MediaInfo, width: usize, height: usize, fps: Option<f64>, start: f64,
                 max_frames: Option<usize>) -> Result<Decoder, String> {
        let downscale = width <= info.width && height <= info.height;
        let flags = if downscale { "area" } else { "bicubic" };
        let mut vf = Vec::new();
        if let (Some(f), false) = (fps, info.is_image) {
            vf.push(format!("fps={f}"));
        }
        vf.push(format!("scale={width}:{height}:flags={flags}"));
        vf.push("format=rgba".into());
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-v", "error", "-nostdin"]);
        if start > 0.0 && !info.is_image {
            cmd.args(["-ss", &format!("{start:.3}")]);
        }
        cmd.arg("-i").arg(&info.path).args(["-an", "-vf", &vf.join(",")]);
        let max_frames = if info.is_image { Some(1) } else { max_frames };
        if let Some(n) = max_frames {
            cmd.args(["-frames:v", &n.to_string()]);
        }
        cmd.args(["-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"]);
        cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| format!("ffmpeg 실행 실패: {e}"))?;
        let mut stdout = child.stdout.take().expect("stdout");
        let size = width * height * 4;
        let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(4);
        let reader = thread::spawn(move || loop {
            let mut buf = vec![0u8; size];
            if stdout.read_exact(&mut buf).is_err() || tx.send(buf).is_err() {
                break;
            }
        });
        Ok(Decoder { child, frames: rx, reader: Some(reader) })
    }

    /// 끝까지 읽은 뒤 ffmpeg 가 실패했으면 그 메시지를 돌려준다.
    pub fn finish(mut self) -> Result<(), String> {
        let status = self.child.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            let mut err = String::new();
            if let Some(mut e) = self.child.stderr.take() {
                let _ = e.read_to_string(&mut err);
            }
            let last = err.trim().lines().last().unwrap_or("").to_string();
            return Err(format!("ffmpeg 디코딩 실패: {last}"));
        }
        Ok(())
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(r) = self.reader.take() {
            // 채널이 꽉 차 있으면 스레드가 send 에 묶이므로 비워서 풀어 준다
            while self.frames.try_recv().is_ok() {}
            drop(r);
        }
    }
}

pub fn read_frame(info: &MediaInfo, width: usize, height: usize, at: f64) -> Result<Vec<u8>, String> {
    let dec = Decoder::spawn(info, width, height, None, at, Some(1))?;
    match dec.frames.recv() {
        Ok(f) => Ok(f),
        Err(_) => {
            dec.finish()?;
            Err(format!("{at:.2}s 위치에서 프레임을 읽지 못했습니다: {}", info.path.display()))
        }
    }
}
