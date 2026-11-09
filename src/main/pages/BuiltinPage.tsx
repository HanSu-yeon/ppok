import { useState } from 'react'
import { LANGUAGE_LABEL, LEVEL_LABEL } from '../../shared/labels'
import type { Language } from '../../shared/types'
import { cx, Segmented } from '../../shared/ui'
import { useStore } from '../store'
import WordList from './WordList'

/** 기본 단어장 목표 분량. 이보다 적으면 샘플 데이터 안내를 보여준다. 일본어는 회화 목표(N5~N3) 기준. */
const TARGET: Record<Language, number> = { en: 10_000, ja: 3_700 }

export default function BuiltinPage() {
  const { boot } = useStore()
  const [langCode, setLangCode] = useState<Language>(boot.settings.language)
  const lang = boot.languages.find((l) => l.code === langCode)
  const [level, setLevel] = useState<string>(boot.settings.level)
  const total = lang?.levels.reduce((n, l) => n + l.total, 0) ?? 0
  const current = lang?.levels.find((l) => l.level === level) ?? lang?.levels[0]

  return (
    <>
      <header className="flex items-end justify-between">
        <div>
          <h1 className="text-[26px] font-bold tracking-tight">기본 단어장</h1>
          <p className="mt-1.5 text-sm text-ink-2">설치하면 바로 쓸 수 있는 검증된 단어 모음이에요.</p>
        </div>
        <Segmented
          value={langCode}
          options={boot.languages.map((l) => ({ value: l.code, label: LANGUAGE_LABEL[l.code] }))}
          onChange={(v) => {
            setLangCode(v)
            setLevel(boot.languages.find((l) => l.code === v)?.levels[0]?.level ?? 'basic')
          }}
        />
      </header>

      {!lang?.available ? (
        <div className="mt-8 rounded-[16px] border border-dashed border-fill-strong px-6 py-12 text-center">
          <p className="text-[15px] font-semibold">{LANGUAGE_LABEL[langCode]} 단어장은 준비 중이에요</p>
          <p className="mt-1.5 text-sm text-ink-3">검증된 단어 데이터가 준비되면 업데이트로 추가돼요.</p>
        </div>
      ) : (
        <>
          {total < TARGET[langCode] && (
            <p className="mt-5 rounded-[12px] bg-accent-soft px-4 py-3 text-[13px] text-accent-strong">
              지금은 샘플 단어 {total.toLocaleString()}개가 들어 있어요. 전체 {TARGET[langCode].toLocaleString()}개 단어장은 검수 후 업데이트로 추가돼요. 학습 기록은 그대로 유지돼요.
            </p>
          )}
          <div className="mt-6 grid grid-cols-4 gap-2.5">
            {lang.levels.map((l) => {
              const pct = l.total ? Math.round((l.known / l.total) * 100) : 0
              return (
                <button
                  key={l.level}
                  onClick={() => setLevel(l.level)}
                  className={cx(
                    'rounded-[14px] border bg-surface px-4 py-3 text-left transition-all',
                    current?.level === l.level ? 'border-accent ring-[3px] ring-accent-soft' : 'border-line hover:border-fill-strong',
                  )}
                >
                  <div className="flex items-center justify-between">
                    <span className="text-[15px] font-semibold">{LEVEL_LABEL[l.level]}</span>
                    {boot.settings.source !== 'user' && boot.settings.language === langCode && boot.settings.level === l.level && (
                      <span className="rounded-full bg-accent px-1.5 py-px text-[10px] font-semibold text-white">학습 중</span>
                    )}
                  </div>
                  <div className="mt-0.5 text-xs text-ink-3">{l.total.toLocaleString()}개 · 외움 {pct}%</div>
                  <div className="mt-2 h-1 overflow-hidden rounded-full bg-fill">
                    <div className="h-full bg-known" style={{ width: `${pct}%` }} />
                  </div>
                </button>
              )
            })}
          </div>
          <div className="mt-5">{lang.deckId != null && current && <WordList key={current.level} deckId={lang.deckId} level={current.level} />}</div>
        </>
      )}
    </>
  )
}
