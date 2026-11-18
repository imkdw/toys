import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline, Tag} from '../Text';

const FriendsInner = () => {
  const frame = useCurrentFrame();
  return (
    <>
      <Background name="card" />
      <Headline
        name="Headline"
        style={{
          left: 0,
          top: 50,
          fontSize: 96,
          width: 1920,
          textAlign: 'center',
          opacity: interpolate(frame, [4, 16], [0, 1], {
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
          translate: interpolate(frame, [4, 22], ['0px 50px', '0px 0px'], {
          easing: Easing.bezier(0.16, 1, 0.3, 1),
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        {"친구는 여덟, 성격은 제각각"}
      </Headline>
      <Pet
        name="Usagi"
        clip="usagi_ura"
        frames={33}
        repeat
        hold={24}
        style={{position: 'absolute', left: 140, top: 170, width: 380, scale: interpolate(frame, [8, 22], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })}}
      />
      <Tag
        name="Usagi name"
        style={{
          left: 330,
          translate: '-50% 0px',
          top: 515,
          fontSize: 44,
          backgroundColor: '#fff3c4',
          scale: interpolate(frame, [20, 34], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        우사기
      </Tag>
      <Pet
        name="Chiikawa"
        clip="chiikawa_teary"
        frames={48}
        repeat
        hold={24}
        style={{position: 'absolute', left: 565, top: 170, width: 380, scale: interpolate(frame, [14, 28], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })}}
      />
      <Tag
        name="Chiikawa name"
        style={{
          left: 755,
          translate: '-50% 0px',
          top: 515,
          fontSize: 44,
          backgroundColor: '#fff3c4',
          scale: interpolate(frame, [26, 40], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        치이카와
      </Tag>
      <Pet
        name="Hachiware"
        clip="hachiware_humming"
        frames={72}
        repeat
        hold={24}
        style={{position: 'absolute', left: 990, top: 170, width: 380, scale: interpolate(frame, [20, 34], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })}}
      />
      <Tag
        name="Hachiware name"
        style={{
          left: 1180,
          translate: '-50% 0px',
          top: 515,
          fontSize: 44,
          backgroundColor: '#fff3c4',
          scale: interpolate(frame, [32, 46], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        하치와레
      </Tag>
      <Pet
        name="Momonga"
        clip="momonga_kirakira"
        frames={48}
        repeat
        hold={24}
        style={{position: 'absolute', left: 1415, top: 170, width: 380, scale: interpolate(frame, [26, 40], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })}}
      />
      <Tag
        name="Momonga name"
        style={{
          left: 1605,
          translate: '-50% 0px',
          top: 515,
          fontSize: 44,
          backgroundColor: '#fff3c4',
          scale: interpolate(frame, [38, 52], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        모몽가
      </Tag>
      <Pet
        name="Kurimanju"
        clip="kuri_haa"
        frames={48}
        repeat
        hold={24}
        style={{position: 'absolute', left: 140, top: 575, width: 380, scale: interpolate(frame, [32, 46], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })}}
      />
      <Tag
        name="Kurimanju name"
        style={{
          left: 330,
          translate: '-50% 0px',
          top: 920,
          fontSize: 44,
          backgroundColor: '#fff3c4',
          scale: interpolate(frame, [44, 58], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        쿠리만쥬
      </Tag>
      <Pet
        name="Rakko"
        clip="rakko_joa"
        frames={48}
        repeat
        hold={24}
        style={{position: 'absolute', left: 565, top: 575, width: 380, scale: interpolate(frame, [38, 52], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })}}
      />
      <Tag
        name="Rakko name"
        style={{
          left: 755,
          translate: '-50% 0px',
          top: 920,
          fontSize: 44,
          backgroundColor: '#fff3c4',
          scale: interpolate(frame, [50, 64], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        랏코
      </Tag>
      <Pet
        name="Shisa"
        clip="shisa_beam"
        frames={45}
        repeat
        hold={24}
        style={{position: 'absolute', left: 990, top: 575, width: 380, scale: interpolate(frame, [44, 58], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })}}
      />
      <Tag
        name="Shisa name"
        style={{
          left: 1180,
          translate: '-50% 0px',
          top: 920,
          fontSize: 44,
          backgroundColor: '#fff3c4',
          scale: interpolate(frame, [56, 70], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        시사
      </Tag>
      <Pet
        name="Kani"
        clip="kani_joy"
        frames={78}
        repeat
        hold={24}
        style={{position: 'absolute', left: 1415, top: 575, width: 380, scale: interpolate(frame, [50, 64], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })}}
      />
      <Tag
        name="Kani name"
        style={{
          left: 1605,
          translate: '-50% 0px',
          top: 920,
          fontSize: 44,
          backgroundColor: '#fff3c4',
          scale: interpolate(frame, [62, 76], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        카니
      </Tag>
    </>
  );
};

export const Friends = Interactive.create({Component: FriendsInner, layout: 'absolute-fill', schema: {}});
