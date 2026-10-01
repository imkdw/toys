import { spawn } from 'node:child_process';
import { createWriteStream } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { config } from './config.ts';

interface RunCodexOptions {
  prompt: string;
  cwd: string;
  image: string;
  lastMessageFile: string;
  logFile: string;
  schemaFile?: string;
}

export async function runCodex(opts: RunCodexOptions): Promise<string> {
  const args = [
    'exec',
    '-s', 'workspace-write',
    '--skip-git-repo-check',
    '--ephemeral',
    '-C', opts.cwd,
    '-i', opts.image,
    '-o', opts.lastMessageFile,
  ];
  if (opts.schemaFile) args.push('--output-schema', opts.schemaFile);
  args.push('-');

  const log = createWriteStream(opts.logFile);
  const child = spawn(config.codexBin, args, { cwd: opts.cwd, stdio: ['pipe', 'pipe', 'pipe'] });
  child.stdout.pipe(log);
  child.stderr.pipe(log);
  child.stdin.end(opts.prompt);

  const timer = setTimeout(() => child.kill('SIGTERM'), config.codexTimeoutMs);
  const code = await new Promise<number | null>((resolve, reject) => {
    child.on('error', reject);
    child.on('close', resolve);
  }).finally(() => clearTimeout(timer));

  if (code !== 0) throw new Error(`codex exec 종료 코드 ${code} (로그: ${opts.logFile})`);
  return (await readFile(opts.lastMessageFile, 'utf8')).trim();
}
