import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline, Tag} from '../Text';

const IdleInner = () => {
  const frame = useCurrentFrame();
  return (
    <>
      <Background name="card" filter="hue-rotate(165deg) saturate(0.7)" />
      <Headline
        name="Headline"
        style={{
          left: 0,
          top: 90,
          fontSize: 120,
          width: 1920,
          textAlign: 'center',
          opacity: interpolate(frame, [6, 18], [0, 1], {
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
          translate: interpolate(frame, [6, 24], ['0px 50px', '0px 0px'], {
          easing: Easing.bezier(0.16, 1, 0.3, 1),
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        {"쉬는 동안엔 각자 딴짓"}
      </Headline>
      <Pet
        name="Usagi"
        clip="usagi_sleep"
        frames={180}
        repeat={false}
        hold={0}
        style={{position: 'absolute', left: 50, top: 330, width: 440, scale: interpolate(frame, [10, 24], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })}}
      />
      <Tag
        name="Usagi label"
        style={{
          left: 270,
          translate: '-50% 0px',
          top: 800,
          fontSize: 50,
          scale: interpolate(frame, [30, 44], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        우사기는 쿨쿨
      </Tag>
      <Pet
        name="Rakko"
        clip="rakko_parfait"
        frames={180}
        repeat={false}
        hold={0}
        style={{position: 'absolute', left: 510, top: 330, width: 440, scale: interpolate(frame, [18, 32], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })}}
      />
      <Tag
        name="Rakko label"
        style={{
          left: 730,
          translate: '-50% 0px',
          top: 800,
          fontSize: 50,
          scale: interpolate(frame, [38, 52], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        랏코는 파르페
      </Tag>
      <Pet
        name="Kurimanju"
        clip="kuri_snack"
        frames={180}
        repeat={false}
        hold={0}
        style={{position: 'absolute', left: 970, top: 330, width: 440, scale: interpolate(frame, [26, 40], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })}}
      />
      <Tag
        name="Kurimanju label"
        style={{
          left: 1190,
          translate: '-50% 0px',
          top: 800,
          fontSize: 50,
          scale: interpolate(frame, [46, 60], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        쿠리만쥬는 안주
      </Tag>
      <Pet
        name="Kani"
        clip="kani_read"
        frames={180}
        repeat={false}
        hold={0}
        style={{position: 'absolute', left: 1430, top: 330, width: 440, scale: interpolate(frame, [34, 48], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })}}
      />
      <Tag
        name="Kani label"
        style={{
          left: 1650,
          translate: '-50% 0px',
          top: 800,
          fontSize: 50,
          scale: interpolate(frame, [54, 68], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        카니는 독서
      </Tag>
    </>
  );
};

export const Idle = Interactive.create({Component: IdleInner, layout: 'absolute-fill', schema: {}});
