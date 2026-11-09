import { useEffect, useState } from 'react'
import { api, errorText } from '../../shared/api'
import type { UserDeck } from '../../shared/types'
import { Button, cx, Panel, Toggle } from '../../shared/ui'
import { useStore } from '../store'
import ImportDialog from './ImportDialog'
import WordList from './WordList'

export default function MyDecksPage() {
  const { boot, tick, save } = useStore()
  const [decks, setDecks] = useState<UserDeck[] | null>(null)
  const [importing, setImporting] = useState(false)
  const [openId, setOpenId] = useState<number | null>(null)
  const [confirmId, setConfirmId] = useState<number | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    api.listUserDecks().then(setDecks)
  }, [tick])

  async function run(f: () => Promise<unknown>) {
    setError(null)
    try {
      await f()
      setDecks(await api.listUserDecks())
    } catch (e) {
      setError(errorText(e))
    }
  }

  const unused = boot.settings.source === 'builtin' && (decks?.length ?? 0) > 0

  return (
    <>
      <header className="flex items-end justify-between">
        <div>
          <h1 className="text-[26px] font-bold tracking-tight">내 단어장</h1>
          <p className="mt-1.5 text-sm text-ink-2">엑셀이나 CSV로 나만의 단어를 올려요. 이 컴퓨터에만 저장돼요.</p>
        </div>
        <Button variant="primary" onClick={() => setImporting(true)}>
          파일 올리기
        </Button>
      </header>

      {unused && (
        <div className="mt-5 flex items-center justify-between rounded-[12px] bg-fill px-4 py-3 text-[13px] text-ink-2">
          지금은 기본 단어장만 학습하고 있어요.
          <Button size="sm" onClick={() => save({ source: 'user' })}>
            내 단어장으로 바꾸기
          </Button>
        </div>
      )}
      {error && <p className="mt-4 text-sm text-danger">{error}</p>}

      {decks && decks.length === 0 && (
        <div className="mt-8 rounded-[16px] border border-dashed border-fill-strong px-6 py-12 text-center">
          <p className="text-[15px] font-semibold">아직 올린 단어장이 없어요</p>
          <p className="mt-1.5 text-sm text-ink-3">A열에 단어, B열에 뜻만 있으면 돼요.</p>
          <Button className="mt-5" onClick={() => setImporting(true)}>
            첫 단어장 올리기
          </Button>
        </div>
      )}

      <div className="mt-6 space-y-3">
        {decks?.map((d) => (
          <Panel key={d.id} className="overflow-hidden">
            <div className="flex items-center gap-4 px-5 py-4">
              <button className="min-w-0 flex-1 text-left" onClick={() => setOpenId(openId === d.id ? null : d.id)}>
                <div className="flex items-center gap-2">
                  <span className={cx('truncate text-[15px] font-semibold', !d.enabled && 'text-ink-3')}>{d.name}</span>
                  <svg viewBox="0 0 24 24" className={cx('size-4 text-ink-3 transition-transform', openId === d.id && 'rotate-180')} fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                    <path d="m6 9 6 6 6-6" />
                  </svg>
                </div>
                <div className="mt-0.5 text-[13px] text-ink-3">
                  {d.total.toLocaleString()}개 · 외움 {d.known.toLocaleString()} · 다시 볼 {d.learning.toLocaleString()}
                </div>
              </button>
              {confirmId === d.id ? (
                <div className="flex items-center gap-2">
                  <span className="text-[13px] text-ink-2">학습 기록도 함께 지워져요</span>
                  <Button size="sm" variant="danger" onClick={() => run(() => api.deleteDeck(d.id)).then(() => setConfirmId(null))}>
                    삭제
                  </Button>
                  <Button size="sm" variant="ghost" onClick={() => setConfirmId(null)}>
                    취소
                  </Button>
                </div>
              ) : (
                <>
                  <div className="flex items-center gap-2 text-[13px] text-ink-2">
                    {d.enabled ? '학습에 포함' : '꺼짐'}
                    <Toggle checked={d.enabled} onChange={(v) => run(() => api.setDeckEnabled(d.id, v))} label={`${d.name} 학습에 포함`} />
                  </div>
                  <Button size="sm" variant="ghost" onClick={() => setConfirmId(d.id)}>
                    삭제
                  </Button>
                </>
              )}
            </div>
            {openId === d.id && (
              <div className="border-t border-line bg-paper/60 px-5 py-4">
                <WordList deckId={d.id} />
              </div>
            )}
          </Panel>
        ))}
      </div>

      {importing && (
        <ImportDialog
          onClose={() => setImporting(false)}
          onDone={async (id) => {
            setImporting(false)
            setDecks(await api.listUserDecks())
            setOpenId(id)
          }}
        />
      )}
    </>
  )
}
