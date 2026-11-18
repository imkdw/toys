import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline, Tag} from '../Text';

const LevelsInner = () => {
  const frame = useCurrentFrame();
  return (
    <>
      <Background name="room" />
      <Pet
        name="Hachiware"
        clip="levels"
        frames={210}
        repeat={false}
        hold={0}
        style={{position: 'absolute', left: 120, top: 60, width: 730}}
      />
      <Headline
        name="Headline"
        style={{
          left: 980,
          top: 220,
          fontSize: 140,
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
        {"빨리 칠수록\n달아오른다"}
      </Headline>
      <Tag
        name="Level normal"
        from={0}
        durationInFrames={45}
        style={{
          left: 1000,
          top: 610,
          fontSize: 84,
          scale: interpolate(frame, [20, 34], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        기본
      </Tag>
      <Tag
        name="Level excited"
        from={45}
        durationInFrames={45}
        style={{
          left: 1000,
          top: 610,
          fontSize: 84,
          backgroundColor: '#fff3c4',
          scale: interpolate(frame, [0, 14], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        신남!
      </Tag>
      <Tag
        name="Level dizzy"
        from={90}
        durationInFrames={45}
        style={{
          left: 1000,
          top: 610,
          fontSize: 84,
          backgroundColor: '#d9ecff',
          scale: interpolate(frame, [0, 14], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        어질어질
      </Tag>
      <Tag
        name="Level overheat"
        from={135}
        durationInFrames={75}
        style={{
          left: 1000,
          top: 610,
          fontSize: 84,
          backgroundColor: '#ffd3d3',
          scale: interpolate(frame, [0, 14], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        과열!!
      </Tag>
      <Headline
        name="Sub"
        style={{
          left: 990,
          top: 810,
          fontSize: 60,
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
        {"단계마다 표정과 행동이 바뀐다"}
      </Headline>
    </>
  );
};

export const Levels = Interactive.create({Component: LevelsInner, layout: 'absolute-fill', schema: {}});
