// 밥풀러스 카카오 채널 글 조회와 판별. 공식 공개 API 가 아니라 채널 웹 페이지가 쓰는 JSON API 다.
export const POSTS_URL = 'https://pf.kakao.com/rocket-web/web/profiles/_hqGbn/posts';

export interface KakaoMedia {
  type: string;
  xlarge_url: string;
  url?: string;
  width: number;
  height: number;
}

export interface KakaoPost {
  id: number;
  title: string;
  status: string;
  type: string;
  published_at: number;
  updated_at: number;
  permalink: string;
  media?: KakaoMedia[];
}

export class KakaoError extends Error {}

export async function fetchPosts(f: typeof fetch = fetch): Promise<KakaoPost[]> {
  const res = await f(POSTS_URL, { headers: { 'User-Agent': 'Mozilla/5.0', Accept: 'application/json' } });
  const text = await res.text();
  if (!res.ok) throw new KakaoError(`카카오 채널 API ${res.status}: ${text.slice(0, 200)}`);
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    throw new KakaoError(`카카오 채널 API 응답이 JSON 이 아님: ${text.slice(0, 200)}`);
  }
  const items = (data as { items?: unknown }).items;
  if (!Array.isArray(items)) throw new KakaoError(`카카오 채널 API 응답에 items 가 없음: ${text.slice(0, 200)}`);
  return items as KakaoPost[];
}

export interface MenuTitle {
  month: number;
  day: number;
  dow: string;
}

// "11/16 (월) 메뉴" 만 맞고 "11/16 (월) 석식메뉴" 는 맞지 않는다.
const TITLE = /^(\d{1,2})\/(\d{1,2}) \(([월화수목금토일])\) 메뉴$/;

export function parseMenuTitle(title: string): MenuTitle | null {
  const m = TITLE.exec(title.trim());
  return m ? { month: Number(m[1]), day: Number(m[2]), dow: m[3] } : null;
}

/** 내일(target) 자 중식 메뉴 글 중 요일이 days 에 들어 있는 것을 고른다. */
export function pickMenuPost(
  posts: KakaoPost[],
  target: { month: number; day: number },
  days: readonly string[],
): KakaoPost | null {
  for (const post of posts) {
    const t = parseMenuTitle(post.title ?? '');
    if (!t || !days.includes(t.dow)) continue;
    if (t.month !== target.month || t.day !== target.day) continue;
    if (post.status !== 'published') continue;
    return post;
  }
  return null;
}

/** 메뉴판은 793x1122, 음식 사진은 3024x4032 이상이다. 가로 1500px 미만만 메뉴판으로 본다. */
export const MENU_BOARD_MAX_WIDTH = 1500;
export const MAX_ATTACHMENTS = 10;

export function pickMenuImages(media: KakaoMedia[] = []): KakaoMedia[] {
  const images = media.filter((m) => m.type === 'image' && m.xlarge_url);
  const boards = images.filter((m) => m.width > 0 && m.width < MENU_BOARD_MAX_WIDTH);
  return (boards.length > 0 ? boards : images).slice(0, MAX_ATTACHMENTS);
}
