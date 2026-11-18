import { hasMarker } from '../../discord.ts';
import type { Attachment } from '../../discord.ts';
import { addDays, kst, ymd } from '../../time.ts';
import type { Feature, JobContext } from '../../types.ts';
import { fetchPosts, pickMenuImages, pickMenuPost } from './kakao.ts';
import type { KakaoMedia, KakaoPost } from './kakao.ts';

export const NAME = 'babplus';
/** 알림을 보낼 식단 요일. 제목의 요일로 판별한다. */
export const DAYS: readonly string[] = ['월', '금'];
const DOW = ['일', '월', '화', '수', '목', '금', '토'];

// 마지막 조회 실패 내용. 실패 알림에 붙인다. 날짜(YYYY-MM-DD)별로 둔다.
const lastErrors = new Map<string, string>();

interface Target {
  date: string; // YYYY-MM-DD
  month: number;
  day: number;
  label: string; // "11/16 (월)"
}

function tomorrow(now: Date): Target {
  const p = kst(addDays(now, 1));
  return { date: ymd(p), month: p.month, day: p.day, label: `${p.month}/${p.day} (${DOW[p.dow]})` };
}

function channelOf(ctx: JobContext): string {
  const id = ctx.env.BABPLUS_CHANNEL_ID;
  if (!id) throw new Error('BABPLUS_CHANNEL_ID 가 없습니다.');
  return id;
}

const postMarker = (post: KakaoPost) => `${NAME}:post:${post.id}`;
const dateMarker = (t: Target) => `date:${t.date}`;
const missMarker = `${NAME}:miss`;

async function download(ctx: JobContext, m: KakaoMedia, name: string): Promise<Attachment> {
  const res = await ctx.fetch(m.xlarge_url, { headers: { 'User-Agent': 'Mozilla/5.0' } });
  if (!res.ok) throw new Error(`이미지 ${res.status}`);
  return { name, data: new Uint8Array(await res.arrayBuffer()), type: res.headers.get('content-type') ?? 'image/jpeg' };
}

export type SendResult = 'sent' | 'already-sent' | 'not-target-day' | 'no-post' | 'no-image' | 'error';

/** 내일이 월/금이면 그날 메뉴판을 한 번만 보낸다. */
export async function sendMenu(ctx: JobContext): Promise<SendResult> {
  const target = tomorrow(ctx.now);
  const dow = kst(addDays(ctx.now, 1)).dow;
  // 수동 실행에서만 BABPLUS_DAYS=수 처럼 바꿔 쓴다. 정기 실행은 월/금이다.
  const days = ctx.env.BABPLUS_DAYS ? ctx.env.BABPLUS_DAYS.split(',').map((d) => d.trim()) : DAYS;
  if (!days.includes(DOW[dow])) return 'not-target-day';
  const channel = channelOf(ctx);

  let posts: KakaoPost[];
  try {
    posts = await fetchPosts(ctx.fetch);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    lastErrors.set(target.date, msg);
    ctx.log(`[${NAME}] ${msg}`);
    return 'error';
  }
  const post = pickMenuPost(posts, target, days);
  if (!post) return 'no-post';
  const images = pickMenuImages(post.media);
  if (images.length === 0) return 'no-image';

  const recent = await ctx.discord.recent(channel, 20);
  if (hasMarker(recent, postMarker(post))) return 'already-sent';

  const files: Attachment[] = [];
  const failed: string[] = [];
  for (const [i, m] of images.entries()) {
    try {
      files.push(await download(ctx, m, `menu-${target.date}-${i + 1}.jpg`));
    } catch (err) {
      ctx.log(`[${NAME}] 이미지 내려받기 실패 ${m.xlarge_url}: ${err instanceof Error ? err.message : err}`);
      failed.push(m.xlarge_url);
    }
  }

  const content = [`🍱 ${target.label} 밥풀러스 식단`, ...failed].join('\n');
  await ctx.discord.send(channel, {
    content,
    embeds: [{ title: post.title, url: post.permalink, footer: { text: `코딩머신 / ${postMarker(post)} / ${dateMarker(target)}` } }],
    files,
  });
  lastErrors.delete(target.date);
  return 'sent';
}

/** 재시도 구간이 끝난 뒤 마지막으로 한 번 더 보내 보고, 그래도 못 보냈으면 실패 알림을 남긴다. */
export async function reportMiss(ctx: JobContext): Promise<string> {
  const result = await sendMenu(ctx);
  if (result === 'sent' || result === 'already-sent' || result === 'not-target-day') return result;

  const target = tomorrow(ctx.now);
  const channel = channelOf(ctx);
  const recent = await ctx.discord.recent(channel, 20);
  if (hasMarker(recent, missMarker, dateMarker(target))) return `${result}, 실패 알림은 이미 보냄`;

  const reason: Record<string, string> = {
    'no-post': '식단 글을 찾지 못함 (공휴일이거나 발행 시각이 바뀌었을 수 있음)',
    'no-image': '식단 글은 있지만 이미지가 없음',
    error: '카카오 채널 조회 실패',
  };
  const detail = lastErrors.get(target.date);
  await ctx.discord.send(channel, {
    embeds: [
      {
        title: `${target.label} 식단을 보내지 못했습니다`,
        description: [reason[result] ?? result, detail ? `\`\`\`\n${detail}\n\`\`\`` : ''].filter(Boolean).join('\n'),
        url: 'https://pf.kakao.com/_hqGbn/posts',
        color: 0xf04452,
        footer: { text: `코딩머신 / ${missMarker} / ${dateMarker(target)}` },
      },
    ],
  });
  return `${result}, 실패 알림 보냄`;
}

export const babplus: Feature = {
  name: NAME,
  jobs: [
    // 월/금 식단 글은 전날(일/목) 20:00 에 예약 발행된다.
    { id: 'send', cron: '5-55/10 20 * * 0,4', run: sendMenu },
    { id: 'miss', cron: '5 21 * * 0,4', run: reportMiss },
  ],
};

