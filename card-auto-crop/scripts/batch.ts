import { mkdir, readdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import sharp from 'sharp';
import { RawImage } from '@huggingface/transformers';
import { Segmenter } from '../src/core/segmenter.ts';
import { warpQuad, type RGBAImage } from '../src/core/geometry.ts';
import { detectCard } from '../src/core/detect.ts';

const args = process.argv.slice(2);
const debug = args.includes('--debug');
const noSave = args.includes('--no-save');
const jsonPath = args.find((a) => a.startsWith('--json='))?.slice('--json='.length);
const [inDir = 'images', outDir = 'out'] = args.filter((a) => !a.startsWith('--'));
const exts = new Set(['.jpg', '.jpeg', '.png', '.webp']);

async function main() {
  if (!noSave) await mkdir(outDir, { recursive: true });
  if (debug) await mkdir(path.join(outDir, 'debug'), { recursive: true });

  const files = (await readdir(inDir)).filter((f) => exts.has(path.extname(f).toLowerCase())).sort();
  console.log(`모델 로딩 중... (${files.length}장)`);
  const seg = await Segmenter.load('cpu');

  let ok = 0;
  const detections: Record<string, unknown>[] = [];
  for (const [i, file] of files.entries()) {
    const t0 = Date.now();
    const { data, info } = await sharp(path.join(inDir, file))
      .rotate()
      .ensureAlpha()
      .raw()
      .toBuffer({ resolveWithObject: true });
    const w = info.width;
    const h = info.height;
    const rgba: RGBAImage = { data: new Uint8ClampedArray(data), width: w, height: h };

    await seg.setImage(new RawImage(rgba.data, w, h, 4).rgb());
    const chosen = await detectCard(seg, w, h);
    const base = path.parse(file).name;
    detections.push({
      file,
      width: w,
      height: h,
      quad: chosen?.fit.quad.map((p) => ({ x: Math.round(p.x * 10) / 10, y: Math.round(p.y * 10) / 10 })) ?? null,
      rectangularity: chosen ? Math.round(chosen.fit.rectangularity * 1000) / 1000 : 0,
    });
    if (!chosen) {
      console.log(`[${i + 1}/${files.length}] ${file} 실패: 카드 못 찾음`);
      continue;
    }
    if (!noSave) {
      const card = warpQuad(rgba, chosen.fit.quad);
      await sharp(Buffer.from(card.data), { raw: { width: card.width, height: card.height, channels: 4 } })
        .png()
        .toFile(path.join(outDir, `${base}.png`));
    }

    if (debug) {
      const dbg = Buffer.from(rgba.data);
      for (let p = 0; p < w * h; p++) if (!chosen.mask[p]) for (let c = 0; c < 3; c++) dbg[p * 4 + c] >>= 2;
      const q = chosen.fit.quad;
      const svg = `<svg width="${w}" height="${h}" xmlns="http://www.w3.org/2000/svg"><polygon points="${q
        .map((p) => `${p.x},${p.y}`)
        .join(' ')}" fill="none" stroke="#0f0" stroke-width="${Math.max(3, w / 250)}"/></svg>`;
      const composed = await sharp(dbg, { raw: { width: w, height: h, channels: 4 } })
        .composite([{ input: Buffer.from(svg) }])
        .png()
        .toBuffer();
      await sharp(composed)
        .resize({ width: 400 })
        .jpeg()
        .toFile(path.join(outDir, 'debug', `${base}.jpg`));
    }
    ok++;
    console.log(
      `[${i + 1}/${files.length}] ${file} 점수=${chosen.score.toFixed(2)} 사각형도=${chosen.fit.rectangularity.toFixed(2)} ${
        Date.now() - t0
      }ms`,
    );
  }
  if (jsonPath) {
    await mkdir(path.dirname(jsonPath), { recursive: true });
    await writeFile(jsonPath, JSON.stringify({ dir: inDir, items: detections }, null, 1));
    console.log(`감지 결과 저장: ${jsonPath}`);
  }
  console.log(`완료: ${ok}/${files.length}${noSave ? '' : ` -> ${outDir}/`}`);
}

main();
