import { networkInterfaces } from 'node:os';
import { serve } from '@hono/node-server';
import { serveStatic } from '@hono/node-server/serve-static';
import { Hono } from 'hono';
import { config } from './config.ts';
import { detectImageType } from './image-type.ts';
import { createJob, getJob, listJobs, loadJobs, setWorker, toView } from './jobs.ts';
import { processJob } from './pipeline.ts';

setWorker(processJob);
await loadJobs();

const app = new Hono();
const origin = (url: string) => new URL(url).origin;

app.post('/api/jobs', async (c) => {
  const body = await c.req.parseBody();
  const image = body.image;
  if (!(image instanceof File)) return c.json({ error: 'image 필드에 파일을 담아 보내야 합니다' }, 400);

  if (image.size > config.maxUploadBytes) return c.json({ error: '파일이 15MB를 넘습니다' }, 413);
  const data = await image.arrayBuffer();
  const ext = detectImageType(new Uint8Array(data));
  if (!ext) return c.json({ error: '지원하지 않는 형식입니다 (jpg/png/webp만 가능)' }, 400);

  const job = await createJob(data, ext, image.name);
  return c.json(toView(job, origin(c.req.url)), 202);
});

app.get('/api/jobs/:id', (c) => {
  const job = getJob(c.req.param('id'));
  if (!job) return c.json({ error: '작업을 찾을 수 없습니다' }, 404);
  return c.json(toView(job, origin(c.req.url)));
});

app.get('/api/jobs', (c) => c.json(listJobs().map((j) => toView(j, origin(c.req.url)))));

app.use('/files/*', serveStatic({ root: config.dataDir, rewriteRequestPath: (p) => p.replace(/^\/files/, '') }));
app.use('/*', serveStatic({ root: config.webDir }));

serve({ fetch: app.fetch, port: config.port, hostname: config.host }, () => {
  const lan = Object.values(networkInterfaces())
    .flat()
    .filter((i) => i && i.family === 'IPv4' && !i.internal)
    .map((i) => `http://${i!.address}:${config.port}`);
  console.log(`card-enhance-demo 실행 중`);
  console.log(`  로컬:   http://localhost:${config.port}`);
  for (const url of lan) console.log(`  네트워크: ${url}`);
});
