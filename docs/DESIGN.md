# 뽁 — 설계 문서 (MVP)

> 일하는 동안 화면 오른쪽 아래에 단어 카드가 "뽁!" 하고 나타나는 Windows 단어 암기 앱.

## 1. 아키텍처 한눈에 보기

```
┌───────────────────────── Tauri 2 (Rust) ─────────────────────────┐
│  db.rs         SQLite(rusqlite, bundled) · 마이그레이션 · 쿼리     │
│  seed.rs       resources/words/**.csv → 기본 단어장 동기화          │
│  validate.rs   단어 데이터 검증 (기본 단어장·업로드 공용)            │
│  import.rs     .xlsx(calamine) / .csv(UTF-8·CP949) 파싱 + 미리보기  │
│  srs.rs        간격 반복(라이트너 박스) · 다음 단어 선택             │
│  scheduler.rs  30초 틱 → 간격/시간대/일시정지/전체화면 판단 → 팝업   │
│  popup.rs      팝업 창 위치(작업 영역) · 포커스 안 뺏는 표시         │
│  tray.rs       트레이 메뉴 (지금 보기 / 일시정지 / 설정 / 종료)      │
│  platform/     Windows 전용: 전체화면 감지, SW_SHOWNOACTIVATE        │
│  commands.rs   프론트엔드에 노출하는 invoke 명령                     │
└──────────────────────────────────────────────────────────────────┘
        ▲ invoke / event                          ▲ event "popup:word"
┌───────┴──────── main 창 (index.html) ──┐  ┌────┴──── popup 창 (popup.html) ┐
│ 온보딩 5단계 → 오늘 / 기본 단어장 /       │  │ 단어 카드 · 10초 자동 닫힘      │
│ 내 단어장 / 설정                         │  │ 호버 시 일시정지 · 외웠어/다시   │
└─────────────────────────────────────────┘  └────────────────────────────────┘
```

- 스케줄러는 **Rust에 있습니다.** 메인 창을 닫아도(트레이로 숨겨도) 팝업은 계속 동작합니다.
- 서버·회원가입 없음. DB는 `%APPDATA%\app.ppok.desktop\ppok.db` 한 파일입니다.

## 2. 폴더 구조

```
ppok/
├─ index.html / popup.html        Vite 멀티 페이지 (메인 창 / 팝업 창)
├─ src/
│  ├─ main/                       메인 창 React
│  │  ├─ App.tsx                  온보딩 여부에 따라 분기
│  │  ├─ onboarding/              STEP 1~5
│  │  └─ pages/                   Today, BuiltinDecks, MyDecks, Settings
│  ├─ popup/                      팝업 창 React (Popup.tsx)
│  ├─ shared/                     api.ts(invoke 래퍼) · types.ts · ui 컴포넌트
│  └─ styles.css                  Tailwind v4 + 디자인 토큰
├─ src-tauri/
│  ├─ resources/words/en/*.csv    기본 영어 단어 (지금은 검증된 샘플 48개)
│  ├─ resources/words/ja/         일본어 (비어 있음 → "준비 중")
│  └─ src/…                       위 1장의 모듈
├─ scripts/validate-words.mjs     배포 전 단어 CSV 검증 스크립트
├─ docs/WINDOWS_TEST.md           Windows 실기 검증 체크리스트
└─ .github/workflows/windows.yml  Windows 설치 파일 빌드 (NSIS/MSI)
```

## 3. 데이터 모델

```sql
decks     (id, kind 'builtin'|'user', language, name, enabled, created_at)
words     (id, deck_id, term, term_key, meaning, level, retired, updated_at)
           -- 기본 단어: UNIQUE(deck_id, term_key)  term_key = 소문자·공백정리
progress  (word_id PK, status 'new'|'learning'|'known', box, due_at,
           seen_count, last_seen_at)        -- 단어 데이터와 분리
review_log(id, word_id, action, at)         -- shown/known/again/dismiss/timeout
settings  (key PK, value JSON)
meta      (key PK, value)                   -- 기본 단어 데이터 해시·검증 리포트
```

**기본 단어 데이터 갱신 시 복습 기록 보존**
- 앱 시작 시 `resources/words/<lang>/*.csv` 전체의 해시를 계산해 이전과 다르면 동기화합니다.
- `(deck, term_key)` 기준 upsert → 뜻·난이도만 갱신되고 `words.id`가 유지되므로 `progress`가 그대로 남습니다.
- 새 파일에서 빠진 단어는 삭제하지 않고 `retired=1` (출제 제외, 기록 유지).
- 10,000개 CSV를 같은 폴더에 넣고 다시 빌드하면 자동 반영됩니다.

**단어 CSV 형식** (`word,meaning,level`)

```csv
word,meaning,level
apple,사과,basic
postpone,연기하다,intermediate
```
level: `basic`(기초) · `intermediate`(중급) · `advanced`(고급) · `expert`(심화)
일본어는 `N5`~`N1`.

## 4. 복습 로직 (간단한 라이트너 박스)

| 동작 | 결과 |
|---|---|
| 외웠어 | box +1, status=known, 다음 복습: 1일 → 3일 → 7일 → 14일 → 30일 → 60일 |
| 다시 볼래 | box=0, status=learning, 20분 뒤 다시 출제 대상 |
| 닫기 / 자동 사라짐 | 상태 변화 없음 (학습 완료 처리 안 함), 노출 기록만 남김 |

**다음 단어 선택 순서**
1. 복습 예정 단어 (`due_at <= now`, 오래 밀린 순)
2. 새 단어 (노출 횟수 적은 순 → 데이터 순서, 즉 빈도순)
3. 둘 다 없으면 가장 빨리 돌아올 단어

## 5. 화면 설계

**온보딩** (메인 창, 한 화면에 한 단계 · 하단 이전/다음)
1. 학습 언어 — 영어 / 일본어(준비 중, 비활성)
2. 단어장 — 기본 / 내 단어장 (내 단어장 선택 시 업로드 안내)
3. 난이도 — 기초·중급·고급·심화 (각 단어 수 표시). "내 단어장만"이면 생략
4. 암기 방식 — 맞혀 보기 / 바로 보기 (작은 카드 미리보기)
5. 팝업 — 간격(15분/30분/1시간/직접), 하루 목표(5/10/20), 알림 시간대
→ 완료하면 첫 단어가 바로 "뽁!"

**메인 창** (왼쪽 얇은 내비게이션)
- 오늘: 확인한 단어 · 외운 단어 · 다시 볼 단어 · 목표 달성률(막대 하나), 다음 팝업 예정 시각, [지금 단어 보기]
- 기본 단어장: 난이도 탭, 검색, 단어별 상태
- 내 단어장: 업로드(미리보기·오류 행 표시·이름 지정), 켜기/끄기, 삭제
- 설정: 온보딩 항목 전부 + 표시 시간, 일시정지, Windows 시작 시 실행

**팝업 카드** (340×150 내외, 작업 표시줄 위 오른쪽 아래)
```
┌──────────────────────────────── × ┐
│ postpone                          │
│ 연기하다          (맞혀 보기: "눌러서 뜻 보기") │
│               [다시 볼래] [외웠어] │
└───────────────────────────────────┘
```
- 맞혀 보기: 뜻 공개 전에는 정답 버튼 숨김
- 10초 후 자동 닫힘, 마우스가 올라가 있거나 상호작용 중이면 타이머 정지
- 전체 화면 앱/프레젠테이션 중에는 표시하지 않고 다음 틱에 재시도

## 6. 팝업이 포커스를 뺏지 않게 (Windows)

- 팝업 창: `decorations:false`, `transparent`, `alwaysOnTop`, `skipTaskbar`, `focused:false`, `focusable:false`
- 표시: `ShowWindow(SW_SHOWNOACTIVATE)` + `SetWindowPos(HWND_TOPMOST, SWP_NOACTIVATE)`
- 위치: 모니터 **작업 영역**(작업 표시줄 제외) 기준 오른쪽 아래 16px 여백
- 전체 화면 감지: `SHQueryUserNotificationState` (BUSY / D3D 전체화면 / 프레젠테이션 모드)
  + 전경 창이 모니터 전체를 덮는지 확인

## 7. 하루 목표

하루 목표(5/10/20)는 "오늘 '외웠어'를 누른 단어 수" 기준 달성률로 표시합니다.
팝업 등장 자체를 막지는 않습니다. (MVP 사용자 테스트 후 조정)
