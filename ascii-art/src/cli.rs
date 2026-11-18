//! 인자 해석과 실행.

use std::path::PathBuf;

use crate::media;
use crate::play::{self, View};
use crate::render::{parse_hex, ramp_preset, render, to_text, BgMode, Options};
use crate::term;

const HELP: &str = "\
ascii-art: 사진/영상을 터미널 컬러 ASCII 아트로 보여준다

사용법: ascii-art <파일> [옵션]
  이미지(png, jpg, webp)는 한 번 출력하고, 영상/GIF(mp4, mov, gif)는 반복 재생한다.
  재생 중 Ctrl+C 로 끝낸다.

출력
  --cols N          가로 칸 수 (기본: 터미널에 맞춤)
  --no-color        색 없이 글자만
  --char-aspect F   글자 세로/가로 비율 (기본 2.0)
  --info            위쪽에 정보 줄(파일명, 경과 시간, 글자 수)
  --intro           글자가 위에서 쏟아지며 완성되는 등장 효과

재생
  --fps F           재생 fps (기본: 원본, 최대 30)
  --once            한 번만 재생

화질
  --ramp NAME|문자열  문자 램프: letters(기본), standard, dense, simple, blocks 또는 직접 지정(어두운 것부터)
  --contrast F      대비 (기본 1.0)
  --gamma F         감마 (기본 1.0, 1 보다 크면 밝아짐)
  --brightness F    밝기 더하기 (-1~1, 기본 0)
  --invert          밝기 반전 (밝은 배경 터미널용)
  --bg MODE         배경 제거: auto(기본: 투명, 단색 배경, 외곽선으로 닫힌 캐릭터 순으로 시도),
                    none(화면 전체를 채움), alpha, outline, #rrggbb
  --bg-tol F        배경색 허용 거리 (0~1, 기본 0.12)
  --shade F         입체 음영 세기 (0~1). 기본은 자동: 배경을 지운 그림 1, 배경 있는 장면 0
  --ink-thr F       이보다 어두운 화소를 선(외곽선, 눈, 입)으로 본다 (0~1, 기본 0.36)
  --ink-gap         선을 선 색 글자로 그리지 않고 빈칸으로 비운다
  --no-bold         굵은 글씨 끄기
  --edges           경계 칸에 방향 글자 - / | \\ 를 쓴다 (기본 끔)
  --edge-thr F      --edges 판정 세기 (기본 0.12, 낮을수록 많이)
  --equalize F      밝기 평활화 비율 (0~1). 기본은 자동: 배경을 지운 그림 0, 배경 있는 장면 0.7
  --color-floor F   어두운 칸 전경색 최소 밝기 (0~1, 기본 0.35)

확인용
  --snapshot        한 프레임을 색 없이 텍스트로 출력 (--cols 없으면 80칸)
  --at SEC          --snapshot 에서 뽑을 시각 (기본 0)
";

struct Args {
    file: PathBuf,
    view: View,
    fps: Option<f64>,
    once: bool,
    snapshot: bool,
    at: f64,
}

fn parse(argv: Vec<String>) -> Result<Option<Args>, String> {
    let mut it = argv.into_iter();
    let mut file = None;
    let mut o = Options::default();
    let mut color = true;
    let (mut cols, mut char_aspect, mut intro, mut info) = (None, 2.0, false, false);
    let (mut fps, mut once, mut snapshot, mut at) = (None, false, false, 0.0);

    fn num<T: std::str::FromStr>(flag: &str, v: Option<String>) -> Result<T, String> {
        let v = v.ok_or_else(|| format!("{flag} 뒤에 값이 필요합니다"))?;
        v.parse().map_err(|_| format!("{flag} 값이 올바르지 않습니다: {v}"))
    }

    while let Some(a) = it.next() {
        let (flag, inline) = match a.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f.to_string(), Some(v.to_string())),
            _ => (a.clone(), None),
        };
        let mut val = || inline.clone().or_else(|| it.next());
        match flag.as_str() {
            "-h" | "--help" => return Ok(None),
            "-V" | "--version" => {
                println!("ascii-art {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--cols" => cols = Some(num::<usize>(&flag, val())?.max(4)),
            "--no-color" => color = false,
            "--char-aspect" => char_aspect = num(&flag, val())?,
            "--info" => info = true,
            "--intro" => intro = true,
            "--fps" => fps = Some(num::<f64>(&flag, val())?.clamp(0.1, 120.0)),
            "--once" => once = true,
            "--ramp" => {
                let v: String = val().ok_or("--ramp 뒤에 값이 필요합니다")?;
                let s = ramp_preset(&v).map(str::to_string).unwrap_or(v);
                if s.chars().count() < 2 {
                    return Err("--ramp 는 두 글자 이상이어야 합니다".into());
                }
                o.ramp = s.chars().collect();
            }
            "--contrast" => o.contrast = num(&flag, val())?,
            "--gamma" => o.gamma = num::<f32>(&flag, val())?.max(0.05),
            "--brightness" => o.brightness = num(&flag, val())?,
            "--invert" => o.invert = true,
            "--bg" => {
                let v: String = val().ok_or("--bg 뒤에 값이 필요합니다")?;
                o.bg = match v.as_str() {
                    "auto" => BgMode::Auto,
                    "none" => BgMode::None,
                    "alpha" => BgMode::Alpha,
                    "outline" => BgMode::Outline,
                    c => BgMode::Color(parse_hex(c).ok_or(format!("--bg 는 auto, none, alpha, outline, #rrggbb 중 하나입니다: {c}"))?),
                };
            }
            "--bg-tol" => o.bg_tol = num(&flag, val())?,
            "--ink-thr" => o.ink_thr = num::<f32>(&flag, val())?.clamp(0.0, 1.0),
            "--shade" => o.shade = Some(num::<f32>(&flag, val())?.clamp(0.0, 1.0)),
            "--ink-gap" => o.ink_gap = true,
            "--no-bold" => o.bold = false,
            "--edges" => o.edges = true,
            "--no-edges" => o.edges = false,
            "--edge-thr" => o.edge_thr = num(&flag, val())?,
            "--equalize" => o.equalize = Some(num::<f32>(&flag, val())?.clamp(0.0, 1.0)),
            "--color-floor" => o.color_floor = num::<f32>(&flag, val())?.clamp(0.0, 1.0),
            "--snapshot" => snapshot = true,
            "--at" => at = num(&flag, val())?,
            f if f.starts_with('-') && f.len() > 1 => return Err(format!("모르는 옵션입니다: {f} (--help 참고)")),
            _ => {
                if file.is_some() {
                    return Err("파일은 하나만 넣을 수 있습니다".into());
                }
                file = Some(PathBuf::from(a));
            }
        }
    }
    let file = file.ok_or("파일 경로가 필요합니다 (--help 참고)")?;
    let view = View { opts: o, color, cols, char_aspect, intro, info };
    Ok(Some(Args { file, view, fps, once, snapshot, at }))
}

pub fn run(argv: Vec<String>) -> i32 {
    let args = match parse(argv) {
        Ok(Some(a)) => a,
        Ok(None) => {
            print!("{HELP}");
            return 0;
        }
        Err(e) => {
            eprintln!("ascii-art: {e}");
            return 2;
        }
    };
    term::install_signals();
    let result = (|| {
        let info = media::probe(&args.file)?;
        if args.snapshot {
            let cols = args.view.cols.or(Some(80));
            let layout = play::fit(&info, cols, args.view.char_aspect, 0);
            let (fw, fh) = layout.frame_size();
            let frame = media::read_frame(&info, fw, fh, args.at)?;
            println!("{}", to_text(&render(&frame, fw, fh, &args.view.opts)));
            Ok(())
        } else if info.is_image {
            play::show_image(&info, &args.view)
        } else {
            play::play_video(&info, &args.view, args.fps, args.once)
        }
    })();
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("ascii-art: {e}");
            1
        }
    }
}
