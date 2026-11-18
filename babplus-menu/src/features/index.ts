import type { Feature } from '../types.ts';
import { babplus } from './babplus/index.ts';

// 새 기능은 features/<이름>/ 폴더를 만들고 여기에 한 줄 추가한다.
export const features: Feature[] = [babplus];
