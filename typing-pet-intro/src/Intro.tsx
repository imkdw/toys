import {linearTiming, TransitionSeries} from '@remotion/transitions';
import {fade} from '@remotion/transitions/fade';
import {Opening} from './scenes/Opening';
import {Typing} from './scenes/Typing';
import {Levels} from './scenes/Levels';
import {Mouse} from './scenes/Mouse';
import {Idle} from './scenes/Idle';
import {Friends} from './scenes/Friends';
import {Jobs} from './scenes/Jobs';
import {Ending} from './scenes/Ending';

// 장면 8개를 0.5초 크로스페이드로 잇는다. 전체 길이 = 장면 합(1446) - 전환 7번 x 15 = 1341 프레임.
export const Intro = () => (
  <TransitionSeries>
    <TransitionSeries.Sequence name="Opening" durationInFrames={135}>
      <Opening />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="Typing" durationInFrames={180}>
      <Typing />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="Levels" durationInFrames={210}>
      <Levels />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="Mouse" durationInFrames={150}>
      <Mouse />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="Idle" durationInFrames={195}>
      <Idle />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="Friends" durationInFrames={210}>
      <Friends />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="Jobs" durationInFrames={216}>
      <Jobs />
    </TransitionSeries.Sequence>
    <TransitionSeries.Transition presentation={fade()} timing={linearTiming({durationInFrames: 15})} />
    <TransitionSeries.Sequence name="Ending" durationInFrames={150}>
      <Ending />
    </TransitionSeries.Sequence>
  </TransitionSeries>
);
