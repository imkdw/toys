import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline, Tag} from '../Text';
import {Alert, ClickRing, Cursor} from './Mac';

const GuideBlockedInner = () => {
  const frame = useCurrentFrame();
  return (
    <>
      <Background name="sky" />
      <Tag name="Step" style={{left: 100, top: 90, fontSize: 48, backgroundColor: '#fff3c4', scale: interpolate(frame, [4, 18], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        STEP 2
      </Tag>
      <Headline name="Headline" style={{left: 100, top: 205, fontSize: 96, opacity: interpolate(frame, [8, 20], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [8, 26], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"처음 열면\n막혀요"}
      </Headline>
      <Headline name="Sub" style={{left: 104, top: 470, fontSize: 44, WebkitTextStroke: '12px #ffffff', opacity: interpolate(frame, [22, 34], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [22, 40], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"놀라지 말고\n'완료' 를 눌러요"}
      </Headline>
      <Pet name="Chiikawa" clip="chiikawa_teary" frames={48} repeat hold={20} style={{position: 'absolute', left: 110, top: 610, width: 450}} />
      <Alert
        name="Blocked alert"
        k={1.3}
        warn
        title="“TypingPet”이(가) 열리지 않음"
        body="Apple은 “TypingPet”에 Mac을 손상시키거나 개인정보를 침해할 수 있는 악성 코드가 없음을 확인할 수 없습니다."
        first="완료"
        second="휴지통으로 이동"
        press={interpolate(frame, [116, 120, 126], [0, 1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}
        style={{
          left: 980,
          top: 230,
          opacity: interpolate(frame, [16, 26, 132, 142], [0, 1, 1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
          scale: interpolate(frame, [16, 30], [0.85, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}),
        }}
      />
      <Tag name="Warning" style={{left: 1400, top: 800, fontSize: 44, backgroundColor: '#ffd9d9', rotate: '4deg', opacity: interpolate(frame, [132, 142], [1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), scale: interpolate(frame, [62, 76], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        휴지통은 누르면 안 돼요!
      </Tag>
      <ClickRing name="Click" from={116} durationInFrames={20} style={{left: 1292, top: 651}} />
      <Cursor name="Cursor" style={{translate: interpolate(frame, [84, 114, 150], ['1700px 1120px', '1292px 651px', '1292px 651px'], {easing: Easing.bezier(0.45, 0, 0.2, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), opacity: interpolate(frame, [132, 142], [1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}} />
      <Headline name="Next" style={{left: 960, top: 420, fontSize: 84, opacity: interpolate(frame, [142, 154], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [142, 160], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"괜찮아요,\n설정에서 열 수 있어요"}
      </Headline>
    </>
  );
};

export const GuideBlocked = Interactive.create({Component: GuideBlockedInner, layout: 'absolute-fill', schema: {}});
