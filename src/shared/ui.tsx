import type { ButtonHTMLAttributes, ReactNode } from 'react'
import type { WordStatus } from './types'

export function cx(...xs: (string | false | null | undefined)[]) {
  return xs.filter(Boolean).join(' ')
}

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger'

const VARIANT: Record<Variant, string> = {
  primary: 'bg-accent text-white hover:bg-accent-strong disabled:bg-fill-strong disabled:text-ink-3',
  secondary: 'bg-surface text-ink border border-line hover:bg-fill disabled:text-ink-3',
  ghost: 'text-ink-2 hover:bg-fill hover:text-ink',
  danger: 'bg-surface text-danger border border-line hover:bg-[#fbeceb]',
}

export function Button({
  variant = 'secondary',
  size = 'md',
  className,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant; size?: 'sm' | 'md' | 'lg' }) {
  return (
    <button
      {...props}
      className={cx(
        'inline-flex items-center justify-center gap-1.5 rounded-[10px] font-medium transition-colors duration-150',
        size === 'sm' && 'h-8 px-3 text-[13px]',
        size === 'md' && 'h-10 px-4 text-sm',
        size === 'lg' && 'h-12 px-6 text-[15px]',
        VARIANT[variant],
        className,
      )}
    />
  )
}

/** 큰 선택 카드 (온보딩·설정) */
export function Choice({
  selected,
  disabled,
  title,
  desc,
  badge,
  onClick,
  children,
}: {
  selected: boolean
  disabled?: boolean
  title: ReactNode
  desc?: ReactNode
  badge?: ReactNode
  onClick: () => void
  children?: ReactNode
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      aria-pressed={selected}
      className={cx(
        'group relative flex w-full items-start gap-3 rounded-[14px] border bg-surface px-4 py-3.5 text-left transition-all duration-150',
        selected ? 'border-accent ring-[3px] ring-accent-soft' : 'border-line hover:border-fill-strong',
        disabled && 'opacity-50',
      )}
    >
      <span
        className={cx(
          'mt-0.5 flex size-[18px] shrink-0 items-center justify-center rounded-full border-[1.5px] transition-colors',
          selected ? 'border-accent bg-accent' : 'border-fill-strong',
        )}
      >
        {selected && <span className="size-1.5 rounded-full bg-white" />}
      </span>
      <span className="min-w-0 flex-1">
        <span className="flex items-center gap-2 text-[15px] font-semibold">
          {title}
          {badge}
        </span>
        {desc && <span className="mt-0.5 block text-[13px] text-ink-2">{desc}</span>}
        {children}
      </span>
    </button>
  )
}

/** 작은 세그먼트 버튼 묶음 */
export function Segmented<T extends string | number>({
  value,
  options,
  onChange,
}: {
  value: T | null
  options: { value: T; label: ReactNode }[]
  onChange: (v: T) => void
}) {
  return (
    <div className="inline-flex flex-wrap gap-1 rounded-[12px] bg-fill p-1">
      {options.map((o) => (
        <button
          key={String(o.value)}
          type="button"
          onClick={() => onChange(o.value)}
          aria-pressed={value === o.value}
          className={cx(
            'h-8 rounded-[9px] px-3.5 text-[13px] font-medium transition-all duration-150',
            value === o.value ? 'bg-surface text-ink shadow-card' : 'text-ink-2 hover:text-ink',
          )}
        >
          {o.label}
        </button>
      ))}
    </div>
  )
}

export function Toggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean
  onChange: (v: boolean) => void
  label?: string
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className={cx(
        'relative h-6 w-10 shrink-0 rounded-full transition-colors duration-150',
        checked ? 'bg-accent' : 'bg-fill-strong',
      )}
    >
      <span
        className={cx(
          'absolute top-0.5 left-0.5 size-5 rounded-full bg-white shadow-sm transition-transform duration-150',
          checked && 'translate-x-4',
        )}
      />
    </button>
  )
}

export function Panel({ className, children }: { className?: string; children: ReactNode }) {
  return <div className={cx('rounded-[16px] border border-line bg-surface shadow-card', className)}>{children}</div>
}

const STATUS_STYLE: Record<WordStatus, string> = {
  new: 'bg-fill text-ink-3',
  learning: 'bg-learning-soft text-learning',
  known: 'bg-known-soft text-known',
}
const STATUS_SHORT: Record<WordStatus, string> = { new: '새 단어', learning: '다시 볼', known: '외움' }

export function StatusBadge({ status }: { status: WordStatus }) {
  return (
    <span className={cx('inline-flex h-6 items-center rounded-full px-2.5 text-xs font-medium', STATUS_STYLE[status])}>
      {STATUS_SHORT[status]}
    </span>
  )
}

export function Field({ label, hint, children }: { label: string; hint?: ReactNode; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-6 py-3.5">
      <div className="min-w-0">
        <div className="text-sm font-medium">{label}</div>
        {hint && <div className="mt-0.5 text-[13px] text-ink-3">{hint}</div>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  )
}

export function Mark({ className }: { className?: string }) {
  return <img src="/icon.png" alt="뽁" draggable={false} className={cx('shrink-0 select-none', className ?? 'size-8')} />
}
