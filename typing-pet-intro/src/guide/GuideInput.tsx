import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline, Tag} from '../Text';
import {Alert, ClickRing, Cursor, SettingsWindow} from './Mac';

const GuideInputInner = () => {
  const frame = useCurrentFrame();
  return (
    <>
      <Background name="sky" />
      <Tag name="Step" style={{left: 100, top: 90, fontSize: 48, backgroundColor: '#fff3c4', scale: interpolate(frame, [4, 18], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        STEP 4
      </Tag>
      <Headline name="Headline" style={{left: 100, top: 205, fontSize: 96, opacity: interpolate(frame, [8, 20], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [8, 26], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"입력 모니터링\n켜기"}
      </Headline>
      <Headline name="Sub" style={{left: 104, top: 470, fontSize: 44, WebkitTextStroke: '12px #ffffff', opacity: interpolate(frame, [22, 34], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [22, 40], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"타이핑에 맞춰 움직이려면\n키 입력 감지가 필요해요"}
      </Headline>
      <Pet name="Momonga" clip="momonga_kirakira" frames={48} repeat hold={20} style={{position: 'absolute', left: 110, top: 610, width: 450}} />
      <SettingsWindow
        name="Settings"
        pane="input"
        scroll={0}
        highlight={0}
        toggle={interpolate(frame, [150, 158], [0, 1], {easing: Easing.bezier(0.45, 0, 0.2, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}
        dim={0}
        style={{left: 760, top: 150, opacity: interpolate(frame, [84, 96], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [84, 104], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}
      />
      <Alert
        name="Permission alert"
        k={1.3}
        warn={false}
        title="키 입력 수신"
        body="“TypingPet”이(가) 모든 응용 프로그램에서 키 입력을 수신하려고 합니다."
        first="시스템 설정 열기"
        second="거부"
        press={interpolate(frame, [68, 72, 78], [0, 1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}
        style={{
          left: 980,
          top: 230,
          opacity: interpolate(frame, [14, 24, 80, 90], [0, 1, 1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
          scale: interpolate(frame, [14, 28], [0.85, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
        }}
      />
      <ClickRing name="Click open settings" from={68} durationInFrames={20} style={{left: 1292, top: 651}} />
      <ClickRing name="Click toggle" from={148} durationInFrames={20} style={{left: 1764, top: 401}} />
      <Cursor name="Cursor" style={{translate: interpolate(frame, [40, 66, 112, 146, 175, 205], ['1700px 1120px', '1292px 653px', '1292px 653px', '1764px 403px', '1764px 403px', '1600px 640px'], {easing: Easing.bezier(0.45, 0, 0.2, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}} />
      <Tag name="Result" style={{left: 1150, top: 620, fontSize: 54, backgroundColor: '#d9f7d0', rotate: '-3deg', scale: interpolate(frame, [172, 186], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        이제 타이핑하면 움직여요!
      </Tag>
    </>
  );
};

export const GuideInput = Interactive.create({Component: GuideInputInner, layout: 'absolute-fill', schema: {}});
