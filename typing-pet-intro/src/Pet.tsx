import type React from 'react';
import {
  Img,
  Interactive,
  staticFile,
  useCurrentFrame,
  type InteractivitySchema,
} from 'remotion';

type PetProps = {
  readonly clip: string;
  readonly frames: number;
  readonly repeat: boolean;
  readonly hold: number;
  readonly style?: React.CSSProperties;
};

// typing-pet 앱이 오프스크린으로 그린 프레임 (public/pets/<clip>/<n>.webp) 을 타임라인 프레임에 맞춰 보여준다.
// repeat 이면 끝난 뒤 hold 프레임만큼 마지막 장면에 머물렀다가 처음부터 다시 한다.
const PetInner: React.FC<PetProps> = ({clip, frames, repeat, hold, style}) => {
  const frame = Math.max(0, useCurrentFrame());
  const index = Math.min(repeat ? frame % (frames + hold) : frame, frames - 1);
  return <Img src={staticFile(`pets/${clip}/${index}.webp`)} style={style} />;
};

const petSchema = {
  clip: {type: 'string', default: 'usagi_ura', description: 'Clip'},
  frames: {type: 'number', min: 1, step: 1, default: 33, description: 'Frames', keyframable: false, hiddenFromList: false},
  repeat: {type: 'boolean', default: false, description: 'Repeat'},
  hold: {type: 'number', min: 0, step: 1, default: 0, description: 'Hold before repeat', keyframable: false, hiddenFromList: false},
} as const satisfies InteractivitySchema;

export const Pet = Interactive.create({
  Component: PetInner,
  layout: 'none',
  schema: petSchema,
});
