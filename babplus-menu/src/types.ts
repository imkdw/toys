import type { Discord } from './discord.ts';

export interface JobContext {
  /** 이 실행의 기준 시각. 스케줄러는 분 단위로 자른 시각을, 수동 실행은 --now 값을 넣는다. */
  now: Date;
  discord: Discord;
  env: Record<string, string | undefined>;
  fetch: typeof fetch;
  log: (msg: string) => void;
}

export interface Job {
  id: string;
  /** KST 기준 5필드 cron 식 */
  cron: string;
  /** 반환한 문자열은 완료 로그에 붙는다. */
  run: (ctx: JobContext) => Promise<string | void>;
}

export interface Feature {
  name: string;
  jobs: Job[];
}
