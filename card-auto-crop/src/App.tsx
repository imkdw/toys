import { useEffect, useRef, useState } from 'react';
import { Editor, type EditorHandle } from './pages/Editor.tsx';
import { Gallery } from './pages/Gallery.tsx';
import { segmenter } from './lib/segmenterClient.ts';

type Tab = 'editor' | 'gallery';

export function App() {
  const [tab, setTab] = useState<Tab>(() => (location.hash === '#gallery' ? 'gallery' : 'editor'));
  const [status, setStatus] = useState(segmenter.status);
  const editor = useRef<EditorHandle>(null);

  useEffect(() => segmenter.subscribe(setStatus), []);
  useEffect(() => {
    history.replaceState(null, '', tab === 'gallery' ? '#gallery' : '#');
  }, [tab]);

  return (
    <div className="app">
      <header className="topbar">
        <h1>카드 자동 크롭</h1>
        <nav className="tabs">
          <button className={tab === 'editor' ? 'on' : ''} onClick={() => setTab('editor')}>
            크롭하기
          </button>
          <button className={tab === 'gallery' ? 'on' : ''} onClick={() => setTab('gallery')}>
            예시 갤러리
          </button>
        </nav>
        <span className="status">{status}</span>
      </header>
      <main hidden={tab !== 'editor'}>
        <Editor ref={editor} />
      </main>
      {tab === 'gallery' && (
        <main className="scroll">
          <Gallery
            onOpen={(file) => {
              editor.current?.addFiles([file]);
              setTab('editor');
            }}
          />
        </main>
      )}
    </div>
  );
}
