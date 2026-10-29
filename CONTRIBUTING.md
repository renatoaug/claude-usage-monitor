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
bun install     # dependencies, and turns on the pre-commit hook
bun start       # the widget, from source
./pet fire      # forces a pet state (./pet auto to go back)
```

[`docs/design/03-setup.md`](docs/design/03-setup.md) has the rest: Node 24, every setting and the simulator's states. [`docs/README.md`](docs/README.md) indexes the other docs.

## Rules and checks

[`AGENTS.md`](AGENTS.md) has the code rules: where things go, the code style, the data kept on disk and how the pet behaves. Read it before the first change.

Before a pull request:

1. `bun run check` is clean. The pre-commit hook runs it on the staged files.
2. `bun run test:coverage` passes: 80% overall, 60% per file, and no file that ships without coverage ([ADR-003](docs/decisions/ADR-003-coverage-counts-every-file.md)). A new file in the app needs its test.
3. `bun run pack` builds and the app still launches.
4. A change to how something works updates its note in [`docs/design/`](docs/design/) in the same pull request, and a structural decision gets a record in [`docs/decisions/`](docs/decisions/).

Three things that bite:

- **`bun run test`, never bare `bun test`.** The suite is three processes; bare `bun test` mixes their mocks and fails for reasons unrelated to the change ([ADR-002](docs/decisions/ADR-002-three-test-processes.md)).
- **`main.js`'s update path runs `curl … | bash`.** A test that reaches `do-update` must stub `child_process.spawn`, or it installs over the app you are running.
- **Never commit `auth.json`** or paste its token anywhere: it lives in `~/.claude-usage-monitor/` and is gitignored.

Windows and Linux artifacts build only on their own runners ([ADR-010](docs/decisions/ADR-010-build-on-each-platform.md)); do not try to reproduce them on a Mac.

## Pull requests

- One change per pull request, branched off `main`.
- The title follows [Conventional Commits](https://www.conventionalcommits.org) with a lowercase subject (`fix(codex): ...`, `feat: ...`). Pull requests are squash-merged and the title becomes the release note: a `feat` bumps the minor version, a `fix` or `perf` the patch, and `docs`, `chore`, `ci`, `test` or `refactor` ship nothing.
- The body follows the template: **What?**, **Why?** and **Acceptance criteria**. An issue it solves goes on the first line of **What?** as `Closes #NN`.
- The acceptance criteria tick what you already ran and leave unticked what a person has to look at, naming the size (expanded, compact, pet only, menu bar) and the state. A change with nothing visible says so in one line.

## License

Clauddy is under the [MIT License](LICENSE). By opening a pull request you agree that your contribution is licensed under it.
