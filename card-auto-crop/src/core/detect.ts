import type { Segmenter, PromptPoint } from './segmenter.ts';
import { cleanMask, fitQuad, maskArea, type QuadFit } from './geometry.ts';

export interface Detection {
  mask: Uint8Array;
  fit: QuadFit;
  score: number;
  prompt: PromptPoint;
}

function edgesTouched(mask: Uint8Array, w: number, h: number): number {
  let top = 0, bottom = 0, left = 0, right = 0;
  for (let x = 0; x < w; x++) (top += mask[x]), (bottom += mask[(h - 1) * w + x]);
  for (let y = 0; y < h; y++) (left += mask[y * w]), (right += mask[y * w + w - 1]);
  return [top / w, bottom / w, left / h, right / h].filter((r) => r > 0.05).length;
}

export function scoreMask(raw: Uint8Array, w: number, h: number): { mask: Uint8Array; fit: QuadFit; score: number } | null {
  const edges = edgesTouched(raw, w, h);
  const mask = cleanMask(raw, w, h);
  const frac = maskArea(mask) / (w * h);
  if (frac < 0.04) return null;
  const fit = fitQuad(mask, w, h);
  if (!fit) return null;
  let score = Math.pow(fit.rectangularity, 6) * Math.sqrt(frac);
  if (edges >= 3) score *= 0.02;
  else if (edges === 2) score *= 0.2;
  else if (edges === 1) score *= 0.6;
  return { mask, fit, score };
}

function overlap(inner: Uint8Array, outer: Uint8Array): number {
  let both = 0;
  let a = 0;
  for (let i = 0; i < inner.length; i++) if (inner[i]) (a++, (both += outer[i]));
  return a ? both / a : 0;
}

export async function detectCard(seg: Segmenter, w: number, h: number, grid = 4): Promise<Detection | null> {
  const all: Detection[] = [];
  for (let gy = 0; gy < grid; gy++) {
    for (let gx = 0; gx < grid; gx++) {
      const prompt: PromptPoint = { x: 0.2 + (0.6 * gx) / (grid - 1), y: 0.2 + (0.6 * gy) / (grid - 1), label: 1 };
      for (const c of await seg.predict([prompt])) {
        const s = scoreMask(c.mask, w, h);
        if (s) all.push({ ...s, prompt });
      }
    }
  }
  if (!all.length) return null;
  all.sort((a, b) => b.score - a.score);
  let best = all[0];
  for (let changed = true; changed; ) {
    changed = false;
    const bestArea = maskArea(best.mask);
    for (const c of all) {
      if (c === best || c.fit.rectangularity < 0.97 || c.score < best.score * 0.4) continue;
      const area = maskArea(c.mask);
      if (area >= bestArea * 0.95 || area < bestArea * 0.3) continue;
      if (overlap(c.mask, best.mask) > 0.97) {
        best = c;
        changed = true;
        break;
      }
    }
  }
  return best;
}
