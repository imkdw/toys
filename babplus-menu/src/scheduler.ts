import { kst } from './time.ts';
import type { Feature, Job, JobContext } from './types.ts';

// 5필드 cron 식(분 시 일 월 요일)을 KST 로 해석한다. *, a-b, a,b, */n, a-b/n 을 지원한다.
const RANGES: [number, number][] = [[0, 59], [0, 23], [1, 31], [1, 12], [0, 7]];

export type Cron = Set<number>[];

export function parseCron(expr: string): Cron {
  const fields = expr.trim().split(/\s+/);
  if (fields.length !== 5) throw new Error(`cron 식은 5필드여야 합니다: ${expr}`);
  return fields.map((field, i) => {
    const [lo, hi] = RANGES[i];
    const set = new Set<number>();
    for (const part of field.split(',')) {
      const [range, stepStr] = part.split('/');
      const step = stepStr === undefined ? 1 : Number(stepStr);
      let a: number;
      let b: number;
      if (range === '*') [a, b] = [lo, hi];
      else if (range.includes('-')) [a, b] = range.split('-').map(Number) as [number, number];
      else {
        a = Number(range);
        b = stepStr === undefined ? a : hi;
      }
      if (![a, b, step].every(Number.isInteger) || a < lo || b > hi || a > b || step < 1) {
        throw new Error(`cron 식 해석 실패: ${expr} (${part})`);
      }
      for (let v = a; v <= b; v += step) set.add(i === 4 && v === 7 ? 0 : v);
    }
    return set;
  });
}

export function cronMatches(cron: Cron, date: Date): boolean {
  const p = kst(date);
  const [min, hour, dom, mon, dow] = cron;
  return min.has(p.minute) && hour.has(p.hour) && dom.has(p.day) && mon.has(p.month) && dow.has(p.dow);
}

export interface Scheduled {
  key: string;
  job: Job;
  cron: Cron;
}

export function collectJobs(features: Feature[]): Scheduled[] {
  return features.flatMap((f) =>
    f.jobs.map((job) => ({ key: `${f.name}:${job.id}`, job, cron: parseCron(job.cron) })),
  );
}

/** 작업 하나를 실행한다. 예외는 로그로만 남기고 밖으로 던지지 않는다. */
export async function runJob(s: Scheduled, ctx: JobContext): Promise<void> {
  const started = Date.now();
  try {
    const result = await s.job.run(ctx);
    ctx.log(`[${s.key}] 완료 ${Date.now() - started}ms${result ? ` ${result}` : ''}`);
  } catch (err) {
    ctx.log(`[${s.key}] 실패: ${err instanceof Error ? err.stack ?? err.message : String(err)}`);
  }
}

/** 매분 0초 직후에 깨어나 cron 식이 맞는 작업을 실행한다. 같은 작업이 실행 중이면 건너뛴다. */
export function startScheduler(jobs: Scheduled[], makeCtx: (now: Date) => JobContext): () => void {
  const running = new Set<string>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  let lastMinute = -1;

  const tick = () => {
    const now = new Date();
    const minute = Math.floor(now.getTime() / 60_000);
    if (minute !== lastMinute) {
      lastMinute = minute;
      const slot = new Date(minute * 60_000);
      for (const s of jobs) {
        if (!cronMatches(s.cron, slot)) continue;
        const ctx = makeCtx(slot);
        if (running.has(s.key)) {
          ctx.log(`[${s.key}] 이전 실행이 아직 끝나지 않아 건너뜀`);
          continue;
        }
        running.add(s.key);
        void runJob(s, ctx).finally(() => running.delete(s.key));
      }
    }
    const wait = 60_000 - (Date.now() % 60_000) + 500;
    timer = setTimeout(tick, wait);
  };
  tick();
  return () => clearTimeout(timer);
}
