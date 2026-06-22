// electron-builder refuses `electron` in "dependencies", but we keep it there
// so `bunx claude-usage-monitor` installs it. Temporarily move it to
// devDependencies for the build, then restore package.json exactly.
const fs = require('node:fs')
const path = require('node:path')
const { execFileSync } = require('node:child_process')

const PKG = path.join(__dirname, '..', 'package.json')
const builder = path.join(__dirname, '..', 'node_modules', '.bin', 'electron-builder')
const original = fs.readFileSync(PKG, 'utf8')

try {
  const pkg = JSON.parse(original)
  if (pkg.dependencies?.electron) {
    pkg.devDependencies = { ...pkg.devDependencies, electron: pkg.dependencies.electron }
    delete pkg.dependencies.electron
    if (Object.keys(pkg.dependencies).length === 0) delete pkg.dependencies
    fs.writeFileSync(PKG, `${JSON.stringify(pkg, null, 2)}\n`)
  }
  execFileSync(builder, process.argv.slice(2), { stdio: 'inherit' })
} finally {
  fs.writeFileSync(PKG, original)
}
