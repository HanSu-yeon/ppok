import { listen } from '@tauri-apps/api/event'
import { useEffect, useState } from 'react'
import { cx, Mark } from '../shared/ui'
import Onboarding from './onboarding/Onboarding'
import BuiltinPage from './pages/BuiltinPage'
import MyDecksPage from './pages/MyDecksPage'
import SettingsPage from './pages/SettingsPage'
import TodayPage from './pages/TodayPage'
import { useStore } from './store'

export type Page = 'today' | 'builtin' | 'mine' | 'settings'

const NAV: { id: Page; label: string; icon: string }[] = [
  { id: 'today', label: '오늘의 학습', icon: 'M4 12h4l2-6 4 12 2-6h4' },
  { id: 'builtin', label: '기본 단어장', icon: 'M5 4h10a4 4 0 0 1 4 4v12H9a4 4 0 0 1-4-4V4Zm0 12a4 4 0 0 1 4-4h10' },
  { id: 'mine', label: '내 단어장', icon: 'M4 6a2 2 0 0 1 2-2h4l2 2h6a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6Z' },
  {
    id: 'settings',
    label: '설정',
    icon: 'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm7.4-3a7.4 7.4 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7.5 7.5 0 0 0-2-1.2L14.5 3h-5l-.4 2.6a7.5 7.5 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6a7.4 7.4 0 0 0 0 2.4l-2 1.6 2 3.4 2.4-1a7.5 7.5 0 0 0 2 1.2l.4 2.6h5l.4-2.6a7.5 7.5 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.2Z',
  },
]

export default function App() {
  const { boot } = useStore()
  const [page, setPage] = useState<Page>('today')

  useEffect(() => {
    const un = listen<Page>('ppok:navigate', (e) => setPage(e.payload))
    return () => {
      un.then((f) => f())
    }
  }, [])

  if (!boot.settings.onboarded) return <Onboarding />

  return (
    <div className="flex h-screen">
      <nav className="flex w-[212px] shrink-0 flex-col border-r border-line px-3 py-5">
        <div className="mb-6 flex items-center gap-2.5 px-2.5">
          <Mark />
          <span className="text-[17px] font-bold tracking-tight">뽁</span>
        </div>
        <ul className="space-y-0.5">
          {NAV.map((n) => (
            <li key={n.id}>
              <button
                onClick={() => setPage(n.id)}
                className={cx(
                  'flex h-10 w-full items-center gap-2.5 rounded-[10px] px-2.5 text-sm font-medium transition-colors',
                  page === n.id ? 'bg-surface text-ink shadow-card' : 'text-ink-2 hover:bg-fill hover:text-ink',
                )}
              >
                <svg viewBox="0 0 24 24" className="size-[18px]" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
                  <path d={n.icon} />
                </svg>
                {n.label}
              </button>
            </li>
          ))}
        </ul>
        <p className="mt-auto px-2.5 text-xs leading-relaxed text-ink-3">
          창을 닫아도 뽁은 작업 표시줄 트레이에서 계속 실행돼요.
        </p>
      </nav>
      <main className="scroll-thin min-w-0 flex-1 overflow-y-auto">
        <div key={page} className="mx-auto max-w-[760px] animate-fade-in px-10 py-9">
          {page === 'today' && <TodayPage onNavigate={setPage} />}
          {page === 'builtin' && <BuiltinPage />}
          {page === 'mine' && <MyDecksPage />}
          {page === 'settings' && <SettingsPage />}
        </div>
      </main>
    </div>
  )
}
