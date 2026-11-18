// 작업 하나를 지금 실행한다. 수동 재전송과 시험용.
//   node src/run.ts babplus:send --dry-run --now "2026-11-15 20:05"
import { contextFactory, loadEnv, log } from './env.ts';
import { features } from './features/index.ts';
import { collectJobs, runJob } from './scheduler.ts';
import { parseKst } from './time.ts';

const args = process.argv.slice(2);
const key = args.find((a) => !a.startsWith('--') && args[args.indexOf(a) - 1] !== '--now');
const nowIdx = args.indexOf('--now');
const dryRun = args.includes('--dry-run');
const jobs = collectJobs(features);
const job = jobs.find((j) => j.key === key);
if (!job) {
  console.error(`사용법: node src/run.ts <작업> [--dry-run] [--now "YYYY-MM-DD HH:mm"]\n작업: ${jobs.map((j) => j.key).join(', ')}`);
  process.exit(64);
}
const env = loadEnv();
const now = nowIdx >= 0 ? parseKst(args[nowIdx + 1] ?? '') : new Date();
log(`${job.key} 수동 실행, 기준 시각 ${now.toISOString()}${dryRun ? ' (dry-run)' : ''}`);
await runJob(job, contextFactory(env, dryRun)(now));
