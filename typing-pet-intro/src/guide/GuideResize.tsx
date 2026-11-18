import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline, Tag} from '../Text';
import {ClickRing, Cursor} from './Mac';
import {Desktop, ResizeFrame, TrayMenu} from './Tray';

// 메뉴의 "크기 조절" 로 틀을 켜고 모서리를 끌어 키우는 장면. 캐릭터는 오른쪽 아래(1780, 890)에 붙어 있다.
const GuideResizeInner = () => {
  const frame = useCurrentFrame();
  const open = frame >= 30 && frame < 72;
  return (
    <>
      <Background name="sky" />
      <Tag name="Step" style={{left: 100, top: 90, fontSize: 48, backgroundColor: '#dff3ff', scale: interpolate(frame, [4, 18], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        TIP 2
      </Tag>
      <Headline name="Headline" style={{left: 100, top: 205, fontSize: 96, opacity: interpolate(frame, [8, 20], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [8, 26], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"크기도\n마음대로"}
      </Headline>
      <Headline name="Sub" style={{left: 104, top: 470, fontSize: 44, WebkitTextStroke: '12px #ffffff', opacity: interpolate(frame, [22, 34], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [22, 40], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"메뉴에서 '크기 조절' 을\n고르면 틀이 나타나요"}
      </Headline>
      <Pet name="Rakko" clip="rakko_joa" frames={48} repeat hold={20} style={{position: 'absolute', left: 110, top: 610, width: 450}} />
      <Desktop name="Desktop" active={open} style={{left: 760, top: 150, opacity: interpolate(frame, [6, 18], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [6, 26], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}} />
      <Pet
        name="Desktop pet"
        clip="typing"
        frames={180}
        repeat
        hold={0}
        style={{
          position: 'absolute',
          left: 1400,
          top: 373,
          width: 380,
          transformOrigin: '100% 100%',
          opacity: interpolate(frame, [10, 22], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
          scale: interpolate(frame, [112, 170], [0.579, 1], {easing: Easing.bezier(0.45, 0, 0.2, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
        }}
      />
      <ResizeFrame
        name="Resize frame"
        style={{
          left: 1400,
          top: 510,
          width: 380,
          height: 380,
          transformOrigin: '100% 100%',
          opacity: interpolate(frame, [72, 80, 214, 224], [0, 1, 1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
          scale: interpolate(frame, [112, 170], [0.579, 1], {easing: Easing.bezier(0.45, 0, 0.2, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
        }}
      />
      {open ? <TrayMenu name="Menu" hot={frame >= 52 ? 6 : -1} sub="none" subHot={-1} character="치이카와" job="개발자" style={{left: 780, top: 202}} /> : null}
      <ClickRing name="Click icon" from={28} durationInFrames={20} style={{left: 910, top: 172}} />
      <ClickRing name="Click resize" from={66} durationInFrames={20} style={{left: 960, top: 470}} />
      <ClickRing name="Grab corner" from={108} durationInFrames={20} style={{left: 1560, top: 670}} />
      <Cursor name="Cursor" style={{translate: interpolate(frame, [8, 26, 38, 62, 78, 106, 112, 170, 186, 215], ['1300px 1120px', '910px 174px', '910px 174px', '960px 472px', '960px 472px', '1560px 670px', '1560px 670px', '1400px 510px', '1400px 510px', '1300px 640px'], {easing: Easing.bezier(0.45, 0, 0.2, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}} />
      <Tag name="Drag hint" style={{left: 900, top: 560, fontSize: 46, backgroundColor: '#fff3c4', rotate: '-3deg', opacity: interpolate(frame, [196, 206], [1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), scale: interpolate(frame, [90, 104], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        모서리를 끌면 커져요
      </Tag>
      <Tag name="Enter" style={{left: 900, top: 560, fontSize: 46, backgroundColor: '#d9f7d0', rotate: '-3deg', scale: interpolate(frame, [200, 214], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        Enter 를 누르면 저장!
      </Tag>
      <Headline name="Move" style={{left: 800, top: 720, fontSize: 46, WebkitTextStroke: '12px #ffffff', opacity: interpolate(frame, [226, 238], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [226, 244], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"틀 안쪽을 끌면 자리도 옮길 수 있어요"}
      </Headline>
    </>
  );
};

export const GuideResize = Interactive.create({Component: GuideResizeInner, layout: 'absolute-fill', schema: {}});
