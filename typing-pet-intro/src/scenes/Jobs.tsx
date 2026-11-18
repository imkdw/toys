import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline, Tag} from '../Text';

const JobsInner = () => {
  const frame = useCurrentFrame();
  return (
    <>
      <Background name="room" />
      <Pet
        name="Desk developer"
        clip="job_developer"
        frames={72}
        repeat={false}
        hold={0}
        from={0}
        durationInFrames={72}
        style={{position: 'absolute', left: 120, top: 310, width: 730}}
      />
      <Pet
        name="Desk designer"
        clip="job_designer"
        frames={72}
        repeat={false}
        hold={0}
        from={72}
        durationInFrames={72}
        style={{position: 'absolute', left: 120, top: 310, width: 730}}
      />
      <Pet
        name="Desk infra"
        clip="job_infra"
        frames={72}
        repeat={false}
        hold={0}
        from={144}
        durationInFrames={72}
        style={{position: 'absolute', left: 120, top: 310, width: 730}}
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
        {"직군에 맞춰\n책상도 바뀐다"}
      </Headline>
      <Tag
        name="Job developer"
        from={0}
        durationInFrames={72}
        style={{
          left: 1000,
          top: 610,
          fontSize: 84,
          backgroundColor: '#fff3c4',
          scale: interpolate(frame, [20, 34], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        개발자
      </Tag>
      <Tag
        name="Job designer"
        from={72}
        durationInFrames={72}
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
        디자이너
      </Tag>
      <Tag
        name="Job infra"
        from={144}
        durationInFrames={72}
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
        인프라
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
        {"개발자부터 보안까지 8가지"}
      </Headline>
    </>
  );
};

export const Jobs = Interactive.create({Component: JobsInner, layout: 'absolute-fill', schema: {}});
