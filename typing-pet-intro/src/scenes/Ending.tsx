import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline, Tag} from '../Text';

const EndingInner = () => {
  const frame = useCurrentFrame();
  return (
    <>
      <Background name="sky" />
      <Pet
        name="Usagi"
        clip="usagi_yaha"
        frames={75}
        repeat
        hold={30}
        style={{position: 'absolute', left: 110, top: 250, width: 780}}
      />
      <Headline
        name="Headline"
        style={{
          left: 930,
          top: 230,
          fontSize: 170,
          opacity: interpolate(frame, [10, 22], [0, 1], {
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
          translate: interpolate(frame, [10, 28], ['0px 50px', '0px 0px'], {
          easing: Easing.bezier(0.16, 1, 0.3, 1),
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        {"오늘도\n같이 일하자!"}
      </Headline>
      <Tag
        name="App"
        style={{
          left: 950,
          top: 700,
          fontSize: 72,
          backgroundColor: '#fff3c4',
          scale: interpolate(frame, [34, 48], [0, 1], {
          easing: Easing.bezier(0.34, 1.56, 0.64, 1),
          output: 'perceptual-scale',
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        typing-pet
      </Tag>
      <Headline
        name="Sub"
        style={{
          left: 950,
          top: 860,
          fontSize: 56,
          opacity: interpolate(frame, [44, 56], [0, 1], {
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
          translate: interpolate(frame, [44, 62], ['0px 50px', '0px 0px'], {
          easing: Easing.bezier(0.16, 1, 0.3, 1),
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        {"macOS 메뉴바에 사는 타이핑 친구"}
      </Headline>
    </>
  );
};

export const Ending = Interactive.create({Component: EndingInner, layout: 'absolute-fill', schema: {}});
