import type { WorkerRequest, WorkerResponse } from '../worker.ts';

type Job = Exclude<WorkerRequest, { type: 'warmup' }>;
type Listener = (text: string) => void;

class SegmenterClient {
  private worker = new Worker(new URL('../worker.ts', import.meta.url), { type: 'module' });
  private pending = new Map<number, (r: WorkerResponse) => void>();
  private chain: Promise<unknown> = Promise.resolve();
  private listeners = new Set<Listener>();
  loadedId = -1;
  status = '모델 불러오는 중...';

  constructor() {
    this.worker.onmessage = (e: MessageEvent<WorkerResponse>) => {
      const r = e.data;
      if (r.type === 'status') return this.setStatus(r.text);
      if (r.type === 'ready') return this.setStatus(`모델 준비 완료 (${r.device === 'webgpu' ? 'GPU' : 'CPU'})`);
      if ('id' in r && r.id !== undefined) {
        this.pending.get(r.id)?.(r);
        this.pending.delete(r.id);
      }
    };
    this.worker.postMessage({ type: 'warmup' } satisfies WorkerRequest);
  }

  setStatus(text: string) {
    this.status = text;
    this.listeners.forEach((l) => l(text));
  }

  subscribe(l: Listener) {
    this.listeners.add(l);
    return () => void this.listeners.delete(l);
  }

  private send(req: Job): Promise<WorkerResponse> {
    return new Promise((resolve) => {
      this.pending.set(req.id, resolve);
      this.worker.postMessage(req);
    });
  }

  enqueue<T>(fn: (send: (req: Job) => Promise<WorkerResponse>) => Promise<T>): Promise<T> {
    const p = this.chain.then(() => fn((req) => this.send(req)));
    this.chain = p.catch(() => {});
    return p;
  }
}

export const segmenter = new SegmenterClient();
