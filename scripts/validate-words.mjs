#!/usr/bin/env node
// 기본 단어장 CSV 검증 — 배포 전에 실행한다.
//   npm run validate:words            (src-tauri/resources/words 전체)
//   npm run validate:words -- path/to/new-10000.csv --lang en
//
// 앱(Rust, src-tauri/src/validate.rs)과 같은 규칙을 쓴다. 오류가 하나라도 있으면 종료 코드 1.

import { readdirSync, readFileSync, statSync } from 'node:fs'
import { basename, join, resolve } from 'node:path'

const LEVELS = { en: ['basic', 'intermediate', 'advanced', 'expert'], ja: ['N5', 'N4', 'N3', 'N2', 'N1'] }
// 일본어는 회화 목표(N5~N3) 기준. JLPT 공식 목록이 없어 흔히 쓰는 누적 추정치(N5 800 → N4 1,500 → N3 3,700)를 나눈 값
const TARGET = {
  en: { basic: 2000, intermediate: 3000, advanced: 3000, expert: 2000 },
  ja: { N5: 800, N4: 700, N3: 2200 },
}
const HANGUL = /[가-힣ᄀ-ᇿ㄰-㆏]/
const EN_TERM = /^[A-Za-z][A-Za-z '.-]*$/
const KANA = /^[\u3040-\u30ff ]*$/
const JA = /[\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\u3005]/
const KANJI = /[\u3400-\u4dbf\u4e00-\u9fff\u3005]/

const args = process.argv.slice(2)
const langFlag = args.indexOf('--lang')
const forcedLang = langFlag >= 0 ? args.splice(langFlag, 2)[1] : null
const root = resolve(import.meta.dirname, '../src-tauri/resources/words')

/** 따옴표·쉼표를 처리하는 최소 CSV 파서 */
function parseCsv(text) {
  const rows = []
  let row = [], cell = '', q = false
  for (let i = 0; i < text.length; i++) {
    const c = text[i]
    if (q) {
      if (c === '"' && text[i + 1] === '"') (cell += '"'), i++
      else if (c === '"') q = false
      else cell += c
    } else if (c === '"') q = true
    else if (c === ',') row.push(cell), (cell = '')
    else if (c === '\n' || c === '\r') {
      if (c === '\r' && text[i + 1] === '\n') i++
      row.push(cell), rows.push(row), (row = []), (cell = '')
    } else cell += c
  }
  if (cell || row.length) row.push(cell), rows.push(row)
  return rows
}

const clean = (s = '') => s.replace(/[\u0000-\u001f\u007f﻿]/g, ' ').split(/\s+/).filter(Boolean).join(' ')
const key = (s) => clean(s).toLowerCase()

function check(lang, term, reading, meaning, level) {
  if (!term) return '단어가 비어 있음'
  if (!meaning) return '뜻이 비어 있음'
  if ([...term].length > 80) return '단어가 80자 초과'
  if ([...meaning].length > 200) return '뜻이 200자 초과'
  if (!LEVELS[lang].includes(level)) return `알 수 없는 난이도 '${level}'`
  if (lang === 'en' && !EN_TERM.test(term)) return '영어 단어 형식 아님'
  if (lang === 'ja') {
    if (!JA.test(term) || HANGUL.test(term)) return '일본어 단어 형식 아님'
    if (!KANA.test(reading)) return '읽기는 히라가나·가타카나만'
    if (!reading && KANJI.test(term)) return '한자 단어인데 읽기 없음'
  }
  if (!HANGUL.test(meaning)) return '뜻에 한국어 없음'
  return null
}

function targets() {
  if (args.length) return [{ lang: forcedLang ?? 'en', files: args.map((f) => resolve(f)) }]
  return Object.keys(LEVELS).map((lang) => {
    const dir = join(root, lang)
    let files = []
    try {
      files = readdirSync(dir).filter((f) => f.toLowerCase().endsWith('.csv')).sort().map((f) => join(dir, f))
    } catch {}
    return { lang, files }
  })
}

let failed = false
for (const { lang, files } of targets()) {
  if (!files.length) {
    console.log(`[${lang}] CSV 없음 — 건너뜀`)
    continue
  }
  const seen = new Map()
  const counts = Object.fromEntries(LEVELS[lang].map((l) => [l, 0]))
  const problems = []
  for (const file of files) {
    if (!statSync(file).isFile()) continue
    const [header, ...rows] = parseCsv(readFileSync(file, 'utf8'))
    const h = header.map((x) => clean(x).toLowerCase())
    const [cw, cm, cl, cr] = ['word', 'meaning', 'level', 'reading'].map((n) => h.indexOf(n))
    if (cw < 0 || cm < 0 || cl < 0) {
      problems.push(`${basename(file)}:1 헤더는 word,meaning,level 이어야 함`)
      continue
    }
    rows.forEach((r, i) => {
      const line = i + 2
      const [term, meaning, level] = [r[cw], r[cm], r[cl]].map(clean)
      const reading = cr >= 0 ? clean(r[cr]) : ''
      if (!term && !meaning) return
      const err = check(lang, term, reading, meaning, level)
      if (err) return problems.push(`${basename(file)}:${line} '${term}' — ${err}`)
      // 앱(builtin_key)과 같게: 읽기가 있으면 함께 본다 (今日 きょう / こんにち)
      const k = reading ? `${key(term)}|${reading}` : key(term)
      if (seen.has(k)) return problems.push(`${basename(file)}:${line} '${term}' — 중복 (처음: ${seen.get(k)})`)
      seen.set(k, `${basename(file)}:${line}`)
      counts[level]++
    })
  }

  console.log(`\n[${lang}] 파일 ${files.length}개 · 유효 단어 ${seen.size.toLocaleString()}개`)
  for (const [level, n] of Object.entries(counts)) {
    const goal = TARGET[lang]?.[level]
    console.log(`  ${level.padEnd(13)} ${String(n).padStart(6)}${goal ? `  / 목표 ${goal.toLocaleString()}` : ''}`)
  }
  if (problems.length) {
    failed = true
    console.log(`  문제 ${problems.length}개:`)
    problems.slice(0, 200).forEach((p) => console.log(`   - ${p}`))
    if (problems.length > 200) console.log(`   … 외 ${problems.length - 200}개`)
  } else {
    console.log('  문제 없음 ✓')
  }
}
process.exit(failed ? 1 : 0)
