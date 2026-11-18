import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline, Tag} from '../Text';
import {Alert, ClickRing, Cursor, SettingsWindow} from './Mac';

const GuideOpenAnywayInner = () => {
  const frame = useCurrentFrame();
  return (
    <>
      <Background name="sky" />
      <Tag name="Step" style={{left: 100, top: 90, fontSize: 48, backgroundColor: '#fff3c4', scale: interpolate(frame, [4, 18], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        STEP 3
      </Tag>
      <Headline name="Headline" style={{left: 100, top: 205, fontSize: 96, opacity: interpolate(frame, [8, 20], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [8, 26], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"설정에서\n'그래도 열기'"}
      </Headline>
      <Headline name="Sub" style={{left: 104, top: 470, fontSize: 44, WebkitTextStroke: '12px #ffffff', opacity: interpolate(frame, [22, 34], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [22, 40], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"시스템 설정 > 개인정보 보호\n및 보안 맨 아래로 스크롤"}
      </Headline>
      <Pet name="Kani" clip="kani_read" frames={180} repeat hold={0} style={{position: 'absolute', left: 110, top: 610, width: 450}} />
      <SettingsWindow
        name="Settings"
        pane="privacy"
        scroll={interpolate(frame, [48, 96], [0, 544], {easing: Easing.bezier(0.45, 0, 0.2, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}
        highlight={interpolate(frame, [98, 110, 136, 144], [0, 1, 1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}
        toggle={0}
        dim={interpolate(frame, [140, 150, 200, 210], [0, 1, 1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}
        style={{left: 760, top: 150, opacity: interpolate(frame, [6, 18], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [6, 26], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}
      />
      <ClickRing name="Click open anyway" from={132} durationInFrames={20} style={{left: 1728, top: 726}} />
      <Alert
        name="Confirm alert"
        k={1}
        warn
        title="“TypingPet”을(를) 여시겠습니까?"
        body="Apple은 “TypingPet”에 Mac을 손상시키거나 개인정보를 침해할 수 있는 악성 코드가 없음을 확인할 수 없습니다."
        first="그래도 열기"
        second="완료"
        press={interpolate(frame, [190, 194, 200], [0, 1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}
        style={{
          left: 1060,
          top: 300,
          opacity: interpolate(frame, [142, 152, 200, 210], [0, 1, 1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
          scale: interpolate(frame, [142, 156], [0.85, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
        }}
      />
      <ClickRing name="Click confirm" from={190} durationInFrames={20} style={{left: 1300, top: 624}} />
      <Cursor name="Cursor" style={{translate: interpolate(frame, [100, 128, 160, 188, 215, 240], ['1500px 1120px', '1728px 728px', '1728px 728px', '1300px 626px', '1300px 626px', '1450px 760px'], {easing: Easing.bezier(0.45, 0, 0.2, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}} />
      <Tag name="Touch ID" style={{left: 1040, top: 520, fontSize: 46, backgroundColor: '#d9f7d0', rotate: '-3deg', scale: interpolate(frame, [212, 226], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        암호나 Touch ID 로 확인하면 열려요
      </Tag>
    </>
  );
};

export const GuideOpenAnyway = Interactive.create({Component: GuideOpenAnywayInner, layout: 'absolute-fill', schema: {}});
