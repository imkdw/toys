// 1~4차 검증 중 3차(화별 결과와 원본 대조)와 4차(최종 파일 검사)
//
// 사용:
//   npx tsx src/verify.ts episodes [--pilot | --episodes 1,5,9]   -> results/verify-episodes.json
//   npx tsx src/verify.ts final [--pilot]                         -> results/verify-final.json (pilot 은 verify-pilot.json)
//
// 실패가 있어도 끝까지 검사해서 결과를 기록하고, 마지막에 실패 목록을 출력한 뒤 종료 코드 1 로 끝난다.
// 통과 기준은 아래 DEFAULTS 이고 settings.json 의 verify 항목으로 덮어쓸 수 있다.

import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import sharp from "sharp";
import {
  AUDIO_SAMPLE_RATE,
  encodedAudioPath,
  encodedPath,
  type Episode,
  expectedEpisodes,
  epId,
  loadEpisodes,
  loadSettings,
  paths,
  type Segment,
  type Settings,
  selectEpisodes,
  sourcePath,
  TITLE_REGION,
  TITLE_SECONDS,
  titleCardPath,
} from "./config.ts";
import {
  type Chapter,
  decodeMono,
  extractFrames,
  ffprobeJson,
  ffprobeLines,
  finalPaths,
  loadSegments,
  pool,
  probeVideo,
  readChapters,
  writeJsonAtomic,
} from "./sheets.ts";

// ---------------------------------------------------------------------------
// 기준값 (범용 기본값). settings.json 의 "verify" 로 덮어쓴다
// ---------------------------------------------------------------------------

export interface VerifyConfig {
  /** 비교용으로 줄인 회색조 이미지의 가로 폭 */
  compareWidth: number;
  /** 첫/끝 프레임 SSIM 최소값 (화수 표시 영역 제외) */
  ssimMin: number;
  /** 정답 프레임의 SSIM 이 인접 프레임보다 이만큼까지는 낮아도 통과 (정적 장면 오차 흡수) */
  neighborMargin: number;
  /** 화수 표시 영역 안쪽 SSIM 최소값 */
  titleRegionSsimMin: number;
  /** 화별 WAV 음성 오차 허용(ms). 같은 샘플을 잘라 쓰므로 작게 잡는다 */
  audioToleranceMs: number;
  /** 상호상관 최소 값. 이보다 낮으면 음성이 다른 내용이라고 본다 */
  audioMinCorr: number;
  /** 이 RMS(전체 범위 1.0 기준) 미만이면 무음으로 보고 판단 불가 처리 */
  silenceRms: number;
  /** 상호상관에 쓰는 구간 길이(초)와 탐색 범위(ms), 줄이는 배수 */
  xcorrWindowSec: number;
  xcorrMaxLagMs: number;
  xcorrDecimate: number;
  /** 무음이면 구간을 뒤로 밀어 다시 시도하는 최대 횟수 */
  xcorrTries: number;
  /** 최종 파일 음성 오차 허용(ms)과 최소 상관 (AAC 재인코딩이라 화별보다 느슨) */
  finalAudioToleranceMs: number;
  finalAudioMinCorr: number;
  /** 최종 파일 챕터 첫 프레임 SSIM 최소값 (화수 표시 영역 포함 전체) */
  finalFrameSsimMin: number;
  /** 파일럿이 아닐 때 음성 싱크를 보는 화 수 (처음/끝 포함 균등 분포) */
  finalSyncSamples: number;
  /** PTS 간격 허용 오차(ms)와 챕터 시작 시각 허용 오차(ms) */
  ptsToleranceMs: number;
  chapterToleranceMs: number;
  concurrency: number;
}

const DEFAULTS: VerifyConfig = {
  compareWidth: 640,
  ssimMin: 0.93,
  neighborMargin: 0.001,
  titleRegionSsimMin: 0.85,
  audioToleranceMs: 10,
  audioMinCorr: 0.8,
  silenceRms: 0.003,
  xcorrWindowSec: 1,
  xcorrMaxLagMs: 500,
  xcorrDecimate: 8,
  xcorrTries: 5,
  finalAudioToleranceMs: 40,
  finalAudioMinCorr: 0.5,
  finalFrameSsimMin: 0.9,
  finalSyncSamples: 5,
  ptsToleranceMs: 2,
  chapterToleranceMs: 10,
  concurrency: 6,
};

export type SettingsWithVerify = Settings & { verify?: Partial<VerifyConfig> };

function loadConfig(): { settings: SettingsWithVerify; cfg: VerifyConfig } {
  const settings = loadSettings() as SettingsWithVerify;
  return { settings, cfg: { ...DEFAULTS, ...(settings.verify ?? {}) } };
}

// ---------------------------------------------------------------------------
// 이미지 비교: 회색조로 줄인 뒤 SSIM/PSNR
// ---------------------------------------------------------------------------

interface Gray {
  data: Uint8Array;
  w: number;
  h: number;
  /** 원본 대비 줄인 배수 */
  factor: number;
}
interface Rect {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

function toGray(rgb: Buffer, w: number, h: number, targetW: number): Gray {
  const factor = Math.max(1, Math.round(w / targetW));
  const gw = Math.floor(w / factor);
  const gh = Math.floor(h / factor);
  const data = new Uint8Array(gw * gh);
  const area = factor * factor;
  for (let y = 0; y < gh; y++) {
    for (let x = 0; x < gw; x++) {
      let sum = 0;
      for (let dy = 0; dy < factor; dy++) {
        let p = ((y * factor + dy) * w + x * factor) * 3;
        for (let dx = 0; dx < factor; dx++, p += 3) {
          sum += 0.299 * rgb[p] + 0.587 * rgb[p + 1] + 0.114 * rgb[p + 2];
        }
      }
      data[y * gw + x] = Math.round(sum / area);
    }
  }
  return { data, w: gw, h: gh, factor };
}

const REGION_ORIG = {
  x: TITLE_REGION.x,
  y: TITLE_REGION.y,
  w: TITLE_REGION.width,
  h: TITLE_REGION.height,
};

function regionRect(g: Gray): Rect {
  return {
    x0: Math.floor(REGION_ORIG.x / g.factor),
    y0: Math.floor(REGION_ORIG.y / g.factor),
    x1: Math.ceil((REGION_ORIG.x + REGION_ORIG.w) / g.factor),
    y1: Math.ceil((REGION_ORIG.y + REGION_ORIG.h) / g.factor),
  };
}

interface Similarity {
  ssim: number;
  psnr: number;
}

const C1 = (0.01 * 255) ** 2;
const C2 = (0.03 * 255) ** 2;

/** exclude: 이 영역과 겹치는 창은 뺀다. only: 이 영역 안에 완전히 들어간 창만 쓴다 */
function similarity(a: Gray, b: Gray, o: { exclude?: Rect; only?: Rect } = {}): Similarity {
  if (a.w !== b.w || a.h !== b.h) return { ssim: Number.NaN, psnr: Number.NaN };
  const W = 8;
  const STEP = 4;
  let total = 0;
  let count = 0;
  for (let y = 0; y + W <= a.h; y += STEP) {
    for (let x = 0; x + W <= a.w; x += STEP) {
      const ex = o.exclude;
      if (ex && x < ex.x1 && x + W > ex.x0 && y < ex.y1 && y + W > ex.y0) continue;
      const on = o.only;
      if (on && !(x >= on.x0 && x + W <= on.x1 && y >= on.y0 && y + W <= on.y1)) continue;
      let sa = 0, sb = 0, saa = 0, sbb = 0, sab = 0;
      for (let dy = 0; dy < W; dy++) {
        let p = (y + dy) * a.w + x;
        for (let dx = 0; dx < W; dx++, p++) {
          const va = a.data[p];
          const vb = b.data[p];
          sa += va; sb += vb; saa += va * va; sbb += vb * vb; sab += va * vb;
        }
      }
      const n = W * W;
      const ma = sa / n, mb = sb / n;
      const va = saa / n - ma * ma, vb = sbb / n - mb * mb, cov = sab / n - ma * mb;
      total += ((2 * ma * mb + C1) * (2 * cov + C2)) / ((ma * ma + mb * mb + C1) * (va + vb + C2));
      count++;
    }
  }
  let se = 0;
  let px = 0;
  for (let y = 0; y < a.h; y++) {
    for (let x = 0; x < a.w; x++) {
      if (o.exclude && x >= o.exclude.x0 && x < o.exclude.x1 && y >= o.exclude.y0 && y < o.exclude.y1) continue;
      if (o.only && !(x >= o.only.x0 && x < o.only.x1 && y >= o.only.y0 && y < o.only.y1)) continue;
      const d = a.data[y * a.w + x] - b.data[y * a.w + x];
      se += d * d;
      px++;
    }
  }
  const mse = px > 0 ? se / px : Number.NaN;
  return {
    ssim: count > 0 ? total / count : Number.NaN,
    psnr: mse === 0 ? 99 : 10 * Math.log10((255 * 255) / mse),
  };
}

const r4 = (v: number) => (Number.isFinite(v) ? Math.round(v * 10000) / 10000 : null);
const r1 = (v: number) => Math.round(v * 10) / 10;

interface EdgeResult {
  ssim: number | null;
  psnr: number | null;
  neighbors: { name: string; ssim: number | null }[];
  ok: boolean;
  why?: string;
}

/** 기준 프레임이 SSIM 최소값 이상이고 인접 프레임보다 나쁘지 않은지 본다 */
function judgeEdge(
  enc: Gray,
  ref: Gray | undefined,
  neighbors: { name: string; g: Gray | undefined }[],
  cfg: VerifyConfig,
  exclude?: Rect,
): EdgeResult {
  if (!ref) return { ssim: null, psnr: null, neighbors: [], ok: false, why: "원본 기준 프레임을 읽지 못함" };
  const base = similarity(enc, ref, { exclude });
  const ns = neighbors
    .filter((n): n is { name: string; g: Gray } => !!n.g)
    .map((n) => ({ name: n.name, ssim: similarity(enc, n.g, { exclude }).ssim }));
  const result: EdgeResult = {
    ssim: r4(base.ssim),
    psnr: r1(base.psnr),
    neighbors: ns.map((n) => ({ name: n.name, ssim: r4(n.ssim) })),
    ok: true,
  };
  if (!Number.isFinite(base.ssim)) return { ...result, ok: false, why: "해상도가 달라 비교 불가" };
  if (base.ssim < cfg.ssimMin) {
    return { ...result, ok: false, why: `SSIM ${base.ssim.toFixed(4)} 이 기준 미만` };
  }
  const better = ns.find((n) => n.ssim > base.ssim + cfg.neighborMargin);
  if (better) {
    return { ...result, ok: false, why: `인접 프레임(${better.name}) SSIM ${better.ssim.toFixed(4)} 이 더 높음, 어긋남` };
  }
  return result;
}

// ---------------------------------------------------------------------------
// 음성: WAV 읽기, 상호상관
// ---------------------------------------------------------------------------

interface WavInfo {
  channels: number;
  rate: number;
  bits: number;
  format: number;
  samples: number;
  /** 모노로 섞은 s16 (bits 가 16 일 때만) */
  mono: Int16Array;
}

function readWav(file: string): WavInfo {
  const buf = readFileSync(file);
  if (buf.toString("ascii", 0, 4) !== "RIFF" || buf.toString("ascii", 8, 12) !== "WAVE") {
    throw new Error("WAV 헤더가 아님");
  }
  let pos = 12;
  let fmt: { format: number; channels: number; rate: number; bits: number } | undefined;
  while (pos + 8 <= buf.length) {
    const id = buf.toString("ascii", pos, pos + 4);
    let size = buf.readUInt32LE(pos + 4);
    const body = pos + 8;
    if (id === "fmt ") {
      fmt = {
        format: buf.readUInt16LE(body),
        channels: buf.readUInt16LE(body + 2),
        rate: buf.readUInt32LE(body + 4),
        bits: buf.readUInt16LE(body + 14),
      };
    } else if (id === "data") {
      if (!fmt) throw new Error("fmt 청크가 data 앞에 없음");
      // 스트리밍으로 쓴 WAV 는 크기가 0xFFFFFFFF 일 수 있어 파일 끝까지로 제한한다
      size = Math.min(size, buf.length - body);
      const frameBytes = (fmt.bits / 8) * fmt.channels;
      const samples = Math.floor(size / frameBytes);
      const mono = new Int16Array(fmt.bits === 16 ? samples : 0);
      if (fmt.bits === 16) {
        for (let i = 0; i < samples; i++) {
          let s = 0;
          for (let c = 0; c < fmt.channels; c++) s += buf.readInt16LE(body + (i * fmt.channels + c) * 2);
          mono[i] = Math.round(s / fmt.channels);
        }
      }
      return { ...fmt, samples, mono };
    }
    pos = body + size + (size % 2);
  }
  throw new Error("data 청크를 찾지 못함");
}

function decimate(x: Int16Array, start: number, len: number, d: number): Float32Array {
  const n = Math.floor(len / d);
  const out = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    let s = 0;
    for (let k = 0; k < d; k++) {
      const p = start + i * d + k;
      if (p >= 0 && p < x.length) s += x[p];
    }
    out[i] = s / d / 32768;
  }
  return out;
}

function rms(x: Int16Array, start: number, len: number): number {
  let s = 0;
  let n = 0;
  for (let i = Math.max(0, start); i < Math.min(x.length, start + len); i++, n++) s += x[i] * x[i];
  return n > 0 ? Math.sqrt(s / n) / 32768 : 0;
}

/** sig[i] 가 ref[maxLag + lag + i] 와 가장 닮은 lag 를 찾는다 (정규화 상호상관) */
function xcorr(sig: Float32Array, ref: Float32Array, maxLag: number): { lag: number; corr: number } {
  const n = sig.length;
  let es = 0;
  for (let i = 0; i < n; i++) es += sig[i] * sig[i];
  const pre = new Float64Array(ref.length + 1);
  for (let i = 0; i < ref.length; i++) pre[i + 1] = pre[i] + ref[i] * ref[i];
  const corrs = new Float64Array(2 * maxLag + 1);
  let best = 0;
  for (let o = 0; o <= 2 * maxLag; o++) {
    let dot = 0;
    for (let i = 0; i < n; i++) dot += sig[i] * ref[o + i];
    const er = pre[o + n] - pre[o];
    corrs[o] = es > 0 && er > 0 ? dot / Math.sqrt(es * er) : 0;
    if (corrs[o] > corrs[best]) best = o;
  }
  let frac = 0;
  if (best > 0 && best < corrs.length - 1) {
    const a = corrs[best - 1], b = corrs[best], c = corrs[best + 1];
    const den = a - 2 * b + c;
    if (den !== 0) frac = (0.5 * (a - c)) / den;
  }
  return { lag: best - maxLag + frac, corr: corrs[best] };
}

export interface SyncResult {
  status: "ok" | "lowcorr" | "silent" | "silent-mismatch";
  /** 양수면 음성이 원본보다 앞당겨짐(원본의 더 뒤쪽 소리가 나옴), 음수면 늦음 */
  lagMs: number | null;
  corr: number | null;
  /** 비교에 쓴 구간의 화 안 위치(초) */
  atSec: number;
}

/**
 * 화 안에서 한 구간을 골라 원본과 상호상관으로 어긋남을 잰다.
 * sig 는 sigBase(화 안 샘플 위치)부터 시작하는 44.1kHz 모노, ref 는 원본 전체 모노이고 epRefStart 가 화 시작 샘플이다.
 * side 가 start 면 화 앞쪽부터, end 면 뒤쪽부터 후보 구간을 잡고, 무음이면 안쪽으로 밀어 다시 시도한다.
 */
function measureSync(
  sig: Int16Array,
  sigBase: number,
  epLen: number,
  ref: Int16Array,
  epRefStart: number,
  side: "start" | "end",
  cfg: VerifyConfig,
): SyncResult {
  const rate = AUDIO_SAMPLE_RATE;
  const win = Math.round(cfg.xcorrWindowSec * rate);
  const d = cfg.xcorrDecimate;
  const maxLag = Math.round((cfg.xcorrMaxLagMs / 1000) * rate / d);
  let lastSilent: SyncResult | undefined;
  for (let k = 0; k < cfg.xcorrTries; k++) {
    const p = side === "start" ? k * win : epLen - win - k * win;
    const li = p - sigBase;
    if (p < 0 || p + win > epLen || li < 0 || li + win > sig.length) break;
    const refPos = epRefStart + p;
    const sigRms = rms(sig, li, win);
    const refRms = rms(ref, refPos, win);
    const atSec = p / rate;
    if (sigRms < cfg.silenceRms) {
      lastSilent = {
        status: refRms >= cfg.silenceRms * 3 ? "silent-mismatch" : "silent",
        lagMs: null,
        corr: null,
        atSec,
      };
      if (lastSilent.status === "silent-mismatch") return lastSilent;
      continue;
    }
    const s = decimate(sig, li, win, d);
    const r = decimate(ref, refPos - maxLag * d, win + 2 * maxLag * d, d);
    const { lag, corr } = xcorr(s, r, maxLag);
    return { status: "ok", lagMs: r1((lag * d * 1000) / rate), corr: r4(corr), atSec };
  }
  return lastSilent ?? { status: "silent", lagMs: null, corr: null, atSec: 0 };
}

/** 상관 결과를 기준값과 비교해 실패 사유를 돌려준다. 통과나 판단 불가면 null */
function syncFailure(r: SyncResult, tolMs: number, minCorr: number): string | null {
  if (r.status === "silent-mismatch") return "인코딩 음성은 무음인데 원본은 소리가 있음";
  if (r.status !== "ok") return null;
  if ((r.corr ?? 0) < minCorr) return `상관 ${r.corr} 이 낮아 음성 내용이 다름`;
  if (Math.abs(r.lagMs ?? 0) > tolMs) return `음성 오차 ${r.lagMs}ms (허용 ${tolMs}ms)`;
  return null;
}

/** 원본 음성 전체(필요한 앞부분)를 44.1kHz 모노로 읽고, 영상 시작 대비 음성 시작 오프셋(초)을 돌려준다 */
async function loadSourceAudio(
  number: number,
  upToSec: number,
): Promise<{ pcm: Int16Array; offsetSamples: number } | null> {
  const src = sourcePath(number);
  if (!existsSync(src)) return null;
  const j = await ffprobeJson(["-show_entries", "stream=codec_type,start_time", src]);
  const start = (type: string) => Number(j.streams?.find((s: any) => s.codec_type === type)?.start_time ?? 0);
  if (!j.streams?.some((s: any) => s.codec_type === "audio")) return null;
  const pcm = await decodeMono(src, AUDIO_SAMPLE_RATE, undefined, upToSec + (start("audio") > 0 ? start("audio") : 0));
  // 비디오 시각 t 의 음성 샘플 위치 = (videoStart + t - audioStart) * rate
  const off = start("video") - start("audio");
  return { pcm, offsetSamples: Math.abs(off) < 0.0005 ? 0 : Math.round(off * AUDIO_SAMPLE_RATE) };
}

const frameSec = (fps: { num: number; den: number }, frames: number) => (frames * fps.den) / fps.num;
const sampleAt = (fps: { num: number; den: number }, frame: number) =>
  Math.round(frameSec(fps, frame) * AUDIO_SAMPLE_RATE);

// ---------------------------------------------------------------------------
// 3차: 화별 검사
// ---------------------------------------------------------------------------

interface EpisodeResult {
  number: number;
  label: string;
  ok: boolean;
  failures: string[];
  notes: string[];
  frames?: { expected: number; actual: number };
  first?: EdgeResult;
  last?: EdgeResult;
  title?: Record<string, unknown>;
  audio?: Record<string, unknown>;
}

async function decodeRgba(png: Buffer | string) {
  const { data, info } = await sharp(png).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  return { data, width: info.width, height: info.height };
}

/** 화수 PNG 가 episode.label 로 다시 만든 것과 같은지 본다 */
async function checkTitleCard(ep: Episode, failures: string[]): Promise<Record<string, unknown>> {
  const file = titleCardPath(ep.number);
  if (!existsSync(file)) {
    failures.push("화수 PNG 파일이 없음");
    return { exists: false };
  }
  try {
    const { renderCard, opaqueBounds } = await import("./title-card.ts");
    const expected = await decodeRgba(await renderCard(ep.label));
    const actual = await decodeRgba(file);
    if (expected.width !== actual.width || expected.height !== actual.height) {
      failures.push(`화수 PNG 크기가 label 로 만든 것과 다름 (${actual.width}x${actual.height})`);
      return { exists: true, sameAsLabel: false };
    }
    let maxDiff = 0;
    for (let i = 0; i < expected.data.length; i++) {
      const d = Math.abs(expected.data[i] - actual.data[i]);
      if (d > maxDiff) maxDiff = d;
    }
    const png = await sharp(file).png().toBuffer();
    const b = await opaqueBounds(png);
    const R = TITLE_REGION;
    const inside = !!b && b.x0 >= R.x && b.y0 >= R.y && b.x1 < R.x + R.width && b.y1 < R.y + R.height;
    if (maxDiff > 2) failures.push(`화수 PNG 가 label(${ep.label})로 만든 것과 다름 (최대 차이 ${maxDiff})`);
    if (!inside) failures.push("화수 PNG 가 TITLE_REGION 밖으로 나오거나 비어 있음");
    return { exists: true, sameAsLabel: maxDiff <= 2, maxDiff, insideRegion: inside };
  } catch (e) {
    failures.push(`화수 PNG 검사 중 오류: ${(e as Error).message}`);
    return { exists: true };
  }
}

async function verifyEpisode(ep: Episode, seg: Segment, cfg: VerifyConfig): Promise<EpisodeResult> {
  const failures: string[] = [];
  const notes: string[] = [];
  const res: EpisodeResult = { number: ep.number, label: ep.label, ok: false, failures, notes };
  const S = seg.startFrame;
  const E = seg.endFrame;
  const expected = E - S;
  const fps = seg.fps;
  const enc = encodedPath(ep.number);
  const wavFile = encodedAudioPath(ep.number);
  const src = sourcePath(ep.number);

  res.title = await checkTitleCard(ep, failures);
  if (!existsSync(enc)) {
    failures.push("encoded 영상이 없음");
    return res;
  }
  if (!existsSync(src)) {
    failures.push("원본 영상이 없음");
    return res;
  }

  // 프레임 수와 형식
  const probe = await ffprobeJson(["-count_frames", "-show_entries", "stream=codec_type,codec_name,width,height,nb_read_frames", enc]);
  const v = probe.streams?.find((s: any) => s.codec_type === "video");
  const actual = Number(v?.nb_read_frames ?? 0);
  res.frames = { expected, actual };
  if (actual !== expected) failures.push(`프레임 수 ${actual}, 계산값 ${expected}`);
  if (v?.codec_name !== "h264") failures.push(`영상 코덱이 h264 가 아님 (${v?.codec_name})`);
  if (probe.streams?.some((s: any) => s.codec_type === "audio")) notes.push("encoded 영상에 음성 스트림이 들어 있음");

  // 첫/끝 프레임 대조
  const idxMid = Math.round(2.5 * (fps.num / fps.den));
  const idxAfter = Math.round((TITLE_SECONDS + 0.5) * (fps.num / fps.den));
  const lastIdx = Math.max(0, actual - 1);
  const srcWant = [S - 1, S, S + 1, E - 2, E - 1, E, S + idxMid, S + idxAfter].filter((f) => f >= 0 && f < seg.totalFrames);
  const needRgb = new Set([S, S + idxMid]);
  const W = v?.width as number;
  const H = v?.height as number;
  const srcInfo = await probeVideo(src);
  if (srcInfo.width !== W || srcInfo.height !== H) failures.push(`해상도가 원본과 다름 (${W}x${H}, 원본 ${srcInfo.width}x${srcInfo.height})`);
  const sf = await extractFrames(
    src,
    srcWant,
    { width: srcInfo.width, height: srcInfo.height },
    (raw, f) => ({ g: toGray(raw, srcInfo.width, srcInfo.height, cfg.compareWidth), rgb: needRgb.has(f) ? raw : undefined }),
  );
  const ef = await extractFrames(
    enc,
    [0, idxMid, idxAfter, lastIdx],
    { width: W, height: H },
    (raw) => toGray(raw, W, H, cfg.compareWidth),
  );
  const e0 = ef.get(0);
  const eL = ef.get(lastIdx);
  if (!e0 || !eL) {
    failures.push("encoded 영상에서 첫/끝 프레임을 읽지 못함");
  } else {
    const ex = regionRect(e0);
    res.first = judgeEdge(
      e0,
      sf.get(S)?.g,
      [
        { name: `S-1(${S - 1})`, g: sf.get(S - 1)?.g },
        { name: `S+1(${S + 1})`, g: sf.get(S + 1)?.g },
      ],
      cfg,
      ex,
    );
    res.last = judgeEdge(
      eL,
      sf.get(E - 1)?.g,
      [
        { name: `E(${E})`, g: sf.get(E)?.g },
        { name: `E-2(${E - 2})`, g: E - 2 >= S ? sf.get(E - 2)?.g : undefined },
      ],
      cfg,
      ex,
    );
    if (!res.first.ok) failures.push(`첫 프레임: ${res.first.why}`);
    if (!res.last.ok) failures.push(`끝 프레임: ${res.last.why}`);
  }

  // 화수 표시가 영상에 실제로 들어갔는지: 표시 영역 안쪽만 비교
  if (e0 && sf.get(S)?.rgb && existsSync(titleCardPath(ep.number)) && W === 1920) {
    const png = await decodeRgba(titleCardPath(ep.number));
    const region = regionRect(e0);
    const withCard = (rgb: Buffer) => {
      const out = Buffer.from(rgb);
      for (let y = 0; y < Math.min(REGION_ORIG.y + REGION_ORIG.h, H); y++) {
        for (let x = 0; x < W; x++) {
          const p = (y * W + x) * 3;
          const q = (y * png.width + x) * 4;
          const a = png.data[q + 3] / 255;
          for (let c = 0; c < 3; c++) out[p + c] = Math.round(rgb[p + c] * (1 - a) + png.data[q + c] * a);
        }
      }
      return out;
    };
    const overlayCheck = (idx: number, name: string) => {
      const srcRgb = sf.get(S + idx)?.rgb;
      const encG = ef.get(idx);
      if (!srcRgb || !encG) return null;
      const plain = similarity(encG, sf.get(S + idx)!.g, { only: region }).ssim;
      const comp = similarity(encG, toGray(withCard(srcRgb), W, H, cfg.compareWidth), { only: region }).ssim;
      const ok = comp >= cfg.titleRegionSsimMin && comp >= plain - 0.001;
      if (!ok) failures.push(`${name}: 화수 표시 영역이 화수 PNG 를 얹은 결과와 다름 (얹음 ${comp.toFixed(4)}, 원본 그대로 ${plain.toFixed(4)})`);
      return { composited: r4(comp), plain: r4(plain), ok };
    };
    const shown: Record<string, unknown> = {
      at0: overlayCheck(0, "첫 프레임"),
      atMid: actual > idxMid ? overlayCheck(idxMid, `${(idxMid / (fps.num / fps.den)).toFixed(1)}초 지점`) : null,
    };
    if (actual > idxAfter && sf.get(S + idxAfter) && ef.get(idxAfter)) {
      const plain = similarity(ef.get(idxAfter)!, sf.get(S + idxAfter)!.g, { only: region }).ssim;
      const ok = plain >= cfg.titleRegionSsimMin;
      if (!ok) failures.push(`${TITLE_SECONDS}초 이후에도 화수 표시가 남아 있음 (원본 대비 영역 SSIM ${plain.toFixed(4)})`);
      shown.afterTitle = { ssim: r4(plain), ok };
    }
    res.title = { ...res.title, overlay: shown };
  } else if (W !== 1920) {
    notes.push("해상도가 1920 이 아니라 화수 표시 영역 합성 검사를 건너뜀");
  }

  // 음성
  const audio: Record<string, unknown> = {};
  res.audio = audio;
  if (!existsSync(wavFile)) {
    failures.push("encoded WAV 가 없음");
    return finish(res);
  }
  try {
    const wav = readWav(wavFile);
    const expectedSamples = sampleAt(fps, expected);
    const wavSec = wav.samples / wav.rate;
    const videoSec = frameSec(fps, actual);
    const frameDur = frameSec(fps, 1);
    Object.assign(audio, {
      format: `${wav.format === 1 ? "PCM" : `fmt${wav.format}`} ${wav.bits}bit ${wav.rate}Hz ${wav.channels}ch`,
      samples: wav.samples,
      expectedSamples,
      sampleDiff: wav.samples - expectedSamples,
      wavSec: r4(wavSec),
      videoSec: r4(videoSec),
    });
    if (wav.format !== 1 || wav.bits !== 16 || wav.rate !== AUDIO_SAMPLE_RATE || wav.channels !== 2) {
      failures.push(`WAV 형식이 PCM s16le ${AUDIO_SAMPLE_RATE}Hz 스테레오가 아님`);
    }
    if (Math.abs(wavSec - videoSec) > frameDur) {
      failures.push(`WAV 길이 ${wavSec.toFixed(3)}초, 영상 ${videoSec.toFixed(3)}초 (1프레임 ${frameDur.toFixed(3)}초 초과)`);
    }
    if (wav.samples !== expectedSamples) notes.push(`WAV 샘플 수가 계산값과 ${wav.samples - expectedSamples} 다름`);

    const source = await loadSourceAudio(ep.number, frameSec(fps, E) + 2);
    if (!source) {
      notes.push("원본 음성을 읽지 못해 음성 대조를 건너뜀");
    } else if (wav.bits === 16) {
      const epRefStart = sampleAt(fps, S) + source.offsetSamples;
      const start = measureSync(wav.mono, 0, wav.samples, source.pcm, epRefStart, "start", cfg);
      const end = measureSync(wav.mono, 0, wav.samples, source.pcm, epRefStart, "end", cfg);
      audio.sourceOffsetSamples = source.offsetSamples;
      audio.start = start;
      audio.end = end;
      for (const [name, r] of [["앞", start], ["뒤", end]] as const) {
        const why = syncFailure(r, cfg.audioToleranceMs, cfg.audioMinCorr);
        if (why) failures.push(`음성 ${name}쪽: ${why}`);
        if (r.status === "silent") notes.push(`음성 ${name}쪽 판단 불가(무음)`);
      }
    }
  } catch (e) {
    failures.push(`음성 검사 중 오류: ${(e as Error).message}`);
  }
  return finish(res);
}

function finish(res: EpisodeResult): EpisodeResult {
  res.ok = res.failures.length === 0;
  return res;
}

async function episodesMain(argv: string[]): Promise<void> {
  const { settings, cfg } = loadConfig();
  const targets = selectEpisodes(loadEpisodes(), settings, argv).sort((a, b) => a.number - b.number);
  if (targets.length === 0) throw new Error("검사할 화가 없습니다. 인자를 확인하세요");
  const segs = loadSegments();
  const results: EpisodeResult[] = new Array(targets.length);
  let done = 0;
  await pool(targets, cfg.concurrency, async (ep, i) => {
    const seg = segs.get(ep.number);
    try {
      results[i] = seg
        ? finish(await verifyEpisode(ep, seg, cfg))
        : { number: ep.number, label: ep.label, ok: false, failures: ["segments.json 에 구간 정보가 없음"], notes: [] };
    } catch (e) {
      results[i] = { number: ep.number, label: ep.label, ok: false, failures: [`검사 중 오류: ${(e as Error).message}`], notes: [] };
    }
    done++;
    if (done % 10 === 0 || done === targets.length) console.log(`화별 검사 ${done}/${targets.length}`);
  });
  // 일부 화만 검사해도 다른 화의 기록이 사라지지 않게 병합한다
  const out = join(paths.results, "verify-episodes.json");
  const verifiedAt = new Date().toISOString();
  const merged = new Map<number, EpisodeResult & { verifiedAt?: string }>(
    existsSync(out)
      ? (JSON.parse(readFileSync(out, "utf8")) as { episodes: (EpisodeResult & { verifiedAt?: string })[] }).episodes.map((e) => [e.number, e])
      : [],
  );
  for (const r of results) merged.set(r.number, { ...r, verifiedAt });
  const all = [...merged.values()].sort((a, b) => a.number - b.number);
  const failedAll = all.filter((r) => !r.ok);
  writeJsonAtomic(out, {
    generatedAt: verifiedAt,
    config: cfg,
    summary: { total: all.length, passed: all.length - failedAll.length, failed: failedAll.map((r) => r.number) },
    episodes: all,
  });
  const failed = results.filter((r) => !r.ok);
  console.log(`결과 저장: ${out}`);
  report(failed.map((r) => ({ name: `${epId(r.number)}화`, failures: r.failures })));
}

function report(failed: { name: string; failures: string[] }[]): void {
  if (failed.length === 0) {
    console.log("실패 없음");
    return;
  }
  console.error(`실패 ${failed.length}건`);
  for (const f of failed) for (const m of f.failures) console.error(`  ${f.name}: ${m}`);
  process.exitCode = 1;
}

// ---------------------------------------------------------------------------
// 4차: 최종 파일 검사
// ---------------------------------------------------------------------------

/** 전체를 -xerror 로 디코드하면서 읽은 프레임 수도 센다 */
function decodeAll(file: string): Promise<{ ok: boolean; frames: number; errors: string }> {
  return new Promise((resolve) => {
    const child = spawn("ffmpeg", ["-v", "error", "-xerror", "-stats", "-nostdin", "-i", file, "-f", "null", "-"], {
      stdio: ["ignore", "ignore", "pipe"],
    });
    let err = "";
    child.stderr.on("data", (d) => (err += d));
    child.on("close", (code) => {
      const m = [...err.matchAll(/frame=\s*(\d+)/g)].pop();
      const errors = err
        .split(/[\r\n]+/)
        .filter((l) => l.trim() && !/^\s*(frame|size)=/.test(l) && !/^\[?out#/.test(l))
        .join("\n");
      resolve({ ok: code === 0, frames: m ? Number(m[1]) : -1, errors });
    });
  });
}

async function finalMain(argv: string[]): Promise<void> {
  const pilot = argv.includes("--pilot");
  const { settings, cfg } = loadConfig();
  const fp = finalPaths(pilot);
  const out = join(paths.results, pilot ? "verify-pilot.json" : "verify-final.json");
  const failures: string[] = [];
  const checks: Record<string, unknown> = {};
  const fail = (m: string) => failures.push(m);
  const save = () => {
    writeJsonAtomic(out, {
      generatedAt: new Date().toISOString(),
      pilot,
      ok: failures.length === 0,
      failures,
      config: cfg,
      checks,
    });
    console.log(`결과 저장: ${out}`);
    report(failures.length ? [{ name: "최종 파일", failures }] : []);
  };

  if (!existsSync(fp.video) || !existsSync(fp.chapters)) {
    fail(`최종 파일이나 chapters.json 이 없음: ${fp.video}, ${fp.chapters}`);
    return save();
  }

  // 기대값: 선택한 화 중 구간 정보가 있는 화
  const segs = loadSegments();
  const wanted = selectEpisodes(loadEpisodes(), settings, pilot ? ["--pilot"] : []).sort((a, b) => a.number - b.number);
  // 수동 제외한 화만 빠질 수 있다. 그 밖에 구간이 없는 화는 실패다
  const expected = pilot ? wanted : expectedEpisodes(wanted);
  for (const e of expected) {
    if (!segs.has(e.number)) fail(`${e.number}화: segments.json 에 구간 정보가 없음`);
  }
  const eps = expected.filter((e) => segs.has(e.number));
  if (eps.length === 0) {
    fail("검사할 화가 없음");
    return save();
  }
  const fps = segs.get(eps[0].number)!.fps;
  const expFrames = eps.map((e) => segs.get(e.number)!.endFrame - segs.get(e.number)!.startFrame);
  const sumFrames = expFrames.reduce((a, b) => a + b, 0);
  const frameDur = frameSec(fps, 1);

  let chapters: Chapter[] = [];
  try {
    chapters = readChapters(fp.chapters);
  } catch (e) {
    fail(`chapters.json 을 읽지 못함: ${(e as Error).message}`);
    return save();
  }

  // 챕터 파일과 기대값
  const chapterIssues: string[] = [];
  if (chapters.length !== eps.length) chapterIssues.push(`챕터 파일 수 ${chapters.length}, 기대 ${eps.length}`);
  let acc = 0;
  eps.forEach((e, i) => {
    const c = chapters[i];
    if (!c) return;
    if (c.number !== e.number) chapterIssues.push(`${i + 1}번째 챕터 화수 ${c.number}, 기대 ${e.number}`);
    if (c.label !== e.label) chapterIssues.push(`${e.number}화 라벨 "${c.label}", 기대 "${e.label}"`);
    if (c.frameCount !== expFrames[i]) chapterIssues.push(`${e.number}화 프레임 수 ${c.frameCount}, 기대 ${expFrames[i]}`);
    if (c.startFrame !== acc) chapterIssues.push(`${e.number}화 시작 프레임 ${c.startFrame}, 앞 화들의 합 ${acc}`);
    acc += expFrames[i];
  });
  checks.chaptersFile = { count: chapters.length, expected: eps.length, issues: chapterIssues };
  chapterIssues.forEach((m) => fail(`챕터 파일: ${m}`));

  // 디코드 오류와 총 프레임
  const dec = await decodeAll(fp.video);
  checks.decode = { ok: dec.ok, frames: dec.frames, expectedFrames: sumFrames, errors: dec.errors.slice(0, 2000) };
  if (!dec.ok) fail(`디코드 오류: ${dec.errors.split("\n")[0] || "ffmpeg 실패"}`);
  if (dec.frames !== sumFrames) fail(`총 프레임 ${dec.frames}, 화별 합 ${sumFrames}`);

  // 스트림 정보, 길이, PTS
  const info = await probeVideo(fp.video);
  const streams = await ffprobeJson(["-show_entries", "stream=codec_type,duration,start_time", "-show_entries", "format=duration", fp.video]);
  const aStream = streams.streams?.find((s: any) => s.codec_type === "audio");
  const vStream = streams.streams?.find((s: any) => s.codec_type === "video");
  const videoSec = frameSec(fps, sumFrames);
  const audioSec = Number(aStream?.duration ?? streams.format?.duration ?? Number.NaN);
  const lengths = {
    expectedSec: r4(videoSec),
    videoStreamSec: r4(Number(vStream?.duration ?? Number.NaN)),
    audioStreamSec: r4(audioSec),
    formatSec: r4(Number(streams.format?.duration ?? Number.NaN)),
  };
  checks.duration = lengths;
  if (!aStream) fail("최종 파일에 음성 스트림이 없음");
  else if (!(Math.abs(audioSec - videoSec) <= frameDur)) fail(`음성 길이 ${audioSec.toFixed(3)}초, 화별 합 ${videoSec.toFixed(3)}초 (1프레임 초과)`);
  const vdur = Number(vStream?.duration ?? Number.NaN);
  if (Number.isFinite(vdur) && Math.abs(vdur - videoSec) > frameDur) fail(`영상 길이 ${vdur.toFixed(3)}초, 화별 합 ${videoSec.toFixed(3)}초`);
  if (Math.abs((info.fps.num / info.fps.den) / (fps.num / fps.den) - 1) > 1e-4) {
    fail(`프레임레이트 ${info.fps.num}/${info.fps.den}, 기대 ${fps.num}/${fps.den}`);
  }

  const ptsTicks = (await ffprobeLines(["-select_streams", "v:0", "-show_entries", "packet=pts", fp.video])).map(Number);
  const tb = info.timeBase.num / info.timeBase.den;
  let ptsTimes: number[] = [];
  if (ptsTicks.length === 0 || ptsTicks.some((p) => !Number.isFinite(p))) {
    fail("패킷 PTS 를 읽지 못함");
  } else {
    ptsTimes = ptsTicks.sort((a, b) => a - b).map((p) => p * tb);
    const tol = cfg.ptsToleranceMs / 1000;
    let worstStep = 0;
    let worstDrift = 0;
    let badAt = -1;
    for (let i = 0; i < ptsTimes.length; i++) {
      const drift = Math.abs(ptsTimes[i] - ptsTimes[0] - i * frameDur);
      if (drift > worstDrift) worstDrift = drift;
      if (i > 0) {
        const step = Math.abs(ptsTimes[i] - ptsTimes[i - 1] - frameDur);
        if (step > worstStep) worstStep = step;
        if (badAt < 0 && step > tol) badAt = i;
      }
    }
    checks.pts = {
      packets: ptsTimes.length,
      firstSec: r4(ptsTimes[0]),
      worstStepErrMs: r4(worstStep * 1000),
      worstDriftMs: r4(worstDrift * 1000),
      toleranceMs: cfg.ptsToleranceMs,
    };
    if (badAt >= 0) fail(`PTS 간격이 균일하지 않음 (프레임 ${badAt}, 오차 ${(worstStep * 1000).toFixed(2)}ms)`);
    else if (worstDrift > tol) fail(`PTS 가 누적으로 어긋남 (${(worstDrift * 1000).toFixed(2)}ms)`);
    if (ptsTimes.length !== sumFrames) fail(`패킷 수 ${ptsTimes.length}, 화별 합 ${sumFrames}`);
  }

  // 챕터 메타데이터
  const meta = await ffprobeJson(["-show_chapters", fp.video]);
  const mchapters: { start_time: string; tags?: { title?: string } }[] = meta.chapters ?? [];
  const metaIssues: string[] = [];
  if (mchapters.length !== eps.length) metaIssues.push(`파일 챕터 수 ${mchapters.length}, 기대 ${eps.length}`);
  const ctol = cfg.chapterToleranceMs / 1000;
  eps.forEach((e, i) => {
    const m = mchapters[i];
    if (!m) return;
    // 기대 시작 프레임은 chapters.json 이 아니라 화별 프레임 수의 누적합으로 구한다
    const startFrame = expFrames.slice(0, i).reduce((a, b) => a + b, 0);
    const t = Number(m.start_time);
    const expectT = ptsTimes[startFrame] ?? frameSec(fps, startFrame);
    if (Math.abs(t - expectT) > ctol) metaIssues.push(`${e.number}화 챕터 시작 ${t.toFixed(3)}초, 첫 프레임 ${expectT.toFixed(3)}초`);
    if (m.tags?.title !== e.label) metaIssues.push(`${e.number}화 챕터 제목 "${m.tags?.title}", 기대 "${e.label}"`);
  });
  checks.chapterMetadata = { count: mchapters.length, issues: metaIssues };
  metaIssues.forEach((m) => fail(`챕터: ${m}`));

  // 챕터 시작 프레임 == 해당 화 encoded 첫 프레임 (화수 표시 포함 전체)
  const W = info.width;
  const H = info.height;
  const want = chapters.flatMap((c) => [c.startFrame - 1, c.startFrame, c.startFrame + 1]);
  const fin = await extractFrames(fp.video, want, { width: W, height: H, fps }, (raw) => toGray(raw, W, H, cfg.compareWidth));
  const frameResults: Record<string, unknown>[] = new Array(chapters.length);
  await pool(chapters, cfg.concurrency, async (c, i) => {
    const entry: Record<string, unknown> = { number: c.number };
    frameResults[i] = entry;
    const enc = encodedPath(c.number);
    if (!existsSync(enc)) {
      entry.ok = false;
      fail(`${c.number}화 챕터 프레임: encoded 영상이 없음`);
      return;
    }
    const first = [...(await extractFrames(enc, [0], { width: W, height: H }, (raw) => toGray(raw, W, H, cfg.compareWidth))).values()][0];
    const j = judgeEdge(
      first,
      fin.get(c.startFrame),
      [
        { name: `앞 프레임(${c.startFrame - 1})`, g: fin.get(c.startFrame - 1) },
        { name: `다음 프레임(${c.startFrame + 1})`, g: fin.get(c.startFrame + 1) },
      ],
      { ...cfg, ssimMin: cfg.finalFrameSsimMin },
    );
    Object.assign(entry, j);
    if (!j.ok) fail(`${c.number}화 챕터 시작 프레임: ${j.why}`);
  });
  checks.chapterFrames = frameResults;

  // 음성 싱크
  const syncResults: Record<string, unknown>[] = [];
  const picks = new Set<number>();
  if (pilot) chapters.forEach((_, i) => picks.add(i));
  else {
    const n = Math.min(chapters.length, Math.max(3, cfg.finalSyncSamples));
    for (let k = 0; k < n; k++) picks.add(Math.round((k * (chapters.length - 1)) / Math.max(1, n - 1)));
  }
  let determined = 0;
  for (const i of [...picks].sort((a, b) => a - b)) {
    const c = chapters[i];
    const seg = segs.get(c.number);
    const entry: Record<string, unknown> = { number: c.number };
    syncResults.push(entry);
    if (!seg) {
      entry.note = "구간 정보 없음";
      continue;
    }
    try {
      const source = await loadSourceAudio(c.number, frameSec(seg.fps, seg.endFrame) + 2);
      if (!source) {
        entry.note = "원본 음성을 읽지 못함";
        continue;
      }
      const epLen = sampleAt(fps, c.frameCount);
      const t0 = ptsTimes[c.startFrame] ?? frameSec(fps, c.startFrame);
      const regionSec = Math.min(frameSec(fps, c.frameCount), (cfg.xcorrTries + 1) * cfg.xcorrWindowSec);
      const head = await decodeMono(fp.video, AUDIO_SAMPLE_RATE, t0, regionSec);
      const tailFrom = Math.max(0, frameSec(fps, c.frameCount) - regionSec);
      const tail = await decodeMono(fp.video, AUDIO_SAMPLE_RATE, t0 + tailFrom, regionSec);
      const epRefStart = sampleAt(seg.fps, seg.startFrame) + source.offsetSamples;
      const start = measureSync(head, 0, epLen, source.pcm, epRefStart, "start", cfg);
      const end = measureSync(tail, Math.round(tailFrom * AUDIO_SAMPLE_RATE), epLen, source.pcm, epRefStart, "end", cfg);
      entry.start = start;
      entry.end = end;
      for (const [name, r] of [["앞", start], ["뒤", end]] as const) {
        if (r.status === "ok") determined++;
        const why = syncFailure(r, cfg.finalAudioToleranceMs, cfg.finalAudioMinCorr);
        if (why) fail(`${c.number}화 음성 싱크 ${name}쪽: ${why}`);
      }
    } catch (e) {
      entry.note = `오류: ${(e as Error).message}`;
      fail(`${c.number}화 음성 싱크 검사 중 오류: ${(e as Error).message}`);
    }
  }
  checks.audioSync = { toleranceMs: cfg.finalAudioToleranceMs, determinedWindows: determined, episodes: syncResults };
  if (determined === 0) fail("음성 싱크를 판단할 수 있는 구간이 하나도 없음");
  save();
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const [cmd, ...rest] = process.argv.slice(2);
  try {
    if (cmd === "episodes") await episodesMain(rest);
    else if (cmd === "final") await finalMain(rest);
    else {
      console.error("사용: npx tsx src/verify.ts episodes|final [--pilot | --episodes 1,5,9]");
      process.exit(2);
    }
  } catch (e) {
    console.error(`오류: ${(e as Error).message}`);
    process.exit(1);
  }
}
