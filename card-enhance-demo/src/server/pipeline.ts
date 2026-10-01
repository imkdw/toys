import { writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Analysis } from '../shared/types.ts';
import { runCodex } from './codex.ts';
import { correctCard, normalizeInput, safeAdjustments } from './image.ts';
import { RESULT_FILE, saveJob, type Job } from './jobs.ts';
import { buildAnalysisPrompt } from './prompts.ts';

const SCHEMA_FILE = fileURLToPath(new URL('./schema/analysis.json', import.meta.url));

export async function processJob(job: Job): Promise<void> {
  const file = (name: string) => path.join(job.dir, name);
  const setStatus = async (status: Job['status']) => {
    job.status = status;
    await saveJob(job);
  };

  const source = file('source.png');
  const size = await normalizeInput(job.inputFile, source);

  await setStatus('analyzing');
  job.prompt = buildAnalysisPrompt(path.basename(source), size.width, size.height);
  await writeFile(file('prompt.txt'), job.prompt);
  const raw = await runCodex({
    prompt: job.prompt,
    cwd: job.dir,
    image: source,
    schemaFile: SCHEMA_FILE,
    lastMessageFile: file('analysis.json'),
    logFile: file('analysis.log'),
  });
  const analysis = JSON.parse(raw) as Analysis;
  job.analysis = { ...analysis, adjustments: safeAdjustments(analysis.adjustments) };

  await setStatus('correcting');
  await correctCard(source, file(RESULT_FILE), analysis, size);
  await setStatus('done');
}
