
import json, sys
from pathlib import Path
from multiprocessing import Pool
from PIL import Image
R = Path(sys.argv[1]); OUT = Path(sys.argv[2]); spec = json.loads(sys.argv[3])
def one(a):
    src, dst = a
    Image.open(src).save(dst, 'WEBP', quality=88, method=4)
if __name__ == '__main__':
    jobs = []; meta = {}
    for name, s in spec.items():
        d = R / s[0]
        n = len(list(d.glob('[0-9][0-9][0-9][0-9][0-9].png')))
        a = s[1] if len(s) > 1 else 0
        b = min(s[2], n) if len(s) > 2 else n
        (OUT / name).mkdir(parents=True, exist_ok=True)
        for i in range(a, b):
            jobs.append((d / f'{i:05d}.png', OUT / name / f'{i - a}.webp'))
        w, h = Image.open(d / '00000.png').size
        meta[name] = {'frames': b - a, 'width': w, 'height': h}
    with Pool(10) as p: p.map(one, jobs, chunksize=8)
    print(json.dumps(meta))
