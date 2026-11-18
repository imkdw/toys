import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { createDiscord } from './discord.ts';
import { retryingFetch } from './net.ts';
import type { JobContext } from './types.ts';

export const ROOT = fileURLToPath(new URL('..', import.meta.url));

/** 저장소 루트의 .env 를 읽는다. 이미 있는 환경 변수는 덮어쓰지 않는다. */
export function loadEnv(): Record<string, string | undefined> {
  const path = ROOT + '.env';
  if (existsSync(path)) process.loadEnvFile(path);
  return process.env;
}

export function log(msg: string): void {
  console.log(`${new Date().toISOString()} ${msg}`);
}

export function contextFactory(env: Record<string, string | undefined>, dryRun: boolean): (now: Date) => JobContext {
  const f = retryingFetch(fetch, undefined, log);
  const discord = createDiscord({ token: env.DISCORD_BOT_TOKEN, dryRun, log, fetch: f });
  return (now) => ({ now, discord, env, fetch: f, log });
}
