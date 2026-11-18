import {linearTiming, TransitionSeries} from '@remotion/transitions';
import {fade} from '@remotion/transitions/fade';
import {GuideTitle} from './guide/GuideTitle';
import {GuideDrag} from './guide/GuideDrag';
import {GuideBlocked} from './guide/GuideBlocked';
import {GuideWhy} from './guide/GuideWhy';
import {GuideOpenAnyway} from './guide/GuideOpenAnyway';
import {GuideInput} from './guide/GuideInput';
import {GuideMenu} from './guide/GuideMenu';
import {GuideResize} from './guide/GuideResize';
import {GuideKeys} from './guide/GuideKeys';
import {GuideDone} from './guide/GuideDone';

// 설치 가이드: 장면 10개를 0.5초 크로스페이드로 잇는다. 전체 길이 = 장면 합 - 전환 9번 x 15 = 2100 프레임.
export const InstallGuide = () => (
  <TransitionSeries>
    <TransitionSeries.Sequence name="GuideTitle" durationInFrames={105}>
      <GuideTitle />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="GuideDrag" durationInFrames={180}>
      <GuideDrag />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="GuideBlocked" durationInFrames={180}>
      <GuideBlocked />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="GuideWhy" durationInFrames={285}>
      <GuideWhy />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="GuideOpenAnyway" durationInFrames={255}>
      <GuideOpenAnyway />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="GuideInput" durationInFrames={225}>
      <GuideInput />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="GuideMenu" durationInFrames={330}>
      <GuideMenu />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="GuideResize" durationInFrames={270}>
      <GuideResize />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="GuideKeys" durationInFrames={270}>
      <GuideKeys />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="GuideDone" durationInFrames={135}>
      <GuideDone />
    </TransitionSeries.Sequence>
  </TransitionSeries>
);
