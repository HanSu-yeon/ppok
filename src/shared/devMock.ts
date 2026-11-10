// 개발 전용: Tauri 밖(일반 브라우저)에서 `npm run dev`로 화면을 확인할 때 쓰는 가짜 백엔드.
// 배포 빌드에는 포함되지 않는다 (import.meta.env.DEV 분기에서만 동적 import).

import { mockIPC } from '@tauri-apps/api/mocks'
import type { Bootstrap, PopupPayload, Settings, Today, UserDeck, WordRow } from './types'

const params = new URLSearchParams(location.search)

const settings: Settings = {
  onboarded: params.get('onboarded') !== '0',
  language: 'en',
  source: 'builtin',
  level: 'intermediate',
  mode: 'reveal',
  intervalMin: 30,
  dailyGoal: 10,
  popupSeconds: 10,
  popupCorner: 'bottom-right',
  hoursEnabled: true,
  hoursStart: '09:00',
  hoursEnd: '18:00',
  pausedUntil: null,
  autostart: false,
}

const words: WordRow[] = [
  ['postpone', '연기하다, 미루다', 'known'],
  ['efficient', '효율적인', 'learning'],
  ['schedule', '일정, 일정을 잡다', 'new'],
  ['deadline', '마감 기한', 'known'],
  ['colleague', '동료', 'new'],
  ['negotiate', '협상하다', 'new'],
].map(([term, meaning, status], i) => ({ id: i + 1, term, reading: null, meaning, level: 'intermediate', status: status as WordRow['status'] }))

const decks: UserDeck[] = [{ id: 9, name: '업무 영어', enabled: true, createdAt: 1_760_000_000, total: 124, learning: 6, known: 31 }]

const card: PopupPayload = {
  wordId: 1,
  term: params.get('term') ?? 'postpone',
  reading: params.get('reading'),
  meaning: params.get('meaning') ?? '연기하다, 미루다',
  deckName: '기본 영어',
  kind: 'new',
  mode: params.get('mode') === 'instant' ? 'instant' : 'reveal',
  seconds: 600,
  corner: (params.get('corner') as PopupPayload['corner']) ?? 'bottom-right',
  seq: 1,
}

mockIPC((cmd, args) => {
  const a = args as Record<string, unknown>
  switch (cmd) {
    case 'get_bootstrap':
      return {
        settings,
        userWordCount: 124,
        languages: [
          {
            code: 'en',
            available: true,
            deckId: 1,
            levels: [
              { level: 'basic', total: 12, learning: 1, known: 4, examples: ['apple', 'water', 'house'] },
              { level: 'intermediate', total: 12, learning: 2, known: 3, examples: ['postpone', 'efficient', 'schedule'] },
              { level: 'advanced', total: 12, learning: 0, known: 0, examples: ['substantial', 'comprehensive', 'implement'] },
              { level: 'expert', total: 12, learning: 0, known: 0, examples: ['ubiquitous', 'meticulous', 'exacerbate'] },
            ],
          },
          { code: 'ja', available: false, deckId: null, levels: ['N5', 'N4', 'N3', 'N2', 'N1'].map((level) => ({ level, total: 0, learning: 0, known: 0, examples: [] })) },
        ],
      } satisfies Bootstrap
    case 'save_settings':
      Object.assign(settings, a.settings)
      return settings
    case 'get_today':
      return {
        seen: 9,
        known: 6,
        again: 2,
        goal: settings.dailyGoal,
        pool: { total: params.get('empty') ? 0 : 12, learning: 2, known: 3, due: 1 },
        nextPopupAt: Date.now() / 1000 + 900,
        pausedUntil: null,
        withinHours: true,
        statusText: '다음 단어 · 15:30',
        counting: !params.get('empty'),
        showing: false,
      } satisfies Today
    case 'list_words':
      return { total: words.length, items: words }
    case 'list_user_decks':
      return decks
    case 'get_autostart':
      return false
    case 'get_seed_reports':
      return [{ language: 'en', files: ['00-sample.csv'], accepted: 48, retired: 0, skipped: [], syncedAt: Date.now() / 1000 }]
    case 'get_data_dir':
      return 'C:\\Users\\me\\AppData\\Roaming\\app.ppok.desktop'
    case 'popup_current':
      return card
    case 'plugin:event|listen':
      return 1
    default:
      return null
  }
})
