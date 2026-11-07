// 8단계: 화별 결과를 하나로 합치고 챕터를 넣는다
//
// 영상은 -c copy 로 이어 붙이고, 화별 WAV 를 이은 전체 음성을 AAC 로 한 번만 인코딩한다.
// 사용법: npx tsx src/concat.ts [--pilot | --episodes 1,5,9]

import { execFile } from "node:child_process";
import { existsSync, mkdirSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import {
  encodedAudioPath,
  encodedPath,
  loadEpisodes,
  loadSettings,
  paths,
  readJson,
  type Segment,
  selectEpisodes,
  writeJson,
} from "./config.ts";
import { type DoneRecord, donePath, type EpisodeResult, loadEncodeResults } from "./encode.ts";

const AUDIO_BITRATE = "192k";

export interface ChapterEntry {
  number: number;
  label: string;
  startFrame: number;
  frames: number;
}

export interface ChaptersFile {
  file: string;
  fps: { num: number; den: number };
  width: number;
  height: number;
  totalFrames: number;
  episodes: ChapterEntry[];
}

function run(cmd: string, args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    execFile(cmd, args, { maxBuffer: 64 * 1024 * 1024 }, (err, stdout, stderr) => {
      if (err) {
        const tail = String(stderr).split("\n").slice(-12).join("\n");
        reject(new Error(`${cmd} 실패: ${err.message.split("\n")[0]}\n${tail}`));
      } else {
        resolve(String(stdout));
      }
    });
  });
}

/** concat demuxer 목록의 파일 경로 이스케이프 */
const listLine = (path: string) => `file '${path.replace(/'/g, "'\\''")}'`;

/** FFMETADATA 값 이스케이프: = ; # \ 와 줄바꿈 */
const metaEscape = (s: string) => s.replace(/([=;#\\\n])/g, "\\$1");

export function buildMetadata(
  chapters: ChapterEntry[],
  fps: { num: number; den: number },
): string {
  // 타임베이스 1/num, 프레임 하나가 den 틱
  const lines = [";FFMETADATA1"];
  for (const c of chapters) {
    const start = c.startFrame * fps.den;
    const end = (c.startFrame + c.frames) * fps.den;
    lines.push(
      "[CHAPTER]",
      `TIMEBASE=1/${fps.num}`,
      `START=${start}`,
      `END=${end}`,
      `title=${metaEscape(c.label)}`,
    );
  }
  return `${lines.join("\n")}\n`;
}

async function main(): Promise<void> {
  const settings = loadSettings();
  const argv = process.argv.slice(2);
  const pilot = argv.includes("--pilot");
  const episodes = selectEpisodes(loadEpisodes(), settings, argv).sort(
    (a, b) => a.number - b.number,
  );
  const results = loadEncodeResults();
  const segByNumber = new Map(
    readJson<Segment[]>(paths.segments).map((s) => [s.number, s]),
  );
  const verifyPath = join(paths.results, "verify-episodes.json");
  const verified = new Map(
    (existsSync(verifyPath)
      ? readJson<{ episodes: { number: number; ok: boolean; verifiedAt?: string }[] }>(verifyPath).episodes
      : []
    ).map((e) => [e.number, e]),
  );

  const targets: { number: number; label: string; res: EpisodeResult }[] = [];
  for (const ep of episodes) {
    const res = results.episodes[String(ep.number)];
    if (res?.status !== "ok") {
      console.warn(`${ep.number}화 제외: 인코딩 결과가 ok 가 아님 (${res?.status ?? "기록 없음"})`);
      continue;
    }
    if (!existsSync(encodedPath(ep.number)) || !existsSync(encodedAudioPath(ep.number))) {
      console.warn(`${ep.number}화 제외: 화별 파일이 없음`);
      continue;
    }
    const seg = segByNumber.get(ep.number);
    const done = existsSync(donePath(ep.number))
      ? readJson<DoneRecord>(donePath(ep.number))
      : null;
    if (!seg || done?.startFrame !== seg.startFrame || done?.endFrame !== seg.endFrame) {
      console.warn(`${ep.number}화 제외: 인코딩 결과가 현재 구간과 다름`);
      continue;
    }
    const check = verified.get(ep.number);
    if (!check?.ok || !check.verifiedAt || check.verifiedAt < (res.finishedAt ?? "")) {
      console.warn(`${ep.number}화 제외: 현재 인코딩 결과가 3차 검증을 통과하지 않음`);
      continue;
    }
    targets.push({ number: ep.number, label: ep.label, res });
  }
  if (targets.length === 0) throw new Error("합칠 화가 없습니다");

  // 규격 확인: 이어 붙이려면 모두 같아야 한다
  const first = targets[0].res;
  for (const t of targets) {
    const r = t.res;
    if (
      r.fps?.num !== first.fps?.num ||
      r.fps?.den !== first.fps?.den ||
      r.width !== first.width ||
      r.height !== first.height
    ) {
      throw new Error(
        `규격이 다른 화가 있습니다: ${t.number}화 ${r.width}x${r.height} ${r.fps?.num}/${r.fps?.den}, ` +
          `${targets[0].number}화 ${first.width}x${first.height} ${first.fps?.num}/${first.fps?.den}`,
      );
    }
  }
  const fps = first.fps as { num: number; den: number };

  let cum = 0;
  const chapters: ChapterEntry[] = targets.map((t) => {
    const entry = {
      number: t.number,
      label: t.label,
      startFrame: cum,
      frames: t.res.frames as number,
    };
    cum += entry.frames;
    return entry;
  });

  mkdirSync(paths.final, { recursive: true });
  // 일부 화만 고른 실행이 전체 산출물을 덮어쓰지 않게 한다
  const name = pilot ? "pilot" : argv.includes("--episodes") ? "partial" : "final";
  const outPath = join(paths.final, `${name}.mp4`);
  const tmpOut = join(paths.final, `${name}.tmp.mp4`);
  const vList = join(paths.final, `${name}.video.txt`);
  const aList = join(paths.final, `${name}.audio.txt`);
  const meta = join(paths.final, `${name}.meta.txt`);

  writeFileSync(vList, `${targets.map((t) => listLine(encodedPath(t.number))).join("\n")}\n`);
  writeFileSync(aList, `${targets.map((t) => listLine(encodedAudioPath(t.number))).join("\n")}\n`);
  writeFileSync(meta, buildMetadata(chapters, fps));

  console.log(`${targets.length}개 화를 합칩니다 (${cum}프레임)`);
  try {
    await run("ffmpeg", [
      "-y", "-v", "error", "-nostdin",
      "-f", "concat", "-safe", "0", "-i", vList,
      "-f", "concat", "-safe", "0", "-i", aList,
      "-i", meta,
      "-map", "0:v:0", "-map", "1:a:0",
      "-map_metadata", "2", "-map_chapters", "2",
      "-c:v", "copy", "-c:a", "aac", "-b:a", AUDIO_BITRATE, "-ar", "44100",
      "-movflags", "+faststart",
      "-f", "mp4", tmpOut,
    ]);
  } catch (e) {
    rmSync(tmpOut, { force: true });
    throw e;
  }
  renameSync(tmpOut, outPath);
  rmSync(vList, { force: true });
  rmSync(aList, { force: true });
  rmSync(meta, { force: true });

  const chaptersFile: ChaptersFile = {
    file: outPath,
    fps,
    width: first.width as number,
    height: first.height as number,
    totalFrames: cum,
    episodes: chapters,
  };
  const chaptersPath = join(paths.final, `${name}.chapters.json`);
  writeJson(`${chaptersPath}.tmp`, chaptersFile);
  renameSync(`${chaptersPath}.tmp`, chaptersPath);

  console.log(`완료: ${outPath}`);
  console.log(`챕터 정보: ${chaptersPath}`);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((e) => {
    console.error(e instanceof Error ? e.message : e);
    process.exit(1);
  });
}
