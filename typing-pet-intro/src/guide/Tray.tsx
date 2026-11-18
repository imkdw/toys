import type React from 'react';
import {Img, Interactive, staticFile} from 'remotion';
import {sf} from './Mac';

// 메뉴바와 TypingPet 메뉴 흉내. 메뉴 항목과 순서는 앱(src/tray.rs)과 같다.
type Styled = {readonly style?: React.CSSProperties; readonly children?: React.ReactNode};

const White: React.FC<{readonly sym: string; readonly size: number}> = ({sym, size}) => (
  <Img src={staticFile(`guide/sf/${sym}.png`)} style={{width: size, height: size}} />
);

// 바탕화면 조각 (1080x780): 위에 메뉴바 44px. 토끼 아이콘 중심은 왼쪽에서 150px.
const DesktopInner: React.FC<Styled & {readonly active: boolean}> = ({active, style, children}) => (
  <div
    style={{
      position: 'absolute',
      width: 1080,
      height: 780,
      borderRadius: 32,
      overflow: 'hidden',
      background: 'linear-gradient(160deg, #2b3a67 0%, #4b3f72 45%, #a8577e 100%)',
      boxShadow: '0 40px 90px rgba(60, 40, 20, 0.35), 0 0 0 1px rgba(0, 0, 0, 0.2)',
      fontFamily: sf,
      ...style,
    }}
  >
    <div style={{position: 'absolute', left: 0, right: 0, top: 0, height: 44, backgroundColor: 'rgba(20, 20, 30, 0.45)', color: '#ffffff', fontSize: 19, fontWeight: 500}}>
      <div style={{position: 'absolute', left: 120, top: 5, width: 60, height: 34, borderRadius: 99, backgroundColor: active ? 'rgba(255,255,255,0.28)' : 'transparent'}} />
      <svg width={34} height={34} viewBox="0 0 44 44" style={{position: 'absolute', left: 133, top: 5}}>
        <circle cx={22} cy={29} r={13} fill="#ffffff" />
        <ellipse cx={16} cy={11} rx={4.2} ry={10} fill="#ffffff" />
        <ellipse cx={28} cy={11} rx={4.2} ry={10} fill="#ffffff" />
      </svg>
      <div style={{position: 'absolute', left: 22, top: 9}}>
        <White sym="apple.logo" size={25} />
      </div>
      <div style={{position: 'absolute', right: 22, top: 0, height: 44, display: 'flex', alignItems: 'center', gap: 22}}>
        <White sym="wifi" size={25} />
        <White sym="battery.100percent" size={30} />
        <White sym="magnifyingglass" size={22} />
        <White sym="switch.2" size={24} />
        11월 19일 (목) 오전 9:41
      </div>
    </div>
    {children}
  </div>
);

const itemH = 40;
const menuBox: React.CSSProperties = {
  position: 'absolute',
  borderRadius: 16,
  padding: 8,
  backgroundColor: 'rgba(244, 244, 246, 0.96)',
  boxShadow: '0 18px 50px rgba(0,0,0,0.38), 0 0 0 1px rgba(0,0,0,0.12)',
  fontFamily: sf,
  fontSize: 20,
  color: '#1d1d1f',
};

const MenuRow: React.FC<{readonly label: string; readonly checked: boolean; readonly arrow: boolean; readonly hot: boolean}> = ({label, checked, arrow, hot}) => (
  <div style={{height: itemH, borderRadius: 10, display: 'flex', alignItems: 'center', paddingRight: 10, backgroundColor: hot ? '#2f6fe4' : 'transparent', color: hot ? '#ffffff' : '#1d1d1f'}}>
    <div style={{width: 34, display: 'flex', justifyContent: 'center'}}>
      {checked ? <Img src={staticFile('guide/sf/checkmark.png')} style={{width: 17, height: 17, filter: hot ? 'none' : 'invert(1)'}} /> : null}
    </div>
    {label}
    {arrow ? <Img src={staticFile('guide/sf/chevron.right.png')} style={{width: 16, height: 16, marginLeft: 'auto', filter: hot ? 'none' : 'invert(1)', opacity: hot ? 1 : 0.6}} /> : null}
  </div>
);

const characters = ['랏코', '모몽가', '시사', '우사기', '치이카와', '카니', '쿠리만쥬', '하치와레'];
const jobs = ['기본', '개발자', '디자이너', '기획자', '마케터', '재무/총무', '인프라', '보안'];

type TrayMenuProps = Styled & {
  // 강조할 줄: 0 캐릭터, 1 직군, 6 크기 조절 ... (-1 이면 없음)
  readonly hot: number;
  readonly sub: 'none' | 'character' | 'job';
  // 하위 메뉴에서 강조할 줄 (-1 이면 없음)
  readonly subHot: number;
  readonly character: string;
  readonly job: string;
  // "입력 키 보기" 체크 표시
  readonly keyEcho?: boolean;
};

// TypingPet 메뉴 (폭 340). 줄 높이 40, 위 여백 8. 하위 메뉴는 오른쪽 344px 에 폭 220 으로 붙는다.
const TrayMenuInner: React.FC<TrayMenuProps> = ({hot, sub, subHot, character, job, keyEcho, style}) => (
  <div style={{position: 'absolute', ...style}}>
    <div style={{...menuBox, left: 0, top: 0, width: 340}}>
      <MenuRow label="캐릭터" checked={false} arrow hot={hot === 0} />
      <MenuRow label="직군 (책상 장비)" checked={false} arrow hot={hot === 1} />
      <MenuRow label="보이기" checked arrow={false} hot={hot === 2} />
      <MenuRow label="숨쉬기" checked arrow={false} hot={hot === 3} />
      <MenuRow label="반응 줄이기" checked={false} arrow={false} hot={hot === 4} />
      <MenuRow label="입력 키 보기" checked={keyEcho === true} arrow={false} hot={hot === 5} />
      <MenuRow label="크기 조절 (329 pt)" checked={false} arrow={false} hot={hot === 6} />
      <MenuRow label="로그인 시 자동 실행" checked arrow={false} hot={hot === 7} />
      <div style={{height: 1, margin: '8px 12px', backgroundColor: 'rgba(0,0,0,0.12)'}} />
      <MenuRow label="종료" checked={false} arrow={false} hot={hot === 8} />
    </div>
    {sub === 'none' ? null : (
      <div style={{...menuBox, left: 344, top: sub === 'character' ? 0 : itemH, width: 220}}>
        {(sub === 'character' ? characters : jobs).map((label, i) => (
          <MenuRow key={label} label={label} checked={label === (sub === 'character' ? character : job)} arrow={false} hot={subHot === i} />
        ))}
      </div>
    )}
  </div>
);

// 크기 조절 틀: 테두리 + 손잡이 8개 + 안내문 (앱이 그리는 문구와 같다).
const ResizeFrameInner: React.FC<Styled> = ({style}) => (
  <div style={{position: 'absolute', border: '3px solid rgba(255,255,255,0.95)', boxShadow: '0 0 0 1px rgba(0,0,0,0.35)', ...style}}>
    {[
      [0, 0],
      [50, 0],
      [100, 0],
      [0, 50],
      [100, 50],
      [0, 100],
      [50, 100],
      [100, 100],
    ].map(([x, y]) => (
      <div key={`${x}-${y}`} style={{position: 'absolute', left: `${x}%`, top: `${y}%`, width: 20, height: 20, marginLeft: -10, marginTop: -10, backgroundColor: '#ffffff', border: '3px solid #2f6fe4', borderRadius: 5}} />
    ))}
    <div style={{position: 'absolute', left: 0, right: 0, top: 18, textAlign: 'center', fontFamily: sf, fontSize: 17, lineHeight: 1.4, color: '#ffffff', textShadow: '0 1px 3px rgba(0,0,0,0.8)'}}>
      모서리를 끌어 크기 조절
      <br />
      Enter 저장 / Esc 취소
    </div>
  </div>
);

export const Desktop = Interactive.create({Component: DesktopInner, layout: 'none', schema: {}});
export const TrayMenu = Interactive.create({Component: TrayMenuInner, layout: 'none', schema: {}});
export const ResizeFrame = Interactive.create({Component: ResizeFrameInner, layout: 'none', schema: {}});
