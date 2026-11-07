const { test, expect } = require('bun:test')
const { assetFor } = require('../bin/clauddy')
const pkg = require('../package.json')

test('each released OS gets the asset package.sh builds', () => {
  expect(assetFor('darwin', 'arm64', '2.0.0').file).toBe('Clauddy-2.0.0-arm64-mac.zip')
  expect(assetFor('win32', 'x64', '2.0.0').file).toBe('Clauddy-2.0.0-win-x64.zip')
  expect(assetFor('linux', 'x64', '2.0.0').file).toBe('Clauddy-2.0.0-linux-x64.tar.gz')
})

test('an OS without a build gets none', () => {
  expect(assetFor('darwin', 'x64', '2.0.0')).toBeUndefined()
  expect(assetFor('linux', 'arm64', '2.0.0')).toBeUndefined()
})

test('the package ships only the launcher, with no runtime dependencies', () => {
  expect(pkg.files).toEqual(['bin'])
  expect(pkg.dependencies).toBeUndefined()
})
