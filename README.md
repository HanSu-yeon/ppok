# 뽁

> 오늘도 단어 하나, 뽁!

일하는 동안 일정한 간격으로 화면 오른쪽 아래에 작은 단어 카드가 나타나는 Windows 단어 암기 앱입니다.
서버·회원가입 없이 오프라인으로 동작하고, 데이터는 이 컴퓨터에만 저장됩니다.

- Tauri 2 · React 19 · TypeScript · Tailwind CSS 4 · SQLite(rusqlite)
- 설계: [docs/DESIGN.md](docs/DESIGN.md) · Windows 검증: [docs/WINDOWS_TEST.md](docs/WINDOWS_TEST.md)

## 개발

```bash
npm install
npm run tauri dev          # 앱 실행 (macOS에서도 동작, 단 포커스·전체 화면 감지는 Windows 전용)
npm run dev                # 브라우저에서 화면만 확인 (가짜 데이터, ?onboarded=0 이면 온보딩)
npm run test:rust          # 복습 로직·검증·업로드·기본 단어 동기화 테스트
npm run validate:words     # 기본 단어 CSV 검증
```

Windows 설치 파일은 Windows에서 `npm run tauri build` 또는 GitHub Actions의 **Windows build** 워크플로로 만듭니다.

## 기본 단어장 데이터

`src-tauri/resources/words/<언어>/*.csv` 에 있는 파일을 앱 시작 시 로컬 DB로 가져옵니다.
지금은 검증된 샘플 48개(난이도별 12개)만 들어 있습니다.

```csv
word,meaning,level
postpone,"연기하다, 미루다",intermediate
```

| level | 표시 | 목표 수 |
|---|---|---|
| `basic` | 기초 | 2,000 |
| `intermediate` | 중급 | 3,000 |
| `advanced` | 고급 | 3,000 |
| `expert` | 심화 | 2,000 |

10,000개 파일을 넣을 때:

1. 같은 폴더에 CSV를 추가합니다 (여러 파일로 나눠도 됨, 파일 이름 순서 = 출제 순서).
2. `npm run validate:words` 로 중복·빈 값·형식 오류를 0개로 만듭니다.
3. 다시 빌드해 배포하면 끝. 파일 해시가 바뀌면 앱이 자동으로 동기화하고,
   같은 단어(대소문자·공백 무시)는 학습 기록을 그대로 유지한 채 뜻·난이도만 갱신합니다.
   새 파일에서 빠진 단어는 지우지 않고 출제에서만 제외합니다.

### 일본어

`ja/` 폴더에 `reading`(읽기) 열을 더해 넣습니다. 한자가 있는 단어는 읽기가 반드시 필요합니다.

```csv
word,reading,meaning,level
間に合う,まにあう,"시간에 맞추다, 늦지 않다",N4
```

목표는 일상 회화 수준(N5~N3, 약 3,700개)입니다. 지금은 검증된 샘플 36개(레벨별 12개)가 들어 있고,
N2·N1은 데이터를 넣으면 자동으로 선택할 수 있게 됩니다.

| level | 목표 수(추정) | 누적 |
|---|---|---|
| `N5` | 800 | 800 |
| `N4` | 700 | 1,500 |
| `N3` | 2,200 | 3,700 |
| `N2` / `N1` | 나중에 | — |

## 폴더 구조

```
src/main/      메인 창 (온보딩, 오늘 / 기본 단어장 / 내 단어장 / 설정)
src/popup/     단어 팝업 카드
src/shared/    API 래퍼, 타입, UI 컴포넌트
src-tauri/src/ Rust 백엔드 (db, seed, validate, import, srs, scheduler, popup, tray, platform)
scripts/       단어 데이터 검증 스크립트
docs/          설계 문서, Windows 체크리스트, 업로드 샘플 파일
```
