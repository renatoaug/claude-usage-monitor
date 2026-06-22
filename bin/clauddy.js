#!/usr/bin/env node

// `clauddy`            → launch the widget (spawns Electron on main.js)
// `clauddy <state>`    → poke the running widget for fun/demo (writes the state
//                        to the data dir the app watches; the app reacts live)
const path = require('node:path')
const fs = require('node:fs')
const os = require('node:os')

// same data dir the app uses (respects CLAUDE_CONFIG_DIR for multi-account)
const dataDir = process.env.CLAUDE_CONFIG_DIR
  ? path.join(process.env.CLAUDE_CONFIG_DIR, 'usage-monitor')
  : path.join(os.homedir(), '.claude-usage-monitor')

const STATES = [
  'fire',
  'sleeping',
  'working',
  'tired',
  'idle',
  'poke',
  'celebrate',
  'auto',
  'clear',
]
const arg = process.argv[2]

if (!arg) {
  // no argument → launch the app
  const { spawn } = require('node:child_process')
  const electron = require('electron')
  const child = spawn(electron, [path.join(__dirname, '..', 'main.js')], { stdio: 'inherit' })
  child.on('close', (code) => process.exit(code ?? 0))
} else if (arg === '--help' || arg === '-h') {
  console.log(
    [
      'clauddy — a cute desktop pet that tracks your Claude Code usage',
      '',
      'Usage:',
      '  clauddy            launch the widget',
      '  clauddy <state>    poke the running widget (just for fun)',
      '',
      'States: fire, sleeping, working, tired, idle, poke, celebrate, auto',
      '(the widget must be running for a state to show)',
    ].join('\n'),
  )
} else if (STATES.includes(arg)) {
  // write the state for the running app to pick up
  fs.mkdirSync(dataDir, { recursive: true })
  fs.writeFileSync(
    path.join(dataDir, 'debug.json'),
    `${JSON.stringify({ state: arg, t: Date.now() })}\n`,
  )
  console.log(`clauddy → ${arg} (the running widget will react)`)
} else {
  console.error(`clauddy: unknown command "${arg}"`)
  console.error('try: fire, sleeping, working, tired, idle, poke, celebrate, auto — or --help')
  process.exit(1)
}
