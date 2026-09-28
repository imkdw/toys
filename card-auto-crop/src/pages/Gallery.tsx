import { useEffect, useMemo, useState } from 'react';
import type { Quad } from '../core/geometry.ts';

interface Detection {
  file: string;
  width: number;
  height: number;
  quad: Quad | null;
  rectangularity: number;
}

type Filter = 'all' | 'check';

const CHECK_THRESHOLD = 0.96;

const needsCheck = (d: Detection) => !d.quad || d.rectangularity < CHECK_THRESHOLD;

export function Gallery({ onOpen }: { onOpen: (file: File) => void }) {
  const [data, setData] = useState<{ dir: string; items: Detection[] } | null>(null);
  const [error, setError] = useState(false);
  const [filter, setFilter] = useState<Filter>('all');
  const [showMark, setShowMark] = useState(true);

  useEffect(() => {
    fetch('/detections.json')
      .then((r) => (r.ok ? r.json() : Promise.reject()))
      .then(setData)
      .catch(() => setError(true));
  }, []);

  const list = useMemo(() => {
    const items = data?.items ?? [];
    return filter === 'check' ? items.filter(needsCheck) : items;
  }, [data, filter]);

  if (error)
    return (
      <div className="gallery-empty">
        <strong>감지 결과가 아직 없어요</strong>
        <span>터미널에서 아래 명령을 먼저 실행해주세요</span>
        <code>npm run detect</code>
      </div>
    );
  if (!data) return <div className="gallery-empty">불러오는 중...</div>;

  const checkCount = data.items.filter(needsCheck).length;

  async function open(d: Detection) {
    const blob = await (await fetch(`/${data!.dir}/${d.file}`)).blob();
    onOpen(new File([blob], d.file, { type: blob.type }));
  }

  return (
    <div className="gallery">
      <div className="gallery-head">
        <div>
          <h2>예시 이미지 {data.items.length}장</h2>
          <p>파란 테두리 안쪽이 잘라낼 영역이에요. 어두운 부분은 버려져요.</p>
        </div>
        <div className="gallery-controls">
          <div className="segmented">
            <button className={filter === 'all' ? 'on' : ''} onClick={() => setFilter('all')}>
              전체 {data.items.length}
            </button>
            <button className={filter === 'check' ? 'on' : ''} onClick={() => setFilter('check')}>
              확인 필요 {checkCount}
            </button>
          </div>
          <label className="switch">
            <input type="checkbox" checked={showMark} onChange={(e) => setShowMark(e.target.checked)} />
            <span />
            영역 표시
          </label>
        </div>
      </div>

      <div className="grid">
        {list.map((d) => (
          <figure key={d.file} className="card">
            <div className="thumb" style={{ aspectRatio: `${d.width} / ${d.height}` }}>
              <img src={`/${data.dir}/${d.file}`} alt="" loading="lazy" />
              {showMark && d.quad && <Mark d={d} />}
            </div>
            <figcaption>
              <span className={`badge ${needsCheck(d) ? 'fail' : 'done'}`}>
                {d.quad ? (needsCheck(d) ? '확인 필요' : '정상') : '못 찾음'}
              </span>
              <span className="file" title={d.file}>
                {d.file}
              </span>
              <button className="btn weak small" onClick={() => open(d)}>
                편집
              </button>
            </figcaption>
          </figure>
        ))}
      </div>
    </div>
  );
}

function Mark({ d }: { d: Detection }) {
  const q = d.quad!;
  const poly = q.map((p) => `${p.x},${p.y}`).join(' ');
  const r = Math.max(d.width, d.height) / 90;
  return (
    <svg viewBox={`0 0 ${d.width} ${d.height}`} preserveAspectRatio="none">
      <path
        d={`M0 0H${d.width}V${d.height}H0Z M${q.map((p) => `${p.x} ${p.y}`).join(' L')}Z`}
        fillRule="evenodd"
        fill="rgba(0,12,30,0.55)"
      />
      <polygon points={poly} fill="rgba(49,130,246,0.1)" stroke="#3182f6" strokeWidth={r / 2.2} strokeLinejoin="round" />
      {q.map((p, i) => (
        <circle key={i} cx={p.x} cy={p.y} r={r} fill="#fff" stroke="#3182f6" strokeWidth={r / 2.5} />
      ))}
    </svg>
  );
}
