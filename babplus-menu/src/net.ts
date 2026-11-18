// N100 집 네트워크는 가끔 새 연결이 10초 넘게 안 맺어진다. 연결 단계 오류만 다시 시도한다.
// 연결 자체가 안 된 경우라 요청이 서버에 닿지 않았으므로 POST 를 다시 보내도 중복되지 않는다.
const CONNECT_ERRORS = new Set([
  'UND_ERR_CONNECT_TIMEOUT',
  'ECONNREFUSED',
  'ENOTFOUND',
  'EAI_AGAIN',
  'ENETUNREACH',
  'EHOSTUNREACH',
]);

export function isConnectError(err: unknown): boolean {
  const code = (err as { cause?: { code?: string } })?.cause?.code;
  return code !== undefined && CONNECT_ERRORS.has(code);
}

export function retryingFetch(
  base: typeof fetch = fetch,
  delays: number[] = [2_000, 5_000, 10_000],
  log: (msg: string) => void = () => {},
): typeof fetch {
  return (async (input: Parameters<typeof fetch>[0], init?: Parameters<typeof fetch>[1]) => {
    for (let attempt = 0; ; attempt++) {
      try {
        return await base(input, init);
      } catch (err) {
        if (attempt >= delays.length || !isConnectError(err)) throw err;
        const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
        log(`연결 실패 (${(err as { cause?: { code?: string } }).cause?.code}), ${delays[attempt]}ms 뒤 다시 시도: ${url.split('?')[0]}`);
        await new Promise((r) => setTimeout(r, delays[attempt]));
      }
    }
  }) as typeof fetch;
}
