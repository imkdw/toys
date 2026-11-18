import type React from 'react';
import {Img, staticFile} from 'remotion';

export const Background: React.FC<{readonly name: 'sky' | 'room' | 'card'; readonly filter?: string}> = ({name, filter}) => (
  <Img
    name="Background"
    src={staticFile(`bg/${name}.png`)}
    style={{position: 'absolute', width: '100%', height: '100%', objectFit: 'cover', filter}}
  />
);
