export type Point = { x: number; y: number };
export type Quad = [Point, Point, Point, Point];

export interface RGBAImage {
  data: Uint8ClampedArray;
  width: number;
  height: number;
}

export function cleanMask(mask: Uint8Array, w: number, h: number): Uint8Array {
  const labels = new Int32Array(w * h);
  const stack: number[] = [];
  let best = 0;
  let bestSize = 0;
  let next = 1;
  for (let i = 0; i < w * h; i++) {
    if (!mask[i] || labels[i]) continue;
    let size = 0;
    labels[i] = next;
    stack.push(i);
    while (stack.length) {
      const p = stack.pop()!;
      size++;
      const x = p % w;
      const y = (p - x) / w;
      if (x > 0 && mask[p - 1] && !labels[p - 1]) (labels[p - 1] = next), stack.push(p - 1);
      if (x < w - 1 && mask[p + 1] && !labels[p + 1]) (labels[p + 1] = next), stack.push(p + 1);
      if (y > 0 && mask[p - w] && !labels[p - w]) (labels[p - w] = next), stack.push(p - w);
      if (y < h - 1 && mask[p + w] && !labels[p + w]) (labels[p + w] = next), stack.push(p + w);
    }
    if (size > bestSize) (bestSize = size), (best = next);
    next++;
  }
  const out = new Uint8Array(w * h);
  if (!best) return out;
  for (let i = 0; i < w * h; i++) out[i] = labels[i] === best ? 1 : 0;

  const outside = new Uint8Array(w * h);
  const push = (p: number) => {
    if (!out[p] && !outside[p]) (outside[p] = 1), stack.push(p);
  };
  for (let x = 0; x < w; x++) push(x), push((h - 1) * w + x);
  for (let y = 0; y < h; y++) push(y * w), push(y * w + w - 1);
  while (stack.length) {
    const p = stack.pop()!;
    const x = p % w;
    if (x > 0) push(p - 1);
    if (x < w - 1) push(p + 1);
    if (p >= w) push(p - w);
    if (p < w * (h - 1)) push(p + w);
  }
  for (let i = 0; i < w * h; i++) out[i] = outside[i] ? 0 : 1;
  return out;
}

export function maskArea(mask: Uint8Array): number {
  let a = 0;
  for (let i = 0; i < mask.length; i++) a += mask[i];
  return a;
}

function boundaryPoints(mask: Uint8Array, w: number, h: number): Point[] {
  const pts: Point[] = [];
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      if (!mask[i]) continue;
      if (x === 0 || y === 0 || x === w - 1 || y === h - 1 || !mask[i - 1] || !mask[i + 1] || !mask[i - w] || !mask[i + w]) {
        pts.push({ x, y });
      }
    }
  }
  return pts;
}

function cross(o: Point, a: Point, b: Point) {
  return (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
}

function convexHull(points: Point[]): Point[] {
  const pts = [...points].sort((a, b) => a.x - b.x || a.y - b.y);
  if (pts.length < 3) return pts;
  const lower: Point[] = [];
  for (const p of pts) {
    while (lower.length >= 2 && cross(lower[lower.length - 2], lower[lower.length - 1], p) <= 0) lower.pop();
    lower.push(p);
  }
  const upper: Point[] = [];
  for (let i = pts.length - 1; i >= 0; i--) {
    const p = pts[i];
    while (upper.length >= 2 && cross(upper[upper.length - 2], upper[upper.length - 1], p) <= 0) upper.pop();
    upper.push(p);
  }
  upper.pop();
  lower.pop();
  return lower.concat(upper);
}

export function polygonArea(poly: Point[]): number {
  let a = 0;
  for (let i = 0; i < poly.length; i++) {
    const p = poly[i];
    const q = poly[(i + 1) % poly.length];
    a += p.x * q.y - q.x * p.y;
  }
  return Math.abs(a) / 2;
}

function reduceToQuad(hull: Point[]): Point[] {
  const poly = [...hull];
  while (poly.length > 4) {
    let minIdx = 0;
    let minLoss = Infinity;
    for (let i = 0; i < poly.length; i++) {
      const a = poly[(i - 1 + poly.length) % poly.length];
      const b = poly[i];
      const c = poly[(i + 1) % poly.length];
      const loss = Math.abs(cross(a, b, c));
      if (loss < minLoss) (minLoss = loss), (minIdx = i);
    }
    poly.splice(minIdx, 1);
  }
  return poly;
}

function distToSegment(p: Point, a: Point, b: Point): { d: number; t: number } {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const len2 = dx * dx + dy * dy || 1;
  const t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / len2;
  const cx = a.x + Math.max(0, Math.min(1, t)) * dx;
  const cy = a.y + Math.max(0, Math.min(1, t)) * dy;
  return { d: Math.hypot(p.x - cx, p.y - cy), t };
}

type Line = { px: number; py: number; dx: number; dy: number };

function fitLine(pts: Point[]): Line | null {
  if (pts.length < 5) return null;
  let mx = 0;
  let my = 0;
  for (const p of pts) (mx += p.x), (my += p.y);
  mx /= pts.length;
  my /= pts.length;
  let sxx = 0;
  let sxy = 0;
  let syy = 0;
  for (const p of pts) {
    const dx = p.x - mx;
    const dy = p.y - my;
    sxx += dx * dx;
    sxy += dx * dy;
    syy += dy * dy;
  }
  const angle = 0.5 * Math.atan2(2 * sxy, sxx - syy);
  return { px: mx, py: my, dx: Math.cos(angle), dy: Math.sin(angle) };
}

function intersect(l1: Line, l2: Line): Point | null {
  const det = l1.dx * l2.dy - l1.dy * l2.dx;
  if (Math.abs(det) < 1e-6) return null;
  const t = ((l2.px - l1.px) * l2.dy - (l2.py - l1.py) * l2.dx) / det;
  return { x: l1.px + t * l1.dx, y: l1.py + t * l1.dy };
}

export function orderQuad(pts: Point[]): Quad {
  const cx = pts.reduce((s, p) => s + p.x, 0) / pts.length;
  const cy = pts.reduce((s, p) => s + p.y, 0) / pts.length;
  const sorted = [...pts].sort((a, b) => Math.atan2(a.y - cy, a.x - cx) - Math.atan2(b.y - cy, b.x - cx));
  let start = 0;
  for (let i = 1; i < 4; i++) if (sorted[i].x + sorted[i].y < sorted[start].x + sorted[start].y) start = i;
  return [0, 1, 2, 3].map((k) => sorted[(start + k) % 4]) as Quad;
}

export interface QuadFit {
  quad: Quad;
  rectangularity: number;
}

export function fitQuad(mask: Uint8Array, w: number, h: number): QuadFit | null {
  const boundary = boundaryPoints(mask, w, h);
  if (boundary.length < 20) return null;
  const hull = convexHull(boundary);
  if (hull.length < 4) return null;
  const rough = reduceToQuad(hull);

  const tol = Math.max(3, Math.min(w, h) * 0.02);
  const sides: Point[][] = [[], [], [], []];
  for (const p of boundary) {
    for (let s = 0; s < 4; s++) {
      const { d, t } = distToSegment(p, rough[s], rough[(s + 1) % 4]);
      if (d < tol && t > 0.12 && t < 0.88) {
        sides[s].push(p);
        break;
      }
    }
  }
  const lines = sides.map(fitLine);
  let corners: Point[] = rough;
  if (lines.every(Boolean)) {
    const refined: Point[] = [];
    for (let s = 0; s < 4; s++) {
      const c = intersect(lines[(s + 3) % 4]!, lines[s]!);
      if (!c) break;
      refined.push(c);
    }
    const limit = Math.min(w, h) * 0.1;
    if (refined.length === 4 && refined.every((c, i) => Math.hypot(c.x - rough[i].x, c.y - rough[i].y) < limit)) {
      corners = refined;
    }
  }
  const quad = orderQuad(corners);
  const area = maskArea(mask);
  const qa = polygonArea(quad);
  return { quad, rectangularity: qa > 0 ? Math.min(area, qa) / Math.max(area, qa) : 0 };
}

export function quadSize(q: Quad): { width: number; height: number } {
  const d = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y);
  return {
    width: Math.round((d(q[0], q[1]) + d(q[3], q[2])) / 2),
    height: Math.round((d(q[0], q[3]) + d(q[1], q[2])) / 2),
  };
}

function homography(q: Quad, W: number, H: number): number[] {
  const src = [
    [0, 0],
    [W, 0],
    [W, H],
    [0, H],
  ];
  const A: number[][] = [];
  const b: number[] = [];
  for (let i = 0; i < 4; i++) {
    const [x, y] = src[i];
    const { x: u, y: v } = q[i];
    A.push([x, y, 1, 0, 0, 0, -u * x, -u * y]);
    b.push(u);
    A.push([0, 0, 0, x, y, 1, -v * x, -v * y]);
    b.push(v);
  }
  for (let c = 0; c < 8; c++) {
    let piv = c;
    for (let r = c + 1; r < 8; r++) if (Math.abs(A[r][c]) > Math.abs(A[piv][c])) piv = r;
    [A[c], A[piv]] = [A[piv], A[c]];
    [b[c], b[piv]] = [b[piv], b[c]];
    for (let r = 0; r < 8; r++) {
      if (r === c) continue;
      const f = A[r][c] / A[c][c];
      for (let k = c; k < 8; k++) A[r][k] -= f * A[c][k];
      b[r] -= f * b[c];
    }
  }
  return [...b.map((v, i) => v / A[i][i]), 1];
}

export function warpQuad(src: RGBAImage, quad: Quad, size = quadSize(quad)): RGBAImage {
  const { width: W, height: H } = size;
  const m = homography(quad, W, H);
  const out = new Uint8ClampedArray(W * H * 4);
  const sw = src.width;
  const sh = src.height;
  const s = src.data;
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const X = x + 0.5;
      const Y = y + 0.5;
      const z = m[6] * X + m[7] * Y + m[8];
      const u = Math.min(sw - 1.001, Math.max(0, (m[0] * X + m[1] * Y + m[2]) / z - 0.5));
      const v = Math.min(sh - 1.001, Math.max(0, (m[3] * X + m[4] * Y + m[5]) / z - 0.5));
      const x0 = Math.floor(u);
      const y0 = Math.floor(v);
      const fx = u - x0;
      const fy = v - y0;
      const i00 = (y0 * sw + x0) * 4;
      const i10 = i00 + 4;
      const i01 = i00 + sw * 4;
      const i11 = i01 + 4;
      const o = (y * W + x) * 4;
      for (let c = 0; c < 4; c++) {
        out[o + c] =
          (s[i00 + c] * (1 - fx) + s[i10 + c] * fx) * (1 - fy) + (s[i01 + c] * (1 - fx) + s[i11 + c] * fx) * fy;
      }
    }
  }
  return { data: out, width: W, height: H };
}

export function cutoutMask(src: RGBAImage, mask: Uint8Array): RGBAImage {
  const { width: w, height: h } = src;
  let x0 = w;
  let y0 = h;
  let x1 = -1;
  let y1 = -1;
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++)
      if (mask[y * w + x]) {
        if (x < x0) x0 = x;
        if (x > x1) x1 = x;
        if (y < y0) y0 = y;
        if (y > y1) y1 = y;
      }
  if (x1 < 0) return { data: new Uint8ClampedArray(4), width: 1, height: 1 };
  const W = x1 - x0 + 1;
  const H = y1 - y0 + 1;
  const out = new Uint8ClampedArray(W * H * 4);
  for (let y = 0; y < H; y++)
    for (let x = 0; x < W; x++) {
      const si = (y + y0) * w + (x + x0);
      const o = (y * W + x) * 4;
      out[o] = src.data[si * 4];
      out[o + 1] = src.data[si * 4 + 1];
      out[o + 2] = src.data[si * 4 + 2];
      out[o + 3] = mask[si] ? 255 : 0;
    }
  return { data: out, width: W, height: H };
}
