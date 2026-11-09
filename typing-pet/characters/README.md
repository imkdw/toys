# characters

캐릭터 하나당 폴더 하나. `_` 로 시작하는 폴더(`_test`)만 git 에 올리고, 나머지는 올리지 않는다.

```
characters/<name>/
├── manifest.toml   레이어 목록, 마스크, 발끝 좌표, 머리 기준점 ([timing] [anim] [keys] 로 기본값 덮어쓰기)
├── layers/
│   ├── base.png             책상과 소품까지 포함한 기본 장면
│   ├── head/*.png           머리 변형 (마스크 안을 교체)
│   ├── paw_l/*.png          왼앞발 자세
│   ├── paw_r/*.png          오른앞발 자세
│   ├── fx/*.png             선글라스 단독 (일반 합성)
│   ├── full/*.png           제스처 같은 전체 프레임
│   └── mask/*.png           head, paw_l, paw_r 영역 마스크
└── baseline/       조합 시트 기준 이미지 (scripts/verify.sh)
```

모든 레이어는 같은 크기의 정사각형 캔버스다. `art/scripts/build.sh` 가 생성 원본에서 만든다.
설정에서 캐릭터를 지정하지 않으면 `_` 로 시작하지 않는 이름순 첫 번째 폴더를 쓴다.
`_test` 는 색 도형으로 만든 검증용 가짜 세트다 (`art/scripts/make_test_layers.py`).
