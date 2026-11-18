import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import type { Discord, Message, OutgoingMessage } from '../src/discord.ts';
import { reportMiss, sendMenu } from '../src/features/babplus/index.ts';
import { parseMenuTitle, pickMenuImages, pickMenuPost, POSTS_URL } from '../src/features/babplus/kakao.ts';
import type { KakaoPost } from '../src/features/babplus/kakao.ts';
import { parseKst } from '../src/time.ts';
import type { JobContext } from '../src/types.ts';

// 2026-11-17 23:50 KST 실제 응답. 11/16 (월) 메뉴 글은 음식 사진이 붙어 12장이다.
const raw = readFileSync(new URL('./fixtures/posts-2026-11-17.json', import.meta.url), 'utf8');
const posts = (JSON.parse(raw) as { items: KakaoPost[] }).items;

test('제목 판별: 중식 메뉴 글만 맞고 석식메뉴 글은 뺀다', () => {
  assert.deepEqual(parseMenuTitle('11/16 (월) 메뉴'), { month: 11, day: 16, dow: '월' });
  assert.equal(parseMenuTitle('11/16 (월) 석식메뉴'), null);
  assert.equal(parseMenuTitle(''), null);
});

test('내일 자 월/금 글만 고른다', () => {
  assert.equal(pickMenuPost(posts, { month: 11, day: 16 }, ['월', '금'])?.id, 115242367);
  // 11/18 은 수요일이라 대상이 아니다
  assert.equal(pickMenuPost(posts, { month: 11, day: 18 }, ['월', '금']), null);
  assert.equal(pickMenuPost(posts, { month: 11, day: 18 }, ['수'])?.title, '11/18 (수) 메뉴');
});

test('음식 사진이 붙은 뒤에도 메뉴판 2장만 고른다', () => {
  const mon = posts.find((p) => p.id === 115242367)!;
  assert.equal(mon.media!.length, 12);
  const picked = pickMenuImages(mon.media);
  assert.equal(picked.length, 2);
  assert.ok(picked.every((m) => m.width === 793));
});

test('메뉴판 크기 이미지가 없으면 앞에서부터 10장', () => {
  const photos = Array.from({ length: 12 }, (_, i) => ({ type: 'image', xlarge_url: `u${i}`, width: 3024, height: 4032 }));
  assert.equal(pickMenuImages(photos).length, 10);
});

// 가짜 Discord 채널. 보낸 메시지를 쌓고 recent 로 돌려준다.
function fakeDiscord(): Discord & { sent: OutgoingMessage[] } {
  const sent: OutgoingMessage[] = [];
  return {
    sent,
    async send(_ch, msg) {
      sent.push(msg);
      return null;
    },
    async recent() {
      return sent.map((m, i): Message => ({ id: String(i), content: m.content ?? '', author: { id: 'me', bot: true }, embeds: m.embeds ?? [] }));
    },
  };
}

function ctxAt(now: string, discord: Discord, items: unknown = { items: posts }): JobContext {
  const f = (async (url: string | URL | Request) => {
    if (String(url) === POSTS_URL) return new Response(JSON.stringify(items));
    return new Response(new Uint8Array([1, 2, 3]), { headers: { 'content-type': 'image/jpeg' } });
  }) as typeof fetch;
  return { now: parseKst(now), discord, env: { BABPLUS_CHANNEL_ID: 'ch' }, fetch: f, log: () => {} };
}

test('일요일 20:05 에 월요일 메뉴판을 보내고, 다음 실행에서는 다시 보내지 않는다', async () => {
  const d = fakeDiscord();
  assert.equal(await sendMenu(ctxAt('2026-11-15 20:05', d)), 'sent');
  assert.equal(await sendMenu(ctxAt('2026-11-15 20:15', d)), 'already-sent');
  assert.equal(d.sent.length, 1);
  const msg = d.sent[0];
  assert.equal(msg.content, '🍱 11/16 (월) 밥풀러스 식단');
  assert.equal(msg.files!.length, 2);
  assert.equal(msg.embeds![0].url, 'http://pf.kakao.com/_hqGbn/115242367');
  assert.match(msg.embeds![0].footer!.text, /babplus:post:115242367 \/ date:2026-11-16/);
  // 21:05 실패 알림 작업도 이미 보냈으면 아무것도 하지 않는다
  assert.equal(await reportMiss(ctxAt('2026-11-15 21:05', d)), 'already-sent');
  assert.equal(d.sent.length, 1);
});

test('월요일/금요일 전날이 아니면 아무것도 하지 않는다', async () => {
  const d = fakeDiscord();
  assert.equal(await sendMenu(ctxAt('2026-11-17 20:05', d)), 'not-target-day');
  assert.equal(d.sent.length, 0);
});

test('목요일 밤에 금요일 글이 없으면 21:05 에 실패 알림을 한 번만 보낸다', async () => {
  const d = fakeDiscord();
  assert.equal(await sendMenu(ctxAt('2026-11-19 20:05', d)), 'no-post');
  assert.equal(await reportMiss(ctxAt('2026-11-19 21:05', d)), 'no-post, 실패 알림 보냄');
  assert.equal(await reportMiss(ctxAt('2026-11-19 21:05', d)), 'no-post, 실패 알림은 이미 보냄');
  assert.equal(d.sent.length, 1);
  assert.equal(d.sent[0].embeds![0].title, '11/20 (금) 식단을 보내지 못했습니다');
});

test('API 구조가 바뀌면 실패 알림에 응답 앞부분을 붙인다', async () => {
  const d = fakeDiscord();
  assert.equal(await sendMenu(ctxAt('2026-11-19 20:05', d, { data: [] })), 'error');
  await reportMiss(ctxAt('2026-11-19 21:05', d, { data: [] }));
  assert.match(d.sent[0].embeds![0].description!, /items 가 없음/);
});
