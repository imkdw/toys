// 7단계: 화별 인코딩
//
// 화별 결과는 2개다.
// - encodedPath(n): H.264 영상만 (음성 없음)
// - encodedAudioPath(n): PCM s16le WAV
// AAC 를 화별로 만들어 이으면 경계마다 priming/패딩으로 싱크가 누적해서 틀어지므로,
// 음성은 concat.ts 에서 전체를 이어 붙인 뒤 AAC 로 한 번만 인코딩한다.
//
// 사용법: npx tsx src/encode.ts [--pilot | --episodes 1,5,9]

import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, statSync } from "node:fs";
import {
  AUDIO_SAMPLE_RATE,
  encodedAudioPath,
  encodedPath,
  epId,
  loadEpisodes,
  loadOverrides,
  loadSettings,
  paths,
  readJson,
  type Segment,
  selectEpisodes,
  sourcePath,
  TITLE_SECONDS,
  titleCardPath,
  writeJson,
} from "./config.ts";
import { join } from "node:path";

/** settings.encode 에서 읽는 선택 값. 없으면 기본값을 쓴다 */
export interface EncodeOptions {
  crf: number;
  preset: string;
  /** 동시에 돌리는 화 수 */
  jobs: number;
  /** 화 하나의 x264 스레드 수 */
  threads: number;
  /** B프레임 수. 이어 붙일 때 PTS 문제가 없는 값을 시험으로 골랐다 */
  bframes: number;
}

export const DEFAULT_ENCODE_OPTIONS: EncodeOptions = {
  crf: 18,
  preset: "slow",
  jobs: 4,
  threads: 4,
  bframes: 0,
};

/** 음성 경계 클릭 방지용 페이드 길이(초) */
const FADE_SECONDS = 0.005;
const MAX_ATTEMPTS = 3;

export type EncodeStatus = "ok" | "failed" | "skipped";

export interface EpisodeResult {
  number: number;
  status: EncodeStatus;
  /** skipped 일 때는 건너뛴 이유, failed 일 때는 오류 */
  error?: string;
  /** 이번 실행에서 건너뛰었지만 이전 결과가 유효한 경우 true */
  cached?: boolean;
  frames?: number;
  samples?: number;
  width?: number;
  height?: number;
  fps?: { num: number; den: number };
  seconds?: number;
  attempts?: number;
  finishedAt?: string;
}

export interface EncodeResults {
  updatedAt: string;
  episodes: Record<string, EpisodeResult>;
}

export const encodeResultsPath = () => join(paths.results, "encode.json");

export function loadEncodeResults(): EncodeResults {
  const p = encodeResultsPath();
  return existsSync(p)
    ? readJson<EncodeResults>(p)
    : { updatedAt: "", episodes: {} };
}

/** 프레임 수를 음성 샘플 수로 바꾼다. 프레임 길이는 den/num 초 */
export function framesToSamples(
  frames: number,
  fps: { num: number; den: number },
): number {
  return Math.round((frames * fps.den * AUDIO_SAMPLE_RATE) / fps.num);
}

function run(cmd: string, args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    execFile(
      cmd,
      args,
      { maxBuffer: 64 * 1024 * 1024 },
      (err, stdout, stderr) => {
        if (err) {
          const tail = String(stderr).split("\n").slice(-12).join("\n");
          reject(new Error(`${cmd} 실패: ${err.message.split("\n")[0]}\n${tail}`));
        } else {
          resolve(String(stdout));
        }
      },
    );
  });
}

interface ProbeStream {
  codec_type: string;
  width?: number;
  height?: number;
  start_time?: string;
  duration_ts?: number;
  nb_read_packets?: string;
  sample_rate?: string;
  channels?: number;
}

async function probe(path: string, countPackets = false): Promise<ProbeStream[]> {
  const args = ["-v", "error", "-print_format", "json", "-show_streams"];
  if (countPackets) args.push("-count_packets");
  args.push(path);
  const out = JSON.parse(await run("ffprobe", args)) as { streams: ProbeStream[] };
  return out.streams;
}

function fileHash(path: string): string {
  return createHash("sha1").update(readFileSync(path)).digest("hex");
}

/** 재실행 때 입력이 같은지 비교하는 값 */
export interface DoneRecord {
  sourceSize: number;
  sourceMtimeMs: number;
  startFrame: number;
  endFrame: number;
  label: string;
  fps: { num: number; den: number };
  titleHash: string;
  options: EncodeOptions;
  frames: number;
  samples: number;
}

export const donePath = (n: number) => join(paths.encoded, `${epId(n)}.done`);

function sameInput(a: DoneRecord, b: DoneRecord): boolean {
  return (
    a.startFrame === b.startFrame &&
    a.endFrame === b.endFrame &&
    a.label === b.label &&
    a.fps.num === b.fps.num &&
    a.fps.den === b.fps.den &&
    a.titleHash === b.titleHash &&
    a.sourceSize === b.sourceSize &&
    a.sourceMtimeMs === b.sourceMtimeMs &&
    a.options.crf === b.options.crf &&
    a.options.preset === b.options.preset &&
    a.options.bframes === b.options.bframes
  );
}

export function buildVideoArgs(
  seg: Segment,
  src: string,
  card: string,
  out: string,
  opt: EncodeOptions,
): string[] {
  const { num, den } = seg.fps;
  const frames = seg.endFrame - seg.startFrame;
  const filter = [
    `[0:v]trim=start_frame=${seg.startFrame}:end_frame=${seg.endFrame},setpts=PTS-STARTPTS,setsar=1[base]`,
    // 화수 PNG 는 RGB 라 영상과 같은 bt709/tv range 로 직접 변환해서 얹는다
    "[1:v]format=rgba,scale=out_color_matrix=bt709:out_range=tv,format=yuva420p[card]",
    "[base][card]overlay=0:0:eof_action=pass,format=yuv420p[out]",
  ].join(";");
  return [
    "-y", "-v", "error", "-nostdin",
    "-i", src,
    "-loop", "1", "-framerate", `${num}/${den}`, "-t", String(TITLE_SECONDS), "-i", card,
    "-filter_complex", filter,
    "-map", "[out]", "-an",
    "-fps_mode", "cfr", "-r", `${num}/${den}`, "-frames:v", String(frames),
    "-c:v", "libx264", "-crf", String(opt.crf), "-preset", opt.preset,
    "-profile:v", "high", "-level:v", "4.2", "-pix_fmt", "yuv420p",
    "-bf", String(opt.bframes),
    "-x264-params", `open-gop=0:keyint=300:min-keyint=30:threads=${opt.threads}`,
    "-flags", "+cgop",
    "-color_primaries", "bt709", "-color_trc", "bt709", "-colorspace", "bt709",
    "-color_range", "tv",
    "-video_track_timescale", String(num),
    "-f", "mp4", out,
  ];
}

export function buildAudioArgs(
  src: string,
  hasAudio: boolean,
  startSample: number,
  samples: number,
  out: string,
): string[] {
  const fade = Math.min(FADE_SECONDS, samples / AUDIO_SAMPLE_RATE / 2);
  const total = samples / AUDIO_SAMPLE_RATE;
  const chain = hasAudio
    ? [
        `aresample=${AUDIO_SAMPLE_RATE}`,
        `atrim=start_sample=${startSample}:end_sample=${startSample + samples}`,
        "asetpts=PTS-STARTPTS",
      ]
    : [];
  chain.push(
    `apad=whole_len=${samples}`,
    `atrim=end_sample=${samples}`,
    `afade=t=in:d=${fade}`,
    `afade=t=out:st=${Math.max(0, total - fade)}:d=${fade}`,
    "aformat=sample_fmts=s16:channel_layouts=stereo",
  );
  const input = hasAudio
    ? ["-i", src]
    : ["-f", "lavfi", "-t", String(total + 1), "-i", `anullsrc=r=${AUDIO_SAMPLE_RATE}:cl=stereo`];
  return [
    "-y", "-v", "error", "-nostdin",
    ...input,
    "-vn", "-af", chain.join(","),
    "-c:a", "pcm_s16le", "-ar", String(AUDIO_SAMPLE_RATE), "-ac", "2",
    "-f", "wav", out,
  ];
}

async function encodeOnce(
  seg: Segment,
  opt: EncodeOptions,
  expect: DoneRecord,
): Promise<{ width: number; height: number }> {
  const n = seg.number;
  const src = sourcePath(n);
  const vTmp = `${encodedPath(n)}.tmp.mp4`;
  const aTmp = `${encodedAudioPath(n)}.tmp.wav`;
  const frames = seg.endFrame - seg.startFrame;
  const samples = expect.samples;

  // 음성 시작 샘플: 영상 첫 프레임과 음성 첫 샘플의 시작 시각 차이를 반영한다
  const srcStreams = await probe(src);
  const sv = srcStreams.find((s) => s.codec_type === "video");
  const sa = srcStreams.find((s) => s.codec_type === "audio");
  if (!sv) throw new Error("원본에 영상 트랙이 없습니다");
  const offset = sa ? Number(sv.start_time ?? 0) - Number(sa.start_time ?? 0) : 0;
  const startSample = Math.max(
    0,
    Math.round(
      ((seg.startFrame * seg.fps.den) / seg.fps.num + offset) * AUDIO_SAMPLE_RATE,
    ),
  );

  try {
    await run("ffmpeg", buildVideoArgs(seg, src, titleCardPath(n), vTmp, opt));
    await run("ffmpeg", buildAudioArgs(src, !!sa, startSample, samples, aTmp));

    const v = (await probe(vTmp, true)).find((s) => s.codec_type === "video");
    const got = Number(v?.nb_read_packets);
    if (got !== frames) throw new Error(`프레임 수 불일치: ${got} (기대 ${frames})`);
    const a = (await probe(aTmp)).find((s) => s.codec_type === "audio");
    if (a?.duration_ts !== samples) {
      throw new Error(`샘플 수 불일치: ${a?.duration_ts} (기대 ${samples})`);
    }
    if (Number(a.sample_rate) !== AUDIO_SAMPLE_RATE || a.channels !== 2) {
      throw new Error("음성 규격 불일치");
    }
    renameSync(vTmp, encodedPath(n));
    renameSync(aTmp, encodedAudioPath(n));
    return { width: v?.width ?? 0, height: v?.height ?? 0 };
  } catch (e) {
    rmSync(vTmp, { force: true });
    rmSync(aTmp, { force: true });
    throw e;
  }
}

export interface EncodeRunSummary {
  ok: number;
  cached: number;
  failed: number;
  skipped: number;
}

async function main(): Promise<void> {
  const settings = loadSettings();
  const opt: EncodeOptions = {
    ...DEFAULT_ENCODE_OPTIONS,
    ...(settings.encode as Partial<EncodeOptions>),
  };
  const episodes = selectEpisodes(loadEpisodes(), settings);
  const segments = existsSync(paths.segments)
    ? readJson<Segment[]>(paths.segments)
    : [];
  const segByNumber = new Map(segments.map((s) => [s.number, s]));
  const overrides = loadOverrides();
  mkdirSync(paths.encoded, { recursive: true });

  const results = loadEncodeResults();
  const save = () => {
    results.updatedAt = new Date().toISOString();
    mkdirSync(paths.results, { recursive: true });
    const tmp = `${encodeResultsPath()}.tmp`;
    writeJson(tmp, results);
    renameSync(tmp, encodeResultsPath());
  };
  const record = (r: EpisodeResult) => {
    results.episodes[String(r.number)] = r;
    save();
  };

  if (segments.length === 0) {
    console.warn(`구간 파일이 없거나 비어 있습니다: ${paths.segments}`);
  }

  const summary: EncodeRunSummary = { ok: 0, cached: 0, failed: 0, skipped: 0 };
  const queue = [...episodes];
  const total = queue.length;
  let finished = 0;

  const handle = async (ep: (typeof episodes)[number]): Promise<void> => {
    const n = ep.number;
    const t0 = Date.now();
    const seg = segByNumber.get(n);
    const skip = (why: string) => {
      summary.skipped++;
      record({ number: n, status: "skipped", error: why });
      console.warn(`[${epId(n)}] 건너뜀: ${why}`);
    };
    const override = overrides[String(n)];
    if (override?.exclude) return skip(`수동 제외: ${override.reason}`);
    if (!seg) return skip("segments.json 에 없음");
    if (!existsSync(sourcePath(n))) return skip("원본 파일 없음");
    if (!existsSync(titleCardPath(n))) return skip("화수 PNG 없음");
    if (seg.endFrame <= seg.startFrame) return skip("구간이 비어 있음");
    // 1차 검사에 걸린 화는 재판정(overrides.json)이 끝나야 인코딩한다
    if (seg.flags.length > 0 && seg.source !== "override") {
      return skip(`재판정되지 않은 표시: ${seg.flags.join("; ")}`);
    }
    if (override && seg.source !== "override") {
      return skip("수동 판정이 구간에 반영되지 않음. detect 를 다시 실행해야 함");
    }
    const sourceSize = statSync(sourcePath(n)).size;
    const detectedSize = (seg.evidence.source as { fileSize?: number } | undefined)?.fileSize;
    if (detectedSize !== sourceSize) {
      return skip("감지 후 원본이 바뀌었거나 원본 크기 기록이 없음. detect 를 다시 실행해야 함");
    }

    const frames = seg.endFrame - seg.startFrame;
    const expect: DoneRecord = {
      startFrame: seg.startFrame,
      endFrame: seg.endFrame,
      label: ep.label,
      fps: seg.fps,
      titleHash: fileHash(titleCardPath(n)),
      sourceSize,
      sourceMtimeMs: statSync(sourcePath(n)).mtimeMs,
      options: opt,
      frames,
      samples: framesToSamples(frames, seg.fps),
    };

    if (
      existsSync(donePath(n)) &&
      existsSync(encodedPath(n)) &&
      existsSync(encodedAudioPath(n))
    ) {
      const prev = readJson<DoneRecord>(donePath(n));
      const prevRes = results.episodes[String(n)];
      if (sameInput(prev, expect) && prevRes?.status === "ok") {
        summary.cached++;
        finished++;
        console.log(`[${epId(n)}] 같은 입력이라 건너뜀 (${finished}/${total})`);
        results.episodes[String(n)] = { ...prevRes, cached: true };
        save();
        return;
      }
    }
    rmSync(donePath(n), { force: true });

    let lastError = "";
    for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
      try {
        const dim = await encodeOnce(seg, opt, expect);
        writeJson(donePath(n), expect);
        const seconds = (Date.now() - t0) / 1000;
        summary.ok++;
        finished++;
        record({
          number: n,
          status: "ok",
          frames,
          samples: expect.samples,
          width: dim.width,
          height: dim.height,
          fps: seg.fps,
          seconds: Math.round(seconds * 10) / 10,
          attempts: attempt,
          finishedAt: new Date().toISOString(),
        });
        console.log(
          `[${epId(n)}] 완료 ${frames}프레임 ${seconds.toFixed(1)}초 (${finished}/${total})`,
        );
        return;
      } catch (e) {
        lastError = e instanceof Error ? e.message : String(e);
        console.warn(`[${epId(n)}] 시도 ${attempt}/${MAX_ATTEMPTS} 실패: ${lastError.split("\n")[0]}`);
      }
    }
    summary.failed++;
    finished++;
    record({
      number: n,
      status: "failed",
      error: lastError,
      attempts: MAX_ATTEMPTS,
      seconds: Math.round((Date.now() - t0) / 100) / 10,
    });
  };

  const workers = Array.from({ length: Math.min(opt.jobs, total) }, async () => {
    for (let ep = queue.shift(); ep; ep = queue.shift()) await handle(ep);
  });
  await Promise.all(workers);

  console.log(
    `요약: 대상 ${total}개, 인코딩 ${summary.ok}개, 재사용 ${summary.cached}개, 실패 ${summary.failed}개, 건너뜀 ${summary.skipped}개`,
  );
  if (summary.failed > 0) process.exitCode = 1;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((e) => {
    console.error(e);
    process.exit(1);
  });
}
