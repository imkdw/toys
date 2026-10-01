import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import type { Analysis } from '../src/shared/types.ts';
import { config } from '../src/server/config.ts';
import { correctCard } from '../src/server/image.ts';
import { RESULT_FILE } from '../src/server/jobs.ts';
import sharp from 'sharp';

const ids = process.argv.slice(2).length ? process.argv.slice(2) : await readdir(config.dataDir);
for (const id of ids) {
  const dir = path.join(config.dataDir, id);
  const job = JSON.parse(await readFile(path.join(dir, 'job.json'), 'utf8').catch(() => '{}'));
  if (job.status !== 'done') continue;
  const analysis = JSON.parse(await readFile(path.join(dir, 'analysis.json'), 'utf8')) as Analysis;
  const source = path.join(dir, 'source.png');
  const { width, height } = await sharp(source).metadata();
  await correctCard(source, path.join(dir, RESULT_FILE), analysis, { width: width!, height: height! });
  console.log(`${id} 다시 보정함`);
}
