/// <reference lib="webworker" />
import { RawImage } from '@huggingface/transformers';
import { Segmenter, type PromptPoint } from './core/segmenter.ts';
import { detectCard, scoreMask } from './core/detect.ts';
import { cleanMask, fitQuad, type Quad } from './core/geometry.ts';

export type WorkerRequest =
  | { type: 'image'; id: number; data: Uint8ClampedArray; width: number; height: number; detect: boolean }
  | { type: 'points'; id: number; points: PromptPoint[] }
  | { type: 'warmup' };

export type WorkerResponse =
  | { type: 'status'; text: string }
  | { type: 'ready'; device: string }
  | { type: 'embedded'; id: number }
  | { type: 'result'; id: number; quad: Quad | null; mask: Uint8Array | null }
  | { type: 'candidates'; id: number; options: Candidate[]; selected: number }
  | { type: 'error'; id?: number; text: string };

export interface Candidate {
  mask: Uint8Array;
  quad: Quad;
}

const post = (msg: WorkerResponse, transfer: Transferable[] = []) => self.postMessage(msg, { transfer });

let segP: Promise<Segmenter> | null = null;
let size = { width: 0, height: 0 };
let currentId = -1;

async function getSegmenter(): Promise<Segmenter> {
  if (!segP) {
    segP = (async () => {
      const hasGPU = 'gpu' in navigator && !!(await (navigator as any).gpu?.requestAdapter().catch(() => null));
      const device = hasGPU ? 'webgpu' : 'wasm';
      const loaded = new Map<string, number>();
      const seg = await Segmenter.load(device, (p: any) => {
        if (p.status === 'progress' && p.total) {
          loaded.set(p.file, p.loaded / p.total);
          const avg = [...loaded.values()].reduce((a, b) => a + b, 0) / loaded.size;
          post({ type: 'status', text: `모델 다운로드 ${Math.round(avg * 100)}% (최초 1회)` });
        }
      });
      post({ type: 'ready', device });
      return seg;
    })();
  }
  return segP;
}

self.onmessage = async (e: MessageEvent<WorkerRequest>) => {
  const msg = e.data;
  try {
    const seg = await getSegmenter();
    if (msg.type === 'warmup') return;
    if (msg.type === 'image') {
      currentId = msg.id;
      size = { width: msg.width, height: msg.height };
      post({ type: 'status', text: '이미지 분석 중...' });
      await seg.setImage(new RawImage(msg.data, msg.width, msg.height, 4).rgb());
      if (!msg.detect) return post({ type: 'embedded', id: msg.id });
      post({ type: 'status', text: '카드 찾는 중...' });
      const det = await detectCard(seg, size.width, size.height);
      post({ type: 'result', id: msg.id, quad: det?.fit.quad ?? null, mask: det?.mask ?? null });
    } else {
      if (msg.id !== currentId) throw new Error('다른 이미지가 로드되어 있습니다');
      const { width: w, height: h } = size;
      const candidates = await seg.predict(msg.points);
      const options: Candidate[] = [];
      let bestIdx = -1;
      let bestScore = -Infinity;
      for (const c of candidates) {
        const mask = cleanMask(c.mask, w, h);
        const fit = fitQuad(mask, w, h);
        if (!fit) continue;
        const s = scoreMask(c.mask, w, h)?.score ?? 0;
        if (s > bestScore) (bestScore = s), (bestIdx = options.length);
        options.push({ mask, quad: fit.quad });
      }
      post({ type: 'candidates', id: msg.id, options, selected: Math.max(0, bestIdx) });
    }
  } catch (err) {
    post({ type: 'error', id: (msg as any).id, text: String((err as Error)?.message ?? err) });
  }
};
