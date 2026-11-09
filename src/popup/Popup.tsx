import { listen } from '@tauri-apps/api/event'
import { useCallback, useEffect, useRef, useState } from 'react'
import { api } from '../shared/api'
import { langOf } from '../shared/labels'
import type { Answer, PopupPayload } from '../shared/types'
import { cx } from '../shared/ui'

const KIND_LABEL: Record<PopupPayload['kind'], string> = { new: '새 단어', learning: '다시 보는 단어', review: '복습' }
/** 마우스가 떠난 뒤나 뜻을 본 뒤에는 최소 이만큼은 더 보여준다 */
const MIN_AFTER_INTERACTION = 4000
const EXIT_MS = 160

export default function Popup() {
  const [card, setCard] = useState<PopupPayload | null>(null)
  const [revealed, setRevealed] = useState(false)
  const [leaving, setLeaving] = useState(false)
  const [progress, setProgress] = useState(1)
  const remaining = useRef(0)
  const hovering = useRef(false)
  const finished = useRef(false)

  const present = useCallback((p: PopupPayload | null) => {
    if (!p) return
    setCard((prev) => {
      if (prev?.seq === p.seq) return prev
      finished.current = false
      hovering.current = false
      remaining.current = p.seconds * 1000
      setRevealed(p.mode === 'instant')
      setLeaving(false)
      setProgress(1)
      return p
    })
  }, [])

  useEffect(() => {
    api.popupCurrent().then(present)
    const un = listen<PopupPayload>('popup:word', (e) => present(e.payload))
    return () => {
      un.then((f) => f())
    }
  }, [present])

  const finish = useCallback(
    (action: Answer) => {
      if (!card || finished.current) return
      finished.current = true
      setLeaving(true)
      setTimeout(() => {
        api.popupAnswer(card.wordId, action).finally(() => setCard(null))
      }, EXIT_MS)
    },
    [card],
  )

  // 자동 닫힘 타이머. 마우스가 올라가 있으면 멈춘다.
  useEffect(() => {
    if (!card) return
    const total = card.seconds * 1000
    let last = performance.now()
    const id = setInterval(() => {
      const now = performance.now()
      if (!hovering.current) remaining.current -= now - last
      last = now
      setProgress(Math.max(0, remaining.current / total))
      if (remaining.current <= 0) {
        clearInterval(id)
        finish('timeout')
      }
    }, 100)
    return () => clearInterval(id)
  }, [card, finish])

  if (!card) return null

  const reveal = () => {
    if (revealed) return
    setRevealed(true)
    remaining.current = Math.max(remaining.current, MIN_AFTER_INTERACTION + 2000)
  }
  const long = card.term.length > 14 || (!!card.reading && card.term.length + card.reading.length > 14)
  const top = card.corner.startsWith('top')
  const left = card.corner.endsWith('left')

  return (
    <div className={cx('flex h-screen p-4', top ? 'items-start pb-0' : 'items-end pt-0', left ? 'justify-start' : 'justify-end')}>
      <div
        key={card.seq}
        role="dialog"
        aria-label={`단어 카드: ${card.term}`}
        onMouseEnter={() => (hovering.current = true)}
        onMouseLeave={() => {
          hovering.current = false
          remaining.current = Math.max(remaining.current, MIN_AFTER_INTERACTION)
        }}
        onClick={reveal}
        className={cx(
          'relative w-[340px] overflow-hidden rounded-[16px] border border-black/[0.06] bg-surface shadow-pop',
          leaving ? (top ? 'animate-pop-out-up' : 'animate-pop-out') : top ? 'animate-pop-in-down' : 'animate-pop-in',
          !revealed && 'cursor-pointer',
        )}
      >
        <div className="px-5 pt-3.5 pb-4">
          <div className="flex items-center justify-between">
            <span className="flex min-w-0 items-center gap-1.5 truncate text-[11.5px] font-medium text-ink-3">
              <span className="size-1.5 shrink-0 rounded-full bg-accent" />
              {card.deckName} · {KIND_LABEL[card.kind]}
            </span>
            <button
              onClick={(e) => {
                e.stopPropagation()
                finish('dismiss')
              }}
              aria-label="닫기"
              className="-mr-2 flex size-7 shrink-0 items-center justify-center rounded-lg text-ink-3 transition-colors hover:bg-fill hover:text-ink"
            >
              <svg viewBox="0 0 24 24" className="size-3.5" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round">
                <path d="M6 6l12 12M18 6 6 18" />
              </svg>
            </button>
          </div>

          <div className="mt-0.5 flex min-w-0 items-baseline gap-2" lang={langOf(card.term)}>
            <span className={cx('truncate font-bold tracking-tight', long ? 'text-[21px]' : 'text-[26px]')}>{card.term}</span>
            {/* 읽기도 정답의 일부라 맞혀 보기에서는 뜻과 함께 공개한다 */}
            {card.reading && revealed && <span className="shrink-0 animate-fade-in text-[14px] text-ink-3">{card.reading}</span>}
          </div>

          <div className="mt-1 flex h-[22px] items-center">
            {revealed ? (
              <p className="animate-fade-in truncate text-[15px] text-ink-2" title={card.meaning}>
                {card.meaning}
              </p>
            ) : (
              <span className="rounded-md border border-dashed border-fill-strong px-2 py-px text-[13px] text-ink-3">눌러서 뜻 보기</span>
            )}
          </div>

          {/* 맞혀 보기에서는 뜻을 보기 전까지 정답 버튼을 숨긴다. 높이는 유지해 카드가 출렁이지 않게. */}
          <div className={cx('mt-3.5 flex justify-end gap-2', !revealed && 'invisible')} aria-hidden={!revealed}>
            <button
              onClick={(e) => {
                e.stopPropagation()
                finish('again')
              }}
              className="h-8 rounded-[9px] border border-line px-3.5 text-[13px] font-medium text-ink-2 transition-colors hover:bg-fill hover:text-ink"
            >
              다시 볼래
            </button>
            <button
              onClick={(e) => {
                e.stopPropagation()
                finish('known')
              }}
              className="h-8 rounded-[9px] bg-accent px-3.5 text-[13px] font-semibold text-white transition-colors hover:bg-accent-strong"
            >
              외웠어
            </button>
          </div>
        </div>
        <div className="absolute inset-x-0 bottom-0 h-[3px] bg-fill">
          <div className="h-full bg-accent/60" style={{ width: `${progress * 100}%`, transition: 'width 100ms linear' }} />
        </div>
      </div>
    </div>
  )
}
