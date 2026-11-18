import assert from 'node:assert/strict';
import { test } from 'node:test';
import { babplus } from '../src/features/babplus/index.ts';
import { cronMatches, parseCron } from '../src/scheduler.ts';
import { parseKst } from '../src/time.ts';

test('식단 전송 cron 은 일/목 20:05 부터 20:55 까지 10분마다 맞는다', () => {
  const cron = parseCron(babplus.jobs[0].cron);
  const hits = ['2026-11-15 20:05', '2026-11-15 20:15', '2026-11-15 20:55', '2026-11-19 20:05'];
  const misses = ['2026-11-15 20:00', '2026-11-15 20:10', '2026-11-15 21:05', '2026-11-16 20:05', '2026-11-20 20:05'];
  for (const s of hits) assert.equal(cronMatches(cron, parseKst(s)), true, s);
  for (const s of misses) assert.equal(cronMatches(cron, parseKst(s)), false, s);
});

test('실패 알림 cron 은 일/목 21:05 에만 맞는다', () => {
  const cron = parseCron(babplus.jobs[1].cron);
  assert.equal(cronMatches(cron, parseKst('2026-11-15 21:05')), true);
  assert.equal(cronMatches(cron, parseKst('2026-11-19 21:05')), true);
  assert.equal(cronMatches(cron, parseKst('2026-11-16 21:05')), false);
});

test('cron 은 서버 시간대가 아니라 KST 로 계산한다', () => {
  // 2026-11-15 11:05 UTC = 일요일 20:05 KST
  assert.equal(cronMatches(parseCron('5 20 * * 0'), new Date('2026-11-15T11:05:00Z')), true);
});

test('cron 식 문법', () => {
  assert.deepEqual([...parseCron('*/15 * * * 7')[0]], [0, 15, 30, 45]);
  assert.deepEqual([...parseCron('* * * * 7')[4]], [0]);
  assert.deepEqual([...parseCron('1,3-5 * * * *')[0]], [1, 3, 4, 5]);
  assert.throws(() => parseCron('* * * *'));
  assert.throws(() => parseCron('60 * * * *'));
});
