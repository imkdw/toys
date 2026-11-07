// 1단계: 플레이리스트에서 화수, 제목, ID를 정리해 episodes.json 생성

import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import {
  type Episode,
  loadSettings,
  paths,
  readJson,
  type Settings,
  writeJson,
} from "./config.ts";

interface RawEntry {
  id: string;
  title: string | null;
  duration: number | null;
}

/** 플레이리스트 항목에서 본편만 골라 화수 순으로 정리한다 */
export function buildEpisodes(
  entries: RawEntry[],
  settings: Pick<Settings, "titlePattern">,
): { episodes: Episode[]; skipped: { index: number; id: string; why: string }[] } {
  const pattern = new RegExp(settings.titlePattern);
  const seenIds = new Set<string>();
  const byNumber = new Map<number, Episode>();
  const skipped: { index: number; id: string; why: string }[] = [];

  entries.forEach((entry, i) => {
    const index = i + 1;
    if (!entry.title) {
      skipped.push({ index, id: entry.id, why: "비공개 또는 삭제" });
      return;
    }
    if (seenIds.has(entry.id)) {
      skipped.push({ index, id: entry.id, why: "중복 영상" });
      return;
    }
    seenIds.add(entry.id);

    const match = entry.title.match(pattern);
    if (!match) {
      skipped.push({ index, id: entry.id, why: "본편 형식이 아님" });
      return;
    }
    const number = Number(match[1]);
    if (byNumber.has(number)) {
      throw new Error(`같은 화수에 서로 다른 영상이 있습니다: ${number}화`);
    }
    const title = match[2].replace(/\s+/g, " ").trim();
    byNumber.set(number, {
      number,
      title,
      label: `${number}화 ${title}`,
      videoId: entry.id,
      duration: entry.duration ?? 0,
      playlistIndex: index,
    });
  });

  const episodes = [...byNumber.values()].sort((a, b) => a.number - b.number);
  return { episodes, skipped };
}

/** 번호 누락과 개수를 확인한다. 문제를 찾으면 설명 목록을 돌려준다 */
export function checkEpisodes(
  episodes: Episode[],
  settings: Pick<Settings, "excludedEpisodes" | "expectedEpisodeCount">,
): string[] {
  const problems: string[] = [];
  if (episodes.length !== settings.expectedEpisodeCount) {
    problems.push(
      `본편 수 ${episodes.length}개, 기대값 ${settings.expectedEpisodeCount}개`,
    );
  }
  const present = new Set(episodes.map((e) => e.number));
  const excluded = new Set(settings.excludedEpisodes);
  const last = Math.max(...present);
  for (let n = 1; n <= last; n++) {
    if (!present.has(n) && !excluded.has(n)) problems.push(`${n}화 누락`);
    if (present.has(n) && excluded.has(n)) {
      problems.push(`${n}화는 제외 대상인데 목록에 있음`);
    }
  }
  return problems;
}

function fetchPlaylist(url: string): { entries: RawEntry[] } {
  const out = execFileSync(
    "uvx",
    ["--python", "3.12", "yt-dlp", "--flat-playlist", "-J", url],
    { encoding: "utf8", maxBuffer: 256 * 1024 * 1024 },
  );
  writeJson(paths.playlistRaw, JSON.parse(out));
  return JSON.parse(out);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const settings = loadSettings();
  const raw =
    !process.argv.includes("--refresh") && existsSync(paths.playlistRaw)
      ? readJson<{ entries: RawEntry[] }>(paths.playlistRaw)
      : fetchPlaylist(settings.playlistUrl);

  const { episodes, skipped } = buildEpisodes(raw.entries, settings);
  const problems = checkEpisodes(episodes, settings);
  writeJson(paths.episodes, episodes);

  console.log(`전체 ${raw.entries.length}개, 본편 ${episodes.length}개`);
  for (const s of skipped) console.log(`제외 #${s.index} ${s.id}: ${s.why}`);
  if (problems.length > 0) {
    for (const p of problems) console.error(`문제: ${p}`);
    process.exit(1);
  }
}
