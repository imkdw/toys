// 7단계: 화수 표시 PNG 생성
//
// 사용: npx tsx src/title-card.ts [--pilot | --episodes 1,5,9]
// 1920x1080 전체 크기의 투명 PNG 를 만든다. encode 단계가 overlay=0:0 으로 얹는다.

import { mkdirSync } from "node:fs";
import sharp from "sharp";
import {
  type Episode,
  loadEpisodes,
  loadSettings,
  paths,
  selectEpisodes,
  TITLE_REGION,
  titleCardPath,
} from "./config.ts";

const WIDTH = 1920;
const HEIGHT = 1080;
const FONT_FILE = "/System/Library/Fonts/AppleSDGothicNeo.ttc";
const FONT_FAMILY = "Apple SD Gothic Neo";
const LEFT = 48;
const TOP = 40;
const PAD_X = 28;
const PAD_Y = 14;
const MAX_FONT = 44;
const MIN_FONT = 24;
const MAX_BOX_WIDTH = WIDTH - LEFT * 2;
const CONCURRENCY = 8;

const escapeXml = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/** 문구를 글자 크기별로 그려 보고 실제 폭을 잰다 */
async function measure(label: string, size: number): Promise<{ buf: Buffer; width: number; height: number }> {
  const { data, info } = await sharp({
    text: {
      text: `<span foreground="white" font_weight="bold">${escapeXml(label)}</span>`,
      font: `${FONT_FAMILY} ${size}`,
      fontfile: FONT_FILE,
      rgba: true,
      dpi: 72,
    },
  })
    .png()
    .toBuffer({ resolveWithObject: true });
  return { buf: data, width: info.width, height: info.height };
}

/** 폭에 맞는 가장 큰 글자 크기로 문구를 그린다 */
async function renderText(label: string) {
  const limit = MAX_BOX_WIDTH - PAD_X * 2;
  let size = MAX_FONT;
  let m = await measure(label, size);
  while (m.width > limit && size > MIN_FONT) {
    size = Math.max(MIN_FONT, Math.floor(size * Math.min(0.95, limit / m.width)));
    m = await measure(label, size);
  }
  if (m.width > limit) throw new Error(`문구가 너무 길어 영역에 들어가지 않습니다: ${label}`);
  return m;
}

/** 불투명 픽셀의 경계 상자를 구한다. 불투명 픽셀이 없으면 null */
export async function opaqueBounds(png: Buffer) {
  const { data, info } = await sharp(png).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  let x0 = info.width, y0 = info.height, x1 = -1, y1 = -1;
  for (let y = 0; y < info.height; y++) {
    for (let x = 0; x < info.width; x++) {
      if (data[(y * info.width + x) * 4 + 3] > 0) {
        if (x < x0) x0 = x;
        if (x > x1) x1 = x;
        if (y < y0) y0 = y;
        if (y > y1) y1 = y;
      }
    }
  }
  return x1 < 0 ? null : { x0, y0, x1, y1 };
}

export async function renderCard(label: string): Promise<Buffer> {
  const text = await renderText(label);
  const boxW = text.width + PAD_X * 2;
  const boxH = text.height + PAD_Y * 2;
  const bg = Buffer.from(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${boxW}" height="${boxH}">` +
      `<rect width="${boxW}" height="${boxH}" rx="${Math.round(boxH / 3)}" fill="rgba(0,0,0,0.6)"/></svg>`,
  );
  const png = await sharp({
    create: { width: WIDTH, height: HEIGHT, channels: 4, background: { r: 0, g: 0, b: 0, alpha: 0 } },
  })
    .composite([
      { input: bg, left: LEFT, top: TOP },
      { input: text.buf, left: LEFT + PAD_X, top: TOP + PAD_Y },
    ])
    .png()
    .toBuffer();

  const b = await opaqueBounds(png);
  const r = TITLE_REGION;
  if (!b || b.x0 < r.x || b.y0 < r.y || b.x1 >= r.x + r.width || b.y1 >= r.y + r.height) {
    throw new Error(`표시가 TITLE_REGION 을 벗어납니다: ${label} ${JSON.stringify(b)}`);
  }
  return png;
}

async function main(): Promise<void> {
  const targets = selectEpisodes(loadEpisodes(), loadSettings());
  mkdirSync(paths.titleCards, { recursive: true });
  let done = 0;
  const queue: Episode[] = [...targets];
  const worker = async () => {
    for (let ep = queue.shift(); ep; ep = queue.shift()) {
      const png = await renderCard(ep.label);
      await sharp(png).toFile(titleCardPath(ep.number));
      done++;
    }
  };
  try {
    await Promise.all(Array.from({ length: CONCURRENCY }, worker));
  } catch (e) {
    console.error(`오류: ${(e as Error).message}`);
    process.exit(1);
  }
  console.log(`화수 표시 ${done}개 생성 (${paths.titleCards})`);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  await main();
}
