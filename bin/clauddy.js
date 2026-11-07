#!/usr/bin/env node

// `clauddy` launches the native app (native/, Rust). The first run of each
// version downloads that version's release build for this OS into
// ~/.claude-usage-monitor/bin/<version>/; later runs open it straight away.
// The app runs on its own: this launcher exits as soon as it has started it.
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { spawn, execFileSync } = require('node:child_process')

const REPO = 'renatoaug/claude-usage-monitor'

// the release asset for an OS, and the file inside it to start
function assetFor(platform, arch, version) {
  if (platform === 'darwin' && arch === 'arm64')
    return { file: `Clauddy-${version}-arm64-mac.zip`, app: 'Clauddy.app' }
  if (platform === 'win32' && arch === 'x64')
    return { file: `Clauddy-${version}-win-x64.zip`, app: 'Clauddy.exe' }
  if (platform === 'linux' && arch === 'x64')
    return { file: `Clauddy-${version}-linux-x64.tar.gz`, app: path.join('Clauddy', 'clauddy') }
  return undefined
}

function dataDir() {
  return process.env.CLAUDE_CONFIG_DIR
    ? path.join(process.env.CLAUDE_CONFIG_DIR, 'usage-monitor')
    : path.join(os.homedir(), '.claude-usage-monitor')
}

async function install(asset, version, dir) {
  const url = `https://github.com/${REPO}/releases/download/v${version}/${asset.file}`
  console.log(`clauddy: downloading ${asset.file}…`)
  const res = await fetch(url)
  if (!res.ok) throw new Error(`download failed (${res.status}): ${url}`)
  const tmp = `${dir}.tmp`
  fs.rmSync(tmp, { recursive: true, force: true })
  fs.mkdirSync(tmp, { recursive: true })
  const archive = path.join(tmp, asset.file)
  fs.writeFileSync(archive, Buffer.from(await res.arrayBuffer()))
  // ditto keeps the app bundle's signature; tar reads zips on Windows 10+
  if (process.platform === 'darwin') execFileSync('ditto', ['-x', '-k', archive, tmp])
  else execFileSync('tar', ['-xf', archive, '-C', tmp])
  fs.rmSync(archive)
  fs.rmSync(dir, { recursive: true, force: true })
  fs.renameSync(tmp, dir)
}

// older versions' builds are dead weight once a newer one is in
function prune(root, keep) {
  for (const name of fs.readdirSync(root)) {
    if (name !== keep) fs.rmSync(path.join(root, name), { recursive: true, force: true })
  }
}

async function main() {
  const arg = process.argv[2]
  if (arg === '--help' || arg === '-h') {
    console.log(
      'clauddy — a desktop pet that tracks your Claude Code usage\n\nUsage:\n  clauddy    launch the app',
    )
    return
  }
  const { version } = require('../package.json')
  const asset = assetFor(process.platform, process.arch, version)
  if (!asset) {
    console.error(
      `clauddy: no build for ${process.platform}-${process.arch} (macOS arm64, Windows x64, Linux x64)`,
    )
    process.exit(1)
  }
  const root = path.join(dataDir(), 'bin')
  const dir = path.join(root, version)
  const app = path.join(dir, asset.app)
  if (!fs.existsSync(app)) {
    await install(asset, version, dir)
    prune(root, version)
  }
  const [cmd, args] = process.platform === 'darwin' ? ['open', [app]] : [app, []]
  spawn(cmd, args, { detached: true, stdio: 'ignore' }).unref()
}

if (require.main === module) {
  main().catch((e) => {
    console.error(`clauddy: ${e.message}`)
    process.exit(1)
  })
}

module.exports = { assetFor }
