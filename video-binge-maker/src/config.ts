// 경로, 설정 파일 로드, 단계 사이에서 주고받는 데이터 형식
//
// 대상 작품에 맞춘 값은 코드에 쓰지 않고 work/settings.json 에서 읽는다.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
export const WORK = process.env.VBM_WORK
  ? resolve(process.env.VBM_WORK)
  : join(ROOT, "work");

export const paths = {
  settings: join(WORK, "settings.json"),
  playlistRaw: join(WORK, "playlist.raw.json"),
  episodes: join(WORK, "episodes.json"),
  segments: join(WORK, "segments.json"),
  overrides: join(WORK, "overrides.json"),
  report: join(WORK, "REPORT.md"),
  refs: join(WORK, "refs"),
  sources: join(WORK, "sources"),
  encoded: join(WORK, "encoded"),
  titleCards: join(WORK, "title-cards"),
  sheets: join(WORK, "sheets"),
  logs: join(WORK, "logs"),
  results: join(WORK, "results"),
  final: join(WORK, "final"),
};

/** 화수 표시가 들어가는 영역. 화수 PNG 는 이 안에 들어가야 하고, 원본 대조에서는 이 영역을 뺀다 */
export const TITLE_REGION = { x: 0, y: 0, width: 1920, height: 180 };
/** 화수 표시 시간(초) */
export const TITLE_SECONDS = 3;
/** 화별 음성 파일과 최종 음성의 샘플레이트 */
export const AUDIO_SAMPLE_RATE = 44100;

export interface Settings {
  playlistUrl: string;
  /** 제목에서 화수와 부제를 뽑는 정규식. 1번 그룹이 화수, 2번 그룹이 부제 */
  titlePattern: string;
  /** 번호가 비어 있어도 되는 화수 */
  excludedEpisodes: number[];
  expectedEpisodeCount: number;
  /** 파일럿에 쓰는 화수 */
  pilotEpisodes: number[];
  /** 구간 감지와 1차 검사 기준값. 형식은 detect.ts 와 verify.ts 가 정한다 */
  detect: Record<string, unknown>;
  /** 화수 표시 설정. 형식은 title-card.ts 가 정한다 */
  titleCard: Record<string, unknown>;
  /** 인코딩 설정. 형식은 encode.ts 가 정한다 */
  encode: Record<string, unknown>;
}

export interface Episode {
  /** 화수 */
  number: number;
  /** 부제 */
  title: string;
  /** 화면과 챕터에 쓰는 문구, 예: "3화 부제" */
  label: string;
  videoId: string;
  /** 목록상 초 단위 길이 */
  duration: number;
  /** 플레이리스트 안의 순번 (1부터) */
  playlistIndex: number;
}

/** 프레임 번호는 모두 0부터 세고, end 는 남기는 구간의 마지막 다음 프레임이다 */
export interface Segment {
  number: number;
  /** 남기는 구간의 첫 프레임 */
  startFrame: number;
  /** 남기는 구간의 끝 (이 프레임은 포함하지 않음) */
  endFrame: number;
  /** 원본 전체 프레임 수 */
  totalFrames: number;
  fps: { num: number; den: number };
  /** detected: 자동 감지 그대로, override: overrides.json 수동 판정 */
  source: "detected" | "override";
  /** 오프닝이나 엔딩을 판단하지 못해 자르지 않은 쪽 */
  uncut: { opening: boolean; ending: boolean };
  /** 판단 근거와 측정값 */
  evidence: Record<string, unknown>;
  /** 1차 검사에서 어긋난 항목. 비어 있으면 통과 */
  flags: string[];
}

/** overrides.json: 화수를 키로 하는 수동 판정. 없는 값은 자동 감지값을 유지한다 */
export interface Override {
  startFrame?: number;
  endFrame?: number;
  uncutOpening?: boolean;
  uncutEnding?: boolean;
  /** 제외한 화. reason 필수 */
  exclude?: boolean;
  reason: string;
}

export function readJson<T>(path: string): T {
  return JSON.parse(readFileSync(path, "utf8")) as T;
}

export function writeJson(path: string, value: unknown): void {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

export function loadSettings(): Settings {
  if (!existsSync(paths.settings)) {
    throw new Error(`설정 파일이 없습니다: ${paths.settings}`);
  }
  return readJson<Settings>(paths.settings);
}

export function loadEpisodes(): Episode[] {
  return readJson<Episode[]>(paths.episodes);
}

export function loadOverrides(): Record<string, Override> {
  return existsSync(paths.overrides)
    ? readJson<Record<string, Override>>(paths.overrides)
    : {};
}

/** 최종 파일에 들어가야 하는 화: 목록에서 수동 제외한 화만 뺀다 */
export function expectedEpisodes(episodes: Episode[]): Episode[] {
  const overrides = loadOverrides();
  return episodes.filter((e) => !overrides[String(e.number)]?.exclude);
}

/** 화수를 파일 이름으로 쓸 때의 형식: 3 -> "003" */
export function epId(number: number): string {
  return String(number).padStart(3, "0");
}

export const sourcePath = (n: number) => join(paths.sources, `${epId(n)}.mp4`);
export const encodedPath = (n: number) => join(paths.encoded, `${epId(n)}.mp4`);
export const encodedAudioPath = (n: number) =>
  join(paths.encoded, `${epId(n)}.wav`);
export const titleCardPath = (n: number) =>
  join(paths.titleCards, `${epId(n)}.png`);

/**
 * 명령행 인자에서 처리할 화수를 고른다.
 * --pilot: settings 의 파일럿 화수, --episodes 1,5,9: 지정한 화수, 없으면 전체
 */
export function selectEpisodes(
  episodes: Episode[],
  settings: Settings,
  argv: string[] = process.argv.slice(2),
): Episode[] {
  if (argv.includes("--pilot")) {
    const pilot = new Set(settings.pilotEpisodes);
    return episodes.filter((e) => pilot.has(e.number));
  }
  const i = argv.indexOf("--episodes");
  if (i >= 0) {
    const chosen = new Set((argv[i + 1] ?? "").split(",").map(Number));
    return episodes.filter((e) => chosen.has(e.number));
  }
  return episodes;
}
