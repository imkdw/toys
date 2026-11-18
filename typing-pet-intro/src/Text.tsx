import type React from 'react';
import {Interactive, type InteractivitySchema} from 'remotion';
import {jua} from './fonts';

type TextProps = {
  readonly children?: React.ReactNode;
  readonly style?: React.CSSProperties;
};

// 갈색 손글씨 느낌 제목: 흰 테두리를 두껍게 둘러 어떤 배경에서도 읽힌다.
const HeadlineInner: React.FC<TextProps> = ({children, style}) => (
  <div
    style={{
      position: 'absolute',
      fontFamily: jua,
      color: '#5a3a28',
      lineHeight: 1.18,
      whiteSpace: 'pre-line',
      WebkitTextStroke: '18px #ffffff',
      paintOrder: 'stroke fill',
      ...style,
    }}
  >
    {children}
  </div>
);

// 말풍선 같은 흰 딱지: 갈색 테두리, 둥근 모서리.
const TagInner: React.FC<TextProps> = ({children, style}) => (
  <div
    style={{
      position: 'absolute',
      fontFamily: jua,
      color: '#5a3a28',
      backgroundColor: '#ffffff',
      border: '6px solid #5a3a28',
      borderRadius: 999,
      padding: '10px 36px 14px',
      whiteSpace: 'nowrap',
      ...style,
    }}
  >
    {children}
  </div>
);

const textSchema = {
  ...Interactive.textSchema,
  ...Interactive.childrenSchema,
} as const satisfies InteractivitySchema;

const tagSchema = {
  ...Interactive.textSchema,
  ...Interactive.backgroundSchema,
  ...Interactive.childrenSchema,
} as const satisfies InteractivitySchema;

export const Headline = Interactive.create({Component: HeadlineInner, layout: 'none', schema: textSchema});
export const Tag = Interactive.create({Component: TagInner, layout: 'none', schema: tagSchema});
