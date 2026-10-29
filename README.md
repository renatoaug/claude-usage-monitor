# 🟫 Clauddy

A pixel-art desktop pet that keeps an eye on your Claude Code usage: the real session and weekly % from your account, the tokens from your local logs, and a pet that works, sleeps and catches fire along with it. It can follow Codex and Cursor too, and you can dress it as a pumpkin, a ghost or Santa.

<p align="center">
  <a href="#running">📚 Get started</a> ·
  <a href="docs/design/02-features.md">✨ Features</a> ·
  <a href="docs/design/01-architecture.md">🏗️ Architecture</a> ·
  <a href="docs/design/03-setup.md">⚙️ Setup</a> ·
  <a href="docs/decisions/">🧭 Decisions</a>
</p>

https://github.com/user-attachments/assets/be8d339c-ac69-4980-96a7-8e286f8b080f

## Stack

An Electron widget in vanilla JS, with Bun for the scripts and the tests, and Biome for format and lint.

More details, with every tool and version, in [`docs/design/01-architecture.md`](docs/design/01-architecture.md#stack).

## Requirements

- macOS on Apple Silicon (the first-class build), Windows x64 or Linux x64
- [Claude Code](https://claude.com/claude-code), whose logs in `~/.claude/projects` give the token counts; a Claude login, done once in Settings, gives the real %
- [Bun](https://bun.sh), or Node 24 for `npx`, only to run it without installing or to work on the code
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
bun run check            # Biome format and lint (autofix)
bun run test             # the three test groups, each in its own process
bun run test:coverage    # the same, failing under 80% overall or 60% per file
bun run pack             # builds dist/mac-arm64/Clauddy.app
./pet fire               # forces a pet state in the running widget (./pet auto to go back)
```

Never run bare `bun test`: the groups under `test/` must each run in their own process ([ADR-002](docs/decisions/ADR-002-three-test-processes.md)). `bun install` turns on a pre-commit hook that formats the staged files and blocks on lint errors. CI runs the coverage gate on every pull request.

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
