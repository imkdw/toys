//! --snapshot 출력이 고정 입력에 대해 바뀌지 않는지 비교한다.
//! 기대값을 갱신하려면: UPDATE_GOLDEN=1 cargo test

use std::path::{Path, PathBuf};
use std::process::Command;

/// 초록 배경 위의 캐릭터 비슷한 그림: 흰 원 + 검은 외곽선 + 눈 두 개 + 분홍 볼.
fn character_rgba(w: usize, h: usize) -> Vec<u8> {
    let mut px = vec![0u8; w * h * 4];
    let (cx, cy, r) = (w as f32 / 2.0, h as f32 * 0.55, h as f32 * 0.38);
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let d = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
            let eye = |ex: f32| ((fx - ex).powi(2) + (fy - (cy - r * 0.15)).powi(2)).sqrt() < r * 0.09;
            let cheek = |ex: f32| ((fx - ex).powi(2) + (fy - (cy + r * 0.12)).powi(2)).sqrt() < r * 0.12;
            let c: [u8; 4] = if d > r {
                [0, 220, 0, 255]
            } else if d > r * 0.92 || eye(cx - r * 0.35) || eye(cx + r * 0.35) {
                [40, 25, 20, 255]
            } else if cheek(cx - r * 0.6) || cheek(cx + r * 0.6) {
                [250, 170, 190, 255]
            } else {
                [255, 250, 240, 255]
            };
            px[(y * w + x) * 4..][..4].copy_from_slice(&c);
        }
    }
    px
}

fn write_pam(path: &Path, w: usize, h: usize, rgba: &[u8]) {
    let mut data = format!("P7\nWIDTH {w}\nHEIGHT {h}\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n").into_bytes();
    data.extend_from_slice(rgba);
    std::fs::write(path, data).unwrap();
}

fn has_ffmpeg() -> bool {
    Command::new("ffmpeg").arg("-version").output().is_ok()
}

fn snapshot(input: &Path, extra: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_ascii-art"))
        .arg(input)
        .args(["--snapshot", "--cols", "48"])
        .args(extra)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

fn check(name: &str, actual: &str) {
    let golden = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() || !golden.exists() {
        std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
        std::fs::write(&golden, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&golden).unwrap();
    assert_eq!(actual, expected, "{name} 스냅샷이 바뀌었습니다. 의도한 변경이면 UPDATE_GOLDEN=1 로 갱신하세요.");
}

#[test]
fn character_snapshots() {
    if !has_ffmpeg() {
        eprintln!("ffmpeg 가 없어 건너뜀");
        return;
    }
    let dir = std::env::temp_dir().join(format!("ascii-art-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("character.pam");
    write_pam(&input, 320, 320, &character_rgba(320, 320));

    let default = snapshot(&input, &[]);
    // 배경이 빠지고 캐릭터만 남아야 한다: 첫 줄은 비고, 가운데 줄에는 글자가 있다
    let lines: Vec<&str> = default.lines().collect();
    assert!(lines[0].trim().is_empty());
    assert!(lines[lines.len() / 2].trim().len() > 20);
    check("character_default.txt", &default);
    check("character_no_edges.txt", &snapshot(&input, &["--no-edges"]));
    check("character_no_bg.txt", &snapshot(&input, &["--bg", "none", "--no-edges"]));

    // 투명 배경도 같은 결과로 빠져야 한다
    let mut rgba = character_rgba(320, 320);
    for p in rgba.chunks_mut(4) {
        if p[..3] == [0, 220, 0] {
            p.copy_from_slice(&[0, 0, 0, 0]);
        }
    }
    let alpha_in = dir.join("character_alpha.pam");
    write_pam(&alpha_in, 320, 320, &rgba);
    let alpha = snapshot(&alpha_in, &["--no-edges"]);
    assert!(alpha.lines().next().unwrap().trim().is_empty());
    check("character_alpha.txt", &alpha);
    let _ = std::fs::remove_dir_all(&dir);
}
