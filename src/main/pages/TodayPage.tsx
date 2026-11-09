import { useEffect, useState, type ReactNode } from 'react'
import { api, errorText } from '../../shared/api'
import { intervalLabel, LANGUAGE_LABEL, LEVEL_LABEL, MODE_LABEL, SOURCE_LABEL } from '../../shared/labels'
import type { Today } from '../../shared/types'
import { Button, cx, Panel } from '../../shared/ui'
import type { Page } from '../App'
import { useStore } from '../store'

export default function TodayPage({ onNavigate }: { onNavigate: (p: Page) => void }) {
  const { boot, tick } = useStore()
  const s = boot.settings
  const [today, setToday] = useState<Today | null>(null)
  const [note, setNote] = useState<string | null>(null)

  useEffect(() => {
    let alive = true
    const load = () => api.today().then((t) => alive && setToday(t))
    load()
    const id = setInterval(load, 30_000)
    return () => {
      alive = false
      clearInterval(id)
    }
  }, [tick])

  async function showNow() {
    setNote(null)
    try {
      const ok = await api.showNow()
      if (!ok) setNote('지금 보여줄 단어가 없어요. 단어장 설정을 확인해 주세요.')
    } catch (e) {
      setNote(errorText(e))
    }
  }

  if (!today) return null
  const paused = today.pausedUntil != null
  const rate = Math.min(1, today.goal ? today.known / today.goal : 0)
  const scope = s.source === 'user' ? SOURCE_LABEL.user : `${LANGUAGE_LABEL[s.language]} ${LEVEL_LABEL[s.level] ?? ''}`

  return (
    <>
      <header className="flex items-end justify-between">
        <div>
          <h1 className="text-[26px] font-bold tracking-tight">오늘도 단어 하나, 뽁!</h1>
          <p className="mt-1.5 flex items-center gap-2 text-sm text-ink-2">
            <span className={cx('size-2 rounded-full', today.pool.total === 0 ? 'bg-learning' : paused || !today.withinHours ? 'bg-fill-strong' : 'bg-known')} />
            {today.statusText}
          </p>
        </div>
        <div className="flex gap-2">
          {paused ? (
            <Button onClick={() => api.resume()}>다시 나오게 하기</Button>
          ) : (
            <Button variant="ghost" onClick={() => api.pause('hour')}>
              1시간 쉬기
            </Button>
          )}
          <Button variant="primary" onClick={showNow} disabled={today.showing}>
            지금 단어 보기
          </Button>
        </div>
      </header>
      {note && <p className="mt-3 text-sm text-danger">{note}</p>}

      {today.pool.total === 0 && (
        <div className="mt-6 flex items-center justify-between gap-4 rounded-[14px] bg-learning-soft px-5 py-4">
          <div>
            <p className="text-sm font-semibold text-learning">지금은 보여줄 단어가 없어요</p>
            <p className="mt-0.5 text-[13px] text-ink-2">
              {s.source === 'user' ? '내 단어장이 비어 있거나 모두 꺼져 있어요.' : '고른 언어·난이도에 단어가 없어요.'} 단어장을 켜거나 다른 단어장을 골라 주세요.
            </p>
          </div>
          <div className="flex shrink-0 gap-2">
            {s.source === 'user' && <Button size="sm" onClick={() => onNavigate('mine')}>내 단어장</Button>}
            <Button size="sm" onClick={() => onNavigate('settings')}>설정</Button>
          </div>
        </div>
      )}

      <div className="mt-7 grid grid-cols-3 gap-3">
        <Stat label="오늘 확인한 단어" value={today.seen} />
        <Stat label="외운 단어" value={today.known} tone="known" />
        <Stat label="다시 볼 단어" value={today.again} tone="learning" />
      </div>

      <Panel className="mt-3 px-6 py-5">
        <div className="flex items-baseline justify-between">
          <span className="text-sm font-medium text-ink-2">오늘의 목표</span>
          <span className="text-sm text-ink-2">
            <b className="text-[22px] font-bold text-ink">{Math.round(rate * 100)}%</b>
            <span className="ml-2">
              {today.known} / {today.goal}개
            </span>
          </span>
        </div>
        <div className="mt-3 h-2 overflow-hidden rounded-full bg-fill">
          <div className="h-full rounded-full bg-accent transition-[width] duration-500" style={{ width: `${rate * 100}%` }} />
        </div>
        <p className="mt-2.5 text-[13px] text-ink-3">{rate >= 1 ? '오늘 목표 달성! 남은 시간은 복습만 해도 충분해요.' : "팝업에서 '외웠어'를 누른 단어가 목표에 쌓여요."}</p>
      </Panel>

      <Panel className="mt-3 divide-y divide-line px-6">
        <Row label="학습 범위" value={scope} action={() => onNavigate('settings')} />
        <Row
          label="진행 상황"
          value={
            <>
              전체 {today.pool.total.toLocaleString()}개 중 외운 단어 {today.pool.known.toLocaleString()}개 · 다시 볼 단어 {today.pool.learning.toLocaleString()}개
              {today.pool.due > 0 && <span className="ml-1 text-accent">· 복습 예정 {today.pool.due}개</span>}
            </>
          }
        />
        <Row label="팝업" value={`${intervalLabel(s.intervalMin)}마다 · ${MODE_LABEL[s.mode]} · ${s.popupSeconds}초 표시`} action={() => onNavigate('settings')} />
      </Panel>
    </>
  )
}

function Stat({ label, value, tone }: { label: string; value: number; tone?: 'known' | 'learning' }) {
  return (
    <Panel className="px-5 py-4">
      <div className="text-[13px] text-ink-2">{label}</div>
      <div className={cx('mt-1 text-[30px] leading-none font-bold tracking-tight', tone === 'known' && 'text-known', tone === 'learning' && 'text-learning')}>
        {value}
      </div>
    </Panel>
  )
}

function Row({ label, value, action }: { label: string; value: ReactNode; action?: () => void }) {
  return (
    <div className="flex items-center gap-4 py-3.5 text-sm">
      <span className="w-20 shrink-0 text-ink-3">{label}</span>
      <span className="min-w-0 flex-1 text-ink-2">{value}</span>
      {action && (
        <button onClick={action} className="text-[13px] font-medium text-ink-3 hover:text-ink">
          변경
        </button>
      )}
    </div>
  )
}
