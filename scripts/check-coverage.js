// Run the whole suite and enforce coverage.
//
// The suite runs as three separate bun processes (see CLAUDE.md), so no single
// `bun test` invocation knows the real total — this merges their lcov reports
// and checks the result.
//
// It enforces two things, and the second is the one that actually stops an
// untested contribution:
//
//   1. a project-wide line-coverage floor, and
//   2. that every runtime source file appears in the report at all.
//
// Coverage tools only measure files the tests *load*. A brand-new module with
// no tests is therefore invisible: it can't drag the percentage down, because
// it isn't in the denominator. Checking the file list closes that hole.
const { execFileSync } = require('node:child_process')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')

const ROOT = path.join(__dirname, '..')
const GROUPS = ['test/', 'test-main/', 'test-dom/']
const TOTAL_MIN = 80
const PER_FILE_MIN = 60
// The source list is *discovered*, never hand-maintained: it comes from the
// electron-builder `files` in package.json — the code that actually ships in
// the app. A hardcoded list would silently skip whatever a contributor forgot
// to add to it, which is exactly the case this check exists to catch. Ship a
// new .js and it is covered by this gate from the first commit.
function discoverSources() {
  const pkg = JSON.parse(fs.readFileSync(path.join(ROOT, 'package.json'), 'utf8'))
  const out = []
  for (const entry of pkg.build.files) {
    if (entry.endsWith('.js')) {
      out.push(entry)
    } else if (entry.endsWith('/**')) {
      const dir = entry.slice(0, -3)
      for (const f of fs.readdirSync(path.join(ROOT, dir))) {
        if (f.endsWith('.js')) out.push(`${dir}/${f}`)
      }
    }
  }
  return out.sort()
}
const SOURCES = discoverSources()

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'clauddy-cov-'))
try {
  GROUPS.forEach((group, i) => {
    execFileSync(
      'bun',
      [
        'test',
        group,
        '--coverage',
        '--coverage-reporter=lcov',
        `--coverage-dir=${path.join(tmp, String(i))}`,
      ],
      { cwd: ROOT, stdio: 'inherit' },
    )
  })

  // merge: a line counts as covered if any group covered it
  const merged = new Map()
  for (let i = 0; i < GROUPS.length; i++) {
    const lcov = path.join(tmp, String(i), 'lcov.info')
    if (!fs.existsSync(lcov)) continue
    let file = null
    for (const raw of fs.readFileSync(lcov, 'utf8').split('\n')) {
      const line = raw.trim()
      if (line.startsWith('SF:')) {
        file = path.relative(ROOT, path.resolve(ROOT, line.slice(3)))
        if (!merged.has(file)) merged.set(file, new Map())
      } else if (line.startsWith('DA:') && file) {
        const [n, hits] = line.slice(3).split(',').map(Number)
        const seen = merged.get(file)
        seen.set(n, Math.max(seen.get(n) || 0, hits))
      }
    }
  }

  const problems = []
  let totalLines = 0
  let totalHit = 0
  const rows = []

  for (const file of SOURCES) {
    const lines = merged.get(file)
    if (!lines || lines.size === 0) {
      problems.push(`${file} is not covered by any test — it never gets loaded`)
      rows.push([file, 0, 0, null])
      continue
    }
    const lf = lines.size
    const lh = [...lines.values()].filter((h) => h > 0).length
    const pct = (lh / lf) * 100
    totalLines += lf
    totalHit += lh
    rows.push([file, lf, lh, pct])
    if (pct < PER_FILE_MIN) {
      problems.push(`${file} at ${pct.toFixed(1)}% is below the ${PER_FILE_MIN}% per-file floor`)
    }
  }

  // anything measured that isn't on the list — a new file someone forgot to add
  for (const file of merged.keys()) {
    if (!SOURCES.includes(file) && !file.startsWith('test')) {
      problems.push(
        `${file} is covered but missing from SOURCES in ${path.relative(ROOT, __filename)}`,
      )
    }
  }

  const total = totalLines ? (totalHit / totalLines) * 100 : 0
  console.log(`\n${'file'.padEnd(20)}${'lines'.padStart(8)}${'hit'.padStart(8)}${'%'.padStart(9)}`)
  console.log('-'.repeat(45))
  for (const [file, lf, lh, pct] of rows) {
    const shown = pct === null ? 'NONE' : `${pct.toFixed(1)}%`
    console.log(
      file.padEnd(20) + String(lf).padStart(8) + String(lh).padStart(8) + shown.padStart(9),
    )
  }
  console.log('-'.repeat(45))
  console.log(
    'TOTAL'.padEnd(20) +
      String(totalLines).padStart(8) +
      String(totalHit).padStart(8) +
      `${total.toFixed(1)}%`.padStart(9),
  )

  if (total < TOTAL_MIN) {
    problems.push(`total coverage ${total.toFixed(1)}% is below the ${TOTAL_MIN}% floor`)
  }

  if (problems.length) {
    console.error('\nCoverage check failed:')
    for (const p of problems) console.error(`  - ${p}`)
    console.error('\nAdd tests for the code you changed, or adjust the floors deliberately.')
    process.exit(1)
  }
  console.log(`\nOK — ${total.toFixed(1)}% (floor ${TOTAL_MIN}%, per file ${PER_FILE_MIN}%)`)
} finally {
  fs.rmSync(tmp, { recursive: true, force: true })
}
