import {Easing, Img, Interactive, interpolate, staticFile, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline, Tag} from '../Text';
import {ClickRing, Cursor, MacWindow, sf} from './Mac';

const GuideDragInner = () => {
  const frame = useCurrentFrame();
  return (
    <>
      <Background name="sky" />
      <Tag name="Step" style={{left: 100, top: 90, fontSize: 48, backgroundColor: '#fff3c4', scale: interpolate(frame, [4, 18], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        STEP 1
      </Tag>
      <Headline name="Headline" style={{left: 100, top: 205, fontSize: 96, opacity: interpolate(frame, [8, 20], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [8, 26], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"응용 프로그램에\n쏙 넣기"}
      </Headline>
      <Headline name="Sub" style={{left: 104, top: 470, fontSize: 44, WebkitTextStroke: '12px #ffffff', opacity: interpolate(frame, [22, 34], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [22, 40], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"DMG 를 열고 TypingPet 을\n오른쪽 폴더로 끌어다 놓아요"}
      </Headline>
      <Pet name="Hachiware" clip="hachiware_humming" frames={72} repeat hold={20} style={{position: 'absolute', left: 110, top: 610, width: 450}} />
      <MacWindow name="DMG window" title="TypingPet" style={{left: 760, top: 150, width: 1080, height: 772, opacity: interpolate(frame, [6, 18], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [6, 26], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        <Img src={staticFile('guide/dmg_bg.png')} style={{position: 'absolute', width: 1080, height: 720}} />
        <Img src={staticFile('guide/app_icon.png')} style={{position: 'absolute', left: 197, top: 168, width: 158}} />
        <Img src={staticFile('guide/applications.png')} style={{position: 'absolute', left: 714, top: 168, width: 158, scale: interpolate(frame, [118, 126, 138], [1, 1.16, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}} />
        <div style={{position: 'absolute', left: 176, top: 338, width: 200, textAlign: 'center', fontFamily: sf, fontSize: 19}}>TypingPet</div>
        <div style={{position: 'absolute', left: 693, top: 338, width: 200, textAlign: 'center', fontFamily: sf, fontSize: 19}}>Applications</div>
      </MacWindow>
      <Img
        name="Dragged icon"
        src={staticFile('guide/app_icon.png')}
        style={{
          position: 'absolute',
          left: 957,
          top: 370,
          width: 158,
          opacity: interpolate(frame, [66, 70, 118, 128], [0, 0.85, 0.85, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
          translate: interpolate(frame, [72, 118], ['0px 0px', '517px 0px'], {easing: Easing.bezier(0.45, 0, 0.2, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
          scale: interpolate(frame, [118, 128], [1, 0.45], {output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
        }}
      />
      <ClickRing name="Press" from={64} durationInFrames={20} style={{left: 1046, top: 459}} />
      <ClickRing name="Drop" from={118} durationInFrames={20} style={{left: 1563, top: 459}} />
      <Cursor name="Cursor" style={{translate: interpolate(frame, [30, 60, 72, 118, 140, 165], ['1500px 1120px', '1046px 459px', '1046px 459px', '1563px 459px', '1563px 459px', '1700px 760px'], {easing: Easing.bezier(0.45, 0, 0.2, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}} />
      <Tag name="Done" style={{left: 1170, top: 548, fontSize: 52, backgroundColor: '#d9f7d0', rotate: '-4deg', scale: interpolate(frame, [132, 146], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        설치 완료!
      </Tag>
    </>
  );
};

export const GuideDrag = Interactive.create({Component: GuideDragInner, layout: 'absolute-fill', schema: {}});
