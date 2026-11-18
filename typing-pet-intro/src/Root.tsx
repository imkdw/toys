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
import {InstallGuide} from './InstallGuide';
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

export const RemotionRoot = () => (
  <>
    <Composition id="TypingPetIntro" component={Intro} width={1920} height={1080} fps={30} durationInFrames={1341} />
    <Composition id="InstallGuide" component={InstallGuide} width={1920} height={1080} fps={30} durationInFrames={2100} />
    <Folder name="GuideScenes">
      <Composition id="GuideTitle" component={GuideTitle} width={1920} height={1080} fps={30} durationInFrames={105} />
      <Composition id="GuideDrag" component={GuideDrag} width={1920} height={1080} fps={30} durationInFrames={180} />
      <Composition id="GuideBlocked" component={GuideBlocked} width={1920} height={1080} fps={30} durationInFrames={180} />
      <Composition id="GuideWhy" component={GuideWhy} width={1920} height={1080} fps={30} durationInFrames={285} />
      <Composition id="GuideOpenAnyway" component={GuideOpenAnyway} width={1920} height={1080} fps={30} durationInFrames={255} />
      <Composition id="GuideInput" component={GuideInput} width={1920} height={1080} fps={30} durationInFrames={225} />
      <Composition id="GuideMenu" component={GuideMenu} width={1920} height={1080} fps={30} durationInFrames={330} />
      <Composition id="GuideResize" component={GuideResize} width={1920} height={1080} fps={30} durationInFrames={270} />
      <Composition id="GuideKeys" component={GuideKeys} width={1920} height={1080} fps={30} durationInFrames={270} />
      <Composition id="GuideDone" component={GuideDone} width={1920} height={1080} fps={30} durationInFrames={135} />
    </Folder>
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
