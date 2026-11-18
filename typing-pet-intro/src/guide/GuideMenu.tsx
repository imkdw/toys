import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline, Tag} from '../Text';
import {ClickRing, Cursor} from './Mac';
import {Desktop, TrayMenu} from './Tray';

// 메뉴바 토끼 아이콘에서 직군과 캐릭터를 바꾸는 장면. 왼쪽 캐릭터가 고른 대로 바뀐다.
// 메뉴가 열리고 닫히는 시점과 강조 줄은 프레임 구간으로 정한다 (Studio 에서 키프레임으로 고칠 수 없는 부분).
const GuideMenuInner = () => {
  const frame = useCurrentFrame();
  const open = (frame >= 36 && frame < 110) || (frame >= 166 && frame < 234);
  const first = frame < 150;
  return (
    <>
      <Background name="sky" />
      <Tag name="Step" style={{left: 100, top: 90, fontSize: 48, backgroundColor: '#dff3ff', scale: interpolate(frame, [4, 18], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        TIP 1
      </Tag>
      <Headline name="Headline" style={{left: 100, top: 205, fontSize: 96, opacity: interpolate(frame, [8, 20], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [8, 26], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"캐릭터와 직군\n바꾸기"}
      </Headline>
      <Headline name="Sub" style={{left: 104, top: 470, fontSize: 44, WebkitTextStroke: '12px #ffffff', opacity: interpolate(frame, [22, 34], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [22, 40], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"메뉴바의 토끼 아이콘을\n누르면 메뉴가 열려요"}
      </Headline>
      <Pet name="Usagi basic" clip="usagi_ura" frames={33} repeat hold={36} from={0} durationInFrames={112} style={{position: 'absolute', left: 110, top: 610, width: 450}} />
      <Pet name="Usagi developer" clip="job_developer" frames={72} repeat hold={0} from={112} durationInFrames={124} style={{position: 'absolute', left: 110, top: 610, width: 450}} />
      <Pet name="Chiikawa developer" clip="typing" frames={180} repeat hold={0} from={236} style={{position: 'absolute', left: 110, top: 448, width: 450}} />
      <Desktop name="Desktop" active={open} style={{left: 760, top: 150, opacity: interpolate(frame, [6, 18], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [6, 26], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}} />
      {open ? (
        <TrayMenu
          name="Menu"
          hot={first ? (frame >= 56 ? 1 : -1) : frame >= 184 ? 0 : -1}
          sub={first ? (frame >= 62 ? 'job' : 'none') : frame >= 190 ? 'character' : 'none'}
          subHot={first ? (frame >= 92 ? 1 : -1) : frame >= 216 ? 4 : -1}
          character="우사기"
          job="기본"
          style={{left: 780, top: 202}}
        />
      ) : null}
      <ClickRing name="Click icon 1" from={34} durationInFrames={20} style={{left: 910, top: 172}} />
      <ClickRing name="Click job" from={104} durationInFrames={20} style={{left: 1234, top: 302}} />
      <ClickRing name="Click icon 2" from={164} durationInFrames={20} style={{left: 910, top: 172}} />
      <ClickRing name="Click character" from={228} durationInFrames={20} style={{left: 1234, top: 382}} />
      <Cursor name="Cursor" style={{translate: interpolate(frame, [10, 32, 44, 58, 70, 82, 100, 140, 162, 174, 186, 198, 210, 226, 250, 280], ['1300px 1120px', '910px 174px', '910px 174px', '940px 272px', '1060px 272px', '1200px 268px', '1234px 304px', '1234px 304px', '910px 174px', '910px 174px', '940px 232px', '1060px 232px', '1200px 232px', '1234px 384px', '1234px 384px', '1400px 620px'], {easing: Easing.bezier(0.45, 0, 0.2, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}} />
      <Tag name="Job changed" style={{left: 520, top: 575, fontSize: 46, backgroundColor: '#fff3c4', rotate: '-4deg', opacity: interpolate(frame, [226, 236], [1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), scale: interpolate(frame, [116, 130], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        개발자 책상으로!
      </Tag>
      <Tag name="Character changed" style={{left: 520, top: 575, fontSize: 46, backgroundColor: '#ffe1ea', rotate: '-4deg', scale: interpolate(frame, [240, 254], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        치이카와 등장!
      </Tag>
      <Headline name="Count" style={{left: 1010, top: 800, fontSize: 52, WebkitTextStroke: '12px #ffffff', opacity: interpolate(frame, [262, 274], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [262, 280], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"캐릭터 8명, 직군 8가지 중에 골라요"}
      </Headline>
    </>
  );
};

export const GuideMenu = Interactive.create({Component: GuideMenuInner, layout: 'absolute-fill', schema: {}});
