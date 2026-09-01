// macOS notifications are keyed off the code signature, not CFBundleIdentifier.
// electron-builder with `identity: null` skips signing entirely, which leaves
// Electron's own linker signature behind: identifier "Electron", Info.plist not
// bound. macOS then never registers the app with the notification centre and
// drops every alert in silence — the app doesn't even appear in
// Settings > Notifications.
//
// An ad-hoc signature with the real bundle id fixes that and needs no developer
// account, so it runs for every macOS build.
const { execFileSync } = require('node:child_process')
const path = require('node:path')

exports.default = async ({ appOutDir, packager, electronPlatformName }) => {
  if (electronPlatformName !== 'darwin') return
  const appId = packager.appInfo.id
  const app = path.join(appOutDir, `${packager.appInfo.productFilename}.app`)
  execFileSync('codesign', ['--force', '--deep', '--sign', '-', '--identifier', appId, app], {
    stdio: 'inherit',
  })
  console.log(`  • ad-hoc signed as ${appId} (so macOS delivers notifications)`)
}
