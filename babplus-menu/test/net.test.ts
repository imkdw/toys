import assert from 'node:assert/strict';
import { test } from 'node:test';
import { retryingFetch } from '../src/net.ts';

const connectErr = () => Object.assign(new TypeError('fetch failed'), { cause: { code: 'UND_ERR_CONNECT_TIMEOUT' } });

test('연결 오류는 다시 시도하고, 그 밖의 오류는 바로 던진다', async () => {
  let n = 0;
  const flaky = (async () => {
    if (++n < 3) throw connectErr();
    return new Response('ok');
  }) as typeof fetch;
  const res = await retryingFetch(flaky, [0, 0, 0])('https://x');
  assert.equal(await res.text(), 'ok');
  assert.equal(n, 3);

  let m = 0;
  const broken = (async () => {
    m++;
    throw new TypeError('other');
  }) as typeof fetch;
  await assert.rejects(retryingFetch(broken, [0, 0])('https://x'));
  assert.equal(m, 1);
});

test('재시도 횟수를 넘기면 마지막 오류를 던진다', async () => {
  let n = 0;
  const down = (async () => {
    n++;
    throw connectErr();
  }) as typeof fetch;
  await assert.rejects(retryingFetch(down, [0, 0])('https://x'));
  assert.equal(n, 3);
});
