// 6단계: 컷 경계 프레임 4장을 묶은 검수 시트 생성
//
// 사용:
//   npx tsx src/sheets.ts cuts      [--pilot | --episodes 1,5,9]   2차 검증: 화마다 컷 경계 프레임 4장
//   npx tsx src/sheets.ts junctions [--pilot | --episodes 1,5,9]   4차 검증: 최종 파일의 화 연결 지점
//   npx tsx src/sheets.ts titles    [--pilot | --episodes 1,5,9]   3차 검증: 화수 표시 상단 크롭
//
// 이 파일의 ffmpeg/ffprobe 도우미(extractFrames 등)는 verify.ts 도 가져다 쓴다.

import { execFile, spawn } from "node:child_process";
import { existsSync, mkdirSync, renameSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { promisify } from "node:util";
import sharp, { type OverlayOptions } from "sharp";
import {
  epId,
  loadEpisodes,
  loadSettings,
  paths,
  readJson,
  type Segment,
  selectEpisodes,
  sourcePath,
  encodedPath,
  TITLE_REGION,
  TITLE_SECONDS,
} from "./config.ts";

const execFileP = promisify(execFile);

// ---------------------------------------------------------------------------
// 공용 도우미
// ---------------------------------------------------------------------------

/** 임시 파일에 쓴 뒤 이름을 바꿔 중간에 끊겨도 반쪽 파일이 남지 않게 한다 */
export function writeJsonAtomic(path: string, value: unknown): void {
  mkdirSync(dirname(path), { recursive: true });
  const tmp = `${path}.tmp-${process.pid}`;
  writeFileSync(tmp, `${JSON.stringify(value, null, 2)}\n`);
  renameSync(tmp, path);
}

export async function pool<T>(items: T[], size: number, fn: (item: T, i: number) => Promise<void>): Promise<void> {
  let next = 0;
  const worker = async () => {
    for (let i = next++; i < items.length; i = next++) await fn(items[i], i);
  };
  await Promise.all(Array.from({ length: Math.max(1, Math.min(size, items.length)) }, worker));
}

export function parseRational(s: string | undefined): { num: number; den: number } {
  const [a, b] = (s ?? "0/1").split("/").map(Number);
  return { num: a, den: b || 1 };
}

export interface VideoInfo {
  codec: string;
  width: number;
  height: number;
  fps: { num: number; den: number };
  /** 스트림 시작 시각(초) */
  startTime: number;
  /** 스트림 time_base 의 분모/분자 */
  timeBase: { num: number; den: number };
  duration: number;
}

export async function ffprobeJson(args: string[], maxBuffer = 256 * 1024 * 1024): Promise<any> {
  const { stdout } = await execFileP("ffprobe", ["-v", "error", ...args, "-of", "json"], { maxBuffer });
  return JSON.parse(stdout);
}

/** 큰 목록은 JSON 대신 한 줄씩 값만 받는다 */
export async function ffprobeLines(args: string[]): Promise<string[]> {
  const { stdout } = await execFileP("ffprobe", ["-v", "error", ...args, "-of", "csv=p=0"], {
    maxBuffer: 1024 * 1024 * 1024,
  });
  return stdout.split("\n").filter((l) => l.length > 0);
}

export async function probeVideo(file: string): Promise<VideoInfo> {
  const j = await ffprobeJson([
    "-select_streams", "v:0",
    "-show_entries", "stream=codec_name,width,height,r_frame_rate,time_base,start_time,duration",
    file,
  ]);
  const s = j.streams?.[0];
  if (!s) throw new Error(`영상 스트림이 없습니다: ${file}`);
  return {
    codec: s.codec_name,
    width: s.width,
    height: s.height,
    fps: parseRational(s.r_frame_rate),
    startTime: Number(s.start_time ?? 0),
    timeBase: parseRational(s.time_base),
    duration: Number(s.duration ?? 0),
  };
}

export interface FrameOpts {
  /** 출력 크기. vf 에 scale/crop 을 넣었다면 그 결과 크기와 같아야 한다 */
  width: number;
  height: number;
  /** select 뒤에 이어 붙일 필터 (예: "scale=480:270") */
  vf?: string;
  pix?: "rgb24" | "gray";
  /** 있으면 묶음마다 해당 지점으로 정확히 seek 해서 긴 파일도 빠르게 뽑는다 */
  fps?: { num: number; den: number };
}

/** ffmpeg select 식의 항이 100개를 넘으면 메모리 오류가 나므로 나눠서 실행한다 */
const MAX_SELECT_TERMS = 80;

/**
 * 프레임 번호(0부터)를 정확히 지정해서 한 번의 디코드로 여러 장을 뽑는다.
 * 시간 seek 추정 대신 select=eq(n,N) 을 쓰므로 가변 시작 시각에도 어긋나지 않는다.
 * 프레임을 받는 즉시 map 으로 줄여서 메모리를 아낀다. 영상 끝을 넘는 번호는 결과에서 빠진다.
 */
export async function extractFrames<T>(
  file: string,
  frames: number[],
  opts: FrameOpts,
  map: (raw: Buffer, frame: number) => T,
): Promise<Map<number, T>> {
  const want = [...new Set(frames.filter((f) => f >= 0))].sort((a, b) => a - b);
  const out = new Map<number, T>();
  for (let i = 0; i < want.length; i += MAX_SELECT_TERMS) {
    await extractChunk(file, want.slice(i, i + MAX_SELECT_TERMS), opts, map, out);
  }
  return out;
}

async function extractChunk<T>(
  file: string,
  want: number[],
  opts: FrameOpts,
  map: (raw: Buffer, frame: number) => T,
  out: Map<number, T>,
): Promise<void> {
  // 프레임 f0 시각 반 프레임 앞으로 정확히 seek 하면 디코드된 첫 프레임이 f0 이다
  const base = opts.fps ? want[0] : 0;
  const seek = opts.fps && base > 0 ? ["-ss", (((base - 0.5) * opts.fps.den) / opts.fps.num).toFixed(6)] : [];
  const pix = opts.pix ?? "rgb24";
  const size = opts.width * opts.height * (pix === "gray" ? 1 : 3);
  const sel = `select='${want.map((f) => `eq(n,${f - base})`).join("+")}'`;
  const vf = opts.vf ? `${sel},${opts.vf}` : sel;
  const args = [
    "-v", "error", "-nostdin", ...seek, "-i", file, "-an", "-sn", "-vf", vf,
    "-fps_mode", "passthrough", "-frames:v", String(want.length),
    "-f", "rawvideo", "-pix_fmt", pix, "pipe:1",
  ];
  await new Promise<void>((resolve, reject) => {
    const child = spawn("ffmpeg", args, { stdio: ["ignore", "pipe", "pipe"] });
    let chunks: Buffer[] = [];
    let have = 0;
    let idx = 0;
    let err = "";
    child.stderr.on("data", (d) => (err += d));
    child.stdout.on("data", (d: Buffer) => {
      chunks.push(d);
      have += d.length;
      if (have < size) return;
      let all = Buffer.concat(chunks);
      while (all.length >= size && idx < want.length) {
        const f = want[idx++];
        out.set(f, map(Buffer.from(all.subarray(0, size)), f));
        all = all.subarray(size);
      }
      chunks = all.length > 0 ? [all] : [];
      have = all.length;
    });
    child.on("error", reject);
    child.on("close", (code) => (code === 0 ? resolve() : reject(new Error(`ffmpeg 실패(${code}): ${file}\n${err.trim()}`))));
  });
}

/** 오디오를 모노 s16 PCM 으로 디코드한다. from/duration 은 초 단위(생략하면 처음부터 끝까지) */
export async function decodeMono(file: string, rate: number, from?: number, duration?: number): Promise<Int16Array> {
  const args = ["-v", "error", "-nostdin"];
  if (from !== undefined) args.push("-ss", String(Math.max(0, from)));
  if (duration !== undefined) args.push("-t", String(duration));
  args.push("-i", file, "-vn", "-ac", "1", "-ar", String(rate), "-f", "s16le", "pipe:1");
  const { stdout } = (await execFileP("ffmpeg", args, {
    encoding: "buffer",
    maxBuffer: 1024 * 1024 * 1024,
  })) as unknown as { stdout: Buffer };
  const n = Math.floor(stdout.length / 2);
  const out = new Int16Array(n);
  for (let i = 0; i < n; i++) out[i] = stdout.readInt16LE(i * 2);
  return out;
}

// 최종 파일 연결 정보 -------------------------------------------------------

export interface Chapter {
  number: number;
  /** 최종 파일 안에서 이 화의 첫 프레임 번호(0부터) */
  startFrame: number;
  frameCount: number;
  label: string;
}

/** concat.ts 가 chapters.json 에 쓰는 키 이름. 다르면 여기만 고친다 */
const CHAPTER_KEYS = {
  number: "number",
  startFrame: "startFrame",
  frameCount: "frames",
  label: "label",
} as const;
/** chapters.json 최상위가 객체일 때 화 목록이 들어 있는 키 */
const CHAPTER_LIST_KEY = "episodes";

export function finalPaths(pilot: boolean) {
  return {
    video: join(paths.final, pilot ? "pilot.mp4" : "final.mp4"),
    chapters: join(paths.final, pilot ? "pilot.chapters.json" : "final.chapters.json"),
  };
}

export function readChapters(file: string): Chapter[] {
  const raw = readJson<any>(file);
  const list: any[] = Array.isArray(raw) ? raw : raw[CHAPTER_LIST_KEY];
  if (!Array.isArray(list)) throw new Error(`chapters 형식이 배열이 아닙니다: ${file}`);
  return list.map((c) => ({
    number: Number(c[CHAPTER_KEYS.number]),
    startFrame: Number(c[CHAPTER_KEYS.startFrame]),
    frameCount: Number(c[CHAPTER_KEYS.frameCount]),
    label: String(c[CHAPTER_KEYS.label]),
  }));
}

export function loadSegments(): Map<number, Segment> {
  const list = readJson<Segment[]>(paths.segments);
  return new Map(list.map((s) => [s.number, s]));
}

// ---------------------------------------------------------------------------
// 시트 그리기
// ---------------------------------------------------------------------------

const FONT_FILE = "/System/Library/Fonts/AppleSDGothicNeo.ttc";
const FONT_FAMILY = "Apple SD Gothic Neo";
const MARGIN = 10;
const BAND = 18;
const GAP = 6;
const ITEM_GAP = 20;
const LINE_GAP = 14;
const HEADER_H = 32;
const CAP_H = 30;

const escapeXml = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

export interface Cell {
  /** rgb24 원시 픽셀. 없으면 빈칸 */
  raw?: Buffer;
  width: number;
  height: number;
  caption: string;
  /** 빈칸일 때 안에 적는 문구 */
  blankText?: string;
}

export interface SheetItem {
  header: string;
  /** 있으면 줄 앞 띠와 머리줄을 빨갛게 하고 문구를 붙인다 */
  flagText?: string;
  cells: Cell[];
}

async function drawText(text: string, size: number, color: string, maxW: number) {
  const { data, info } = await sharp({
    text: {
      text: `<span foreground="${color}">${escapeXml(text)}</span>`,
      font: `${FONT_FAMILY} ${size}`,
      ...(existsSync(FONT_FILE) ? { fontfile: FONT_FILE } : {}),
      rgba: true,
      dpi: 72,
    },
  })
    .png()
    .toBuffer({ resolveWithObject: true });
  if (info.width <= maxW) return { input: data, width: info.width, height: info.height };
  const small = await sharp(data).resize({ width: maxW }).png().toBuffer({ resolveWithObject: true });
  return { input: small.data, width: small.info.width, height: small.info.height };
}

export async function renderSheet(
  items: SheetItem[],
  o: { cellW: number; cellH: number; perLine: number },
): Promise<Buffer> {
  const cellsPer = Math.max(...items.map((i) => i.cells.length));
  const itemW = BAND + cellsPer * o.cellW + (cellsPer - 1) * GAP;
  const itemH = HEADER_H + o.cellH + CAP_H;
  const lines = Math.ceil(items.length / o.perLine);
  const perLine = Math.min(o.perLine, items.length);
  const W = MARGIN * 2 + perLine * itemW + (perLine - 1) * ITEM_GAP;
  const H = MARGIN * 2 + lines * itemH + (lines - 1) * LINE_GAP;

  const rects: string[] = [`<rect width="${W}" height="${H}" fill="#161616"/>`];
  const texts: Promise<OverlayOptions>[] = [];
  const layers: OverlayOptions[] = [];
  const text = (t: string, size: number, color: string, maxW: number, left: number, top: number) =>
    texts.push(drawText(t, size, color, maxW).then((r) => ({ input: r.input, left, top })));

  items.forEach((item, k) => {
    const x0 = MARGIN + (k % o.perLine) * (itemW + ITEM_GAP);
    const y0 = MARGIN + Math.floor(k / o.perLine) * (itemH + LINE_GAP);
    const flagged = !!item.flagText;
    if (flagged) rects.push(`<rect x="${x0}" y="${y0}" width="${BAND - 4}" height="${itemH}" fill="#ff1744"/>`);
    const cx0 = x0 + BAND;
    const rowW = itemW - BAND;
    rects.push(`<rect x="${cx0}" y="${y0}" width="${rowW}" height="${HEADER_H}" fill="${flagged ? "#b00020" : "#333333"}"/>`);
    const head = flagged ? `${item.header}    FLAG: ${item.flagText}` : item.header;
    text(head, 20, "#ffffff", rowW - 12, cx0 + 8, y0 + 4);

    item.cells.forEach((cell, c) => {
      const cx = cx0 + c * (o.cellW + GAP);
      const cy = y0 + HEADER_H;
      if (cell.raw) {
        layers.push({ input: cell.raw, raw: { width: cell.width, height: cell.height, channels: 3 }, left: cx, top: cy });
      } else {
        rects.push(`<rect x="${cx}" y="${cy}" width="${o.cellW}" height="${o.cellH}" fill="#2a2a2a" stroke="#555" stroke-dasharray="8 6"/>`);
        text(cell.blankText ?? "범위 밖", 22, "#999999", o.cellW - 20, cx + 10, cy + o.cellH / 2 - 14);
      }
      rects.push(`<rect x="${cx}" y="${cy + o.cellH}" width="${o.cellW}" height="${CAP_H}" fill="#222222"/>`);
      text(cell.caption, 18, "#e8e8e8", o.cellW - 12, cx + 6, cy + o.cellH + 4);
    });
  });

  const bg = Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}">${rects.join("")}</svg>`);
  // 사각형 배경 위에 영상 칸을 얹고, 글자는 마지막에 얹는다
  const textLayers = await Promise.all(texts);
  return sharp(bg)
    .composite([...layers, ...textLayers])
    .jpeg({ quality: 82 })
    .toBuffer();
}

async function saveSheet(name: string, jpg: Buffer): Promise<string> {
  mkdirSync(paths.sheets, { recursive: true });
  const file = join(paths.sheets, name);
  const tmp = `${file}.tmp-${process.pid}`;
  writeFileSync(tmp, jpg);
  renameSync(tmp, file);
  return file;
}

interface IndexEntry {
  file: string;
  episodes: number[];
  flagged: number[];
}

function writeIndex(kind: string, sheets: IndexEntry[], note?: string): void {
  // 색인은 시트와 같은 폴더에 있으므로 파일 이름만 적는다
  writeJsonAtomic(join(paths.sheets, `${kind}-index.json`), {
    kind,
    generatedAt: new Date().toISOString(),
    ...(note ? { note } : {}),
    sheets: sheets.map((e) => ({ ...e, file: basename(e.file) })),
  });
}

const chunk = <T>(a: T[], n: number): T[][] =>
  Array.from({ length: Math.ceil(a.length / n) }, (_, i) => a.slice(i * n, i * n + n));

const sheetName = (kind: string, i: number) => `${kind}-${String(i + 1).padStart(3, "0")}.jpg`;

// ---------------------------------------------------------------------------
// cuts: 화마다 컷 경계 프레임 4장
// ---------------------------------------------------------------------------

const CUT_W = 480;
const CUT_H = 270;
const FIT = (w: number, h: number) =>
  `scale=${w}:${h}:force_original_aspect_ratio=decrease,pad=${w}:${h}:(ow-iw)/2:(oh-ih)/2:color=black`;
const PER_SHEET = 10;

async function cutsItem(number: number, label: string, seg: Segment): Promise<SheetItem> {
  const S = seg.startFrame;
  const E = seg.endFrame;
  const slots = [
    { f: S - 1, name: "오프닝 직전", uncut: seg.uncut.opening },
    { f: S, name: "오프닝 직후", uncut: seg.uncut.opening },
    { f: E - 1, name: "엔딩 직전", uncut: seg.uncut.ending },
    { f: E, name: "엔딩 직후", uncut: seg.uncut.ending },
  ];
  const valid = slots.map((s) => s.f).filter((f) => f >= 0 && f < seg.totalFrames);
  const src = sourcePath(number);
  const got = existsSync(src)
    ? await extractFrames(src, valid, { width: CUT_W, height: CUT_H, vf: FIT(CUT_W, CUT_H) }, (b) => b)
    : new Map<number, Buffer>();
  const missingSrc = !existsSync(src);
  const cells: Cell[] = slots.map((s) => {
    const raw = got.get(s.f);
    const tag = `${s.name}${s.uncut ? " (미절단)" : ""}`;
    return {
      raw,
      width: CUT_W,
      height: CUT_H,
      caption: `${epId(number)}화  프레임 ${s.f}  ${tag}`,
      blankText: missingSrc ? "원본 없음" : s.f < 0 ? "범위 밖 (영상 시작 이전)" : "범위 밖 (영상 끝 이후)",
    };
  });
  const extra = [seg.uncut.opening ? "오프닝 미절단" : "", seg.uncut.ending ? "엔딩 미절단" : ""].filter(Boolean).join("/");
  return {
    header: `${epId(number)}화  ${label}   [${S} ~ ${E}, 전체 ${seg.totalFrames}]${extra ? `   ${extra}` : ""}`,
    flagText: seg.flags.length > 0 ? seg.flags.join(", ") : undefined,
    cells,
  };
}

async function cutsMain(argv: string[]): Promise<void> {
  const settings = loadSettings();
  const targets = selectEpisodes(loadEpisodes(), settings, argv).sort((a, b) => a.number - b.number);
  const segs = loadSegments();
  const usable = targets.filter((e) => {
    if (!segs.has(e.number)) console.warn(`구간 정보 없음, 건너뜀: ${e.number}화`);
    return segs.has(e.number);
  });
  const index: IndexEntry[] = [];
  for (const [i, group] of chunk(usable, PER_SHEET).entries()) {
    const items: SheetItem[] = new Array(group.length);
    await pool(group, 5, async (ep, k) => {
      items[k] = await cutsItem(ep.number, ep.label, segs.get(ep.number)!);
    });
    const file = await saveSheet(sheetName("cuts", i), await renderSheet(items, { cellW: CUT_W, cellH: CUT_H, perLine: 1 }));
    index.push({
      file,
      episodes: group.map((e) => e.number),
      flagged: group.filter((e) => segs.get(e.number)!.flags.length > 0).map((e) => e.number),
    });
    console.log(`컷 시트 ${file} (${group[0].number}~${group[group.length - 1].number}화)`);
  }
  writeIndex("cuts", index);
}

// ---------------------------------------------------------------------------
// junctions: 최종 파일의 화 연결 지점
// ---------------------------------------------------------------------------

async function junctionsMain(argv: string[]): Promise<void> {
  const pilot = argv.includes("--pilot");
  const fp = finalPaths(pilot);
  if (!existsSync(fp.video) || !existsSync(fp.chapters)) {
    throw new Error(`최종 파일이나 chapters.json 이 없습니다: ${fp.video}`);
  }
  let chapters = readChapters(fp.chapters);
  const i = argv.indexOf("--episodes");
  if (i >= 0) {
    const chosen = new Set((argv[i + 1] ?? "").split(",").map(Number));
    chapters = chapters.filter((c) => chosen.has(c.number));
  }
  const frames = chapters.flatMap((c) => [c.startFrame - 1, c.startFrame]);
  const fps = [...loadSegments().values()][0]?.fps;
  const got = await extractFrames(fp.video, frames, { width: CUT_W, height: CUT_H, vf: FIT(CUT_W, CUT_H), fps }, (b) => b);

  const index: IndexEntry[] = [];
  for (const [n, group] of chunk(chapters, PER_SHEET).entries()) {
    const items: SheetItem[] = group.map((c) => ({
      header: `${epId(c.number)}화  ${c.label}   [최종 파일 프레임 ${c.startFrame}, ${c.frameCount}프레임]`,
      cells: [
        {
          raw: got.get(c.startFrame - 1),
          width: CUT_W,
          height: CUT_H,
          caption: `연결 직전 (앞 화의 끝)  프레임 ${c.startFrame - 1}`,
          blankText: "첫 화 (앞 프레임 없음)",
        },
        {
          raw: got.get(c.startFrame),
          width: CUT_W,
          height: CUT_H,
          caption: `연결 직후 (${epId(c.number)}화 첫 프레임)  프레임 ${c.startFrame}`,
          blankText: "프레임 없음",
        },
      ],
    }));
    const file = await saveSheet(sheetName("junctions", n), await renderSheet(items, { cellW: CUT_W, cellH: CUT_H, perLine: 2 }));
    index.push({ file, episodes: group.map((c) => c.number), flagged: [] });
    console.log(`연결 지점 시트 ${file}`);
  }
  writeIndex("junctions", index, pilot ? "pilot" : undefined);
}

// ---------------------------------------------------------------------------
// titles: 화수 표시 검수 (encoded 첫 프레임 상단)
// ---------------------------------------------------------------------------

const TITLE_CELL_W = 960;
const TITLE_CELL_H = Math.round((TITLE_REGION.height * TITLE_CELL_W) / TITLE_REGION.width);
const TITLES_PER_SHEET = 20;

async function titlesMain(argv: string[]): Promise<void> {
  const settings = loadSettings();
  const targets = selectEpisodes(loadEpisodes(), settings, argv).sort((a, b) => a.number - b.number);
  const segs = loadSegments();
  const R = TITLE_REGION;
  const crop = `crop=${R.width}:${R.height}:${R.x}:${R.y},scale=${TITLE_CELL_W}:${TITLE_CELL_H}`;
  const index: IndexEntry[] = [];
  for (const [n, group] of chunk(targets, TITLES_PER_SHEET).entries()) {
    const items: SheetItem[] = new Array(group.length);
    await pool(group, 5, async (ep, k) => {
      const seg = segs.get(ep.number);
      const fps = seg ? (seg.fps.num / seg.fps.den) : 30;
      const after = Math.round((TITLE_SECONDS + 0.5) * fps);
      const file = encodedPath(ep.number);
      const got = existsSync(file)
        ? await extractFrames(file, [0, after], { width: TITLE_CELL_W, height: TITLE_CELL_H, vf: crop }, (b) => b)
        : new Map<number, Buffer>();
      items[k] = {
        header: `${epId(ep.number)}화  예상 문구: ${ep.label}`,
        cells: [
          { raw: got.get(0), width: TITLE_CELL_W, height: TITLE_CELL_H, caption: "첫 프레임 상단 (예상 문구가 보여야 함)", blankText: "encoded 파일 없음" },
          { raw: got.get(after), width: TITLE_CELL_W, height: TITLE_CELL_H, caption: `${TITLE_SECONDS + 0.5}초 지점 상단 (표시가 없어야 함)`, blankText: "영상이 더 짧음" },
        ],
      };
    });
    const file = await saveSheet(sheetName("titles", n), await renderSheet(items, { cellW: TITLE_CELL_W, cellH: TITLE_CELL_H, perLine: 1 }));
    index.push({ file, episodes: group.map((e) => e.number), flagged: [] });
    console.log(`화수 표시 시트 ${file}`);
  }
  writeIndex("titles", index);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const [cmd, ...rest] = process.argv.slice(2);
  const table: Record<string, (argv: string[]) => Promise<void>> = {
    cuts: cutsMain,
    junctions: junctionsMain,
    titles: titlesMain,
  };
  if (!cmd || !table[cmd]) {
    console.error("사용: npx tsx src/sheets.ts cuts|junctions|titles [--pilot | --episodes 1,5,9]");
    process.exit(2);
  }
  try {
    await table[cmd](rest);
  } catch (e) {
    console.error(`오류: ${(e as Error).message}`);
    process.exit(1);
  }
}
