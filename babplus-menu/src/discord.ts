const API = 'https://discord.com/api/v10';

export interface Attachment {
  name: string;
  data: Uint8Array;
  type: string;
}

export interface Embed {
  title?: string;
  url?: string;
  description?: string;
  color?: number;
  footer?: { text: string };
}

export interface OutgoingMessage {
  content?: string;
  embeds?: Embed[];
  files?: Attachment[];
}

export interface Message {
  id: string;
  content: string;
  author: { id: string; bot?: boolean };
  embeds: Embed[];
}

export interface Discord {
  send(channelId: string, msg: OutgoingMessage): Promise<Message | null>;
  recent(channelId: string, limit?: number): Promise<Message[]>;
}

export class DiscordError extends Error {
  status: number;
  constructor(status: number, body: string) {
    super(`Discord ${status}: ${body.slice(0, 300)}`);
    this.status = status;
  }
}

export interface DiscordOptions {
  token?: string;
  /** true 면 send 는 보내지 않고 로그만 남긴다. recent 는 토큰이 있으면 실제로 읽는다. */
  dryRun?: boolean;
  fetch?: typeof fetch;
  log?: (msg: string) => void;
  sleep?: (ms: number) => Promise<void>;
}

export function createDiscord(opts: DiscordOptions): Discord {
  const f = opts.fetch ?? fetch;
  const log = opts.log ?? console.log;
  const sleep = opts.sleep ?? ((ms: number) => new Promise<void>((r) => setTimeout(r, ms)));

  async function request(method: string, path: string, body?: () => BodyInit, json = false): Promise<unknown> {
    if (!opts.token) throw new Error('DISCORD_BOT_TOKEN 이 없습니다.');
    for (let attempt = 0; ; attempt++) {
      const headers: Record<string, string> = {
        Authorization: `Bot ${opts.token}`,
        'User-Agent': 'DiscordBot (kimdongmu, 1.0)',
      };
      if (json) headers['Content-Type'] = 'application/json';
      const res = await f(API + path, { method, headers, body: body?.() });
      if (res.status === 429 && attempt === 0) {
        const data = (await res.json().catch(() => ({}))) as { retry_after?: number };
        const wait = Math.ceil((data.retry_after ?? 1) * 1000);
        log(`Discord 429, ${wait}ms 뒤 다시 시도`);
        await sleep(wait);
        continue;
      }
      if (!res.ok) throw new DiscordError(res.status, await res.text().catch(() => ''));
      return res.status === 204 ? null : res.json();
    }
  }

  return {
    async send(channelId, msg) {
      const files = msg.files ?? [];
      const payload = {
        content: msg.content,
        embeds: msg.embeds,
        attachments: files.map((file, id) => ({ id, filename: file.name })),
        allowed_mentions: { parse: [] },
      };
      if (opts.dryRun) {
        log(`[dry-run] #${channelId} 로 전송: ${JSON.stringify(payload)} (첨부 ${files.map((x) => `${x.name} ${x.data.byteLength}B`).join(', ') || '없음'})`);
        return null;
      }
      if (files.length === 0) {
        return (await request('POST', `/channels/${channelId}/messages`, () => JSON.stringify(payload), true)) as Message;
      }
      return (await request('POST', `/channels/${channelId}/messages`, () => {
        const form = new FormData();
        form.append('payload_json', JSON.stringify(payload));
        files.forEach((file, i) => form.append(`files[${i}]`, new Blob([file.data], { type: file.type }), file.name));
        return form;
      })) as Message;
    },
    async recent(channelId, limit = 20) {
      if (opts.dryRun && !opts.token) return [];
      return (await request('GET', `/channels/${channelId}/messages?limit=${limit}`)) as Message[];
    },
  };
}

/** 최근 메시지 중 봇이 보낸 것의 footer 에 marker 가 들어 있는지 본다. */
export function hasMarker(messages: Message[], ...markers: string[]): boolean {
  return messages.some(
    (m) => m.author.bot && m.embeds.some((e) => markers.every((mk) => e.footer?.text.includes(mk))),
  );
}
