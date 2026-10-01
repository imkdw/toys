import { readdir, readFile, stat } from 'node:fs/promises';
import path from 'node:path';

const args = process.argv.slice(2);
const server = args.find((a) => a.startsWith('--server='))?.slice('--server='.length) ?? 'http://localhost:4000';
const targets = args.filter((a) => !a.startsWith('--'));
if (targets.length === 0) {
  console.error('사용법: pnpm submit <폴더|파일...> [--server=http://localhost:4000]');
  process.exit(1);
}

const files: string[] = [];
for (const t of targets) {
  if ((await stat(t)).isDirectory()) {
    for (const f of (await readdir(t)).sort()) if (/\.(jpe?g|png|webp)$/i.test(f)) files.push(path.join(t, f));
  } else files.push(t);
}

for (const f of files) {
  const form = new FormData();
  form.append('image', new Blob([await readFile(f)]), path.basename(f));
  const res = await fetch(`${server}/api/jobs`, { method: 'POST', body: form });
  const body = await res.json();
  console.log(res.ok ? `${body.id}  ${path.basename(f)}` : `실패  ${path.basename(f)}: ${body.error}`);
}
