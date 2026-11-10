// Rust(src-tauri/src) 구조체와 1:1로 맞춘 타입

export type Language = 'en' | 'ja'
export type Source = 'builtin' | 'user'
export type Mode = 'reveal' | 'instant'
export type WordStatus = 'new' | 'learning' | 'known'
export type Answer = 'known' | 'again' | 'dismiss' | 'timeout'
export type PauseFor = 'hour' | 'today' | 'forever'
export type Corner = 'bottom-right' | 'bottom-left' | 'top-right' | 'top-left'

export interface Settings {
  onboarded: boolean
  language: Language
  source: Source
  level: string
  mode: Mode
  intervalMin: number
  dailyGoal: number
  popupSeconds: number
  popupCorner: Corner | 'random'
  hoursEnabled: boolean
  hoursStart: string
  hoursEnd: string
  pausedUntil: number | null
  autostart: boolean
}

export interface LevelCount {
  level: string
  total: number
  learning: number
  known: number
  examples: string[]
}

export interface LanguageInfo {
  code: Language
  available: boolean
  deckId: number | null
  levels: LevelCount[]
}

export interface Bootstrap {
  settings: Settings
  languages: LanguageInfo[]
  userWordCount: number
}

export interface Today {
  seen: number
  known: number
  again: number
  goal: number
  pool: { total: number; learning: number; known: number; due: number }
  nextPopupAt: number
  pausedUntil: number | null
  withinHours: boolean
  statusText: string
  counting: boolean
  showing: boolean
}

export interface WordRow {
  id: number
  term: string
  reading: string | null
  meaning: string
  level: string | null
  status: WordStatus
}

export interface WordPage {
  total: number
  items: WordRow[]
}

export interface UserDeck {
  id: number
  name: string
  enabled: boolean
  createdAt: number
  total: number
  learning: number
  known: number
}

export interface PreviewRow {
  row: number
  term: string
  meaning: string
  status: 'ok' | 'duplicate' | 'error'
  message: string | null
}

export interface Preview {
  fileName: string
  suggestedName: string
  rows: PreviewRow[]
  okCount: number
  duplicateCount: number
  errorCount: number
  skippedBlank: number
}

export interface SeedReport {
  language: Language
  files: string[]
  accepted: number
  retired: number
  skipped: { file: string; row: number; term: string; reason: string }[]
  syncedAt: number
}

export interface PopupPayload {
  wordId: number
  term: string
  reading: string | null
  meaning: string
  deckName: string
  kind: 'new' | 'learning' | 'review'
  mode: Mode
  seconds: number
  corner: Corner
  seq: number
}
