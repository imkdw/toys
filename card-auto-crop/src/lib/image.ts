import { cutoutMask, warpQuad, type Quad, type RGBAImage } from '../core/geometry.ts';

export type OutputMode = 'warp' | 'cutout';

export const PROC_MAX = 1024;

export async function decode(file: Blob, max = Infinity): Promise<{ img: RGBAImage; scale: number }> {
  const bmp = await createImageBitmap(file, { imageOrientation: 'from-image' });
  const scale = Math.min(1, max / Math.max(bmp.width, bmp.height));
  const w = Math.round(bmp.width * scale);
  const h = Math.round(bmp.height * scale);
  const c = new OffscreenCanvas(w, h);
  const cx = c.getContext('2d')!;
  cx.drawImage(bmp, 0, 0, w, h);
  bmp.close();
  return { img: { data: cx.getImageData(0, 0, w, h).data, width: w, height: h }, scale };
}

function resizeMask(m: Uint8Array, w: number, h: number, W: number, H: number): Uint8Array {
  const out = new Uint8Array(W * H);
  for (let y = 0; y < H; y++) {
    const sy = Math.min(h - 1, Math.floor((y * h) / H));
    for (let x = 0; x < W; x++) out[y * W + x] = m[sy * w + Math.min(w - 1, Math.floor((x * w) / W))];
  }
  return out;
}

export function renderOutput(
  src: RGBAImage,
  quad: Quad,
  mask: Uint8Array | null,
  maskW: number,
  maskH: number,
  mode: OutputMode,
): RGBAImage {
  const s = src.width / maskW;
  if (mode === 'cutout' && mask) {
    const full = s === 1 ? mask : resizeMask(mask, maskW, maskH, src.width, src.height);
    return cutoutMask(src, full);
  }
  return warpQuad(src, quad.map((p) => ({ x: p.x * s, y: p.y * s })) as Quad);
}

export function toImageData(img: RGBAImage): ImageData {
  return new ImageData(new Uint8ClampedArray(img.data), img.width, img.height);
}

export async function toPng(img: RGBAImage): Promise<Blob> {
  const c = new OffscreenCanvas(img.width, img.height);
  c.getContext('2d')!.putImageData(toImageData(img), 0, 0);
  return c.convertToBlob({ type: 'image/png' });
}

export function download(blob: Blob, name: string) {
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}
