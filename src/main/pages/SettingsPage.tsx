import { useEffect, useState, type ReactNode } from 'react'
import { api, errorText } from '../../shared/api'
import { hm, LANGUAGE_LABEL } from '../../shared/labels'
import type { SeedReport, Settings } from '../../shared/types'
import { Button, Field, Panel, Segmented, Toggle } from '../../shared/ui'
import { CornerPicker, GoalPicker, HoursPicker, IntervalPicker, LanguagePicker, levelHint, LevelPicker, ModePicker, SourcePicker } from '../controls'
import { useStore } from '../store'

/** Rust의 i64::MAX(다시 켤 때까지)는 JS에서 정밀도가 깨지므로 충분히 먼 미래인지로 판단한다. */
const isForever = (ts: number) => ts > 4e12

export default function SettingsPage() {
  const { boot, save, tick } = useStore()
  const s = boot.settings
  const [error, setError] = useState<string | null>(null)
  const [autostart, setAutostart] = useState(s.autostart)
  const [reports, setReports] = useState<SeedReport[]>([])
  const [dataDir, setDataDir] = useState('')

  useEffect(() => {
    api.getAutostart().then(setAutostart)
    api.seedReports().then(setReports)
    api.dataDir().then(setDataDir)
  }, [tick])

  const patch = (p: Partial<Settings>) => {
    setError(null)
    save(p).catch((e) => setError(errorText(e)))
  }
  const language = boot.languages.find((l) => l.code === s.language)
  const paused = s.pausedUntil != null && s.pausedUntil * 1000 > Date.now()

  return (
    <>
      <h1 className="text-[26px] font-bold tracking-tight">설정</h1>
      <p className="mt-1.5 text-sm text-ink-2">바꾼 내용은 바로 저장돼요.</p>
      {error && <p className="mt-3 text-sm text-danger">{error}</p>}

      <Section title="잠시 쉬기">
        <Field
          label={paused ? '쉬는 중이에요' : '단어가 나오고 있어요'}
          hint={paused ? (isForever(s.pausedUntil!) ? "'다시 나오게 하기'를 누를 때까지 단어가 안 나와요" : `${hm(s.pausedUntil!)}부터 다시 나와요`) : '회의나 집중할 때 잠깐 단어 카드를 멈출 수 있어요'}
        >
          {paused ? (
            <Button variant="primary" onClick={() => api.resume()}>다시 나오게 하기</Button>
          ) : (
            <div className="flex gap-1.5">
              <Button size="sm" onClick={() => api.pause('hour')}>
                1시간 쉬기
              </Button>
              <Button size="sm" onClick={() => api.pause('today')}>
                오늘은 그만
              </Button>
              <Button size="sm" onClick={() => api.pause('forever')}>
                계속 쉬기
              </Button>
            </div>
          )}
        </Field>
      </Section>

      <Section title="학습">
        <Block label="학습 언어">
          <LanguagePicker value={s.language} languages={boot.languages} onChange={patch} />
        </Block>
        <Block label="단어장">
          <SourcePicker value={s.source} onChange={patch} />
          {s.source !== 'builtin' && boot.userWordCount === 0 && <p className="mt-2 text-[13px] text-learning">아직 올린 단어장이 없어요. '내 단어장'에서 파일을 올려 주세요.</p>}
        </Block>
        {s.source !== 'user' && (
          <Block label="난이도" hint={levelHint(s.language)}>
            <LevelPicker value={s.level} language={language} onChange={patch} />
          </Block>
        )}
        <Block label="암기 방식">
          <ModePicker value={s.mode} onChange={patch} />
        </Block>
      </Section>

      <Section title="팝업">
        <Field label="등장 간격">
          <IntervalPicker value={s.intervalMin} onChange={patch} />
        </Field>
        <Field label="표시 시간" hint="마우스를 올려 두면 사라지지 않아요">
          <Segmented value={s.popupSeconds} options={[5, 10, 15, 20].map((n) => ({ value: n, label: `${n}초` }))} onChange={(v) => patch({ popupSeconds: v })} />
        </Field>
        <Block label="등장 위치" hint="작업 표시줄을 피해서 화면 모서리에 나타나요">
          <CornerPicker value={s.popupCorner} onChange={(v) => patch({ popupCorner: v })} />
        </Block>
        <Field label="하루 목표" hint="'외웠어'를 누른 단어 기준">
          <GoalPicker value={s.dailyGoal} onChange={patch} />
        </Field>
        <Field label="알림 시간대">
          <HoursPicker settings={s} onChange={patch} />
        </Field>
      </Section>

      <Section title="시스템">
        <Field label="컴퓨터를 켜면 자동으로 시작" hint="켜 두면 로그인할 때 창 없이 조용히 시작해요">
          <Toggle
            checked={autostart}
            label="컴퓨터를 켜면 자동으로 시작"
            onChange={(v) =>
              api
                .setAutostart(v)
                .then(setAutostart)
                .catch((e) => setError(errorText(e)))
            }
          />
        </Field>
      </Section>

      <Section title="데이터">
        {reports.map((r) => (
          <Field
            key={r.language}
            label={`기본 ${LANGUAGE_LABEL[r.language]} 단어 ${r.accepted.toLocaleString()}개`}
            hint={
              <>
                {r.files.join(', ')} · {new Date(r.syncedAt * 1000).toLocaleDateString('ko-KR')} 반영
                {r.skipped.length > 0 && ` · 검증에서 제외된 행 ${r.skipped.length}개`}
              </>
            }
          >
            <span />
          </Field>
        ))}
        <Field label="처음부터 다시 설정하기" hint="온보딩 화면을 다시 띄워요. 학습 기록과 내 단어장은 그대로예요">
          <Button size="sm" onClick={() => patch({ onboarded: false })}>
            다시 설정
          </Button>
        </Field>
        <Field label="저장 위치" hint="단어장과 학습 기록은 이 컴퓨터에만 저장돼요">
          <span className="block max-w-[300px] truncate text-right text-xs text-ink-3" title={dataDir}>
            {dataDir}
          </span>
        </Field>
      </Section>
      <div className="h-6" />
    </>
  )
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="mt-8">
      <h2 className="mb-2.5 text-[13px] font-semibold text-ink-3">{title}</h2>
      <Panel className="divide-y divide-line px-5">{children}</Panel>
    </section>
  )
}

function Block({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="py-4">
      <div className="mb-3">
        <div className="text-sm font-medium">{label}</div>
        {hint && <div className="mt-0.5 text-[13px] text-ink-3">{hint}</div>}
      </div>
      {children}
    </div>
  )
}
