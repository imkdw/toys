import type { JobStatus, JobView } from '../src/shared/types.ts';

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const drop = $<HTMLLabelElement>('drop');
const fileInput = $<HTMLInputElement>('file');
const errorBox = $<HTMLParagraphElement>('error');
const list = $<HTMLElement>('jobs');

const jobs = new Map<string, JobView>();
const POLL_MS = 3000;
const STEPS: [JobStatus, string][] = [['queued', '대기'], ['analyzing', '분석 (꼭짓점/색)'], ['correcting', '원근 보정 + 색 보정'], ['done', '완료']];

const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
const isRunning = (j: JobView) => j.status !== 'done' && j.status !== 'failed';

function showError(msg: string | null) {
  errorBox.hidden = !msg;
  errorBox.textContent = msg ?? '';
}

async function upload(files: FileList | File[]) {
  showError(null);
  for (const file of files) {
    const form = new FormData();
    form.append('image', file);
    const res = await fetch('/api/jobs', { method: 'POST', body: form });
    const body = await res.json();
    if (!res.ok) {
      showError(`${file.name}: ${body.error}`);
      continue;
    }
    jobs.set(body.id, body);
  }
  render();
}

function stepsHtml(job: JobView) {
  if (job.status === 'failed') return `<span class="failed">실패</span>`;
  const current = STEPS.findIndex(([s]) => s === job.status);
  return STEPS.map(([, label], i) => `<span class="${i === current ? 'active' : i < current ? 'past' : ''}">${label}</span>`).join('');
}

function elapsed(job: JobView) {
  const end = job.finishedAt ? new Date(job.finishedAt) : new Date();
  return `${Math.round((+end - +new Date(job.createdAt)) / 1000)}초`;
}

function jobHtml(job: JobView) {
  const a = job.analysis;
  const result = job.imageUrl
    ? `<a href="${job.imageUrl}" target="_blank"><img src="${job.imageUrl}" /></a>`
    : `<div class="placeholder">${job.status === 'failed' ? esc(job.error ?? '실패') : '처리 중...'}</div>`;
  return `
    <article class="job" data-id="${job.id}">
      <header>
        <div class="steps">${stepsHtml(job)}</div>
        <span class="meta">#${job.id} / ${esc(job.name)} / ${elapsed(job)}</span>
      </header>
      <div class="pair">
        <figure><a href="${job.originalUrl}" target="_blank"><img src="${job.originalUrl}" /></a><figcaption>원본</figcaption></figure>
        <figure>${result}<figcaption>크롭 + 원근 보정 + 색 보정</figcaption></figure>
      </div>
      ${job.imageUrl ? `<a class="url" href="${job.imageUrl}" target="_blank">${esc(job.imageUrl)}</a>` : ''}
      ${a ? `<details><summary>분석 결과 (꼭짓점, 개선안, 적용한 보정 값)</summary><pre>${esc(JSON.stringify(a, null, 2))}</pre></details>` : ''}
      ${job.prompt ? `<details><summary>codex에 보낸 프롬프트</summary><pre>${esc(job.prompt)}</pre></details>` : ''}
    </article>`;
}

function render() {
  const sorted = [...jobs.values()].sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  const keyOf = (d: Element) => `${d.closest<HTMLElement>('.job')?.dataset.id}:${d.querySelector('summary')?.textContent}`;
  const open = new Set([...list.querySelectorAll('details[open]')].map(keyOf));
  list.innerHTML = sorted.map(jobHtml).join('');
  list.querySelectorAll('details').forEach((d) => {
    if (open.has(keyOf(d))) d.open = true;
  });
}

async function poll() {
  const running = [...jobs.values()].filter(isRunning);
  const updated = await Promise.all(running.map((j) => fetch(`/api/jobs/${j.id}`).then((r) => r.json() as Promise<JobView>)));
  updated.forEach((j) => jobs.set(j.id, j));
  if (running.length > 0) render();
  setTimeout(poll, POLL_MS);
}

drop.addEventListener('dragover', (e) => { e.preventDefault(); drop.classList.add('over'); });
drop.addEventListener('dragleave', () => drop.classList.remove('over'));
drop.addEventListener('drop', (e) => {
  e.preventDefault();
  drop.classList.remove('over');
  if (e.dataTransfer?.files.length) upload(e.dataTransfer.files);
});
fileInput.addEventListener('change', () => {
  if (fileInput.files?.length) upload(fileInput.files);
  fileInput.value = '';
});

const initial: JobView[] = await fetch('/api/jobs').then((r) => r.json());
initial.forEach((j) => jobs.set(j.id, j));
render();
poll();
