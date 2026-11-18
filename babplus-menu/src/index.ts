// 코딩머신 봇 진입점. 상시 실행되면서 매분 기능별 작업을 cron 식에 맞춰 실행한다.
import { contextFactory, loadEnv, log } from './env.ts';
import { features } from './features/index.ts';
import { collectJobs, startScheduler } from './scheduler.ts';

const env = loadEnv();
const dryRun = env.DRY_RUN === '1';
if (!env.DISCORD_BOT_TOKEN && !dryRun) {
  log('DISCORD_BOT_TOKEN 이 없습니다. .env 를 채우거나 DRY_RUN=1 로 실행하세요.');
  process.exit(78);
}

const jobs = collectJobs(features);
log(`코딩머신 시작${dryRun ? ' (dry-run)' : ''}: ${jobs.map((j) => `${j.key} [${j.job.cron}]`).join(', ')}`);
const stop = startScheduler(jobs, contextFactory(env, dryRun));

for (const sig of ['SIGTERM', 'SIGINT'] as const) {
  process.on(sig, () => {
    log(`${sig} 받음, 종료`);
    stop();
    process.exit(0);
  });
}
