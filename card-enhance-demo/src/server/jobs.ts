import { randomUUID } from 'node:crypto';
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import type { Analysis, JobStatus, JobView } from '../shared/types.ts';
import { config } from './config.ts';

export interface Job {
  id: string;
  name: string;
  dir: string;
  inputFile: string;
  status: JobStatus;
  createdAt: Date;
  finishedAt: Date | null;
  analysis: Analysis | null;
  prompt: string | null;
  error: string | null;
}

export const RESULT_FILE = 'result.png';
const JOB_FILE = 'job.json';

const jobs = new Map<string, Job>();
const queue: Job[] = [];
let running = 0;
let worker: (job: Job) => Promise<void>;

export function setWorker(fn: (job: Job) => Promise<void>) {
  worker = fn;
}

export async function saveJob(job: Job) {
  const { dir, ...rest } = job;
  await writeFile(path.join(dir, JOB_FILE), JSON.stringify(rest, null, 2));
}

export async function loadJobs() {
  await mkdir(config.dataDir, { recursive: true });
  for (const id of await readdir(config.dataDir)) {
    const dir = path.join(config.dataDir, id);
    const saved = await readFile(path.join(dir, JOB_FILE), 'utf8').catch(() => null);
    if (!saved) continue;
    const data = JSON.parse(saved);
    const job: Job = { ...data, dir, createdAt: new Date(data.createdAt), finishedAt: data.finishedAt ? new Date(data.finishedAt) : null };
    if (job.status !== 'done' && job.status !== 'failed') {
      job.status = 'failed';
      job.error = '서버 재시작으로 중단됨';
      await saveJob(job);
    }
    jobs.set(job.id, job);
  }
}

export async function createJob(data: ArrayBuffer, ext: string, name: string): Promise<Job> {
  const id = randomUUID().slice(0, 8);
  const dir = path.join(config.dataDir, id);
  await mkdir(dir, { recursive: true });
  const inputFile = path.join(dir, `input.${ext}`);
  await writeFile(inputFile, Buffer.from(data));

  const job: Job = {
    id, name, dir, inputFile,
    status: 'queued',
    createdAt: new Date(),
    finishedAt: null,
    analysis: null,
    prompt: null,
    error: null,
  };
  await saveJob(job);
  jobs.set(id, job);
  queue.push(job);
  drain();
  return job;
}

export const getJob = (id: string) => jobs.get(id);
export const listJobs = () => [...jobs.values()].sort((a, b) => +b.createdAt - +a.createdAt);

function drain() {
  while (running < config.concurrency && queue.length > 0) {
    const job = queue.shift()!;
    running++;
    worker(job)
      .catch((err: unknown) => {
        job.status = 'failed';
        job.error = err instanceof Error ? err.message : String(err);
      })
      .finally(async () => {
        job.finishedAt = new Date();
        await saveJob(job).catch(() => {});
        running--;
        drain();
      });
  }
}

export function toView(job: Job, origin: string): JobView {
  const fileUrl = (name: string) => `${origin}/files/${job.id}/${name}`;
  return {
    id: job.id,
    name: job.name,
    status: job.status,
    createdAt: job.createdAt.toISOString(),
    finishedAt: job.finishedAt?.toISOString() ?? null,
    originalUrl: fileUrl(path.basename(job.inputFile)),
    imageUrl: job.status === 'done' ? fileUrl(RESULT_FILE) : null,
    analysis: job.analysis,
    prompt: job.prompt,
    error: job.error,
  };
}
