// 4단계: 원본 다운로드와 이어받기, 실패 시 간격을 늘려 재시도
//
// 사용: npx tsx src/download.ts [--pilot | --episodes 1,5,9]
// 무인으로 오래 도는 스크립트라 화 하나가 끝날 때마다 결과를 저장한다.

import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import {
  type Episode,
  loadEpisodes,
  loadSettings,
  paths,
  selectEpisodes,
  sourcePath,
} from "./config.ts";

type Status = "ok" | "skipped" | "failed" | "mismatch";

export interface ProbeInfo {
  hasVideo: boolean;
  hasAudio: boolean;
  height: number;
  fps: number;
  duration: number;
}

interface Result {
  status: Status;
  format: string;
  attempts: number;
  error: string | null;
  updatedAt: string;
}

const FORMAT = "(270/137/bv*[height=1080])+(140/ba)";
const EXPECTED_HEIGHT = 1080;
const DURATION_TOLERANCE = 3;
/** 재시도 전 대기(초). 시도 횟수는 이 길이 + 1 */
const RETRY_DELAYS = [30, 120, 600];
/** 접속 제한이 연속 이 횟수에 이르면 전체를 쉰다 */
const LIMIT_STREAK = 2;
const LIMIT_PAUSE_SEC = 30 * 60;
const LIMIT_PATTERN = /429|too many requests|sign in to confirm|not a bot|rate.?limit|captcha/i;

const resultFile = join(paths.results, "download.json");

const sleep = (sec: number) => new Promise((r) => setTimeout(r, sec * 1000));
const log = (msg: string) => console.log(`[${new Date().toLocaleTimeString("ko-KR", { hour12: false })}] ${msg}`);

/** ffprobe 로 영상과 음성 스트림, 해상도, 프레임레이트, 길이를 읽는다. 읽지 못하면 null */
export function probe(path: string): ProbeInfo | null {
  try {
    const out = execFileSync(
      "ffprobe",
      ["-v", "error", "-show_entries", "stream=codec_type,height,avg_frame_rate,r_frame_rate:format=duration", "-of", "json", path],
      { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
    );
    const j = JSON.parse(out) as {
      streams?: { codec_type: string; height?: number; avg_frame_rate?: string; r_frame_rate?: string }[];
      format?: { duration?: string };
    };
    const streams = j.streams ?? [];
    const v = streams.find((s) => s.codec_type === "video");
    const ratio = (s?: string) => {
      const [a, b] = (s ?? "0/1").split("/").map(Number);
      return b ? a / b : 0;
    };
    return {
      hasVideo: Boolean(v),
      hasAudio: streams.some((s) => s.codec_type === "audio"),
      height: v?.height ?? 0,
      fps: ratio(v?.avg_frame_rate && v.avg_frame_rate !== "0/0" ? v.avg_frame_rate : v?.r_frame_rate),
      duration: Number(j.format?.duration ?? 0),
    };
  } catch {
    return null;
  }
}

const isComplete = (p: ProbeInfo | null): p is ProbeInfo => Boolean(p?.hasVideo && p.hasAudio);

/** 받은 파일이 기대와 어긋나는 부분을 설명 목록으로 돌려준다 */
export function checkProbe(p: ProbeInfo, episode: Pick<Episode, "duration">): string[] {
  const problems: string[] = [];
  if (p.height !== EXPECTED_HEIGHT) problems.push(`높이 ${p.height}`);
  if (p.fps <= 0) problems.push("프레임레이트 확인 불가");
  if (!p.hasAudio) problems.push("음성 없음");
  if (Math.abs(p.duration - episode.duration) > DURATION_TOLERANCE) {
    problems.push(`길이 ${p.duration.toFixed(1)}초, 목록 ${episode.duration}초`);
  }
  return problems;
}

const describe = (p: ProbeInfo) => `${p.height}p ${p.fps.toFixed(2)}fps ${p.duration.toFixed(1)}초`;

function runYtDlp(url: string, out: string): Promise<{ code: number; stderr: string }> {
  return new Promise((resolve) => {
    const child = spawn(
      "uvx",
      ["--python", "3.12", "--from", "yt-dlp[default]", "yt-dlp", "--js-runtimes", "bun", "--no-warnings", "--no-progress", "-f", FORMAT, "--merge-output-format", "mp4", "-o", out, url],
      { stdio: ["ignore", "inherit", "pipe"] },
    );
    let stderr = "";
    child.stderr.on("data", (d: Buffer) => {
      process.stderr.write(d);
      stderr = (stderr + d.toString()).slice(-4000);
    });
    child.on("error", (e) => resolve({ code: -1, stderr: String(e) }));
    child.on("close", (code) => resolve({ code: code ?? -1, stderr }));
  });
}

function loadResults(): Record<string, Result> {
  try {
    return JSON.parse(readFileSync(resultFile, "utf8")) as Record<string, Result>;
  } catch {
    return {};
  }
}

/** 임시 파일에 쓴 뒤 rename 해서 중간에 끊겨도 깨지지 않게 한다 */
function saveResults(results: Record<string, Result>): void {
  mkdirSync(paths.results, { recursive: true });
  const tmp = `${resultFile}.tmp`;
  writeFileSync(tmp, `${JSON.stringify(results, null, 2)}\n`);
  renameSync(tmp, resultFile);
}

async function downloadOne(ep: Episode): Promise<Result> {
  const file = sourcePath(ep.number);
  const done = (status: Status, format: string, attempts: number, error: string | null): Result => ({
    status,
    format,
    attempts,
    error,
    updatedAt: new Date().toISOString(),
  });

  if (existsSync(file)) {
    const p = probe(file);
    if (isComplete(p)) {
      return done("skipped", describe(p), 0, null);
    }
    log(`${ep.number}화 기존 파일이 깨져 있어 다시 받습니다`);
    rmSync(file, { force: true });
  }

  const url = `https://www.youtube.com/watch?v=${ep.videoId}`;
  let lastError = "";
  let limitStreak = 0;
  for (let attempt = 1; attempt <= RETRY_DELAYS.length + 1; attempt++) {
    log(`${ep.number}화 다운로드 시도 ${attempt}/${RETRY_DELAYS.length + 1}`);
    mkdirSync(paths.sources, { recursive: true });
    const { code, stderr } = await runYtDlp(url, file);
    const p = code === 0 ? probe(file) : null;
    if (code === 0 && isComplete(p)) {
      const problems = checkProbe(p, ep);
      if (problems.length > 0) {
        log(`${ep.number}화 검증 불일치: ${problems.join(", ")}`);
        return done("mismatch", describe(p), attempt, problems.join(", "));
      }
      return done("ok", describe(p), attempt, null);
    }

    lastError = code === 0 ? "받은 파일에서 영상 또는 음성 스트림을 확인하지 못함" : (stderr.trim().split("\n").slice(-3).join(" / ") || `종료 코드 ${code}`);
    log(`${ep.number}화 실패: ${lastError}`);
    if (code === 0) rmSync(file, { force: true });

    limitStreak = LIMIT_PATTERN.test(stderr) ? limitStreak + 1 : 0;
    if (limitStreak >= LIMIT_STREAK) {
      log(`접속 제한이 연속되어 ${LIMIT_PAUSE_SEC / 60}분 쉽니다`);
      await sleep(LIMIT_PAUSE_SEC);
      limitStreak = 0;
    } else if (attempt <= RETRY_DELAYS.length) {
      log(`${RETRY_DELAYS[attempt - 1]}초 뒤 재시도`);
      await sleep(RETRY_DELAYS[attempt - 1]);
    }
  }
  return done("failed", "", RETRY_DELAYS.length + 1, lastError);
}

async function main(): Promise<void> {
  const targets = selectEpisodes(loadEpisodes(), loadSettings());
  const results = loadResults();
  log(`대상 ${targets.length}화`);

  for (const [i, ep] of targets.entries()) {
    const r = await downloadOne(ep);
    results[String(ep.number)] = r;
    saveResults(results);
    log(`[${i + 1}/${targets.length}] ${ep.number}화 ${r.status}${r.format ? ` (${r.format})` : ""}`);
    if (r.status !== "skipped" && i < targets.length - 1) {
      await sleep(2 + Math.random() * 3);
    }
  }

  const count = (s: Status) => targets.filter((e) => results[String(e.number)]?.status === s).length;
  log(`완료: ok ${count("ok")} / skipped ${count("skipped")} / mismatch ${count("mismatch")} / failed ${count("failed")}`);
  const failed = targets.filter((e) => results[String(e.number)]?.status === "failed").map((e) => e.number);
  if (failed.length > 0) console.log(`실패 ${failed.length}화: ${failed.join(",")}`);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  await main();
}
