# <img src="https://github.com/user-attachments/assets/e5487d3e-aadb-466d-bd2b-cfba5476d3db" alt="" width="40" align="top"> Clauddy

A pixel-art desktop pet that keeps an eye on your Claude Code usage: the real session and weekly % from your account, the tokens from your local logs, and a pet that works, sleeps and catches fire along with it. It can follow Codex and Cursor too, and you can dress it as a wizard, Santa or a gingerbread cookie.

<p align="center">
  <a href="#running">📚 Get started</a> ·
  <a href="docs/design/02-features.md">✨ Features</a> ·
  <a href="docs/design/01-architecture.md">🏗️ Architecture</a> ·
  <a href="docs/design/03-setup.md">⚙️ Setup</a> ·
  <a href="docs/decisions/">🧭 Decisions</a>
</p>

https://github.com/user-attachments/assets/036dc92e-44f1-41d0-9463-a52879007e4d

## Stack

A native app in Rust with egui, no browser engine: about 1–2% CPU and a 20 MB app. The `clauddy` command on npm is a small launcher that downloads it.

More details, with every tool and version, in [`docs/design/01-architecture.md`](docs/design/01-architecture.md#stack).

## Requirements

- macOS on Apple Silicon (the first-class build), Windows x64 or Linux x64
- [Claude Code](https://claude.com/claude-code), whose logs in `~/.claude/projects` give the token counts; a Claude login, done once in Settings, gives the real %
- [Bun](https://bun.sh), or Node 18+ for `npx`, only to run it with `bunx clauddy`
- [Rust](https://rustup.rs), only to work on the code
- Optional: [Codex](https://openai.com/codex) or the [Cursor](https://cursor.com) app on the same machine, to follow them too

## Running

On macOS, as an app that opens at login:

```bash
curl -fsSL https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/install.sh | bash
```

Anywhere, without installing:

```bash
bunx clauddy
```

[`docs/design/03-setup.md`](docs/design/03-setup.md) has Windows and Linux, updating, running from the code and every setting in `config.json`.

## Test

```bash
cd native
cargo fmt --check                  # formatting
cargo clippy --release --all-targets -- -D warnings   # lint
cargo test --release               # unit tests and the snapshot gallery
scripts/bundle.sh                  # builds target/Clauddy.app (macOS)
CLAUDDY_STATE=fire cargo run --release   # forces a mood: idle, working, fire, tired, sleeping
```

The snapshot gallery renders every screen state into `native/tests/snapshots/`; after an intended visual change, `UPDATE_SNAPSHOTS=1 cargo test --release` rewrites it. At the root, `bun run test` tests the npm launcher, and `bun install` turns on a pre-commit hook that formats it. CI runs all of it on every pull request.

## Releases

Every merge to `main` can be a release: [semantic-release](https://semantic-release.gitbook.io) reads the Conventional Commit titles since the last tag (a `feat` bumps the minor, a `fix` or `perf` the patch, `docs` and `chore` nothing), builds the app on macOS, Windows and Linux runners, publishes the GitHub release and `clauddy` on npm. The whole pipeline is in [`docs/design/07-releases.md`](docs/design/07-releases.md).

## Docs

Everything about how Clauddy works, runs and was decided is indexed in [`docs/`](docs/README.md). Code rules and the checks before calling a change done are in [`AGENTS.md`](AGENTS.md).

We follow [Conventional Commits](https://www.conventionalcommits.org).

## Privacy

Everything stays on your machine, in `~/.claude-usage-monitor/`. Nothing leaves it except the OAuth and usage calls to Anthropic and, with Cursor connected, the usage calls to Cursor with the Cursor app's own token. What is kept and what is read: [`docs/design/01-architecture.md`](docs/design/01-architecture.md#data-on-disk).

## Community

- 🐛 [GitHub Issues](https://github.com/renatoaug/claude-usage-monitor/issues): report a bug or propose a feature
- 🔧 Pull requests are welcome: read [`CONTRIBUTING.md`](CONTRIBUTING.md) first for how to start and the checks
- 🐙 [@renatoaug on GitHub](https://github.com/renatoaug): the maintainer

## License

Clauddy is licensed under the [MIT License](LICENSE).
