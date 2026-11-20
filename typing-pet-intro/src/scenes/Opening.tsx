import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline} from '../Text';

// 첫 프레임이 채팅 앱의 영상 썸네일이 되므로 제목/부제는 흐리게 시작하지 않고 처음부터 보인다 (미끄러져 올라오기만 한다).
const OpeningInner = () => {
  const frame = useCurrentFrame();
  return (
    <>
      <Background name="sky" />
      <Pet
        name="Usagi"
        clip="usagi_ura"
        frames={33}
        repeat
        hold={36}
        style={{position: 'absolute', left: 110, top: 250, width: 780, translate: interpolate(frame, [0, 20], ['0px 120px', '0px 0px'], {
          easing: Easing.bezier(0.16, 1, 0.3, 1),
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })}}
      />
      <Headline
        name="Title"
        style={{
          left: 930,
          top: 260,
          fontSize: 230,
          translate: interpolate(frame, [12, 30], ['0px 50px', '0px 0px'], {
          easing: Easing.bezier(0.16, 1, 0.3, 1),
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        {"타이핑 펫"}
      </Headline>
      <Headline
        name="Subtitle"
        style={{
          left: 940,
          top: 560,
          fontSize: 76,
          translate: interpolate(frame, [30, 48], ['0px 50px', '0px 0px'], {
          easing: Easing.bezier(0.16, 1, 0.3, 1),
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        }),
        }}
      >
        {"키보드를 치면\n같이 일하는 책상 친구"}
      </Headline>
    </>
  );
};

export const Opening = Interactive.create({Component: OpeningInner, layout: 'absolute-fill', schema: {}});
