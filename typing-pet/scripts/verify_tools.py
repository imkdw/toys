"""verify.sh 보조 도구 (PLAN 6.5, 6.6). 표준 라이브러리 + Pillow + numpy.

  compare <렌더 폴더> <기준 폴더> [--update]   조합 이미지를 기준 이미지와 픽셀 비교
  capture <창 캡처 png> <오프스크린 png>        실제 창이 오프스크린과 같은지, 배경이 투명한지
  frames <트레이스 jsonl>                       프레임 간격/입력 지연 통계
  cpu <ps 로그>                                  평균 CPU
"""

import json
import shutil
import statistics
import sys
from pathlib import Path

import numpy as np
from PIL import Image

# 조합 1장당 허용 평균 차이 (0~255). 그림을 일부러 바꿨으면 --update 로 새 기준을 확인 받는다.
MEAN_TOL = 1.0
MAX_BAD_FRAC = 0.002  # 차이 > 32 인 픽셀 비율


def load(p: Path) -> np.ndarray:
    return np.array(Image.open(p).convert("RGBA")).astype(np.float32)


def compare(render: Path, base: Path, update: bool) -> int:
    if update or not base.exists():
        if base.exists():
            shutil.rmtree(base)
        shutil.copytree(render, base)
        print(f"기준 이미지 저장: {base} ({len(list(base.glob('*.png')))}장). 사람이 시트를 확인해야 함")
        return 0
    fails = []
    files = sorted(render.glob("*.png"))
    for f in files:
        b = base / f.name
        if not b.exists():
            fails.append(f"{f.name}: 기준 없음")
            continue
        x, y = load(f), load(b)
        if x.shape != y.shape:
            fails.append(f"{f.name}: 크기 다름 {x.shape} vs {y.shape}")
            continue
        d = np.abs(x - y).max(axis=-1)
        mean, bad = float(d.mean()), float((d > 32).mean())
        if mean > MEAN_TOL or bad > MAX_BAD_FRAC:
            fails.append(f"{f.name}: 평균 차이 {mean:.2f}, 큰 차이 픽셀 {bad * 100:.3f}%")
    missing = sorted({p.name for p in base.glob("*.png")} - {p.name for p in files})
    fails += [f"{m}: 렌더 결과 없음" for m in missing]
    print(json.dumps({"compared": len(files), "fails": fails}, ensure_ascii=False))
    return 1 if fails else 0


def capture(win: Path, off: Path) -> int:
    w = load(win)
    o = np.array(Image.open(off).convert("RGBA").resize((w.shape[1], w.shape[0]), Image.LANCZOS)).astype(np.float32)
    transparent = float((w[..., 3] == 0).mean())
    want_transparent = float((o[..., 3] == 0).mean())
    # 둘 다 불투명한 곳의 색 차이 (premultiplied)
    pw = w[..., :3] * w[..., 3:4] / 255
    po = o[..., :3] * o[..., 3:4] / 255
    d = np.abs(pw - po).max(axis=-1)
    res = {
        "window_transparent_frac": round(transparent, 4),
        "offscreen_transparent_frac": round(want_transparent, 4),
        "mean_diff": round(float(d.mean()), 3),
        "p99_diff": round(float(np.percentile(d, 99)), 2),
    }
    ok = abs(transparent - want_transparent) < 0.02 and res["mean_diff"] < 4.0 and transparent > 0.2
    res["pass"] = ok
    print(json.dumps(res))
    return 0 if ok else 1


def slow_frac(trace: Path) -> float:
    gaps = []
    for line in trace.read_text().splitlines():
        try:
            v = json.loads(line)
        except json.JSONDecodeError:
            continue
        if v.get("kind") == "frame" and v.get("gap_us"):
            gaps.append(v["gap_us"])
    if not gaps:
        return 0.0
    p50 = statistics.median(gaps)
    return sum(1 for g in gaps if g > p50 * 1.5) / len(gaps)


def frames(trace: Path, ref: Path | None = None) -> int:
    gaps, draws, lat = [], [], []
    last_input = None
    for line in trace.read_text().splitlines():
        try:
            v = json.loads(line)
        except json.JSONDecodeError:
            continue
        if v.get("kind") == "input":
            last_input = v["t_us"]
        elif v.get("kind") == "frame":
            if v.get("gap_us"):
                gaps.append(v["gap_us"])
            draws.append(v.get("draw_us", 0))
            if last_input is not None:
                lat.append(v["t_us"] - last_input)
                last_input = None
    if not gaps:
        print(json.dumps({"frames": 0}))
        return 0
    p50 = statistics.median(gaps)
    slow = sum(1 for g in gaps if g > p50 * 1.5)
    res = {
        "frames": len(gaps),
        "gap_p50_ms": round(p50 / 1000, 2),
        "slow_frac": round(slow / len(gaps), 4),
        "draw_p99_ms": round(float(np.percentile(draws, 99)) / 1000, 2),
        "input_to_frame_p50_ms": round(statistics.median(lat) / 1000, 2) if lat else None,
        "input_to_frame_p99_ms": round(float(np.percentile(lat, 99)) / 1000, 2) if lat else None,
    }
    # 기준: 1.5배 넘는 프레임 1% 이하, 입력->그리기 지연 p99 가 2프레임 이하.
    # 시스템(컴포지터) 자체가 고르게 못 주는 환경이면, 지우기만 하는 기준 측정(ref)보다 1%p 이상 나쁘지 않아야 한다.
    allowed = 0.01
    if ref is not None and ref.exists():
        res["ref_slow_frac"] = round(slow_frac(ref), 4)
        allowed = max(0.01, res["ref_slow_frac"] + 0.01)
    res["allowed_slow_frac"] = round(allowed, 4)
    res["pass"] = res["slow_frac"] <= allowed and (not lat or res["input_to_frame_p99_ms"] <= 2 * res["gap_p50_ms"] + 2)
    print(json.dumps(res))
    return 0 if res["pass"] else 1


def cpu(log: Path) -> int:
    """ps -o time= 두 번(시작/끝)과 측정 초. CPU 시간 차이 / 경과 시간 = 평균 CPU 사용률."""

    def secs(t: str) -> float:
        parts = [float(x) for x in t.strip().split(":")]
        v = 0.0
        for p in parts:
            v = v * 60 + p
        return v

    lines = [l for l in log.read_text().splitlines() if l.strip()]
    t0, t1, wall = secs(lines[0]), secs(lines[1]), float(lines[2])
    avg = (t1 - t0) / wall * 100
    ok = avg <= 0.5
    print(json.dumps({"cpu_time_s": round(t1 - t0, 3), "wall_s": wall, "avg_cpu_percent": round(avg, 3), "pass": ok}))
    return 0 if ok else 1


def main() -> int:
    cmd, *rest = sys.argv[1:]
    if cmd == "compare":
        return compare(Path(rest[0]), Path(rest[1]), "--update" in rest)
    if cmd == "capture":
        return capture(Path(rest[0]), Path(rest[1]))
    if cmd == "frames":
        return frames(Path(rest[0]), Path(rest[1]) if len(rest) > 1 else None)
    if cmd == "cpu":
        return cpu(Path(rest[0]))
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main())
