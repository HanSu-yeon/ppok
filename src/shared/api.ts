import { invoke } from '@tauri-apps/api/core'
import type {
  Answer,
  Bootstrap,
  PauseFor,
  PopupPayload,
  Preview,
  SeedReport,
  Settings,
  Today,
  UserDeck,
  WordPage,
} from './types'

export const api = {
  bootstrap: () => invoke<Bootstrap>('get_bootstrap'),
  saveSettings: (settings: Settings) => invoke<Settings>('save_settings', { settings }),
  today: () => invoke<Today>('get_today'),
  showNow: () => invoke<boolean>('show_now'),
  pause: (how: PauseFor) => invoke<void>('pause', { how }),
  resume: () => invoke<void>('resume'),

  listWords: (q: { deckId: number; level?: string | null; query?: string; offset: number; limit: number }) =>
    invoke<WordPage>('list_words', { q }),
  listUserDecks: () => invoke<UserDeck[]>('list_user_decks'),
  setDeckEnabled: (id: number, enabled: boolean) => invoke<void>('set_deck_enabled', { id, enabled }),
  deleteDeck: (id: number) => invoke<void>('delete_deck', { id }),
  previewImport: (path: string) => invoke<Preview>('preview_import', { path }),
  commitImport: (path: string, name: string) =>
    invoke<{ deckId: number; count: number }>('commit_import', { path, name }),
  saveTemplate: (path: string) => invoke<void>('save_template', { path }),

  getAutostart: () => invoke<boolean>('get_autostart'),
  setAutostart: (enabled: boolean) => invoke<boolean>('set_autostart', { enabled }),
  seedReports: () => invoke<SeedReport[]>('get_seed_reports'),
  dataDir: () => invoke<string>('get_data_dir'),

  popupCurrent: () => invoke<PopupPayload | null>('popup_current'),
  popupAnswer: (wordId: number, action: Answer) => invoke<void>('popup_answer', { wordId, action }),
}

export function errorText(e: unknown): string {
  return typeof e === 'string' ? e : e instanceof Error ? e.message : '알 수 없는 오류가 생겼어요'
}
