#!/usr/bin/env node

// Entry point when installed from npm / run via `bunx claude-usage-monitor`:
// spawn the Electron runtime pointed at the app's main process.
const { spawn } = require('node:child_process')
const path = require('node:path')
const electron = require('electron')

const child = spawn(electron, [path.join(__dirname, '..', 'main.js')], {
  stdio: 'inherit',
})
child.on('close', (code) => process.exit(code ?? 0))
