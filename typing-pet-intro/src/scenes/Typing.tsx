import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline, Tag} from '../Text';

const TypingInner = () => {
  const frame = useCurrentFrame();
  return (
    <>
      <Background name="room" />
      <Pet
        name="Chiikawa"
        clip="typing"
        frames={180}
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
        {"내가 치면\n얘도 친다"}
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
        {"누른 키가 머리 위로 퐁퐁"}
      </Headline>
      <Tag
        name="Name"
        style={{
          left: 1000,
          top: 790,
          fontSize: 52,
          backgroundColor: '#fff3c4',
          scale: interpolate(frame, [45, 59], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        치이카와
      </Tag>
    </>
  );
};

export const Typing = Interactive.create({Component: TypingInner, layout: 'absolute-fill', schema: {}});
