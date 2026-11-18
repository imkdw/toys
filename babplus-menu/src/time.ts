// 시간 계산은 전부 Asia/Seoul 기준이다. 서버 시간대와 상관없이 같은 결과가 나온다.
export const TZ = 'Asia/Seoul';

export interface KstParts {
  year: number;
  month: number; // 1-12
  day: number;
  hour: number;
  minute: number;
  dow: number; // 0=일 ... 6=토
}

const fmt = new Intl.DateTimeFormat('en-US', {
  timeZone: TZ,
  year: 'numeric',
  month: 'numeric',
  day: 'numeric',
  hour: 'numeric',
  minute: 'numeric',
  weekday: 'short',
  hourCycle: 'h23',
});

const WEEKDAYS: Record<string, number> = { Sun: 0, Mon: 1, Tue: 2, Wed: 3, Thu: 4, Fri: 5, Sat: 6 };

export function kst(date: Date): KstParts {
  const p: Record<string, string> = {};
  for (const { type, value } of fmt.formatToParts(date)) p[type] = value;
  return {
    year: Number(p.year),
    month: Number(p.month),
    day: Number(p.day),
    hour: Number(p.hour),
    minute: Number(p.minute),
    dow: WEEKDAYS[p.weekday],
  };
}

export function addDays(date: Date, days: number): Date {
  return new Date(date.getTime() + days * 86_400_000);
}

/** "2026-11-15 20:05" 를 KST 로 읽는다. */
export function parseKst(s: string): Date {
  const m = /^(\d{4})-(\d{2})-(\d{2})[ T](\d{2}):(\d{2})$/.exec(s.trim());
  if (!m) throw new Error(`시각 형식은 "YYYY-MM-DD HH:mm" 입니다: ${s}`);
  return new Date(`${m[1]}-${m[2]}-${m[3]}T${m[4]}:${m[5]}:00+09:00`);
}

export function ymd(p: KstParts): string {
  return `${p.year}-${String(p.month).padStart(2, '0')}-${String(p.day).padStart(2, '0')}`;
}
