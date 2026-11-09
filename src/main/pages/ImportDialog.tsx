import { open, save } from '@tauri-apps/plugin-dialog'
import { useEffect, useState } from 'react'
import { createPortal } from 'react-dom'
import { api, errorText } from '../../shared/api'
import type { Preview } from '../../shared/types'
import { Button, cx } from '../../shared/ui'

export default function ImportDialog({ onClose, onDone }: { onClose: () => void; onDone: (deckId: number) => void }) {
  const [path, setPath] = useState<string | null>(null)
  const [preview, setPreview] = useState<Preview | null>(null)
  const [name, setName] = useState('')
  const [onlyProblems, setOnlyProblems] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [savedTemplate, setSavedTemplate] = useState(false)

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && !busy && onClose()
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [busy, onClose])

  async function pick() {
    setError(null)
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [{ name: '엑셀 또는 CSV', extensions: ['xlsx', 'xls', 'csv'] }],
    })
    if (typeof picked !== 'string') return
    setBusy(true)
    try {
      const p = await api.previewImport(picked)
      setPath(picked)
      setPreview(p)
      setName(p.suggestedName)
      setOnlyProblems(false)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  async function saveTemplate() {
    setError(null)
    const dest = await save({ defaultPath: '뽁-단어장-양식.xlsx', filters: [{ name: '엑셀', extensions: ['xlsx'] }] })
    if (!dest) return
    try {
      await api.saveTemplate(dest)
      setSavedTemplate(true)
    } catch (e) {
      setError(errorText(e))
    }
  }

  async function commit() {
    if (!path) return
    setBusy(true)
    setError(null)
    try {
      const r = await api.commitImport(path, name)
      onDone(r.deckId)
    } catch (e) {
      setError(errorText(e))
      setBusy(false)
    }
  }

  const rows = preview?.rows.filter((r) => !onlyProblems || r.status !== 'ok') ?? []
  const problems = preview ? preview.duplicateCount + preview.errorCount : 0

  // 페이지 전환 애니메이션(transform) 안에 있으면 fixed가 페이지 영역에 갇히므로 body에 바로 그린다.
  return createPortal(
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-ink/25 p-6 animate-fade-in" onMouseDown={(e) => e.target === e.currentTarget && !busy && onClose()}>
      <div className="flex max-h-full w-full max-w-[620px] flex-col rounded-[18px] bg-surface shadow-pop">
        <div className="flex items-center justify-between border-b border-line px-6 py-4">
          <h2 className="text-[17px] font-bold">내 단어장 올리기</h2>
          <button onClick={onClose} disabled={busy} className="flex size-8 items-center justify-center rounded-lg text-ink-3 hover:bg-fill hover:text-ink" aria-label="닫기">
            <svg viewBox="0 0 24 24" className="size-4" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M6 6l12 12M18 6 6 18" />
            </svg>
          </button>
        </div>

        {!preview ? (
          <div className="px-6 py-6">
            <p className="text-sm text-ink-2">첫 번째 시트의 A열에 단어, B열에 뜻을 적어 주세요. 첫 줄의 제목(단어 / 뜻)은 자동으로 건너뛰어요.</p>
            <table className="mt-4 w-full overflow-hidden rounded-[10px] border border-line text-[13px]">
              <thead className="bg-fill text-ink-2">
                <tr>
                  <th className="w-10 border-r border-line py-1.5 font-medium" />
                  <th className="border-r border-line py-1.5 font-medium">A · 단어</th>
                  <th className="py-1.5 font-medium">B · 뜻</th>
                </tr>
              </thead>
              <tbody>
                {[
                  ['apple', '사과'],
                  ['postpone', '연기하다'],
                  ['efficient', '효율적인'],
                ].map(([w, m], i) => (
                  <tr key={w} className="border-t border-line">
                    <td className="border-r border-line py-1.5 text-center text-ink-3">{i + 2}</td>
                    <td className="border-r border-line px-3 py-1.5">{w}</td>
                    <td className="px-3 py-1.5">{m}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            <p className="mt-3 text-xs text-ink-3">.xlsx · .xls · .csv 지원 · 빈 행은 무시 · 파일은 이 컴퓨터에만 저장돼요</p>
            {error && <p className="mt-4 text-sm text-danger">{error}</p>}
            <div className="mt-6 flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Button onClick={saveTemplate}>양식 파일 받기</Button>
                {savedTemplate && <span className="text-[13px] text-known">저장했어요. 채운 뒤 '파일 선택'으로 올려 주세요</span>}
              </div>
              <Button variant="primary" onClick={pick} disabled={busy}>
                {busy ? '읽는 중…' : '파일 선택'}
              </Button>
            </div>
          </div>
        ) : (
          <>
            <div className="space-y-4 px-6 pt-5">
              <div className="flex flex-wrap items-center gap-2 text-[13px]">
                <span className="mr-1 truncate font-medium text-ink-2">{preview.fileName}</span>
                <Count tone="ok" label="정상" n={preview.okCount} />
                <Count tone="dup" label="중복" n={preview.duplicateCount} />
                <Count tone="err" label="오류" n={preview.errorCount} />
                {preview.skippedBlank > 0 && <span className="text-ink-3">빈 행 {preview.skippedBlank}개 무시</span>}
              </div>
              <label className="block">
                <span className="text-[13px] font-medium text-ink-2">단어장 이름</span>
                <input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  maxLength={40}
                  className="mt-1.5 h-10 w-full rounded-[10px] border border-line bg-surface px-3 text-sm focus:border-accent focus:outline-none"
                />
              </label>
              {problems > 0 && (
                <label className="flex items-center gap-2 text-[13px] text-ink-2">
                  <input type="checkbox" checked={onlyProblems} onChange={(e) => setOnlyProblems(e.target.checked)} className="accent-accent" />
                  문제 있는 행만 보기 · 중복·오류 행은 저장되지 않아요
                </label>
              )}
            </div>
            <div className="scroll-thin mx-6 mt-3 min-h-[120px] flex-1 overflow-y-auto rounded-[12px] border border-line">
              <table className="w-full text-[13px]">
                <thead className="sticky top-0 bg-fill text-left text-ink-2">
                  <tr>
                    <th className="w-14 px-3 py-2 font-medium">행</th>
                    <th className="px-3 py-2 font-medium">단어</th>
                    <th className="px-3 py-2 font-medium">뜻</th>
                    <th className="w-[180px] px-3 py-2 font-medium">상태</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.slice(0, 500).map((r) => (
                    <tr key={r.row} className={cx('border-t border-line', r.status === 'error' && 'bg-[#fdf3f2]', r.status === 'duplicate' && 'bg-learning-soft/60')}>
                      <td className="px-3 py-1.5 text-ink-3">{r.row}</td>
                      <td className="max-w-[160px] truncate px-3 py-1.5 font-medium">{r.term || <span className="text-ink-3">(비어 있음)</span>}</td>
                      <td className="max-w-[180px] truncate px-3 py-1.5 text-ink-2">{r.meaning || <span className="text-ink-3">(비어 있음)</span>}</td>
                      <td className={cx('px-3 py-1.5 text-xs', r.status === 'ok' ? 'text-known' : r.status === 'duplicate' ? 'text-learning' : 'text-danger')}>
                        {r.status === 'ok' ? '정상' : r.message}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {rows.length > 500 && <p className="py-2 text-center text-xs text-ink-3">미리보기는 500행까지만 보여요 · 저장은 전체 반영</p>}
              {rows.length === 0 && <p className="py-8 text-center text-sm text-ink-3">표시할 행이 없어요</p>}
            </div>
            {error && <p className="px-6 pt-3 text-sm text-danger">{error}</p>}
            <div className="flex items-center justify-between px-6 py-4">
              <Button variant="ghost" onClick={pick} disabled={busy}>
                다른 파일
              </Button>
              <Button variant="primary" onClick={commit} disabled={busy || preview.okCount === 0 || !name.trim()}>
                {preview.okCount.toLocaleString()}개 단어 저장
              </Button>
            </div>
          </>
        )}
      </div>
    </div>,
    document.body,
  )
}

function Count({ tone, label, n }: { tone: 'ok' | 'dup' | 'err'; label: string; n: number }) {
  return (
    <span
      className={cx(
        'inline-flex h-6 items-center gap-1 rounded-full px-2.5 font-medium',
        tone === 'ok' && 'bg-known-soft text-known',
        tone === 'dup' && (n ? 'bg-learning-soft text-learning' : 'bg-fill text-ink-3'),
        tone === 'err' && (n ? 'bg-[#fbeceb] text-danger' : 'bg-fill text-ink-3'),
      )}
    >
      {label} {n.toLocaleString()}
    </span>
  )
}
