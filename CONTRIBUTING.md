# Contributing

Thanks for wanting to help Clauddy. This page is the short version: how a contribution starts, what it needs to pass and how a pull request is written. The details live in the docs it links to.

## Where to start

| You have | Start with |
|---|---|
| A bug you can reproduce | An issue from the [Bug report](https://github.com/renatoaug/claude-usage-monitor/issues/new?template=bug_report.yml) template: what you did, what you expected and what happened |
| An idea or a new feature | An issue from the [Feature request](https://github.com/renatoaug/claude-usage-monitor/issues/new?template=feature_request.yml) template, before writing code, so we agree on the shape first |
| A small, obvious fix (a typo, a doc, a one-line bug) | A pull request straight away |

Work on an issue someone else is already on only after asking in it.

## Running it

```bash
cd native
cargo run --release                      # the app, from source
CLAUDDY_STATE=fire cargo run --release   # forces a mood
```

[`docs/design/03-setup.md`](docs/design/03-setup.md) has the rest: the Linux build dependencies, every setting and the moods you can force. [`docs/README.md`](docs/README.md) indexes the other docs.

## Rules and checks

[`AGENTS.md`](AGENTS.md) has the code rules: where things go, the code style, the data kept on disk and how the pet behaves. Read it before the first change.

Before a pull request:

1. `cargo fmt --check` and `cargo test --release` pass in `native/`. A visual change updates the snapshot gallery (`UPDATE_SNAPSHOTS=1 cargo test --release`), and a new state gets a new case.
2. `native/scripts/bundle.sh` builds an app that launches.
3. A change to the npm launcher passes `bun run check` and `bun run test`.
4. A change to how something works updates its note in [`docs/design/`](docs/design/) in the same pull request, and a structural decision gets a record in [`docs/decisions/`](docs/decisions/).

Two things that bite:

- **Updating runs `curl … | bash`.** Never call `Updater::install` from a test: it installs over the app you are running.
- **Never commit `auth.json`** or paste its token anywhere: it lives in `~/.claude-usage-monitor/` and is gitignored.

Windows and Linux artifacts build only on their own runners ([ADR-010](docs/decisions/ADR-010-build-on-each-platform.md)); do not try to reproduce them on a Mac.

## Pull requests

- One change per pull request, branched off `main`.
- The title follows [Conventional Commits](https://www.conventionalcommits.org) with a lowercase subject (`fix(codex): ...`, `feat: ...`). Pull requests are squash-merged and the title becomes the release note: a `feat` bumps the minor version, a `fix` or `perf` the patch, and `docs`, `chore`, `ci`, `test` or `refactor` ship nothing.
- The body follows the template: **What?**, **Why?** and **Acceptance criteria**. An issue it solves goes on the first line of **What?** as `Closes #NN`.
- The acceptance criteria tick what you already ran and leave unticked what a person has to look at, naming the size (expanded, compact, pet only, menu bar) and the state. A change with nothing visible says so in one line.

## License

Clauddy is under the [MIT License](LICENSE). By opening a pull request you agree that your contribution is licensed under it.
