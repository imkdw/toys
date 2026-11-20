# typing-pet-intro

[typing-pet](../typing-pet) (macOS 메뉴바에 사는 타이핑 친구 앱)을 알리는 영상을 만드는 Remotion 프로젝트다. 소개 영상과 설치 가이드 영상 두 편을 React 컴포넌트로 만들고, 장면마다 따로 미리 보고 렌더할 수 있다.

영상 속 캐릭터 움직임은 손으로 그린 애니메이션이 아니라 typing-pet 앱이 실제 상태 머신으로 오프스크린 렌더한 프레임이다. `scripts/render-pets.sh` 가 앱에서 프레임을 뽑아 `public/pets/` 에 webp 로 저장하고, 영상은 이 프레임을 타임라인에 맞춰 보여 준다.

## 주요 기능

- **TypingPetIntro** (1920x1080, 30fps, 1341프레임): 장면 8개(Opening / Typing / Levels / Mouse / Idle / Friends / Jobs / Ending)를 0.5초 크로스페이드로 이은 소개 영상. 타이핑 따라 치기, 속도에 따른 단계 변화, 마우스 따라 잡기, 쉬는 동안의 딴짓, 캐릭터 8명, 직군별 책상을 보여 준다.
- **InstallGuide** (1920x1080, 30fps, 2100프레임): 장면 10개(GuideTitle / GuideDrag / GuideBlocked / GuideWhy / GuideOpenAnyway / GuideInput / GuideMenu / GuideResize / GuideKeys / GuideDone)로 된 설치 가이드. DMG 설치, 처음 열 때 막히는 이유와 '그래도 열기', 입력 모니터링 허용, 메뉴바 메뉴로 캐릭터/직군 바꾸기, 크기 조절, 입력 키 보기를 비개발자 눈높이로 설명한다.
- 각 장면은 Studio 의 `Scenes` / `GuideScenes` 폴더에 개별 컴포지션으로도 등록되어 있어 따로 렌더할 수 있다.
- macOS 화면(창, 경고창, 시스템 설정, 메뉴바와 TypingPet 메뉴)은 `src/guide/Mac.tsx`, `src/guide/Tray.tsx` 에서 단순화해 직접 그린다. 메뉴 항목과 순서는 앱의 `src/tray.rs` 와 맞춘다.

## 요구 사항

- Node.js `^20.19.0 || ^22.13.0 || >=24`
- 캐릭터 프레임을 다시 뽑을 때만 필요:
  - 옆 디렉터리의 `../typing-pet` 저장소와 Rust 툴체인(cargo)
  - Pillow 가 설치된 `../typing-pet/art/.venv` 파이썬 가상환경

## 설치와 실행

```console
npm i
scripts/render-pets.sh   # public/pets/ 생성 (git 에 올리지 않는 생성물이라 처음 한 번은 필요)
npm run dev              # Remotion Studio 로 미리 보기
```

`render-pets.sh` 는 `../typing-pet` 을 release 로 빌드한 뒤 `--render-behaviors` / `--render-timeline` 옵션으로 캐릭터별 행동과 시나리오(demo / levels / mouse / burst) 프레임을 800px 투명 PNG 로 뽑고, `scripts/conv.py` 로 클립별 webp 로 바꿔 `public/pets/<clip>/<n>.webp` 에 둔다.

## 사용법

| 명령 | 설명 |
| --- | --- |
| `npm run dev` | Remotion Studio 실행 |
| `npm run build` | `remotion bundle` 로 번들 생성 |
| `npm run lint` | `eslint src` 와 `tsc` 검사 |
| `npm run upgrade` | Remotion 패키지 업그레이드 |

영상 렌더 예시 (결과는 git 에서 제외되는 `out/` 에 둔다):

```console
npx remotion render TypingPetIntro out/typing-pet-intro.mp4
npx remotion render InstallGuide out/typing-pet-install-guide.mp4
npx remotion render GuideMenu out/guide-menu.mp4   # 장면 하나만
```

렌더 설정은 `remotion.config.ts` 에 있다 (Rspack 사용, 프레임 이미지 형식 jpeg, 출력 덮어쓰기, 기본 프리마운트 2초).

## 디렉터리 구조

```
src/
  Root.tsx           컴포지션 등록 (TypingPetIntro, InstallGuide, 장면별 컴포지션)
  Intro.tsx          소개 영상 장면 연결
  InstallGuide.tsx   설치 가이드 장면 연결
  scenes/            소개 영상 장면 8개
  guide/             설치 가이드 장면 10개와 macOS 화면 흉내 부품(Mac.tsx, Tray.tsx)
  Pet.tsx            public/pets 프레임을 타임라인에 맞춰 재생하는 컴포넌트
  Background.tsx     public/bg 배경 그림
  Text.tsx           제목/딱지 글자 스타일
  fonts.ts           Google Fonts Jua 로드
public/
  bg/                배경 그림 (room / sky / card)
  guide/             설치 가이드용 이미지 (앱 아이콘, DMG 배경, SF Symbols 기호 등)
  pets/              render-pets.sh 생성물 (git 제외)
gen/<name>/prompt.txt  public/bg 배경 그림을 만들 때 쓴 이미지 생성 프롬프트
scripts/
  render-pets.sh     typing-pet 앱에서 캐릭터 프레임 뽑기
  conv.py            PNG 프레임을 클립별 webp 로 변환
```
