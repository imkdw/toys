import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline} from '../Text';

const MouseInner = () => {
  const frame = useCurrentFrame();
  return (
    <>
      <Background name="room" />
      <Pet
        name="Momonga"
        clip="mouse"
        frames={150}
        repeat={false}
        hold={0}
        style={{position: 'absolute', left: 120, top: 60, width: 730}}
      />
      <Headline
        name="Headline"
        style={{
          left: 980,
          top: 250,
          fontSize: 150,
          opacity: interpolate(frame, [8, 20], [0, 1], {
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
          translate: interpolate(frame, [8, 26], ['0px 50px', '0px 0px'], {
          easing: Easing.bezier(0.16, 1, 0.3, 1),
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        {"마우스도\n같이 잡는다"}
      </Headline>
      <Headline
        name="Sub"
        style={{
          left: 990,
          top: 640,
          fontSize: 66,
          opacity: interpolate(frame, [30, 42], [0, 1], {
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
          translate: interpolate(frame, [30, 48], ['0px 50px', '0px 0px'], {
          easing: Easing.bezier(0.16, 1, 0.3, 1),
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        {"커서를 따라 손이 움직이고\n클릭하면 꾹"}
      </Headline>
    </>
  );
};

export const Mouse = Interactive.create({Component: MouseInner, layout: 'absolute-fill', schema: {}});
