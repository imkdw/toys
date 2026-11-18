import type React from 'react';
import {Img, Interactive, interpolate, staticFile, useCurrentFrame} from 'remotion';

// 설치 가이드에 쓰는 macOS 화면 흉내 부품. 실제 화면을 그대로 옮긴 것이 아니라 알아볼 수 있게 단순화했다.
export const sf = '-apple-system, "SF Pro Text", "Apple SD Gothic Neo", sans-serif';
const ink = '#1d1d1f';
const blue = '#0a84ff';

type Styled = {readonly style?: React.CSSProperties; readonly children?: React.ReactNode};

// 창: 신호등 버튼이 달린 제목 막대(52px) + 내용.
const MacWindowInner: React.FC<Styled & {readonly title: string}> = ({title, style, children}) => (
  <div
    style={{
      position: 'absolute',
      borderRadius: 26,
      overflow: 'hidden',
      backgroundColor: '#f5f5f7',
      boxShadow: '0 40px 90px rgba(60, 40, 20, 0.35), 0 0 0 1px rgba(0, 0, 0, 0.12)',
      fontFamily: sf,
      color: ink,
      ...style,
    }}
  >
    <div style={{height: 52, backgroundColor: '#ffffff', display: 'flex', alignItems: 'center', paddingLeft: 20, gap: 10, borderBottom: '1px solid rgba(0,0,0,0.08)'}}>
      <div style={{width: 17, height: 17, borderRadius: 99, backgroundColor: '#ff5f57'}} />
      <div style={{width: 17, height: 17, borderRadius: 99, backgroundColor: '#febc2e'}} />
      <div style={{width: 17, height: 17, borderRadius: 99, backgroundColor: '#28c840'}} />
      <div style={{marginLeft: 14, fontSize: 20, fontWeight: 700, color: '#4a4a4f'}}>{title}</div>
    </div>
    <div style={{position: 'relative', height: 'calc(100% - 52px)'}}>{children}</div>
  </div>
);

// 마우스 포인터. 화살 끝이 요소의 왼쪽 위 모서리다.
const CursorInner: React.FC<Styled> = ({style}) => (
  <svg width={54} height={78} viewBox="0 0 18 26" style={{position: 'absolute', left: 0, top: 0, filter: 'drop-shadow(0 4px 6px rgba(0,0,0,0.35))', ...style}}>
    <path d="M1 1 L1 20 L5.6 15.8 L8.9 23.6 L12 22.3 L8.8 14.6 L15 14.4 Z" fill="#111111" stroke="#ffffff" strokeWidth={1.4} strokeLinejoin="round" />
  </svg>
);

// 클릭 표시: 퍼지면서 사라지는 고리. left/top 이 고리 중심이다.
const ClickRingInner: React.FC<Styled> = ({style}) => {
  const frame = useCurrentFrame();
  return (
    <div
      style={{
        position: 'absolute',
        width: 120,
        height: 120,
        marginLeft: -60,
        marginTop: -60,
        borderRadius: 999,
        border: '8px solid #ff8a3d',
        opacity: interpolate(frame, [0, 4, 18], [0, 0.95, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
        scale: interpolate(frame, [0, 18], [0.25, 1.15], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
        ...style,
      }}
    />
  );
};

// 경고 삼각형이 붙은 앱 아이콘.
const AppBadge: React.FC<{readonly size: number; readonly warn: boolean}> = ({size, warn}) => (
  <div style={{position: 'relative', width: size, height: size}}>
    <Img src={staticFile('guide/app_icon.png')} style={{width: size, height: size}} />
    {warn ? (
      <svg width={size * 0.5} height={size * 0.5} viewBox="0 0 40 40" style={{position: 'absolute', right: -size * 0.1, bottom: -size * 0.06}}>
        <path d="M20 4 L37 34 L3 34 Z" fill="#ffcc00" stroke="#ffffff" strokeWidth={3} strokeLinejoin="round" />
        <rect x={18.2} y={14} width={3.6} height={11} rx={1.8} fill="#1d1d1f" />
        <circle cx={20} cy={29} r={2.1} fill="#1d1d1f" />
      </svg>
    ) : null}
  </div>
);

type AlertProps = Styled & {
  readonly k: number;
  readonly warn: boolean;
  readonly title: string;
  readonly body: string;
  readonly first: string;
  readonly second: string;
  // 0..1: 첫 버튼을 누른 정도
  readonly press: number;
};

// 시스템 경고창. 기준 크기 480x440 에 k 를 곱한다. 첫 버튼은 위에서 300k, 둘째는 360k 위치(높이 48k).
const AlertInner: React.FC<AlertProps> = ({k, warn, title, body, first, second, press, style}) => (
  <div
    style={{
      position: 'absolute',
      width: 480 * k,
      height: 440 * k,
      borderRadius: 34 * k,
      backgroundColor: 'rgba(248, 248, 250, 0.98)',
      boxShadow: '0 40px 100px rgba(40, 25, 10, 0.45), 0 0 0 1px rgba(0, 0, 0, 0.14)',
      fontFamily: sf,
      color: ink,
      textAlign: 'center',
      ...style,
    }}
  >
    <div style={{position: 'absolute', left: 192 * k, top: 36 * k}}>
      <AppBadge size={96 * k} warn={warn} />
    </div>
    <div style={{position: 'absolute', left: 30 * k, right: 30 * k, top: 150 * k, fontSize: 24 * k, fontWeight: 700}}>{title}</div>
    <div style={{position: 'absolute', left: 40 * k, right: 40 * k, top: 194 * k, fontSize: 17.5 * k, lineHeight: 1.45, color: '#3a3a3f', wordBreak: 'keep-all'}}>{body}</div>
    <div
      style={{
        position: 'absolute',
        left: 40 * k,
        right: 40 * k,
        top: 300 * k,
        height: 48 * k,
        borderRadius: 99,
        backgroundColor: blue,
        filter: `brightness(${1 - 0.25 * press})`,
        color: '#ffffff',
        fontSize: 19 * k,
        fontWeight: 600,
        lineHeight: `${48 * k}px`,
      }}
    >
      {first}
    </div>
    <div
      style={{
        position: 'absolute',
        left: 40 * k,
        right: 40 * k,
        top: 360 * k,
        height: 48 * k,
        borderRadius: 99,
        backgroundColor: '#e3e3e8',
        fontSize: 19 * k,
        fontWeight: 600,
        lineHeight: `${48 * k}px`,
      }}
    >
      {second}
    </div>
  </div>
);

// 색 타일 위에 흰 시스템 기호(public/guide/sf, SF Symbols 를 뽑은 것)를 얹은 설정 아이콘.
const Tile: React.FC<{readonly sym: string; readonly color: string; readonly size: number}> = ({sym, color, size}) => (
  <div style={{width: size, height: size, flexShrink: 0, borderRadius: size * 0.26, background: `linear-gradient(rgba(255,255,255,0.22), rgba(255,255,255,0) 60%), ${color}`, boxShadow: 'inset 0 0 0 1px rgba(0,0,0,0.06)', display: 'flex', alignItems: 'center', justifyContent: 'center'}}>
    <Img src={staticFile(`guide/sf/${sym}.png`)} style={{width: size * 0.82, height: size * 0.82}} />
  </div>
);

// 밝은 바탕에 놓는 회색 기호.
const Glyph: React.FC<{readonly sym: string; readonly size: number; readonly opacity: number}> = ({sym, size, opacity}) => (
  <Img src={staticFile(`guide/sf/${sym}.png`)} style={{width: size, height: size, filter: 'invert(1)', opacity, flexShrink: 0}} />
);

const gray = '#8e8e93';
const black = '#1c1c1e';
const red = '#ff3b30';
const indigo = '#5e5ce6';

type Item = readonly [label: string, sym: string, color: string];
const sideGroups: readonly (readonly Item[])[] = [
  [
    ['일반', 'gear', gray],
    ['데스크탑 및 Dock', 'dock.rectangle', black],
    ['디스플레이', 'sun.max.fill', blue],
    ['손쉬운 사용', 'accessibility', blue],
    ['화면 모드', 'circle.lefthalf.filled', black],
    ['Spotlight', 'magnifyingglass', blue],
  ],
  [
    ['알림', 'bell.badge.fill', red],
    ['사운드', 'speaker.wave.2.fill', '#ff2d55'],
    ['집중 모드', 'moon.fill', indigo],
    ['스크린 타임', 'hourglass', indigo],
  ],
  [
    ['잠금 화면', 'lock.fill', black],
    ['개인정보 보호 및 보안', 'hand.raised.fill', blue],
    ['Touch ID 및 암호', 'touchid', red],
  ],
];

type Row = readonly [label: string, sym: string, color: string, value: string];
const groupA: readonly Row[] = [
  ['위치 서비스', 'location.fill', blue, ''],
  ['캘린더', 'calendar', red, '2'],
  ['연락처', 'person.crop.circle.fill', '#a2845e', '1'],
  ['사진', 'photo.fill', '#ff9f0a', '3'],
  ['파일 및 폴더', 'folder.fill', '#32ade6', '5'],
];
const groupB: readonly Row[] = [
  ['카메라', 'camera.fill', '#34c759', '2'],
  ['마이크', 'mic.fill', '#ff9500', '4'],
  ['입력 모니터링', 'keyboard.fill', gray, '1'],
  ['화면 및 시스템 오디오 녹음', 'record.circle', red, '6'],
];
const groupC: readonly Row[] = [['민감한 콘텐츠 경고', 'eye.fill', blue, '끔']];
const groupD: readonly Row[] = [
  ['분석 및 향상', 'chart.bar.fill', blue, ''],
  ['Apple 광고', 'megaphone.fill', blue, ''],
];
const groupE: readonly Row[] = [
  ['FileVault', 'house.fill', gray, '켬'],
  ['차단 모드', 'hand.raised.fill', blue, ''],
];

const box: React.CSSProperties = {backgroundColor: 'rgba(0, 0, 0, 0.045)', borderRadius: 14, position: 'relative'};
const rowH = 62;

const Group: React.FC<{readonly rows: readonly Row[]}> = ({rows}) => (
  <div style={{...box, marginBottom: 16}}>
    {rows.map(([label, sym, color, value], i) => (
      <div key={label} style={{height: rowH, display: 'flex', alignItems: 'center', gap: 14, paddingLeft: 16, paddingRight: 16, fontSize: 19.5, position: 'relative'}}>
        {i > 0 ? <div style={{position: 'absolute', left: 60, right: 16, top: 0, height: 1, backgroundColor: 'rgba(0,0,0,0.08)'}} /> : null}
        <Tile sym={sym} color={color} size={30} />
        {label}
        <div style={{marginLeft: 'auto', color: '#8a8a8e'}}>{value}</div>
        <Glyph sym="chevron.right" size={17} opacity={0.3} />
      </div>
    ))}
  </div>
);

// macOS 26 스위치: 넓은 캡슐.
const Toggle: React.FC<{readonly on: number}> = ({on}) => (
  <div style={{width: 60, height: 30, borderRadius: 99, backgroundColor: on > 0.5 ? '#34c759' : '#d4d4d8', position: 'relative', boxShadow: 'inset 0 0 0 1px rgba(0,0,0,0.05)'}}>
    <div style={{position: 'absolute', top: 3, left: 3 + 18 * on, width: 36, height: 24, borderRadius: 99, backgroundColor: '#ffffff', boxShadow: '0 1px 4px rgba(0,0,0,0.28)'}} />
  </div>
);

type SettingsProps = Styled & {
  readonly pane: 'privacy' | 'input';
  // privacy: 내용을 위로 민 거리(px). 맨 아래는 544.
  readonly scroll: number;
  // privacy: 차단 안내 강조 0..1
  readonly highlight: number;
  // input: TypingPet 스위치 0..1
  readonly toggle: number;
  // 창 위에 깔리는 어두운 막 0..1 (확인창이 뜰 때)
  readonly dim: number;
};

// 시스템 설정 창 (1080x780). macOS 26 모양: 창 안에 떠 있는 사이드바(신호등 포함), 위쪽 도구 막대 74px, 회색 묶음 상자.
const SettingsWindowInner: React.FC<SettingsProps> = ({pane, scroll, highlight, toggle, dim, style}) => (
  <div
    style={{
      position: 'absolute',
      width: 1080,
      height: 780,
      borderRadius: 32,
      overflow: 'hidden',
      backgroundColor: '#ffffff',
      boxShadow: '0 40px 90px rgba(60, 40, 20, 0.35), 0 0 0 1px rgba(0, 0, 0, 0.14)',
      fontFamily: sf,
      color: ink,
      ...style,
    }}
  >
    <div style={{position: 'absolute', left: 10, top: 10, bottom: 10, width: 326, borderRadius: 24, backgroundColor: '#f4f4f6', boxShadow: '0 2px 12px rgba(0,0,0,0.10), 0 0 0 1px rgba(0,0,0,0.06)', overflow: 'hidden'}}>
      <div style={{position: 'absolute', left: 20, top: 20, display: 'flex', gap: 12}}>
        <div style={{width: 19, height: 19, borderRadius: 99, backgroundColor: '#ff5f57', boxShadow: 'inset 0 0 0 1px rgba(0,0,0,0.12)'}} />
        <div style={{width: 19, height: 19, borderRadius: 99, backgroundColor: '#febc2e', boxShadow: 'inset 0 0 0 1px rgba(0,0,0,0.12)'}} />
        <div style={{width: 19, height: 19, borderRadius: 99, backgroundColor: '#28c840', boxShadow: 'inset 0 0 0 1px rgba(0,0,0,0.12)'}} />
      </div>
      <div style={{position: 'absolute', left: 14, right: 14, top: 62, height: 42, borderRadius: 99, backgroundColor: '#e2e2e7', display: 'flex', alignItems: 'center', gap: 8, paddingLeft: 14, color: '#85858b', fontSize: 19.5}}>
        <Glyph sym="magnifyingglass" size={20} opacity={0.5} />
        검색
      </div>
      <div style={{position: 'absolute', left: 14, right: 14, top: 116}}>
        {sideGroups.map((items, g) => (
          <div key={items[0][0]} style={{marginTop: g === 0 ? 0 : 16}}>
            {items.map(([label, sym, color]) => (
              <div
                key={label}
                style={{
                  height: 46,
                  borderRadius: 12,
                  display: 'flex',
                  alignItems: 'center',
                  gap: 10,
                  paddingLeft: 9,
                  fontSize: 19.5,
                  backgroundColor: label === '개인정보 보호 및 보안' ? '#2f6fe4' : 'transparent',
                  color: label === '개인정보 보호 및 보안' ? '#ffffff' : ink,
                  fontWeight: label === '개인정보 보호 및 보안' ? 600 : 400,
                }}
              >
                <Tile sym={sym} color={color} size={30} />
                {label}
              </div>
            ))}
          </div>
        ))}
      </div>
    </div>

    <div style={{position: 'absolute', left: 346, right: 0, top: 74, bottom: 0, overflow: 'hidden'}}>
      {pane === 'privacy' ? (
        <div style={{position: 'absolute', left: 18, right: 30, top: 14 - scroll}}>
          <Group rows={groupA} />
          <Group rows={groupB} />
          <Group rows={groupC} />
          <Group rows={groupD} />
          <div style={{height: 56, fontSize: 19.5, fontWeight: 700, display: 'flex', alignItems: 'center', paddingLeft: 12}}>보안</div>
          <div style={{...box, height: rowH, display: 'flex', alignItems: 'center', paddingLeft: 16, paddingRight: 14, fontSize: 19.5, marginBottom: 16}}>
            다음에서 응용 프로그램 허용
            <div style={{marginLeft: 'auto', marginRight: 12}}>App Store 및 알려진 개발자</div>
            <div style={{width: 28, height: 28, borderRadius: 99, backgroundColor: 'rgba(0,0,0,0.08)', display: 'flex', alignItems: 'center', justifyContent: 'center'}}>
              <Glyph sym="chevron.up.chevron.down" size={17} opacity={0.75} />
            </div>
          </div>
          <div style={{...box, height: 124, marginBottom: 16, boxShadow: `0 0 0 ${6 * highlight}px rgba(255, 138, 61, ${0.9 * highlight})`}}>
            <div style={{position: 'absolute', left: 16, top: 18, fontSize: 19.5, color: '#3a3a3f'}}>“TypingPet”이(가) Mac을 보호하기 위해 차단되었습니다.</div>
            <div style={{position: 'absolute', right: 16, bottom: 16, width: 132, height: 36, borderRadius: 99, backgroundColor: '#ffffff', boxShadow: '0 1px 3px rgba(0,0,0,0.18), 0 0 0 1px rgba(0,0,0,0.05)', fontSize: 19.5, textAlign: 'center', lineHeight: '36px'}}>그래도 열기</div>
          </div>
          <Group rows={groupE} />
        </div>
      ) : (
        <div style={{position: 'absolute', left: 18, right: 30, top: 14}}>
          <div style={{height: 70, paddingLeft: 12, paddingRight: 12, fontSize: 17.5, lineHeight: 1.45, color: '#7a7a80', wordBreak: 'keep-all'}}>
            아래 응용 프로그램이 다른 응용 프로그램을 사용하는 중에도 키보드로부터의 입력을 모니터링하도록 허용합니다.
          </div>
          <div style={box}>
            <div style={{height: rowH, display: 'flex', alignItems: 'center', gap: 14, paddingLeft: 16, paddingRight: 16, fontSize: 19.5}}>
              <Img src={staticFile('guide/terminal.png')} style={{width: 36, height: 36}} />
              터미널
              <div style={{marginLeft: 'auto'}}>
                <Toggle on={1} />
              </div>
            </div>
            <div style={{height: rowH, display: 'flex', alignItems: 'center', gap: 14, paddingLeft: 16, paddingRight: 16, fontSize: 19.5, position: 'relative'}}>
              <div style={{position: 'absolute', left: 66, right: 16, top: 0, height: 1, backgroundColor: 'rgba(0,0,0,0.08)'}} />
              <Img src={staticFile('guide/app_icon.png')} style={{width: 36, height: 36}} />
              TypingPet
              <div style={{marginLeft: 'auto'}}>
                <Toggle on={toggle} />
              </div>
            </div>
            <div style={{height: 44, display: 'flex', alignItems: 'center', gap: 18, paddingLeft: 18, position: 'relative'}}>
              <div style={{position: 'absolute', left: 0, right: 0, top: 0, height: 1, backgroundColor: 'rgba(0,0,0,0.08)'}} />
              <Glyph sym="plus" size={17} opacity={0.6} />
              <Glyph sym="minus" size={17} opacity={0.3} />
            </div>
          </div>
        </div>
      )}
      {pane === 'privacy' ? (
        <div style={{position: 'absolute', right: 5, top: 6 + (scroll / 544) * 304, width: 9, height: 390, borderRadius: 99, backgroundColor: 'rgba(0,0,0,0.42)'}} />
      ) : null}
    </div>

    <div style={{position: 'absolute', left: 346, right: 0, top: 0, height: 74, background: 'linear-gradient(rgba(255,255,255,1) 70%, rgba(255,255,255,0.86))', display: 'flex', alignItems: 'center', paddingLeft: 16, gap: 18}}>
      <div style={{height: 44, borderRadius: 99, backgroundColor: '#ffffff', boxShadow: '0 2px 10px rgba(0,0,0,0.14), 0 0 0 1px rgba(0,0,0,0.05)', display: 'flex', alignItems: 'center', gap: 18, paddingLeft: 16, paddingRight: 16}}>
        <Glyph sym="chevron.left" size={21} opacity={0.8} />
        <Glyph sym="chevron.right" size={21} opacity={pane === 'privacy' ? 0.25 : 0.25} />
      </div>
      <div style={{fontSize: 20.5, fontWeight: 700}}>{pane === 'privacy' ? '개인정보 보호 및 보안' : '입력 모니터링'}</div>
    </div>
    <div style={{position: 'absolute', inset: 0, backgroundColor: `rgba(0, 0, 0, ${0.28 * dim})`}} />
  </div>
);

// 설명용 흰 카드: 갈색 테두리, 둥근 모서리.
const PanelInner: React.FC<Styled> = ({style, children}) => (
  <div style={{position: 'absolute', backgroundColor: '#ffffff', border: '6px solid #5a3a28', borderRadius: 44, boxShadow: '0 14px 0 rgba(90, 58, 40, 0.18)', ...style}}>{children}</div>
);

// 도장: 기울어진 둥근 테두리 글자.
const StampInner: React.FC<Styled & {readonly color: string}> = ({color, style, children}) => (
  <div style={{position: 'absolute', border: `7px solid ${color}`, color, borderRadius: 22, padding: '6px 26px 10px', whiteSpace: 'nowrap', backgroundColor: 'rgba(255,255,255,0.85)', ...style}}>{children}</div>
);

export const Panel = Interactive.create({Component: PanelInner, layout: 'none', schema: {...Interactive.backgroundSchema}});
export const Stamp = Interactive.create({Component: StampInner, layout: 'none', schema: {color: {type: 'color', default: '#2fa84f', description: 'Color'}, ...Interactive.textSchema, ...Interactive.childrenSchema}});
export const MacWindow = Interactive.create({Component: MacWindowInner, layout: 'none', schema: {title: {type: 'string', default: '', description: 'Title'}}});
export const Cursor = Interactive.create({Component: CursorInner, layout: 'none', schema: {}});
export const ClickRing = Interactive.create({Component: ClickRingInner, layout: 'none', schema: {}});
export const Alert = Interactive.create({
  Component: AlertInner,
  layout: 'none',
  schema: {
    title: {type: 'string', default: '', description: 'Title'},
    body: {type: 'string', default: '', description: 'Body'},
    first: {type: 'string', default: '', description: 'First button'},
    second: {type: 'string', default: '', description: 'Second button'},
  },
});
export const SettingsWindow = Interactive.create({Component: SettingsWindowInner, layout: 'none', schema: {}});
