import sharp, { type Sharp } from 'sharp';
import type { Adjustments, Analysis, Point } from '../shared/types.ts';

export async function normalizeInput(input: string, output: string) {
  const info = await sharp(input).rotate().png().toFile(output);
  return { width: info.width, height: info.height };
}

const dist = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y);
const clamp = (v: number, min: number, max: number) => Math.min(max, Math.max(min, Number.isFinite(v) ? v : 1));

export function orderCorners(points: Point[], topFacing: Analysis['cardTopFacing']): Point[] {
  const cx = points.reduce((s, p) => s + p.x, 0) / 4;
  const cy = points.reduce((s, p) => s + p.y, 0) / 4;
  const clockwise = [...points].sort((a, b) => Math.atan2(a.y - cy, a.x - cx) - Math.atan2(b.y - cy, b.x - cx));
  const tl = clockwise.reduce((best, p, i) => (p.x + p.y < clockwise[best]!.x + clockwise[best]!.y ? i : best), 0);
  const image = [0, 1, 2, 3].map((k) => clockwise[(tl + k) % 4]!);
  const shift = { up: 0, right: 1, down: 2, left: 3 }[topFacing];
  return [0, 1, 2, 3].map((k) => image[(shift + k) % 4]!);
}

function squareToQuad([p0, p1, p2, p3]: Point[]) {
  const dx1 = p1!.x - p2!.x, dx2 = p3!.x - p2!.x, dx3 = p0!.x - p1!.x + p2!.x - p3!.x;
  const dy1 = p1!.y - p2!.y, dy2 = p3!.y - p2!.y, dy3 = p0!.y - p1!.y + p2!.y - p3!.y;
  const det = dx1 * dy2 - dx2 * dy1;
  const g = (dx3 * dy2 - dx2 * dy3) / det;
  const h = (dx1 * dy3 - dx3 * dy1) / det;
  const a = p1!.x - p0!.x + g * p1!.x, b = p3!.x - p0!.x + h * p3!.x, c = p0!.x;
  const d = p1!.y - p0!.y + g * p1!.y, e = p3!.y - p0!.y + h * p3!.y, f = p0!.y;
  return (u: number, v: number): Point => {
    const w = g * u + h * v + 1;
    return { x: (a * u + b * v + c) / w, y: (d * u + e * v + f) / w };
  };
}

interface WarpOptions {
  corners: Point[];
  width: number;
  height: number;
  inset: number;
}

async function warp(source: string, opts: WarpOptions) {
  const { data: src, info } = await sharp(source).removeAlpha().raw().toBuffer({ resolveWithObject: true });
  const map = squareToQuad(opts.corners);
  const out = Buffer.alloc(opts.width * opts.height * 3);
  const span = 1 - 2 * opts.inset;
  const px = (x: number, y: number, c: number) => src[(y * info.width + x) * 3 + c]!;

  for (let y = 0; y < opts.height; y++) {
    for (let x = 0; x < opts.width; x++) {
      const s = map(opts.inset + ((x + 0.5) / opts.width) * span, opts.inset + ((y + 0.5) / opts.height) * span);
      const sx = clamp(s.x - 0.5, 0, info.width - 1.001), sy = clamp(s.y - 0.5, 0, info.height - 1.001);
      const x0 = Math.floor(sx), y0 = Math.floor(sy), fx = sx - x0, fy = sy - y0;
      for (let c = 0; c < 3; c++) {
        const top = px(x0, y0, c) * (1 - fx) + px(x0 + 1, y0, c) * fx;
        const bottom = px(x0, y0 + 1, c) * (1 - fx) + px(x0 + 1, y0 + 1, c) * fx;
        out[(y * opts.width + x) * 3 + c] = Math.round(top * (1 - fy) + bottom * fy);
      }
    }
  }
  return sharp(out, { raw: { width: opts.width, height: opts.height, channels: 3 } });
}

export function safeAdjustments(a: Adjustments): Adjustments {
  return {
    brightness: clamp(a.brightness, 0.85, 1.4),
    contrast: clamp(a.contrast, 0.85, 1.25),
    saturation: clamp(a.saturation, 0.85, 1.25),
    redGain: clamp(a.redGain, 0.88, 1.12),
    greenGain: clamp(a.greenGain, 0.88, 1.12),
    blueGain: clamp(a.blueGain, 0.88, 1.12),
  };
}

function percentile(hist: Uint32Array, total: number, q: number) {
  let acc = 0;
  for (let v = 0; v < 256; v++) {
    acc += hist[v]!;
    if (acc >= total * q) return v;
  }
  return 255;
}

const mix = (a: number, b: number, t: number) => a + (b - a) * t;

async function autoLevels(image: Sharp) {
  const { data, info } = await image.raw().toBuffer({ resolveWithObject: true });
  const hists = [0, 1, 2].map(() => new Uint32Array(256));
  for (let i = 0; i < data.length; i += 3) for (let c = 0; c < 3; c++) hists[c]![data[i + c]!]++;
  const total = info.width * info.height;
  const lo = hists.map((h) => percentile(h, total, 0.005));
  const hi = hists.map((h) => percentile(h, total, 0.995));
  const loAll = Math.min(...lo), hiAll = Math.max(...hi);

  const PER_CHANNEL = 0.6;
  const MAX_GAIN = 1.8;
  const scale: number[] = [], offset: number[] = [];
  for (let c = 0; c < 3; c++) {
    const l = mix(loAll, lo[c]!, PER_CHANNEL), h = mix(hiAll, hi[c]!, PER_CHANNEL);
    const k = Math.min(MAX_GAIN, 255 / Math.max(1, h - l));
    scale.push(k);
    offset.push(-l * k);
  }
  return sharp(data, { raw: info }).linear(scale, offset).png().toBuffer();
}

async function adjust(image: Sharp, a: Adjustments) {
  const c = a.contrast;
  const gained = await image
    .linear([c * a.redGain, c * a.greenGain, c * a.blueGain], [128 * (1 - c), 128 * (1 - c), 128 * (1 - c)])
    .png()
    .toBuffer();
  return sharp(gained).modulate({ brightness: a.brightness, saturation: a.saturation * 1.12 });
}

function roundedMask(width: number, height: number, radius: number) {
  return Buffer.from(`<svg width="${width}" height="${height}"><rect width="${width}" height="${height}" rx="${radius}" ry="${radius}" fill="#fff"/></svg>`);
}

export async function correctCard(source: string, output: string, a: Analysis, size: { width: number; height: number }) {
  if (a.corners.length !== 4) throw new Error(`꼭짓점이 4개가 아님: ${a.corners.length}`);
  const corners = orderCorners(
    a.corners.map((p) => ({ x: clamp(p.x, 0, 1) * size.width, y: clamp(p.y, 0, 1) * size.height })),
    a.cardTopFacing,
  );
  const [tl, tr, br, bl] = corners as [Point, Point, Point, Point];

  const w = Math.max(dist(tl, tr), dist(bl, br));
  const measured = (dist(tl, tr) + dist(bl, br)) / (dist(tl, bl) + dist(tr, br));
  const ratio = Math.abs(measured - a.cardAspectRatio) / a.cardAspectRatio < 0.12 ? a.cardAspectRatio : measured;
  const width = Math.round(w);
  const height = Math.round(w / ratio);
  if (width < 50 || height < 50) throw new Error(`카드 영역이 너무 작음: ${width}x${height}`);

  const warped = await warp(source, { corners, width, height, inset: 0.006 });
  const leveled = sharp(await autoLevels(warped));
  const adjusted = (await adjust(leveled, safeAdjustments(a.adjustments))).sharpen({ sigma: 0.7 });
  const radius = Math.round(clamp(a.cornerRadiusRatio, 0, 0.12) * width);
  await adjusted
    .ensureAlpha()
    .composite([{ input: roundedMask(width, height, radius), blend: 'dest-in' }])
    .png()
    .toFile(output);
  return { width, height };
}
