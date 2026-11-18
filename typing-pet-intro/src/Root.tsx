import {Composition, Folder} from 'remotion';
import {Intro} from './Intro';
import {Opening} from './scenes/Opening';
import {Typing} from './scenes/Typing';
import {Levels} from './scenes/Levels';
import {Mouse} from './scenes/Mouse';
import {Idle} from './scenes/Idle';
import {Friends} from './scenes/Friends';
import {Jobs} from './scenes/Jobs';
import {Ending} from './scenes/Ending';

export const RemotionRoot = () => (
  <>
    <Composition id="TypingPetIntro" component={Intro} width={1920} height={1080} fps={30} durationInFrames={1341} />
    <Folder name="Scenes">
      <Composition id="Opening" component={Opening} width={1920} height={1080} fps={30} durationInFrames={135} />
      <Composition id="Typing" component={Typing} width={1920} height={1080} fps={30} durationInFrames={180} />
      <Composition id="Levels" component={Levels} width={1920} height={1080} fps={30} durationInFrames={210} />
      <Composition id="Mouse" component={Mouse} width={1920} height={1080} fps={30} durationInFrames={150} />
      <Composition id="Idle" component={Idle} width={1920} height={1080} fps={30} durationInFrames={195} />
      <Composition id="Friends" component={Friends} width={1920} height={1080} fps={30} durationInFrames={210} />
      <Composition id="Jobs" component={Jobs} width={1920} height={1080} fps={30} durationInFrames={216} />
      <Composition id="Ending" component={Ending} width={1920} height={1080} fps={30} durationInFrames={150} />
    </Folder>
  </>
);
