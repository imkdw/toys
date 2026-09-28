import { forwardRef, useCallback, useEffect, useImperativeHandle, useRef, useState } from 'react';
import type { PromptPoint } from '../core/segmenter.ts';
import type { Quad, RGBAImage } from '../core/geometry.ts';
import type { Candidate, WorkerResponse } from '../worker.ts';
import { segmenter } from '../lib/segmenterClient.ts';
import { PROC_MAX, decode, download, renderOutput, toImageData, toPng, type OutputMode } from '../lib/image.ts';
import { makeZip } from '../zip.ts';

type Status = 'wait' | 'busy' | 'done' | 'fail';

interface Item {
  id: number;
  file: File;
  name: string;
  thumb: string;
  proc?: RGBAImage;
  quad: Quad | null;
  mask: Uint8Array | null;
  points: PromptPoint[];
  options: Candidate[];
  selected: number;
  status: Status;
}

export interface EditorHandle {
  addFiles(files: File[]): void;
}

const STATUS_LABEL: Record<Status, string> = { wait: '대기 중', busy: '찾는 중', done: '완료', fail: '직접 지정 필요' };
let nextId = 1;

export const Editor = forwardRef<EditorHandle>(function Editor(_, ref) {
  const [items, setItems] = useState<Item[]>([]);
  const [currentId, setCurrentId] = useState<number | null>(null);
  const [mode, setMode] = useState<OutputMode>('warp');
  const [dragOver, setDragOver] = useState(false);
  const itemsRef = useRef(items);
  itemsRef.current = items;

  const current = items.find((i) => i.id === currentId) ?? null;

  const update = useCallback((id: number, patch: Partial<Item>) => {
    setItems((list) => list.map((i) => (i.id === id ? { ...i, ...patch } : i)));
  }, []);

  const get = (id: number) => itemsRef.current.find((i) => i.id === id)!;

  async function ensureProc(id: number): Promise<RGBAImage> {
    const item = get(id);
    if (item.proc) return item.proc;
    const { img } = await decode(item.file, PROC_MAX);
    item.proc = img;
    update(id, { proc: img });
    return img;
  }

  async function embed(id: number, detect: boolean, send: (r: any) => Promise<WorkerResponse>) {
    const proc = await ensureProc(id);
    const res = await send({ type: 'image', id, data: new Uint8ClampedArray(proc.data), width: proc.width, height: proc.height, detect });
    segmenter.loadedId = id;
    if (res.type === 'error') throw new Error(res.text);
    if (res.type === 'result') update(id, { quad: res.quad, mask: res.mask, options: [], status: res.quad ? 'done' : 'fail' });
  }

  function autoDetect(id: number) {
    update(id, { status: 'busy', points: [], options: [] });
    segmenter
      .enqueue((send) => embed(id, true, send))
      .catch((err) => {
        console.error(err);
        update(id, { status: 'fail' });
      });
  }

  function addFiles(files: File[]) {
    const added: Item[] = files
      .filter((f) => f.type.startsWith('image/'))
      .map((file) => ({
        id: nextId++,
        file,
        name: file.name.replace(/\.[^.]+$/, ''),
        thumb: URL.createObjectURL(file),
        quad: null,
        mask: null,
        points: [],
        options: [],
        selected: 0,
        status: 'wait',
      }));
    if (!added.length) return;
    itemsRef.current = [...itemsRef.current, ...added];
    setItems(itemsRef.current);
    setCurrentId((c) => c ?? added[0].id);
    added.forEach((i) => autoDetect(i.id));
  }

  useImperativeHandle(ref, () => ({ addFiles }));

  useEffect(() => {
    if (current && !current.proc) void ensureProc(current.id);
  }, [current?.id]);

  function addPoint(id: number, p: PromptPoint) {
    const points = [...get(id).points, p];
    update(id, { points });
    segmenter.setStatus('선택 영역 계산 중...');
    segmenter.enqueue(async (send) => {
      if (segmenter.loadedId !== id) await embed(id, false, send);
      const res = await send({ type: 'points', id, points });
      if (res.type === 'candidates' && res.options.length) {
        const o = res.options[res.selected];
        update(id, { options: res.options, selected: res.selected, mask: o.mask, quad: o.quad, status: 'done' });
      }
      segmenter.setStatus('선택 완료');
    });
  }

  function pickSize(idx: number) {
    if (!current) return;
    const o = current.options[idx];
    update(current.id, { selected: idx, mask: o.mask, quad: o.quad.map((p) => ({ ...p })) as Quad });
  }

  async function renderFull(item: Item): Promise<Blob | null> {
    if (!item.quad) return null;
    const proc = item.proc ?? (await decode(item.file, PROC_MAX)).img;
    const { img } = await decode(item.file);
    return toPng(renderOutput(img, item.quad, item.mask, proc.width, proc.height, mode));
  }

  async function saveOne() {
    if (!current) return;
    const blob = await renderFull(current);
    if (blob) download(blob, `${current.name}_crop.png`);
  }

  async function saveAll() {
    const files: { name: string; data: Uint8Array }[] = [];
    for (const [i, item] of items.entries()) {
      segmenter.setStatus(`ZIP 만드는 중 ${i + 1}/${items.length}`);
      const blob = await renderFull(item);
      if (blob) files.push({ name: `${item.name}_crop.png`, data: new Uint8Array(await blob.arrayBuffer()) });
    }
    if (!files.length) return segmenter.setStatus('저장할 결과가 없어요');
    download(makeZip(files), 'cards.zip');
    segmenter.setStatus(`${files.length}장 저장 완료`);
  }

  useEffect(() => {
    const onPaste = (e: ClipboardEvent) => {
      const files = [...(e.clipboardData?.files ?? [])];
      if (files.length) addFiles(files);
    };
    window.addEventListener('paste', onPaste);
    return () => window.removeEventListener('paste', onPaste);
  }, []);

  const doneCount = items.filter((i) => i.status === 'done').length;

  return (
    <div
      className="editor"
      onDragOver={(e) => {
        e.preventDefault();
        setDragOver(true);
      }}
      onDragLeave={() => setDragOver(false)}
      onDrop={(e) => {
        e.preventDefault();
        setDragOver(false);
        addFiles([...e.dataTransfer.files]);
      }}
    >
      <aside className="panel list">
        <label className={`drop ${dragOver ? 'over' : ''}`}>
          <strong>사진 올리기</strong>
          <span>끌어다 놓거나, 눌러서 선택하거나, 붙여넣기</span>
          <input type="file" accept="image/*" multiple hidden onChange={(e) => addFiles([...(e.target.files ?? [])])} />
        </label>
        {items.length > 0 && (
          <p className="count">
            {doneCount}/{items.length}장 완료
          </p>
        )}
        <ul>
          {items.map((i) => (
            <li key={i.id} className={i.id === currentId ? 'active' : ''} onClick={() => setCurrentId(i.id)}>
              <img src={i.thumb} alt="" />
              <div>
                <span className="name">{i.file.name}</span>
                <span className={`badge ${i.status}`}>{STATUS_LABEL[i.status]}</span>
              </div>
            </li>
          ))}
        </ul>
      </aside>

      <section className="panel stage-wrap">
        {current?.proc ? (
          <StageCanvas
            item={current}
            onQuad={(quad) => update(current.id, { quad, status: 'done' })}
            onPoint={(p) => addPoint(current.id, p)}
          />
        ) : (
          <div className="empty">
            <strong>카드 사진을 올려주세요</strong>
            <span>카드 영역을 자동으로 찾아서 반듯하게 잘라드려요</span>
          </div>
        )}
        <div className="toolbar">
          <p className="hint">
            <b>클릭</b> 이 영역 선택 <b>Alt+클릭</b> 제외 <b>꼭짓점 드래그</b> 미세 조정
          </p>
          <div className="actions">
            {current && current.options.length > 1 && (
              <div className="segmented">
                {['작게', '중간', '크게'].map((label, idx) => (
                  <button
                    key={label}
                    disabled={idx >= current.options.length}
                    className={idx === current.selected ? 'on' : ''}
                    onClick={() => pickSize(idx)}
                  >
                    {label}
                  </button>
                ))}
              </div>
            )}
            <button className="btn weak" disabled={!current} onClick={() => current && update(current.id, { points: [], options: [] })}>
              점 지우기
            </button>
            <button className="btn weak" disabled={!current} onClick={() => current && autoDetect(current.id)}>
              다시 찾기
            </button>
          </div>
        </div>
      </section>

      <aside className="panel output">
        <div className="segmented full">
          <button className={mode === 'warp' ? 'on' : ''} onClick={() => setMode('warp')}>
            반듯하게 펴기
          </button>
          <button className={mode === 'cutout' ? 'on' : ''} onClick={() => setMode('cutout')}>
            모양대로 오리기
          </button>
        </div>
        <Preview item={current} mode={mode} />
        <button className="btn primary" disabled={!current?.quad} onClick={saveOne}>
          이 사진 저장
        </button>
        <button className="btn weak" disabled={!doneCount} onClick={saveAll}>
          전체 ZIP으로 저장
        </button>
      </aside>
    </div>
  );
});

function StageCanvas({ item, onQuad, onPoint }: { item: Item; onQuad: (q: Quad) => void; onPoint: (p: PromptPoint) => void }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const baseRef = useRef<HTMLCanvasElement>(document.createElement('canvas'));
  const overlayRef = useRef<HTMLCanvasElement>(document.createElement('canvas'));
  const drag = useRef<{ idx: number; moved: boolean; quad: Quad } | null>(null);
  const [, force] = useState(0);
  const proc = item.proc!;

  useEffect(() => {
    const b = baseRef.current;
    b.width = proc.width;
    b.height = proc.height;
    b.getContext('2d')!.putImageData(toImageData(proc), 0, 0);
  }, [proc]);

  useEffect(() => {
    const o = overlayRef.current;
    o.width = proc.width;
    o.height = proc.height;
    const cx = o.getContext('2d')!;
    cx.clearRect(0, 0, o.width, o.height);
    if (!item.mask) return;
    const d = new ImageData(proc.width, proc.height);
    for (let i = 0; i < item.mask.length; i++) if (!item.mask[i]) d.data[i * 4 + 3] = 140;
    cx.putImageData(d, 0, 0);
  }, [item.mask, proc]);

  const quad = drag.current?.quad ?? item.quad;

  useEffect(() => {
    const c = canvasRef.current!;
    c.width = proc.width;
    c.height = proc.height;
    const ctx = c.getContext('2d')!;
    const s = proc.width / c.getBoundingClientRect().width || 1;
    ctx.drawImage(baseRef.current, 0, 0);
    ctx.drawImage(overlayRef.current, 0, 0);
    if (quad) {
      ctx.lineWidth = 2.5 * s;
      ctx.strokeStyle = '#3182f6';
      ctx.fillStyle = 'rgba(49,130,246,0.08)';
      ctx.beginPath();
      quad.forEach((p, i) => (i ? ctx.lineTo(p.x, p.y) : ctx.moveTo(p.x, p.y)));
      ctx.closePath();
      ctx.fill();
      ctx.stroke();
      for (const p of quad) {
        ctx.beginPath();
        ctx.arc(p.x, p.y, 8 * s, 0, Math.PI * 2);
        ctx.fillStyle = '#fff';
        ctx.fill();
        ctx.lineWidth = 3 * s;
        ctx.stroke();
      }
    }
    for (const p of item.points) {
      ctx.beginPath();
      ctx.arc(p.x * proc.width, p.y * proc.height, 6 * s, 0, Math.PI * 2);
      ctx.fillStyle = p.label ? '#3182f6' : '#f04452';
      ctx.fill();
      ctx.lineWidth = 2 * s;
      ctx.strokeStyle = '#fff';
      ctx.stroke();
    }
  });

  const toImage = (e: React.PointerEvent) => {
    const r = canvasRef.current!.getBoundingClientRect();
    return { x: ((e.clientX - r.left) / r.width) * proc.width, y: ((e.clientY - r.top) / r.height) * proc.height };
  };

  return (
    <div className="stage">
      <canvas
        ref={canvasRef}
        onContextMenu={(e) => e.preventDefault()}
        onPointerDown={(e) => {
          const p = toImage(e);
          const s = proc.width / canvasRef.current!.getBoundingClientRect().width;
          const idx = item.quad?.findIndex((q) => Math.hypot(q.x - p.x, q.y - p.y) < 16 * s) ?? -1;
          if (idx >= 0) {
            drag.current = { idx, moved: false, quad: item.quad!.map((q) => ({ ...q })) as Quad };
            canvasRef.current!.setPointerCapture(e.pointerId);
          }
        }}
        onPointerMove={(e) => {
          if (!drag.current) return;
          drag.current.quad[drag.current.idx] = toImage(e);
          drag.current.moved = true;
          force((n) => n + 1);
        }}
        onPointerUp={(e) => {
          const d = drag.current;
          drag.current = null;
          if (d?.moved) return onQuad(d.quad);
          const p = toImage(e);
          onPoint({ x: p.x / proc.width, y: p.y / proc.height, label: e.button === 2 || e.altKey ? 0 : 1 });
        }}
      />
    </div>
  );
}

function Preview({ item, mode }: { item: Item | null; mode: OutputMode }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const c = ref.current!;
    if (!item?.proc || !item.quad) {
      c.width = c.height = 0;
      return;
    }
    const out = renderOutput(item.proc, item.quad, item.mask, item.proc.width, item.proc.height, mode);
    c.width = out.width;
    c.height = out.height;
    c.getContext('2d')!.putImageData(toImageData(out), 0, 0);
  }, [item?.proc, item?.quad, item?.mask, mode]);
  return (
    <div className="preview">
      <canvas ref={ref} />
      {!item?.quad && <span>결과가 여기에 보여요</span>}
    </div>
  );
}
