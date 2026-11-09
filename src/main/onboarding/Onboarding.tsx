import { useState } from 'react'
import { api, errorText } from '../../shared/api'
import type { Settings } from '../../shared/types'
import { Button, cx, Field, Mark, Toggle } from '../../shared/ui'
import { GoalPicker, HoursPicker, IntervalPicker, LanguagePicker, levelHint, LevelPicker, ModePicker, SourcePicker } from '../controls'
import ImportDialog from '../pages/ImportDialog'
import { useStore } from '../store'

type StepId = 'language' | 'source' | 'level' | 'mode' | 'popup'

const COPY: Record<StepId, { title: string; desc: string }> = {
  language: { title: '어떤 언어를 외울까요?', desc: '나중에 설정에서 언제든 바꿀 수 있어요.' },
  source: { title: '어떤 단어장으로 시작할까요?', desc: '기본 단어장은 설치하자마자 바로 쓸 수 있어요.' },
  level: { title: '난이도를 골라 주세요', desc: '' },
  mode: { title: '어떻게 외울까요?', desc: '팝업 카드가 보이는 방식이에요.' },
  popup: { title: '언제 뽁! 할까요?', desc: '일하는 흐름을 깨지 않는 간격을 골라 주세요.' },
}

export default function Onboarding() {
  const { boot, save, reload } = useStore()
  const [draft, setDraft] = useState<Settings>(boot.settings)
  const [index, setIndex] = useState(0)
  const [importing, setImporting] = useState(false)
  // 자동 실행이 꺼져 있으면 다음 날부터 카드가 조용히 끊기므로 기본으로 켠다.
  const [autostart, setAutostart] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const patch = (p: Partial<Settings>) => setDraft((d) => ({ ...d, ...p }))
  const steps: StepId[] = ['language', 'source', ...(draft.source === 'user' ? [] : (['level'] as StepId[])), 'mode', 'popup']
  const step = steps[Math.min(index, steps.length - 1)]
  const language = boot.languages.find((l) => l.code === draft.language)
  const isLast = index >= steps.length - 1
  const needsUpload = draft.source === 'user' && boot.userWordCount === 0

  const canNext =
    step === 'language' ? !!language?.available : step === 'source' ? !needsUpload : step === 'level' ? !!language?.levels.find((l) => l.level === draft.level && l.total > 0) : true

  async function finish() {
    setSaving(true)
    setError(null)
    try {
      // 저장하면 온보딩 화면이 바로 사라지므로 자동 실행을 먼저 맞춘다. 실패해도 시작은 막지 않는다.
      await api.setAutostart(autostart).catch(() => {})
      await save({ ...draft, onboarded: true })
    } catch (e) {
      setError(errorText(e))
      setSaving(false)
    }
  }

  return (
    <div className="flex min-h-screen flex-col items-center justify-center px-6 py-10">
      <div className="w-full max-w-[540px]">
        <div className="mb-8 flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <Mark />
            <span className="text-[15px] font-semibold text-ink-2">오늘도 단어 하나, 뽁!</span>
          </div>
          <div className="flex gap-1.5" aria-label={`${steps.length}단계 중 ${index + 1}단계`}>
            {steps.map((s, i) => (
              <span key={s} className={cx('h-1.5 rounded-full transition-all duration-200', i === index ? 'w-5 bg-accent' : i < index ? 'w-1.5 bg-accent/50' : 'w-1.5 bg-fill-strong')} />
            ))}
          </div>
        </div>

        <div key={step} className="animate-fade-in">
          <p className="text-[13px] font-semibold text-accent">STEP {index + 1}</p>
          <h1 className="mt-1 text-[26px] font-bold tracking-tight">{COPY[step].title}</h1>
          <p className="mt-1.5 mb-7 text-[15px] text-ink-2">{step === 'level' ? levelHint(draft.language) : COPY[step].desc}</p>

          {step === 'language' && <LanguagePicker value={draft.language} languages={boot.languages} onChange={patch} />}
          {step === 'source' && (
            <>
              <SourcePicker value={draft.source} onChange={patch} />
              {draft.source !== 'builtin' && (
                <div className="mt-4 flex items-center justify-between rounded-[12px] bg-fill px-4 py-3 text-[13px] text-ink-2">
                  <span>
                    {boot.userWordCount > 0
                      ? `내 단어장에 ${boot.userWordCount.toLocaleString()}개 단어가 있어요.`
                      : '엑셀(.xlsx)이나 CSV 파일에 단어, 뜻 두 열만 있으면 돼요.'}
                  </span>
                  <Button size="sm" onClick={() => setImporting(true)}>
                    파일 올리기
                  </Button>
                </div>
              )}
            </>
          )}
          {step === 'level' && <LevelPicker value={draft.level} language={language} onChange={patch} />}
          {step === 'mode' && <ModePicker value={draft.mode} onChange={patch} />}
          {step === 'popup' && (
            <div className="divide-y divide-line rounded-[16px] border border-line bg-surface px-5">
              <Field label="등장 간격">
                <IntervalPicker value={draft.intervalMin} onChange={patch} />
              </Field>
              <Field label="하루 목표" hint="'외웠어'를 누른 단어 기준">
                <GoalPicker value={draft.dailyGoal} onChange={patch} />
              </Field>
              <Field label="알림 시간대" hint="이 시간에만 단어가 나와요">
                <HoursPicker settings={draft} onChange={patch} />
              </Field>
              <Field label="컴퓨터를 켜면 자동으로 시작" hint="끄면 컴퓨터를 다시 켰을 때 뽁을 직접 열어야 해요">
                <Toggle checked={autostart} onChange={setAutostart} label="컴퓨터를 켜면 자동으로 시작" />
              </Field>
            </div>
          )}
        </div>

        {error && <p className="mt-4 text-sm text-danger">{error}</p>}

        <div className="mt-9 flex items-center justify-between">
          <Button variant="ghost" onClick={() => setIndex((i) => i - 1)} className={cx(index === 0 && 'invisible')}>
            이전
          </Button>
          {isLast ? (
            <Button variant="primary" size="lg" disabled={saving} onClick={finish}>
              시작하기
            </Button>
          ) : (
            <Button variant="primary" size="lg" disabled={!canNext} onClick={() => setIndex((i) => i + 1)}>
              다음
            </Button>
          )}
        </div>
        {step === 'source' && needsUpload && <p className="mt-3 text-right text-[13px] text-ink-3">내 단어장만 쓰려면 먼저 파일을 올려 주세요.</p>}
      </div>

      {importing && (
        <ImportDialog
          onClose={() => setImporting(false)}
          onDone={async () => {
            setImporting(false)
            await reload()
          }}
        />
      )}
    </div>
  )
}
