export type JobStatus = 'queued' | 'analyzing' | 'correcting' | 'done' | 'failed';

export interface Point {
  x: number;
  y: number;
}

export interface Adjustments {
  brightness: number;
  contrast: number;
  saturation: number;
  redGain: number;
  greenGain: number;
  blueGain: number;
}

export interface Analysis {
  summary: string;
  cardType: string;
  subjects: string[];
  problems: string[];
  fixes: string[];
  corners: Point[];
  cardTopFacing: 'up' | 'down' | 'left' | 'right';
  orientationReason: string;
  cardAspectRatio: number;
  cornerRadiusRatio: number;
  adjustments: Adjustments;
}

export interface JobView {
  id: string;
  name: string;
  status: JobStatus;
  createdAt: string;
  finishedAt: string | null;
  originalUrl: string;
  imageUrl: string | null;
  analysis: Analysis | null;
  prompt: string | null;
  error: string | null;
}
