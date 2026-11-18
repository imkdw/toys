import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline, Tag} from '../Text';

const GuideDoneInner = () => {
  const frame = useCurrentFrame();
  return (
    <>
      <Background name="sky" />
      <Pet name="Usagi" clip="usagi_yaha" frames={75} repeat hold={30} style={{position: 'absolute', left: 110, top: 250, width: 780}} />
      <Headline name="Headline" style={{left: 930, top: 230, fontSize: 190, opacity: interpolate(frame, [10, 22], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [10, 28], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"설치 끝!"}
      </Headline>
      <Headline name="Sub" style={{left: 940, top: 500, fontSize: 62, opacity: interpolate(frame, [26, 38], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [26, 44], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"이제 키보드를 치면\n옆에서 같이 일해요"}
      </Headline>
      <Tag name="App" style={{left: 950, top: 760, fontSize: 72, backgroundColor: '#fff3c4', scale: interpolate(frame, [44, 58], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        typing-pet
      </Tag>
    </>
  );
};

export const GuideDone = Interactive.create({Component: GuideDoneInner, layout: 'absolute-fill', schema: {}});
