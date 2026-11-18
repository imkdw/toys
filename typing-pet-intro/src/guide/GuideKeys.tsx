import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline, Tag} from '../Text';
import {ClickRing, Cursor} from './Mac';
import {Desktop, TrayMenu} from './Tray';

// 메뉴의 "입력 키 보기" 를 켜면 누른 키가 캐릭터 위 말풍선으로 뜨는 장면.
// 바탕화면 캐릭터는 같은 타이핑을 말풍선 없이(typing_plain) 보여 주다가 켠 뒤에는 말풍선과 함께(typing) 보여 준다.
const GuideKeysInner = () => {
  const frame = useCurrentFrame();
  const open = frame >= 30 && frame < 78;
  return (
    <>
      <Background name="sky" />
      <Tag name="Step" style={{left: 100, top: 90, fontSize: 48, backgroundColor: '#dff3ff', scale: interpolate(frame, [4, 18], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        TIP 3
      </Tag>
      <Headline name="Headline" style={{left: 100, top: 205, fontSize: 96, opacity: interpolate(frame, [8, 20], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [8, 26], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"입력 키 보기"}
      </Headline>
      <Headline name="Sub" style={{left: 104, top: 470, fontSize: 44, WebkitTextStroke: '12px #ffffff', opacity: interpolate(frame, [22, 34], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [22, 40], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"방금 누른 키가 캐릭터 위에\n말풍선으로 떠올라요"}
      </Headline>
      <Pet name="Kani" clip="kani_joy" frames={78} repeat hold={20} style={{position: 'absolute', left: 110, top: 610, width: 450}} />
      <Desktop name="Desktop" active={open} style={{left: 760, top: 150, opacity: interpolate(frame, [6, 18], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [6, 26], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}} />
      <Pet name="Desktop pet plain" clip="typing_plain" frames={180} repeat hold={0} from={0} durationInFrames={78} style={{position: 'absolute', left: 1250, top: 390, width: 520, opacity: interpolate(frame, [10, 22], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}} />
      <Pet name="Desktop pet keys" clip="typing" frames={180} repeat hold={0} from={78} style={{position: 'absolute', left: 1250, top: 203, width: 520}} />
      {open ? <TrayMenu name="Menu" hot={frame >= 50 ? 5 : -1} sub="none" subHot={-1} character="치이카와" job="개발자" keyEcho={frame >= 66} style={{left: 780, top: 202}} /> : null}
      <ClickRing name="Click icon" from={28} durationInFrames={20} style={{left: 910, top: 172}} />
      <ClickRing name="Click key echo" from={64} durationInFrames={20} style={{left: 940, top: 430}} />
      <Cursor name="Cursor" style={{translate: interpolate(frame, [8, 26, 38, 60, 84, 110], ['1300px 1120px', '910px 174px', '910px 174px', '940px 432px', '940px 432px', '1000px 700px'], {easing: Easing.bezier(0.45, 0, 0.2, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}} />
      <Tag name="Bubble hint" style={{left: 810, top: 330, fontSize: 46, backgroundColor: '#fff3c4', rotate: '-3deg', scale: interpolate(frame, [104, 118], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        누른 키가 말풍선으로!
      </Tag>
      <Headline name="Off" style={{left: 960, top: 955, fontSize: 46, WebkitTextStroke: '12px #ffffff', opacity: interpolate(frame, [180, 192], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [180, 198], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"메뉴에서 한 번 더 누르면 꺼져요"}
      </Headline>
    </>
  );
};

export const GuideKeys = Interactive.create({Component: GuideKeysInner, layout: 'absolute-fill', schema: {}});
