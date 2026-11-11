import type { Language, Mode, Source, WordStatus } from './types'

export const LANGUAGE_LABEL: Record<Language, string> = { en: '영어', ja: '일본어' }

export const LEVEL_LABEL: Record<string, string> = {
  basic: '기초',
  intermediate: '중급',
  advanced: '고급',
  expert: '심화',
  N5: 'N5',
  N4: 'N4',
  N3: 'N3',
  N2: 'N2',
  N1: 'N1',
}

export const LEVEL_DESC: Record<string, string> = {
  basic: '일상 필수 단어',
  intermediate: '회화·업무 빈출 단어',
  advanced: '뉴스·독해·비즈니스',
  expert: '전문적·저빈도 어휘',
  N5: '인사·생활 기본',
  N4: '일상 회화 기본',
  N3: '자연스러운 일상 회화',
  N2: '뉴스·업무',
  N1: '고급·전문',
}

export const SOURCE_LABEL: Record<Source, string> = {
  builtin: '기본 단어장',
  user: '내 단어장',
}

export const MODE_LABEL: Record<Mode, string> = { reveal: '맞혀 보기', instant: '바로 보기' }

export const STATUS_LABEL: Record<WordStatus, string> = {
  new: '처음 보는 단어',
  learning: '다시 볼 단어',
  known: '외운 단어',
}

export const INTERVAL_PRESETS = [1, 3, 5, 10]
export const POPUP_SECONDS_PRESETS = [10, 20, 30, 60]
export const GOAL_PRESETS = [30, 50, 100]

/** 예전에 저장한 값이 프리셋에 없어도 선택된 채로 보이게 끼워 넣는다 */
export function presetsWith(presets: number[], current: number) {
  return presets.includes(current) ? presets : [...presets, current].sort((a, b) => a - b)
}

export function secondsLabel(sec: number) {
  return sec % 60 === 0 ? `${sec / 60}분` : `${sec}초`
}

export function intervalLabel(min: number) {
  if (min % 60 === 0) return `${min / 60}시간`
  if (min > 60) return `${Math.floor(min / 60)}시간 ${min % 60}분`
  return `${min}분`
}

/** 일본어 글자가 있으면 lang="ja"를 붙여 일본어 글꼴과 글자 모양으로 그리게 한다 */
export const langOf = (s: string) => (/[\u3040-\u30ff\u3400-\u9fff]/.test(s) ? 'ja' : undefined)

export function hm(ts: number) {
  const d = new Date(ts * 1000)
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}
