// 5단계: 화별 오프닝 끝과 엔딩 시작을 프레임 단위로 감지해 segments.json 생성
//
// 실행
//   npx tsx src/detect.ts [--pilot | --episodes 1,5,9]   구간 감지 (부분 실행 시 다른 화 항목은 유지)
//   npx tsx src/detect.ts --build-refs                   settings.detect.refs 의 시드로 기준 이미지 생성
//
// 원칙: 본편이 잘리면 안 된다. 확신하지 못한 쪽은 자르지 않고(uncut) flags 에 남긴다.
//
// 프레임 번호는 시간 seek 로 추정하지 않는다. 처음부터 끝까지 디코드한 순서를 그대로 쓰고,
// 경계 프레임은 select=eq(n\,N) 로 별도 디코드해 같은 그림인지 다시 확인한다.
//
// 대상 작품에 맞춘 값은 전부 work/settings.json 의 detect 키에서 읽는다.

import { spawn } from "node:child_process";
import { existsSync, mkdirSync, renameSync, statSync } from "node:fs";
import { availableParallelism } from "node:os";
import { join } from "node:path";
import sharp from "sharp";
import {
  AUDIO_SAMPLE_RATE,
  type Episode,
  type Override,
  type Segment,
  epId,
  loadEpisodes,
  loadOverrides,
  loadSettings,
  paths,
  readJson,
  selectEpisodes,
  sourcePath,
  writeJson,
} from "./config.ts";

// ---------------------------------------------------------------------------
// 설정
// ---------------------------------------------------------------------------

interface Region {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface RefSeed {
  episode: number;
  /** 크레딧 첫 프레임 (육안으로 확인한 값) */
  creditsStart: number;
  /** 크레딧 다음 프레임. 뒤에 카드가 있으면 카드 첫 프레임, 없으면 전체 프레임 수 */
  creditsEnd: number;
}

interface DetectSettings {
  /** 분석용으로 줄인 해상도 */
  analysis: { width: number; height: number };
  /** 원본 규격 검사 */
  spec: {
    width: number;
    height: number;
    fpsNum: number;
    fpsDen: number;
    maxFpsDiff: number;
    /** 목록상 길이와 실제 길이의 허용 차이(초) */
    maxDurationDiffSeconds: number;
  };
  opening: {
    minSeconds: number;
    maxSeconds: number;
    /** 이 값을 넘는 프레임 차이가 처음 나오면 장면 전환으로 본다 */
    cutMinDiff: number;
    /** 타이틀 카드 안에서 허용하는 프레임 차이 */
    staticMaxDiff: number;
    /** 첫 프레임과 전환 직전 프레임의 허용 차이 */
    staticFirstLastMaxDiff: number;
    logoRegion: Region;
    /** 로고가 있다고 보는 정규화 상관계수 하한 */
    logoMinNcc: number;
    /** 전환 뒤 로고가 사라졌다고 보는 상관계수 상한 */
    logoGoneMaxNcc: number;
  };
  credits: {
    minSeconds: number;
    maxSeconds: number;
    /** 기준 이미지와의 거리가 이 값 이하면 크레딧으로 본다 */
    matchMaxDistance: number;
    /** 직전 프레임 거리가 이 값 이하면 하드컷이 아니라 페이드로 본다 */
    fadeMaxDistance: number;
    /** 이 길이 이상 이어져야 크레딧 후보로 본다(초) */
    minRunSeconds: number;
    /** 크레딧 후보 안에서 허용하는 불일치 프레임 수 */
    maxGapFrames: number;
  };
  afterCredits: {
    /** 크레딧 뒤 카드의 최대 길이(초) */
    cardMaxSeconds: number;
    /** 기준 이미지와의 거리가 이 값 이하면 추천 영상 카드로 본다 */
    cardMatchMaxDistance: number;
    /** 크레딧 뒤 프레임 중 카드로 판정되어야 하는 비율 */
    cardMinMatchRatio: number;
  };
  main: { minSeconds: number; maxSeconds: number };
  audio: {
    /** 컷 바로 앞뒤 음량을 재는 구간(ms) */
    edgeMs: number;
    /** 컷 주변 흐름을 재는 구간(ms) */
    contextMs: number;
    /** 이 값 이상이면 소리가 나고 있다고 본다(dBFS) */
    activeMinDb: number;
  };
  refs: {
    /** 크레딧 기준 이미지를 만들 때 쓰는 화 */
    seeds: RefSeed[];
    /** 기준 이미지를 만들 때 프레임을 뽑는 간격 */
    sampleEveryFrames: number;
    /** 샘플 사이 값 차이가 이 값 이하인 픽셀만 비교에 쓴다 */
    stableMaxSpread: number;
  };
}

function loadDetectSettings(): DetectSettings {
  const raw = loadSettings().detect;
  // 필수 값이 빠진 채로 돌아가면 잘못 자를 수 있으므로 시작 전에 전부 확인한다
  const required = [
    "analysis.width",
    "analysis.height",
    "spec.width",
    "spec.height",
    "spec.fpsNum",
    "spec.fpsDen",
    "spec.maxFpsDiff",
    "spec.maxDurationDiffSeconds",
    "opening.minSeconds",
    "opening.maxSeconds",
    "opening.cutMinDiff",
    "opening.staticMaxDiff",
    "opening.staticFirstLastMaxDiff",
    "opening.logoRegion.x",
    "opening.logoRegion.y",
    "opening.logoRegion.width",
    "opening.logoRegion.height",
    "opening.logoMinNcc",
    "opening.logoGoneMaxNcc",
    "credits.minSeconds",
    "credits.maxSeconds",
    "credits.matchMaxDistance",
    "credits.fadeMaxDistance",
    "credits.minRunSeconds",
    "credits.maxGapFrames",
    "afterCredits.cardMaxSeconds",
    "afterCredits.cardMatchMaxDistance",
    "afterCredits.cardMinMatchRatio",
    "main.minSeconds",
    "main.maxSeconds",
    "audio.edgeMs",
    "audio.contextMs",
    "audio.activeMinDb",
    "refs.sampleEveryFrames",
    "refs.stableMaxSpread",
  ];
  const missing = required.filter((key) => {
    let cur: unknown = raw;
    for (const part of key.split(".")) {
      cur = (cur as Record<string, unknown> | undefined)?.[part];
    }
    return typeof cur !== "number" || !Number.isFinite(cur);
  });
  if (missing.length > 0) {
    throw new Error(
      `settings.json 의 detect 에 값이 없습니다: ${missing.join(", ")}`,
    );
  }
  const settings = raw as unknown as DetectSettings;
  if (!Array.isArray(settings.refs.seeds) || settings.refs.seeds.length === 0) {
    throw new Error("settings.json 의 detect.refs.seeds 가 비어 있습니다");
  }
  return settings;
}

// ---------------------------------------------------------------------------
// 외부 명령
// ---------------------------------------------------------------------------

interface RunResult {
  code: number | null;
  stdout: Buffer;
  stderr: string;
}

function run(cmd: string, args: string[]): Promise<RunResult> {
  return new Promise((resolve, reject) => {
    const child = spawn(cmd, args, { stdio: ["ignore", "pipe", "pipe"] });
    const out: Buffer[] = [];
    const err: Buffer[] = [];
    child.stdout.on("data", (d: Buffer) => out.push(d));
    child.stderr.on("data", (d: Buffer) => err.push(d));
    child.on("error", reject);
    child.on("close", (code) =>
      resolve({
        code,
        stdout: Buffer.concat(out),
        stderr: Buffer.concat(err).toString("utf8"),
      }),
    );
  });
}

async function runOk(cmd: string, args: string[]): Promise<Buffer> {
  const r = await run(cmd, args);
  if (r.code !== 0) {
    throw new Error(`${cmd} 실패(${r.code}): ${r.stderr.slice(0, 500)}`);
  }
  return r.stdout;
}

// ---------------------------------------------------------------------------
// 원본 정보와 디코드
// ---------------------------------------------------------------------------

interface Probe {
  width: number;
  height: number;
  fps: { num: number; den: number };
  /** 패킷 수로 센 프레임 수 */
  packetFrames: number;
  videoStartSeconds: number;
  videoDurationSeconds: number;
  /** 표시 순서로 정렬한 패킷 간격 (프레임 단위) */
  minGapFrames: number;
  maxGapFrames: number;
  audio: { codec: string; sampleRate: number; startSeconds: number } | null;
}

function parseRate(rate: string): { num: number; den: number } {
  const [num, den] = rate.split("/").map(Number);
  if (!num || !den) throw new Error(`프레임레이트를 읽지 못했습니다: ${rate}`);
  return { num, den };
}

async function probe(file: string): Promise<Probe> {
  const info = JSON.parse(
    (
      await runOk("ffprobe", [
        "-v",
        "error",
        "-show_entries",
        "stream=codec_type,codec_name,width,height,r_frame_rate,sample_rate,start_time,duration,time_base",
        "-of",
        "json",
        file,
      ])
    ).toString("utf8"),
  ) as { streams: Record<string, string | number | undefined>[] };
  const video = info.streams.find((s) => s.codec_type === "video");
  if (!video) throw new Error("영상 스트림이 없습니다");
  const audio = info.streams.find((s) => s.codec_type === "audio");
  const fps = parseRate(String(video.r_frame_rate));
  const timeBase = parseRate(String(video.time_base));

  const ptsText = (
    await runOk("ffprobe", [
      "-v",
      "error",
      "-select_streams",
      "v:0",
      "-show_entries",
      "packet=pts",
      "-of",
      "csv=p=0",
      file,
    ])
  ).toString("utf8");
  const pts = ptsText
    .split("\n")
    .map((line) => Number.parseInt(line, 10))
    .filter((v) => Number.isFinite(v))
    .sort((a, b) => a - b);
  // 프레임 하나의 길이를 time_base 단위로 바꾼 값
  const ticksPerFrame = (timeBase.den * fps.den) / (timeBase.num * fps.num);
  let minGap = Number.POSITIVE_INFINITY;
  let maxGap = 0;
  for (let i = 1; i < pts.length; i++) {
    const gap = (pts[i] - pts[i - 1]) / ticksPerFrame;
    if (gap < minGap) minGap = gap;
    if (gap > maxGap) maxGap = gap;
  }

  return {
    width: Number(video.width),
    height: Number(video.height),
    fps,
    packetFrames: pts.length,
    videoStartSeconds: Number(video.start_time ?? 0),
    videoDurationSeconds: Number(video.duration ?? 0),
    minGapFrames: round(minGap === Number.POSITIVE_INFINITY ? 0 : minGap, 3),
    maxGapFrames: round(maxGap, 3),
    audio: audio
      ? {
          codec: String(audio.codec_name),
          sampleRate: Number(audio.sample_rate),
          startSeconds: Number(audio.start_time ?? 0),
        }
      : null,
  };
}

/** 분석용 축소 회색 프레임. 필터는 전체 디코드와 경계 확인에서 같아야 한다 */
function grayFilter(cfg: DetectSettings): string {
  const { width, height } = cfg.analysis;
  return `scale=${width}:${height}:flags=area,format=gray`;
}

/** 모든 프레임을 디코드 순서 그대로 받는다. n 번째 프레임이 곧 프레임 번호 n 이다 */
async function decodeAll(
  file: string,
  cfg: DetectSettings,
  threads: number,
): Promise<{ frames: Buffer; count: number; decodeErrors: string }> {
  const r = await run("ffmpeg", [
    "-v",
    "error",
    "-threads",
    String(threads),
    "-i",
    file,
    "-map",
    "0:v:0",
    "-an",
    "-vf",
    grayFilter(cfg),
    "-fps_mode",
    "passthrough",
    "-f",
    "rawvideo",
    "-",
  ]);
  if (r.code !== 0) {
    throw new Error(`전체 디코드 실패(${r.code}): ${r.stderr.slice(0, 500)}`);
  }
  const frameBytes = cfg.analysis.width * cfg.analysis.height;
  if (r.stdout.length % frameBytes !== 0) {
    throw new Error("디코드 결과가 프레임 크기로 나누어떨어지지 않습니다");
  }
  return {
    frames: r.stdout,
    count: r.stdout.length / frameBytes,
    decodeErrors: r.stderr.trim().slice(0, 300),
  };
}

/** 지정한 프레임만 select 로 디코드한다. 번호 오름차순으로 돌려준다 */
async function selectFrames(
  file: string,
  numbers: number[],
  filter: string,
  frameBytes: number,
  threads: number,
): Promise<Map<number, Buffer>> {
  const sorted = [...new Set(numbers)].sort((a, b) => a - b);
  const expr = sorted.map((n) => `eq(n\\,${n})`).join("+");
  const out = await runOk("ffmpeg", [
    "-v",
    "error",
    "-threads",
    String(threads),
    "-i",
    file,
    "-map",
    "0:v:0",
    "-an",
    "-vf",
    `select='${expr}',${filter}`,
    "-fps_mode",
    "passthrough",
    "-frames:v",
    String(sorted.length),
    "-f",
    "rawvideo",
    "-",
  ]);
  if (out.length !== frameBytes * sorted.length) {
    throw new Error(
      `select 결과 크기가 다릅니다: 요청 ${sorted.length}장, ${out.length / frameBytes}장 분량`,
    );
  }
  const map = new Map<number, Buffer>();
  sorted.forEach((n, i) => {
    map.set(n, out.subarray(i * frameBytes, (i + 1) * frameBytes));
  });
  return map;
}

/** 음성 전체를 모노 float 로 받는다 */
async function decodeAudio(file: string): Promise<Float32Array> {
  const out = await runOk("ffmpeg", [
    "-v",
    "error",
    "-i",
    file,
    "-map",
    "0:a:0",
    "-vn",
    "-ac",
    "1",
    "-ar",
    String(AUDIO_SAMPLE_RATE),
    "-f",
    "f32le",
    "-",
  ]);
  const samples = new Float32Array(Math.floor(out.length / 4));
  for (let i = 0; i < samples.length; i++) samples[i] = out.readFloatLE(i * 4);
  return samples;
}

// ---------------------------------------------------------------------------
// 측정
// ---------------------------------------------------------------------------

function round(value: number, digits = 2): number {
  const f = 10 ** digits;
  return Math.round(value * f) / f;
}

/** 프레임 번호를 초로 바꾼다 */
function toSeconds(frame: number, fps: { num: number; den: number }): number {
  return (frame * fps.den) / fps.num;
}

/** 프레임 a 와 a+1 의 평균 절대 차이를 전 구간 계산한다. diffs[i] 는 i-1 -> i 의 차이 */
function frameDiffs(
  frames: Buffer,
  count: number,
  frameBytes: number,
): Float64Array {
  const diffs = new Float64Array(count);
  for (let f = 1; f < count; f++) {
    const a = (f - 1) * frameBytes;
    const b = f * frameBytes;
    let sum = 0;
    for (let i = 0; i < frameBytes; i++) {
      sum += Math.abs(frames[a + i] - frames[b + i]);
    }
    diffs[f] = sum / frameBytes;
  }
  return diffs;
}

function meanAbsDiff(a: Uint8Array, b: Uint8Array): number {
  let sum = 0;
  for (let i = 0; i < a.length; i++) sum += Math.abs(a[i] - b[i]);
  return sum / a.length;
}

interface MaskedRef {
  /** 비교에 쓸 픽셀 위치 */
  index: Uint32Array;
  /** 그 위치의 기준값 */
  value: Uint8Array;
}

/** 프레임별 기준 이미지와의 거리 (마스크 안의 평균 절대 차이) */
function refDistances(
  frames: Buffer,
  from: number,
  to: number,
  frameBytes: number,
  ref: MaskedRef,
): Float64Array {
  const out = new Float64Array(Math.max(0, to - from));
  for (let f = from; f < to; f++) {
    const base = f * frameBytes;
    let sum = 0;
    for (let k = 0; k < ref.index.length; k++) {
      sum += Math.abs(frames[base + ref.index[k]] - ref.value[k]);
    }
    out[f - from] = sum / ref.index.length;
  }
  return out;
}

/** 정규화 상관계수: 밝기와 대비가 달라도 모양이 같으면 1 에 가깝다 */
function ncc(a: Uint8Array, b: Uint8Array): number {
  let ma = 0;
  let mb = 0;
  for (let i = 0; i < a.length; i++) {
    ma += a[i];
    mb += b[i];
  }
  ma /= a.length;
  mb /= b.length;
  let ab = 0;
  let aa = 0;
  let bb = 0;
  for (let i = 0; i < a.length; i++) {
    const da = a[i] - ma;
    const db = b[i] - mb;
    ab += da * db;
    aa += da * da;
    bb += db * db;
  }
  return aa > 0 && bb > 0 ? ab / Math.sqrt(aa * bb) : 0;
}

function rmsDb(samples: Float32Array, from: number, to: number): number | null {
  const a = Math.max(0, from);
  const b = Math.min(samples.length, to);
  if (b <= a) return null;
  let sum = 0;
  for (let i = a; i < b; i++) sum += samples[i] * samples[i];
  const rms = Math.sqrt(sum / (b - a));
  return rms > 0 ? round(20 * Math.log10(rms), 1) : -120;
}

// ---------------------------------------------------------------------------
// 기준 이미지
// ---------------------------------------------------------------------------

const REF_FILES = {
  creditsBg: "credits-bg.png",
  creditsMask: "credits-mask.png",
  cardBg: "card-bg.png",
  cardMask: "card-mask.png",
  logo: "logo.png",
};

interface Refs {
  credits: MaskedRef;
  card: MaskedRef;
  /** 원본 해상도의 로고 영역 */
  logo: Uint8Array;
  creditsPixels: number;
  cardPixels: number;
}

async function writeGrayPng(
  name: string,
  data: Uint8Array,
  width: number,
  height: number,
): Promise<void> {
  mkdirSync(paths.refs, { recursive: true });
  await sharp(Buffer.from(data), { raw: { width, height, channels: 1 } })
    .png()
    .toFile(join(paths.refs, name));
}

async function readGrayPng(
  name: string,
  width: number,
  height: number,
): Promise<Uint8Array> {
  const path = join(paths.refs, name);
  if (!existsSync(path)) {
    throw new Error(
      `기준 이미지가 없습니다: ${path} (npx tsx src/detect.ts --build-refs 로 만드세요)`,
    );
  }
  const { data, info } = await sharp(path)
    .extractChannel(0)
    .raw()
    .toBuffer({ resolveWithObject: true });
  if (info.width !== width || info.height !== height || info.channels !== 1) {
    throw new Error(
      `기준 이미지 크기가 설정과 다릅니다: ${name} ${info.width}x${info.height}`,
    );
  }
  return data;
}

function toMaskedRef(bg: Uint8Array, mask: Uint8Array): MaskedRef {
  const positions: number[] = [];
  for (let i = 0; i < mask.length; i++) if (mask[i] > 127) positions.push(i);
  if (positions.length === 0) throw new Error("기준 마스크가 비어 있습니다");
  return {
    index: Uint32Array.from(positions),
    value: Uint8Array.from(positions.map((i) => bg[i])),
  };
}

async function loadRefs(cfg: DetectSettings): Promise<Refs> {
  const { width, height } = cfg.analysis;
  const credits = toMaskedRef(
    await readGrayPng(REF_FILES.creditsBg, width, height),
    await readGrayPng(REF_FILES.creditsMask, width, height),
  );
  const card = toMaskedRef(
    await readGrayPng(REF_FILES.cardBg, width, height),
    await readGrayPng(REF_FILES.cardMask, width, height),
  );
  const logo = await readGrayPng(
    REF_FILES.logo,
    cfg.opening.logoRegion.width,
    cfg.opening.logoRegion.height,
  );
  return {
    credits,
    card,
    logo,
    creditsPixels: credits.index.length,
    cardPixels: card.index.length,
  };
}

/** 샘플 프레임들의 픽셀별 중앙값과, 샘플 사이에서 값이 거의 변하지 않는 픽셀 마스크 */
function medianAndMask(
  samples: Uint8Array[],
  frameBytes: number,
  maxSpread: number,
): { median: Uint8Array; mask: Uint8Array } {
  const median = new Uint8Array(frameBytes);
  const mask = new Uint8Array(frameBytes);
  const values = new Uint8Array(samples.length);
  for (let i = 0; i < frameBytes; i++) {
    for (let s = 0; s < samples.length; s++) values[s] = samples[s][i];
    values.sort();
    median[i] = values[values.length >> 1];
    // 샘플 중 하나라도 글자나 캐릭터가 지나간 픽셀은 비교에서 뺀다
    const spread = values[values.length - 1] - values[0];
    mask[i] = spread <= maxSpread ? 255 : 0;
  }
  return { median, mask };
}

async function buildRefs(cfg: DetectSettings): Promise<void> {
  const { width, height } = cfg.analysis;
  const frameBytes = width * height;
  const creditSamples: Uint8Array[] = [];
  const cardSamples: Uint8Array[] = [];
  const logoSum = new Float64Array(
    cfg.opening.logoRegion.width * cfg.opening.logoRegion.height,
  );
  let logoCount = 0;
  const every = cfg.refs.sampleEveryFrames;
  const { x, y, width: rw, height: rh } = cfg.opening.logoRegion;

  for (const seed of cfg.refs.seeds) {
    const file = sourcePath(seed.episode);
    const { frames, count } = await decodeAll(file, cfg, 8);
    for (let f = seed.creditsStart; f < seed.creditsEnd; f += every) {
      creditSamples.push(frames.subarray(f * frameBytes, (f + 1) * frameBytes));
    }
    for (let f = seed.creditsEnd; f < count; f += every) {
      cardSamples.push(frames.subarray(f * frameBytes, (f + 1) * frameBytes));
    }
    // 로고는 타이틀 카드 중간쯤의 원본 해상도 프레임에서 뽑는다
    const logoFrame = Math.round(cfg.spec.fpsNum / cfg.spec.fpsDen);
    const crop = await selectFrames(
      file,
      [logoFrame],
      `format=gray,crop=${rw}:${rh}:${x}:${y}`,
      rw * rh,
      8,
    );
    const pixels = crop.get(logoFrame);
    if (!pixels) throw new Error("로고 프레임을 읽지 못했습니다");
    for (let i = 0; i < logoSum.length; i++) logoSum[i] += pixels[i];
    logoCount += 1;
    console.error(`[기준 이미지] ${epId(seed.episode)}화 샘플 수집`);
  }
  if (cardSamples.length === 0) {
    throw new Error("추천 영상 카드가 있는 시드가 없어 카드 기준을 만들 수 없습니다");
  }

  const credits = medianAndMask(
    creditSamples,
    frameBytes,
    cfg.refs.stableMaxSpread,
  );
  const card = medianAndMask(cardSamples, frameBytes, cfg.refs.stableMaxSpread);
  await writeGrayPng(REF_FILES.creditsBg, credits.median, width, height);
  await writeGrayPng(REF_FILES.creditsMask, credits.mask, width, height);
  await writeGrayPng(REF_FILES.cardBg, card.median, width, height);
  await writeGrayPng(REF_FILES.cardMask, card.mask, width, height);
  await writeGrayPng(
    REF_FILES.logo,
    Uint8Array.from(logoSum, (v) => Math.round(v / logoCount)),
    rw,
    rh,
  );
  const ratio = (m: Uint8Array) =>
    round(m.reduce((n, v) => n + (v > 127 ? 1 : 0), 0) / m.length, 3);
  console.error(
    `[기준 이미지] 크레딧 샘플 ${creditSamples.length}장, 비교 영역 ${ratio(credits.mask)} / 카드 샘플 ${cardSamples.length}장, 비교 영역 ${ratio(card.mask)}`,
  );
}

// ---------------------------------------------------------------------------
// 화별 감지
// ---------------------------------------------------------------------------

interface Run {
  start: number;
  /** 마지막 다음 프레임 */
  end: number;
}

/** 거리가 기준 이하인 연속 구간을 찾는다. maxGap 프레임 이하의 불일치는 잇는다 */
function matchRuns(
  distances: Float64Array,
  maxDistance: number,
  maxGap: number,
): Run[] {
  const runs: Run[] = [];
  let current: Run | null = null;
  for (let i = 0; i < distances.length; i++) {
    if (distances[i] > maxDistance) continue;
    if (current && i - current.end <= maxGap) {
      current.end = i + 1;
    } else {
      if (current) runs.push(current);
      current = { start: i, end: i + 1 };
    }
  }
  if (current) runs.push(current);
  return runs;
}

interface OpeningResult {
  candidate: number | null;
  issues: string[];
  evidence: Record<string, unknown>;
}

async function detectOpening(
  file: string,
  frames: Buffer,
  count: number,
  diffs: Float64Array,
  cfg: DetectSettings,
  refs: Refs,
  fps: { num: number; den: number },
  threads: number,
): Promise<OpeningResult> {
  const c = cfg.opening;
  const frameBytes = cfg.analysis.width * cfg.analysis.height;
  const issues: string[] = [];
  const evidence: Record<string, unknown> = {};

  // 앞쪽 일정 구간만 본다. 그 안에 전환이 없으면 타이틀 카드로 인정하지 않는다
  const limit = Math.min(
    count,
    Math.ceil((c.maxSeconds * fps.num) / fps.den) + 1,
  );
  let cut = -1;
  for (let f = 1; f < limit; f++) {
    if (diffs[f] >= c.cutMinDiff) {
      cut = f;
      break;
    }
  }
  if (cut < 0) {
    issues.push(
      `opening_length: ${c.maxSeconds}초 안에 장면 전환이 없음 (기준 차이 ${c.cutMinDiff})`,
    );
    return { candidate: null, issues, evidence };
  }

  const seconds = toSeconds(cut, fps);
  evidence.candidateStartFrame = cut;
  evidence.seconds = round(seconds, 3);
  evidence.cutDiff = round(diffs[cut]);
  if (seconds < c.minSeconds || seconds > c.maxSeconds) {
    issues.push(
      `opening_length: ${round(seconds, 3)}초 (허용 ${c.minSeconds}~${c.maxSeconds})`,
    );
  }

  // 타이틀 카드는 전환 전까지 정지해 있어야 한다
  let maxInside = 0;
  for (let f = 1; f < cut; f++) if (diffs[f] > maxInside) maxInside = diffs[f];
  const firstLast = meanAbsDiff(
    frames.subarray(0, frameBytes),
    frames.subarray((cut - 1) * frameBytes, cut * frameBytes),
  );
  evidence.maxDiffInsideCard = round(maxInside);
  evidence.firstLastDiff = round(firstLast);
  if (maxInside > c.staticMaxDiff || firstLast > c.staticFirstLastMaxDiff) {
    issues.push(
      `title_card: 정지 화면이 아님 (내부 최대 차이 ${round(maxInside)}, 처음/끝 차이 ${round(firstLast)})`,
    );
  }

  // 로고는 카드 전체에 있고 전환 뒤에는 없어야 한다
  const { x, y, width: rw, height: rh } = c.logoRegion;
  const logoFrames = [0, cut >> 1, cut - 1, cut];
  const crops = await selectFrames(
    file,
    logoFrames,
    `format=gray,crop=${rw}:${rh}:${x}:${y}`,
    rw * rh,
    threads,
  );
  const score = (n: number) => round(ncc(crops.get(n) as Buffer, refs.logo), 3);
  const logo = {
    first: score(0),
    middle: score(cut >> 1),
    last: score(cut - 1),
    afterCut: score(cut),
  };
  evidence.logoNcc = logo;
  const minInside = Math.min(logo.first, logo.middle, logo.last);
  if (minInside < c.logoMinNcc) {
    issues.push(
      `title_card: 로고를 찾지 못함 (상관계수 ${minInside}, 하한 ${c.logoMinNcc})`,
    );
  }
  if (logo.afterCut > c.logoGoneMaxNcc) {
    issues.push(
      `title_card: 전환 뒤에도 로고가 남아 있음 (상관계수 ${logo.afterCut}, 상한 ${c.logoGoneMaxNcc})`,
    );
  }

  return { candidate: cut, issues, evidence };
}

/** 프레임마다 기준과의 거리가 이만큼도 줄지 않으면 페이드가 끝났다고 본다. 압축 잡음보다 크게 잡는다 */
const FADE_SETTLED_STEP = 0.2;

interface EndingResult {
  candidate: number | null;
  issues: string[];
  /** 잘라도 본편이 다치지 않지만 사람이 볼 필요가 있는 항목 */
  notices: string[];
  evidence: Record<string, unknown>;
}

function detectEnding(
  frames: Buffer,
  count: number,
  diffs: Float64Array,
  cfg: DetectSettings,
  refs: Refs,
  fps: { num: number; den: number },
): EndingResult {
  const c = cfg.credits;
  const frameBytes = cfg.analysis.width * cfg.analysis.height;
  const issues: string[] = [];
  const notices: string[] = [];
  const evidence: Record<string, unknown> = {};

  const distances = refDistances(frames, 0, count, frameBytes, refs.credits);
  const minRunFrames = Math.ceil((c.minRunSeconds * fps.num) / fps.den);
  const runs = matchRuns(distances, c.matchMaxDistance, c.maxGapFrames).filter(
    (r) => r.end - r.start >= minRunFrames,
  );
  evidence.creditRuns = runs.map((r) => [r.start, r.end]);
  if (runs.length === 0) {
    issues.push(
      `credits_length: 크레딧 배경이 이어지는 구간이 없음 (기준 거리 ${c.matchMaxDistance})`,
    );
    return { candidate: null, issues, notices, evidence };
  }
  if (runs.length > 1) {
    issues.push(
      `credits_length: 크레딧 후보가 ${runs.length}개 (${runs.map((r) => `${r.start}~${r.end}`).join(", ")})`,
    );
  }
  const best = runs.reduce((a, b) =>
    b.end - b.start > a.end - a.start ? b : a,
  );

  // 페이드라면 거리가 더 줄지 않을 때까지 경계를 뒤로 민다. 본편이 비치는 프레임을 남기는 쪽이다
  const isFade =
    best.start > 0 && distances[best.start - 1] <= c.fadeMaxDistance;
  let creditsStart = best.start;
  if (isFade) {
    while (
      creditsStart + 1 < best.end &&
      distances[creditsStart] - distances[creditsStart + 1] > FADE_SETTLED_STEP
    ) {
      creditsStart += 1;
    }
    evidence.fadeFirstMatchFrame = best.start;
  }

  const seconds = toSeconds(best.end - creditsStart, fps);
  evidence.candidateEndFrame = creditsStart;
  evidence.creditsEndFrame = best.end;
  evidence.creditsSeconds = round(seconds, 3);
  evidence.startDistance = round(distances[creditsStart]);
  evidence.prevDistance =
    creditsStart > 0 ? round(distances[creditsStart - 1]) : null;
  evidence.transitionDiff = round(diffs[creditsStart]);
  let maxDistance = 0;
  for (let f = creditsStart; f < best.end; f++) {
    if (distances[f] > maxDistance) maxDistance = distances[f];
  }
  evidence.maxDistanceInCredits = round(maxDistance);
  // 크레딧 밖에서 기준에 가장 가까웠던 프레임. 임계값과의 여유를 전체 화에서 비교하려고 남긴다
  let nearestOutside = Number.POSITIVE_INFINITY;
  for (let f = 0; f < count; f++) {
    if (f >= creditsStart && f < best.end) continue;
    if (distances[f] < nearestOutside) nearestOutside = distances[f];
  }
  evidence.nearestDistanceOutsideCredits = round(nearestOutside);
  if (seconds < c.minSeconds || seconds > c.maxSeconds) {
    issues.push(
      `credits_length: ${round(seconds, 3)}초 (허용 ${c.minSeconds}~${c.maxSeconds})`,
    );
  }

  if (best.start === 0) {
    issues.push("credits_length: 영상 첫 프레임부터 크레딧으로 판정됨");
  } else if (isFade) {
    notices.push(
      `credits_boundary: 하드컷이 아님 (처음 일치한 프레임 ${best.start}, 직전 거리 ${round(distances[best.start - 1])}, 페이드 기준 ${c.fadeMaxDistance}), 경계를 ${creditsStart} 로 둠`,
    );
  }

  // 크레딧 뒤에 남은 내용을 확인한다
  const tail = count - best.end;
  const tailSeconds = toSeconds(tail, fps);
  evidence.afterCredits = { frames: tail, seconds: round(tailSeconds, 3) };
  if (tail > 0) {
    const a = cfg.afterCredits;
    const tailDist = refDistances(frames, best.end, count, frameBytes, refs.card);
    let matched = 0;
    let worst = 0;
    for (const d of tailDist) {
      if (d <= a.cardMatchMaxDistance) matched += 1;
      if (d > worst) worst = d;
    }
    const ratio = matched / tail;
    evidence.afterCredits = {
      frames: tail,
      seconds: round(tailSeconds, 3),
      cardMatchRatio: round(ratio, 3),
      cardMaxDistance: round(worst),
    };
    if (tailSeconds > a.cardMaxSeconds || ratio < a.cardMinMatchRatio) {
      issues.push(
        `after_credits: 크레딧 뒤 ${round(tailSeconds, 2)}초가 추천 영상 카드와 다름 (일치율 ${round(ratio, 3)})`,
      );
    }
  }

  return { candidate: creditsStart, issues, notices, evidence };
}

/**
 * 컷 지점 앞뒤 음량. 소리를 직접 들을 수 없으므로 숫자를 남긴다.
 * 컷 직전 소리가 나고 있으면 before 가, 컷 직후 소리가 나고 있으면 after 가 활성으로 나온다.
 */
function measureCutAudio(
  samples: Float32Array,
  frame: number,
  fps: { num: number; den: number },
  offsetSeconds: number,
  cfg: DetectSettings,
): {
  evidence: Record<string, unknown>;
  beforeActive: boolean;
  afterActive: boolean;
  beforeDb: number | null;
  afterDb: number | null;
} {
  const a = cfg.audio;
  const sr = AUDIO_SAMPLE_RATE;
  const cut = Math.round((toSeconds(frame, fps) - offsetSeconds) * sr);
  const edge = Math.round((a.edgeMs / 1000) * sr);
  const ctx = Math.round((a.contextMs / 1000) * sr);
  const beforeDb = rmsDb(samples, cut - edge, cut);
  const afterDb = rmsDb(samples, cut, cut + edge);

  // 컷에서 가장 가까운 소리까지의 거리. 경계에 얼마나 붙어 있는지 보려고 잰다
  const hop = Math.round(sr / 100);
  const nearestLoudMs = (direction: -1 | 1): number | null => {
    for (let i = 0; i < ctx * 2; i += hop) {
      const from = direction < 0 ? cut - i - hop : cut + i;
      const db = rmsDb(samples, from, from + hop);
      if (db !== null && db >= a.activeMinDb) return round((i / sr) * 1000, 0);
    }
    return null;
  };

  return {
    beforeDb,
    afterDb,
    beforeActive: beforeDb !== null && beforeDb >= a.activeMinDb,
    afterActive: afterDb !== null && afterDb >= a.activeMinDb,
    evidence: {
      cutSeconds: round(toSeconds(frame, fps), 3),
      beforeEdgeDb: beforeDb,
      afterEdgeDb: afterDb,
      beforeContextDb: rmsDb(samples, cut - ctx, cut),
      afterContextDb: rmsDb(samples, cut, cut + ctx),
      nearestLoudBeforeMs: nearestLoudMs(-1),
      nearestLoudAfterMs: nearestLoudMs(1),
      activeMinDb: a.activeMinDb,
      limit:
        "음량만 재므로 대사인지 음악인지, 말이 실제로 끊겼는지는 구분하지 못한다. 최종 판단은 사람이 들어야 한다",
    },
  };
}

function uncutSegment(
  ep: Episode,
  totalFrames: number,
  fps: { num: number; den: number },
  flags: string[],
  evidence: Record<string, unknown>,
): Segment {
  return {
    number: ep.number,
    startFrame: 0,
    endFrame: totalFrames,
    totalFrames,
    fps,
    source: "detected",
    uncut: { opening: true, ending: true },
    evidence,
    flags,
  };
}

async function detectEpisode(
  ep: Episode,
  cfg: DetectSettings,
  refs: Refs,
  threads: number,
): Promise<Segment> {
  const file = sourcePath(ep.number);
  const fallbackFps = { num: cfg.spec.fpsNum, den: cfg.spec.fpsDen };
  let total = 0;
  let fps = fallbackFps;
  try {
    const info = await probe(file);
    total = info.packetFrames;
    fps = info.fps;

    const flags: string[] = [];
    const evidence: Record<string, unknown> = {};

    // 원본 규격
    const fpsValue = fps.num / fps.den;
    const expectedFps = cfg.spec.fpsNum / cfg.spec.fpsDen;
    evidence.source = {
      width: info.width,
      height: info.height,
      fps: `${fps.num}/${fps.den}`,
      packetFrames: info.packetFrames,
      videoStartSeconds: info.videoStartSeconds,
      videoDurationSeconds: info.videoDurationSeconds,
      listedDurationSeconds: ep.duration,
      fileSize: statSync(file).size,
      minGapFrames: info.minGapFrames,
      maxGapFrames: info.maxGapFrames,
      audio: info.audio,
    };
    if (info.width !== cfg.spec.width || info.height !== cfg.spec.height) {
      flags.push(`source_spec: 해상도 ${info.width}x${info.height}`);
    }
    if (Math.abs(fpsValue - expectedFps) > cfg.spec.maxFpsDiff) {
      flags.push(`source_spec: 프레임레이트 ${round(fpsValue, 3)}`);
    }
    if (info.minGapFrames < 0.5 || info.maxGapFrames > 1.5) {
      flags.push(
        `source_spec: 프레임 간격이 일정하지 않음 (${info.minGapFrames}~${info.maxGapFrames} 프레임)`,
      );
    }
    if (!info.audio) flags.push("source_spec: 음성 없음");
    const durationDiff = info.videoDurationSeconds - ep.duration;
    if (Math.abs(durationDiff) > cfg.spec.maxDurationDiffSeconds) {
      flags.push(
        `source_spec: 목록 길이와 ${round(durationDiff, 2)}초 차이 (받다 끊겼을 수 있음)`,
      );
    }

    // 전체 디코드. 패킷 수와 다르면 프레임 번호를 믿을 수 없으므로 자르지 않는다
    const decoded = await decodeAll(file, cfg, threads);
    evidence.decodedFrames = decoded.count;
    if (decoded.decodeErrors) evidence.decodeErrors = decoded.decodeErrors;
    if (decoded.count !== info.packetFrames) {
      flags.push(
        `source_spec: 디코드 프레임 ${decoded.count}, 패킷 ${info.packetFrames} 로 다름`,
      );
      return uncutSegment(ep, decoded.count, fps, flags, evidence);
    }
    if (decoded.decodeErrors) {
      flags.push(`source_spec: 디코드 오류 (${decoded.decodeErrors.slice(0, 80)})`);
    }

    const frameBytes = cfg.analysis.width * cfg.analysis.height;
    const diffs = frameDiffs(decoded.frames, decoded.count, frameBytes);

    const opening = await detectOpening(
      file,
      decoded.frames,
      decoded.count,
      diffs,
      cfg,
      refs,
      fps,
      threads,
    );
    const ending = detectEnding(
      decoded.frames,
      decoded.count,
      diffs,
      cfg,
      refs,
      fps,
    );
    evidence.opening = opening.evidence;
    evidence.ending = ending.evidence;
    flags.push(...opening.issues, ...ending.issues, ...ending.notices);

    // 해당 쪽에 문제가 하나라도 있으면 그쪽은 자르지 않는다
    let startFrame =
      opening.candidate !== null && opening.issues.length === 0
        ? opening.candidate
        : 0;
    let endFrame =
      ending.candidate !== null && ending.issues.length === 0
        ? ending.candidate
        : decoded.count;
    let uncutOpening = startFrame === 0;
    let uncutEnding = endFrame === decoded.count;

    if (startFrame >= endFrame) {
      flags.push(`main_length: 시작 ${startFrame}, 끝 ${endFrame} 로 구간이 없음`);
      startFrame = 0;
      endFrame = decoded.count;
      uncutOpening = true;
      uncutEnding = true;
    }

    // 본편 길이: 양쪽을 다 잘랐을 때만 의미가 있다
    const mainSeconds = toSeconds(endFrame - startFrame, fps);
    evidence.mainSeconds = round(mainSeconds, 3);
    if (
      !uncutOpening &&
      !uncutEnding &&
      (mainSeconds < cfg.main.minSeconds || mainSeconds > cfg.main.maxSeconds)
    ) {
      flags.push(
        `main_length: ${round(mainSeconds, 2)}초 (허용 ${cfg.main.minSeconds}~${cfg.main.maxSeconds})`,
      );
    }

    // 컷 지점 음성
    if (info.audio) {
      const samples = await decodeAudio(file);
      const offset = info.audio.startSeconds;
      const audioEvidence: Record<string, unknown> = {};
      if (!uncutOpening) {
        // 오프닝: 잘려 나가는 앞쪽에 소리가 있으면 본편 소리가 카드 위에서 시작한 것일 수 있다
        const m = measureCutAudio(samples, startFrame, fps, offset, cfg);
        audioEvidence.opening = m.evidence;
        if (m.beforeActive) {
          flags.push(
            `cut_audio: 오프닝 컷 직전에 소리가 있음 (${m.beforeDb}dB, 기준 ${cfg.audio.activeMinDb}dB)`,
          );
        }
      }
      if (!uncutEnding) {
        // 엔딩: 앞쪽 소리가 경계까지 이어지면 말소리가 잘릴 수 있고, 뒤쪽에 소리가 있으면 본편 소리가 넘어간 것일 수 있다
        const m = measureCutAudio(samples, endFrame, fps, offset, cfg);
        audioEvidence.ending = m.evidence;
        if (m.beforeActive || m.afterActive) {
          flags.push(
            `cut_audio: 엔딩 컷 부근에 소리가 있음 (앞 ${m.beforeDb}dB, 뒤 ${m.afterDb}dB, 기준 ${cfg.audio.activeMinDb}dB)`,
          );
        }
      }
      evidence.audio = audioEvidence;
    }

    // 경계 프레임을 별도 디코드해 전체 디코드와 같은 그림인지 확인한다
    const checkFrames = [startFrame, endFrame]
      .flatMap((f) => [f - 1, f])
      .filter((f) => f >= 0 && f < decoded.count);
    const selected = await selectFrames(
      file,
      checkFrames,
      grayFilter(cfg),
      frameBytes,
      threads,
    );
    const crossCheck: Record<string, number> = {};
    let worstCross = 0;
    for (const [n, pixels] of selected) {
      const d = meanAbsDiff(
        pixels,
        decoded.frames.subarray(n * frameBytes, (n + 1) * frameBytes),
      );
      crossCheck[String(n)] = round(d, 3);
      if (d > worstCross) worstCross = d;
    }
    evidence.selectCrossCheck = crossCheck;
    if (worstCross > 0.5) {
      flags.push(
        `source_spec: select 프레임이 전체 디코드와 다름 (차이 ${round(worstCross, 2)}), 양쪽 자르지 않음`,
      );
      startFrame = 0;
      endFrame = decoded.count;
      uncutOpening = true;
      uncutEnding = true;
    }

    // 원본 규격이나 본편 길이가 어긋나면 감지값 자체를 믿을 수 없으므로 자르지 않는다
    if (flags.some((f) => f.startsWith("source_spec:") || f.startsWith("main_length:"))) {
      startFrame = 0;
      endFrame = decoded.count;
      uncutOpening = true;
      uncutEnding = true;
    }

    return {
      number: ep.number,
      startFrame,
      endFrame,
      totalFrames: decoded.count,
      fps,
      source: "detected",
      uncut: { opening: uncutOpening, ending: uncutEnding },
      evidence,
      flags,
    };
  } catch (err) {
    // 한 화의 오류가 전체를 멈추면 안 된다
    const message = err instanceof Error ? err.message : String(err);
    return uncutSegment(ep, total, fps, [`detect_error: ${message.slice(0, 300)}`], {});
  }
}

// ---------------------------------------------------------------------------
// 수동 판정과 저장
// ---------------------------------------------------------------------------

interface DetectedSnapshot {
  startFrame: number;
  endFrame: number;
  uncut: { opening: boolean; ending: boolean };
}

/**
 * 자동 감지값 위에 수동 판정을 덮어쓴다. 감지값을 evidence.detected 에 보관하므로
 * 몇 번을 다시 적용해도 결과가 같고, 수동 판정을 지우면 감지값으로 돌아간다.
 */
function applyOverride(segment: Segment, override: Override | undefined): Segment {
  const evidence = { ...segment.evidence };
  const detected =
    (evidence.detected as DetectedSnapshot | undefined) ?? {
      startFrame: segment.startFrame,
      endFrame: segment.endFrame,
      uncut: segment.uncut,
    };
  evidence.detected = detected;
  delete evidence.override;
  const base: Segment = {
    ...segment,
    startFrame: detected.startFrame,
    endFrame: detected.endFrame,
    uncut: detected.uncut,
    source: "detected",
    evidence,
    flags: segment.flags.filter((f) => !f.startsWith("override_invalid:")),
  };
  if (!override) return base;

  let startFrame = override.startFrame ?? detected.startFrame;
  let endFrame = override.endFrame ?? detected.endFrame;
  let opening = override.startFrame !== undefined ? false : detected.uncut.opening;
  let ending = override.endFrame !== undefined ? false : detected.uncut.ending;
  if (override.uncutOpening === true) {
    startFrame = 0;
    opening = true;
  }
  if (override.uncutEnding === true) {
    endFrame = segment.totalFrames;
    ending = true;
  }
  if (startFrame < 0 || endFrame > segment.totalFrames || startFrame >= endFrame) {
    // 잘못된 수동 판정으로 자르지 않는다
    base.flags = [
      ...base.flags,
      `override_invalid: 범위를 벗어남 (시작 ${startFrame}, 끝 ${endFrame}, 전체 ${segment.totalFrames})`,
    ];
    return base;
  }
  return {
    ...base,
    startFrame,
    endFrame,
    uncut: { opening, ending },
    source: "override",
    evidence: { ...evidence, override },
  };
}

function saveSegments(segments: Map<number, Segment>): void {
  const sorted = [...segments.values()].sort((a, b) => a.number - b.number);
  // 쓰는 중에 멈춰도 기존 파일이 깨지지 않게 임시 파일을 거친다
  const tmp = `${paths.segments}.tmp`;
  writeJson(tmp, sorted);
  renameSync(tmp, paths.segments);
}

// ---------------------------------------------------------------------------
// 실행
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  const cfg = loadDetectSettings();
  if (argv.includes("--build-refs")) {
    await buildRefs(cfg);
    return;
  }

  const refs = await loadRefs(cfg);
  const settings = loadSettings();
  const selected = selectEpisodes(loadEpisodes(), settings, argv);
  if (selected.length === 0) {
    throw new Error("처리할 화가 없습니다. --episodes 값을 확인하세요");
  }
  const available = selected.filter((ep) => {
    if (existsSync(sourcePath(ep.number))) return true;
    console.error(`[건너뜀] ${epId(ep.number)}화: 원본 없음`);
    return false;
  });

  const overrides = loadOverrides();
  const segments = new Map<number, Segment>();
  if (existsSync(paths.segments)) {
    for (const s of readJson<Segment[]>(paths.segments)) {
      segments.set(s.number, s);
    }
  }

  const cpus = availableParallelism();
  const concurrency = Math.max(4, Math.min(6, Math.floor(cpus / 3)));
  const threads = Math.max(2, Math.min(4, Math.floor(cpus / concurrency)));
  console.error(
    `[시작] ${available.length}화, 동시 ${concurrency}개, 화별 스레드 ${threads}개`,
  );

  const queue = [...available];
  const times: number[] = [];
  const startedAt = Date.now();
  const worker = async () => {
    for (;;) {
      const ep = queue.shift();
      if (!ep) return;
      const t0 = Date.now();
      const segment = await detectEpisode(ep, cfg, refs, threads);
      const elapsed = Date.now() - t0;
      times.push(elapsed);
      // 화마다 저장하므로 중간 저장본에도 수동 판정이 들어가야 한다
      segments.set(ep.number, applyOverride(segment, overrides[String(ep.number)]));
      // 오래 걸리는 작업이므로 화마다 저장해 중간에 멈춰도 결과를 남긴다
      saveSegments(segments);
      console.error(
        `[완료] ${epId(ep.number)}화 ${round(elapsed / 1000, 1)}초 감지값 시작=${segment.startFrame} 끝=${segment.endFrame}/${segment.totalFrames} 표시 ${segment.flags.length}개`,
      );
    }
  };
  await Promise.all(Array.from({ length: concurrency }, worker));

  // 부분 실행으로 유지한 화까지 수동 판정을 다시 적용한다
  for (const [number, segment] of segments) {
    segments.set(number, applyOverride(segment, overrides[String(number)]));
  }
  saveSegments(segments);

  const wall = (Date.now() - startedAt) / 1000;
  const mean = times.length
    ? times.reduce((a, b) => a + b, 0) / times.length / 1000
    : 0;
  console.error(
    `[끝] ${available.length}화, 전체 ${round(wall, 1)}초, 화당 평균 ${round(mean, 1)}초 (벽시계 기준 화당 ${round(wall / Math.max(1, available.length), 2)}초)`,
  );
  for (const s of [...segments.values()].sort((a, b) => a.number - b.number)) {
    if (!available.some((e) => e.number === s.number)) continue;
    console.log(
      `${epId(s.number)}\t${s.startFrame}\t${s.endFrame}\t${s.totalFrames}\t${s.source}\t${s.flags.join(" | ") || "-"}`,
    );
  }
}

main().catch((err: unknown) => {
  console.error(err instanceof Error ? err.message : err);
  process.exit(1);
});
