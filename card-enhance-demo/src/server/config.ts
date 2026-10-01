import path from 'node:path';

const num = (v: string | undefined, fallback: number) => (v ? Number(v) : fallback);

export const config = {
  port: num(process.env.PORT, 4000),
  host: process.env.HOST ?? '0.0.0.0',
  dataDir: path.resolve(process.env.DATA_DIR ?? 'data/jobs'),
  webDir: path.resolve('dist/web'),
  codexBin: process.env.CODEX_BIN ?? 'codex',
  codexTimeoutMs: num(process.env.CODEX_TIMEOUT_MS, 12 * 60_000),
  concurrency: num(process.env.CONCURRENCY, 2),
  maxUploadBytes: 15 * 1024 * 1024,
};
