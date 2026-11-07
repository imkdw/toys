// REPORT.md 생성: 제외, 수동 판정, 자르지 않은 화, 실패한 검사를 빠짐없이 모은다
//
// 실행: npx tsx src/report.ts
// work/report-notes.md 가 있으면 소요 시간과 기준값 변경 같은 내용으로 그대로 덧붙인다.

import { existsSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import {
  type Episode,
  expectedEpisodes,
  loadEpisodes,
  loadOverrides,
  loadSettings,
  paths,
  readJson,
  type Segment,
} from "./config.ts";

interface Chapter {
  number: number;
  label: string;
  startFrame: number;
  frames: number;
}

interface EpisodeCheck {
  number: number;
  ok: boolean;
  failures: string[];
}

function readOptional<T>(path: string): T | null {
  return existsSync(path) ? readJson<T>(path) : null;
}

function timecode(seconds: number): string {
  const s = Math.floor(seconds);
  const pad = (v: number) => String(v).padStart(2, "0");
  return `${Math.floor(s / 3600)}:${pad(Math.floor(s / 60) % 60)}:${pad(s % 60)}`;
}

function table(headers: string[], rows: (string | number)[][]): string {
  const line = (cells: (string | number)[]) => `| ${cells.join(" | ")} |`;
  return [line(headers), line(headers.map(() => "---")), ...rows.map(line)].join("\n");
}

function section(title: string, rows: string | null): string {
  return `## ${title}\n\n${rows ?? "없음"}\n`;
}

function main(): void {
  const settings = loadSettings();
  const episodes = loadEpisodes();
  const byNumber = new Map<number, Episode>(episodes.map((e) => [e.number, e]));
  const label = (n: number) => byNumber.get(n)?.label ?? `${n}화`;
  const overrides = loadOverrides();
  const segments = readOptional<Segment[]>(paths.segments) ?? [];
  const download = readOptional<Record<string, { status: string; error?: string | null }>>(
    join(paths.results, "download.json"),
  );
  const encode = readOptional<{ episodes: Record<string, { status: string; error?: string }> }>(
    join(paths.results, "encode.json"),
  );
  const verifyEpisodes = readOptional<{ episodes: EpisodeCheck[] }>(
    join(paths.results, "verify-episodes.json"),
  );
  const verifyFinal = readOptional<{ ok: boolean; failures: string[] }>(
    join(paths.results, "verify-final.json"),
  );
  const chapterFile = readOptional<{ episodes: Chapter[] }>(join(paths.final, "final.chapters.json"));
  const chapters = chapterFile?.episodes ?? [];
  const finalPath = join(paths.final, "final.mp4");
  const included = new Set(chapters.map((c) => c.number));

  const fps = segments[0]?.fps;
  const frameSeconds = fps ? fps.den / fps.num : 0;
  const totalFrames = chapters.reduce((a, c) => a + c.frames, 0);

  const expected = expectedEpisodes(episodes);
  const checkedNumbers = new Set(verifyEpisodes?.episodes.map((e) => e.number) ?? []);
  const failedEpisodes = (verifyEpisodes?.episodes ?? []).filter(
    (e) => !e.ok && included.has(e.number),
  );
  const missing = episodes.filter((e) => !included.has(e.number));
  const missingExpected = expected.filter((e) => !included.has(e.number));
  const unchecked = expected.filter((e) => !checkedNumbers.has(e.number));
  const finalVerifyPath = join(paths.results, "verify-final.json");
  // 최종 파일을 다시 만들었다면 검증도 그 뒤에 다시 돌았어야 한다
  const verifyIsCurrent =
    existsSync(finalPath) &&
    existsSync(finalVerifyPath) &&
    statSync(finalVerifyPath).mtimeMs >= statSync(finalPath).mtimeMs;
  const finalOk =
    Boolean(verifyFinal?.ok) &&
    verifyIsCurrent &&
    failedEpisodes.length === 0 &&
    missingExpected.length === 0 &&
    unchecked.length === 0;

  const out: string[] = [];
  out.push("# 작업 리포트\n");
  out.push(
    finalOk
      ? "**상태: 완성.** 최종 검증을 통과했습니다.\n"
      : "**상태: 미완성.** 최종 검증을 통과하지 못했으므로 완성본으로 보지 않습니다. 아래 실패 항목을 확인하세요.\n",
  );
  out.push(
    table(
      ["항목", "값"],
      [
        ["목록상 본편", `${episodes.length}개`],
        ["최종 파일에 포함", `${chapters.length}개`],
        ["번호를 건너뛴 화", settings.excludedEpisodes.map((n) => `${n}화`).join(", ") || "없음"],
        ["전체 길이", fps ? timecode(totalFrames * frameSeconds) : "측정 전"],
        [
          "파일 크기",
          existsSync(finalPath) ? `${(statSync(finalPath).size / 1e9).toFixed(2)}GB` : "파일 없음",
        ],
        ["파일 경로", "work/final/final.mp4"],
      ],
    ) + "\n",
  );

  out.push(
    section(
      "최종 파일에서 빠진 화",
      missing.length
        ? table(
            ["화", "다운로드", "인코딩", "사유"],
            missing.map((e) => {
              const d = download?.[String(e.number)];
              const c = encode?.episodes[String(e.number)];
              return [
                label(e.number),
                d?.status ?? "기록 없음",
                c?.status ?? "기록 없음",
                c?.error ?? d?.error ?? overrides[String(e.number)]?.reason ?? "",
              ];
            }),
          )
        : null,
    ),
  );

  const overrideEntries = Object.entries(overrides);
  out.push(
    section(
      "수동 판정한 화",
      overrideEntries.length
        ? table(
            ["화", "구간", "제외", "판정 근거"],
            overrideEntries.map(([n, o]) => {
              const s = segments.find((x) => x.number === Number(n));
              return [
                label(Number(n)),
                s ? `${s.startFrame} ~ ${s.endFrame}` : "",
                o.exclude ? "제외" : "",
                o.reason,
              ];
            }),
          )
        : null,
    ),
  );

  const uncut = segments.filter((s) => s.uncut.opening || s.uncut.ending);
  out.push(
    section(
      "자르지 않은 화",
      uncut.length
        ? table(
            ["화", "오프닝", "엔딩"],
            uncut.map((s) => [
              label(s.number),
              s.uncut.opening ? "남김" : "자름",
              s.uncut.ending ? "남김" : "자름",
            ]),
          )
        : null,
    ),
  );

  const flagged = segments.filter((s) => s.flags.length > 0);
  out.push(
    section(
      "1차 자동 검사에서 어긋난 항목",
      flagged.length
        ? table(
            ["화", "항목", "처리"],
            flagged.map((s) => [
              label(s.number),
              s.flags.join(", "),
              s.source === "override" ? "수동 판정" : "자동값 유지",
            ]),
          )
        : null,
    ),
  );

  out.push(
    section(
      "3차 화별 대조 실패",
      failedEpisodes.length
        ? table(
            ["화", "실패 내용"],
            failedEpisodes.map((e) => [label(e.number), e.failures.join("; ")]),
          )
        : verifyEpisodes
          ? null
          : "검사가 실행되지 않았습니다",
    ),
  );

  out.push(
    section(
      "4차 최종 파일 검사 실패",
      !verifyFinal
        ? "검사가 실행되지 않았습니다"
        : verifyFinal.failures.length
          ? verifyFinal.failures.map((f) => `- ${f}`).join("\n")
          : null,
    ),
  );

  if (chapters.length > 0 && fps) {
    const pick = new Map<number, string>();
    const add = (c: Chapter | undefined, why: string) => {
      if (c && !pick.has(c.number)) pick.set(c.number, why);
    };
    add(chapters[0], "처음");
    add(chapters[Math.floor(chapters.length / 2)], "중간");
    add(chapters.at(-1), "마지막 화");
    // 5분 안에 볼 수 있도록 경계를 직접 지정한 화 중 일부와 자르지 않은 화만 고른다
    for (const s of uncut) add(chapters.find((c) => c.number === s.number), "자르지 않음");
    const moved = overrideEntries.filter(([, o]) => o.startFrame !== undefined || o.endFrame !== undefined);
    const step = Math.max(1, Math.ceil(moved.length / 6));
    moved
      .filter((_, i) => i % step === 0)
      .forEach(([n]) => add(chapters.find((c) => c.number === Number(n)), "경계를 수동 지정"));
    const rows = chapters
      .filter((c) => pick.has(c.number))
      .map((c) => [timecode(c.startFrame * frameSeconds), c.label, pick.get(c.number) ?? ""]);
    out.push(
      section(
        "사용자 확인 시점",
        `Claude는 소리를 들을 수 없으므로 아래 시점을 Wi-Fi를 끈 상태에서 재생해 입 모양과 소리, 챕터 이동을 확인하세요.\n\n${table(["시점", "화", "이유"], rows)}`,
      ),
    );
  }

  const notesPath = join(paths.report, "..", "report-notes.md");
  if (existsSync(notesPath)) out.push(readFileSync(notesPath, "utf8").trim() + "\n");

  writeFileSync(paths.report, out.join("\n"));
  console.log(`리포트 저장: ${paths.report}`);
}

if (import.meta.url === `file://${process.argv[1]}`) main();
