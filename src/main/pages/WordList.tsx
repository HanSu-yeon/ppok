import { useEffect, useState } from 'react'
import { api } from '../../shared/api'
import { langOf } from '../../shared/labels'
import type { WordRow } from '../../shared/types'
import { Button, StatusBadge } from '../../shared/ui'
import { useStore } from '../store'

const PAGE = 100

export default function WordList({ deckId, level }: { deckId: number; level?: string | null }) {
  const { tick } = useStore()
  const [query, setQuery] = useState('')
  const [items, setItems] = useState<WordRow[]>([])
  const [total, setTotal] = useState(0)
  const [limit, setLimit] = useState(PAGE)

  useEffect(() => {
    let alive = true
    const t = setTimeout(async () => {
      const page = await api.listWords({ deckId, level, query, offset: 0, limit })
      if (!alive) return
      setItems(page.items)
      setTotal(page.total)
    }, 120)
    return () => {
      alive = false
      clearTimeout(t)
    }
  }, [deckId, level, query, limit, tick])

  return (
    <div>
      <div className="relative">
        <svg viewBox="0 0 24 24" className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-ink-3" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
          <circle cx="11" cy="11" r="6.5" />
          <path d="m16 16 4 4" />
        </svg>
        <input
          value={query}
          onChange={(e) => {
            setQuery(e.target.value)
            setLimit(PAGE)
          }}
          placeholder="단어나 뜻으로 찾기"
          className="h-10 w-full rounded-[10px] border border-line bg-surface pr-3 pl-9 text-sm placeholder:text-ink-3 focus:border-accent focus:outline-none"
        />
      </div>
      <div className="mt-3 overflow-hidden rounded-[14px] border border-line bg-surface">
        {items.length === 0 ? (
          <p className="py-10 text-center text-sm text-ink-3">{query ? '찾는 단어가 없어요' : '단어가 없어요'}</p>
        ) : (
          <ul className="divide-y divide-line">
            {items.map((w) => (
              <li key={w.id} className="flex items-center gap-4 px-4 py-2.5">
                <span className="w-[38%] min-w-0 truncate" lang={langOf(w.term)}>
                  <span className="text-[15px] font-semibold">{w.term}</span>
                  {w.reading && <span className="ml-2 text-[13px] text-ink-3">{w.reading}</span>}
                </span>
                <span className="min-w-0 flex-1 truncate text-sm text-ink-2">{w.meaning}</span>
                <StatusBadge status={w.status} />
              </li>
            ))}
          </ul>
        )}
      </div>
      <div className="mt-3 flex items-center justify-between text-[13px] text-ink-3">
        <span>
          {items.length.toLocaleString()} / {total.toLocaleString()}개
        </span>
        {items.length < total && (
          <Button size="sm" variant="ghost" onClick={() => setLimit((l) => l + PAGE)}>
            더 보기
          </Button>
        )}
      </div>
    </div>
  )
}
