// 온보딩과 설정 화면이 함께 쓰는 입력 묶음

import { useState } from 'react'
import {
  GOAL_PRESETS,
  INTERVAL_PRESETS,
  intervalLabel,
  LANGUAGE_LABEL,
  langOf,
  LEVEL_DESC,
  LEVEL_LABEL,
  POPUP_SECONDS_PRESETS,
  presetsWith,
  secondsLabel,
} from '../shared/labels'
import type { LanguageInfo, Mode, Settings, Source } from '../shared/types'
import { Choice, cx, Segmented, Toggle } from '../shared/ui'

type Patch = (p: Partial<Settings>) => void

export function LanguagePicker({ value, languages, onChange }: { value: string; languages: LanguageInfo[]; onChange: Patch }) {
  return (
    <div className="space-y-2.5">
      {languages.map((l) => (
        <Choice
          key={l.code}
          selected={value === l.code}
          disabled={!l.available}
          title={LANGUAGE_LABEL[l.code]}
          desc={l.code === 'en' ? '기초부터 심화까지 4단계' : 'JLPT N5 ~ N3 · 일상 회화 중심'}
          badge={!l.available && <span className="rounded-full bg-fill px-2 py-0.5 text-[11px] font-medium text-ink-3">준비 중</span>}
          onClick={() => onChange({ language: l.code, level: l.levels[0]?.level ?? 'basic' })}
        />
      ))}
    </div>
  )
}

const SOURCES: { value: Source; title: string; desc: string }[] = [
  { value: 'builtin', title: '기본 단어장', desc: '설치하자마자 바로 시작해요' },
  { value: 'user', title: '내 단어장', desc: '엑셀·CSV로 올린 단어만 학습해요' },
]

export function SourcePicker({ value, onChange }: { value: Source; onChange: Patch }) {
  return (
    <div className="space-y-2.5">
      {SOURCES.map((s) => (
        <Choice key={s.value} selected={value === s.value} title={s.title} desc={s.desc} onClick={() => onChange({ source: s.value })} />
      ))}
    </div>
  )
}

/** 자기 수준을 모를 때 어디서 시작하면 좋은지 */
export function levelHint(language: string) {
  return language === 'ja'
    ? '잘 모르겠으면 N4부터 시작해 보세요. 너무 쉽거나 어려우면 설정에서 언제든 바꿀 수 있어요.'
    : '잘 모르겠으면 중급부터 시작해 보세요. 너무 쉽거나 어려우면 설정에서 언제든 바꿀 수 있어요.'
}

export function LevelPicker({ value, language, onChange }: { value: string; language?: LanguageInfo; onChange: Patch }) {
  if (!language) return null
  return (
    <div className="grid grid-cols-2 gap-2.5">
      {language.levels.map((l) => (
        <Choice
          key={l.level}
          selected={value === l.level}
          disabled={l.total === 0}
          title={LEVEL_LABEL[l.level] ?? l.level}
          desc={
            <>
              {LEVEL_DESC[l.level]}
              {l.examples.length > 0 && (
                <span className="mt-1 block truncate text-xs text-ink-2" lang={langOf(l.examples[0])}>
                  예: {l.examples.join(' · ')}
                </span>
              )}
              <span className="mt-0.5 block text-xs text-ink-3">{l.total > 0 ? `${l.total.toLocaleString()}개 단어` : '준비 중'}</span>
            </>
          }
          onClick={() => onChange({ level: l.level })}
        />
      ))}
    </div>
  )
}

function MiniCard({ revealed }: { revealed: boolean }) {
  return (
    <div className="mt-3 w-full max-w-[220px] rounded-[12px] border border-line bg-paper px-3.5 py-3">
      <div className="text-[17px] font-semibold tracking-tight">postpone</div>
      {revealed ? (
        <div className="mt-0.5 text-[13px] text-ink-2">연기하다, 미루다</div>
      ) : (
        <div className="mt-1 inline-block rounded-md border border-dashed border-fill-strong px-2 text-[12px] text-ink-3">
          눌러서 뜻 보기
        </div>
      )}
    </div>
  )
}

export function ModePicker({ value, onChange }: { value: Mode; onChange: Patch }) {
  return (
    <div className="grid grid-cols-2 gap-2.5">
      <Choice selected={value === 'reveal'} title="맞혀 보기" desc="단어를 누르면 뜻을 보여줘요" onClick={() => onChange({ mode: 'reveal' })}>
        <MiniCard revealed={false} />
      </Choice>
      <Choice selected={value === 'instant'} title="바로 보기" desc="단어와 뜻을 함께 보여줘요" onClick={() => onChange({ mode: 'instant' })}>
        <MiniCard revealed />
      </Choice>
    </div>
  )
}

export function IntervalPicker({ value, onChange }: { value: number; onChange: Patch }) {
  const isPreset = INTERVAL_PRESETS.includes(value)
  const [custom, setCustom] = useState(!isPreset)
  const [draft, setDraft] = useState(String(value))

  return (
    <div className="flex flex-wrap items-center gap-2">
      <Segmented<number | 'custom'>
        value={custom ? 'custom' : value}
        options={[...INTERVAL_PRESETS.map((m) => ({ value: m as number | 'custom', label: intervalLabel(m) })), { value: 'custom', label: '직접 입력' }]}
        onChange={(v) => {
          if (v === 'custom') {
            setCustom(true)
            setDraft(String(value))
          } else {
            setCustom(false)
            onChange({ intervalMin: v })
          }
        }}
      />
      {custom && (
        <label className="flex items-center gap-1.5 text-[13px] text-ink-2">
          <input
            type="number"
            min={1}
            max={240}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={() => {
              const n = Math.min(240, Math.max(1, Math.round(Number(draft) || 30)))
              setDraft(String(n))
              onChange({ intervalMin: n })
            }}
            className="h-8 w-16 rounded-[8px] border border-line bg-surface px-2 text-center text-ink"
          />
          분마다
        </label>
      )}
    </div>
  )
}

export function GoalPicker({ value, onChange }: { value: number; onChange: Patch }) {
  return (
    <Segmented
      value={value}
      options={presetsWith(GOAL_PRESETS, value).map((g) => ({ value: g, label: `${g}개` }))}
      onChange={(v) => onChange({ dailyGoal: v })}
    />
  )
}

export function PopupSecondsPicker({ value, onChange }: { value: number; onChange: Patch }) {
  return (
    <Segmented
      value={value}
      options={presetsWith(POPUP_SECONDS_PRESETS, value).map((s) => ({ value: s, label: secondsLabel(s) }))}
      onChange={(v) => onChange({ popupSeconds: v })}
    />
  )
}

export function HoursPicker({ settings, onChange }: { settings: Settings; onChange: Patch }) {
  return (
    <div className="flex flex-col items-end gap-2">
      <div className="flex items-center gap-2 text-[13px] text-ink-2">
        {settings.hoursEnabled ? '정한 시간에만' : '언제든지'}
        <Toggle checked={settings.hoursEnabled} onChange={(v) => onChange({ hoursEnabled: v })} label="알림 시간대 사용" />
      </div>
      <div className={cx('flex items-center gap-1.5 transition-opacity', !settings.hoursEnabled && 'pointer-events-none opacity-40')}>
        <TimeInput value={settings.hoursStart} onChange={(v) => onChange({ hoursStart: v })} />
        <span className="text-ink-3">~</span>
        <TimeInput value={settings.hoursEnd} onChange={(v) => onChange({ hoursEnd: v })} />
      </div>
    </div>
  )
}

function TimeInput({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  return (
    <input
      type="time"
      value={value}
      onChange={(e) => e.target.value && onChange(e.target.value)}
      className="h-8 rounded-[8px] border border-line bg-surface px-2 text-[13px] text-ink"
    />
  )
}

const CORNERS: { value: Settings['popupCorner']; label: string }[] = [
  { value: 'bottom-right', label: '오른쪽 아래' },
  { value: 'bottom-left', label: '왼쪽 아래' },
  { value: 'top-right', label: '오른쪽 위' },
  { value: 'top-left', label: '왼쪽 위' },
  { value: 'random', label: '랜덤 모서리' },
]

/** 화면 모양 미리보기가 붙은 모서리 선택 */
export function CornerPicker({ value, onChange }: { value: Settings['popupCorner']; onChange: (v: Settings['popupCorner']) => void }) {
  return (
    <div className="grid grid-cols-5 gap-2">
      {CORNERS.map((c) => {
        const selected = value === c.value
        const spots = c.value === 'random' ? ['bottom-right', 'bottom-left', 'top-right', 'top-left'] : [c.value]
        return (
          <button
            key={c.value}
            type="button"
            onClick={() => onChange(c.value)}
            aria-pressed={selected}
            className={cx(
              'rounded-[12px] border bg-surface p-2 text-center transition-all duration-150',
              selected ? 'border-accent ring-[3px] ring-accent-soft' : 'border-line hover:border-fill-strong',
            )}
          >
            <span className="relative block h-12 rounded-[6px] border border-line bg-paper">
              {spots.map((spot) => (
                <span
                  key={spot}
                  className={cx(
                    'absolute h-2.5 w-5 rounded-[3px]',
                    c.value === 'random' ? 'bg-accent/45' : 'bg-accent',
                    spot.startsWith('top') ? 'top-1.5' : 'bottom-1.5',
                    spot.endsWith('left') ? 'left-1.5' : 'right-1.5',
                  )}
                />
              ))}
              <span className="absolute inset-x-0 bottom-0 h-1 rounded-b-[5px] bg-fill-strong" />
            </span>
            <span className={cx('mt-1.5 block text-xs font-medium', selected ? 'text-ink' : 'text-ink-2')}>{c.label}</span>
          </button>
        )
      })}
    </div>
  )
}
