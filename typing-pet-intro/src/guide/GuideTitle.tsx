import {Easing, Interactive, interpolate, useCurrentFrame} from 'remotion';
import {Background} from '../Background';
import {Pet} from '../Pet';
import {Headline, Tag} from '../Text';

const GuideTitleInner = () => {
  const frame = useCurrentFrame();
  return (
    <>
      <Background name="sky" />
      <Pet name="Usagi" clip="usagi_ura" frames={33} repeat hold={36} style={{position: 'absolute', left: 110, top: 250, width: 780, translate: interpolate(frame, [0, 20], ['0px 120px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}} />
      <Headline name="Title" style={{left: 930, top: 150, fontSize: 180, opacity: interpolate(frame, [10, 22], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [10, 28], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"타이핑 펫\n설치 가이드"}
      </Headline>
      <Headline name="Subtitle" style={{left: 940, top: 610, fontSize: 60, opacity: interpolate(frame, [26, 38], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}), translate: interpolate(frame, [26, 44], ['0px 50px', '0px 0px'], {easing: Easing.bezier(0.16, 1, 0.3, 1), extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        {"처음 한 번만 허용하면 끝!"}
      </Headline>
      <Tag name="Chip 1" style={{left: 940, top: 760, fontSize: 46, backgroundColor: '#fff3c4', scale: interpolate(frame, [44, 58], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        1 옮기기
      </Tag>
      <Tag name="Chip 2" style={{left: 1190, top: 760, fontSize: 46, backgroundColor: '#ffe1ea', scale: interpolate(frame, [52, 66], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        2 그래도 열기
      </Tag>
      <Tag name="Chip 3" style={{left: 1520, top: 760, fontSize: 46, backgroundColor: '#dff3ff', scale: interpolate(frame, [60, 74], [0, 1], {easing: Easing.bezier(0.34, 1.56, 0.64, 1), output: 'perceptual-scale', extrapolateLeft: 'clamp', extrapolateRight: 'clamp'})}}>
        3 입력 허용
      </Tag>
    </>
  );
};

export const GuideTitle = Interactive.create({Component: GuideTitleInner, layout: 'absolute-fill', schema: {}});
