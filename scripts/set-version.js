// Stamp a version into package.json before an electron-builder run.
// The release pipeline computes the next version once (semantic-release
// dry-run) and each per-OS build job stamps it so artifacts are named with the
// real version instead of the 0.1.0 placeholder.
const fs = require('node:fs')
const path = require('node:path')

const version = process.argv[2]
if (!version) {
  console.error('usage: node scripts/set-version.js <version>')
  process.exit(1)
}

const PKG = path.join(__dirname, '..', 'package.json')
const pkg = JSON.parse(fs.readFileSync(PKG, 'utf8'))
pkg.version = version
fs.writeFileSync(PKG, `${JSON.stringify(pkg, null, 2)}\n`)
console.log(`package.json version set to ${version}`)
