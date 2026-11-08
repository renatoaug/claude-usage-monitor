# AGENTS.md

Clauddy is a pixel-art desktop pet that tracks Claude Code usage: the real session and weekly % from the account (OAuth) and the token counts from local logs, plus Codex and Cursor when connected. It is a native app in Rust (`native/`), macOS first, with Windows and Linux (x64) builds; the `clauddy` npm package is a launcher that downloads it. How it works is in [`docs/design/`](docs/design/); why it is like this, in [`docs/decisions/`](docs/decisions/).

## Layout

- `native/` — the app (Rust, eframe/egui, no webview). Everything below under "Native app"
- `bin/clauddy.js` — the npm launcher: downloads the release build for the OS into `~/.claude-usage-monitor/bin/<version>/` and starts it ([ADR-014](docs/decisions/ADR-014-npm-launches-the-native-app.md)); `test/launcher.test.js` is its test
- `install.sh` — the macOS installer (latest release into `/Applications`)
- `.github/workflows/` — `pull_request.yml` (launcher lint and test, native build on 3 OS, fmt, tests on macOS) and `release.yml`

## Commands

```bash
cd native
cargo run --release                       # dev run
CLAUDDY_STATE=fire cargo run --release    # pin a mood: idle, working, fire, tired, sleeping
cargo fmt                                 # format (native/rustfmt.toml)
cargo clippy --release --all-targets -- -D warnings   # lint; pull requests fail on any warning
cargo test --release                      # unit tests + snapshot gallery
UPDATE_SNAPSHOTS=1 cargo test --release   # rewrite the gallery after an intended visual change
scripts/bundle.sh                         # native/target/Clauddy.app (macOS, app.clauddy.dev)
scripts/package.sh <version>              # the release artifact for this OS, into dist/

# at the root, for the launcher
bun install        # Biome, semantic-release, and the pre-commit hook
bun run check      # Biome format + lint (autofix)
bun run test       # the launcher's test
```

> Each platform's app is built on its own runner. `release.yml` handles that in three stages: `version` (semantic-release dry-run) → `build` (matrix on macOS/Windows/Linux, each running `native/scripts/package.sh` with the computed version) → `publish` (downloads every artifact and runs semantic-release for real). Details in [`docs/design/07-releases.md`](docs/design/07-releases.md), [ADR-010](docs/decisions/ADR-010-build-on-each-platform.md) and [ADR-012](docs/decisions/ADR-012-the-native-app-replaces-electron.md).

## Native app (Rust)

`native/` is the app (eframe/egui, no webview; [ADR-012](docs/decisions/ADR-012-the-native-app-replaces-electron.md), [design/08-native.md](docs/design/08-native.md)). `cargo test --release` and `cargo build --release` run from `native/`; `cargo fmt` follows `native/rustfmt.toml`, and pull requests check it.

- **Layout**: `src/ui/` holds generic pieces that know nothing about Clauddy (theme, icon, controls, surface, bar, tag, text); `src/components/` holds Clauddy's own pieces (pet, logos, account, service row, meter, status, breakdown, heat map); `src/screens/` holds the compact and expanded layouts; `src/app.rs` is the shell; `src/format.rs` is how numbers and times read. Data: `usage.rs`, `auth.rs`, `accounts.rs`, `poller.rs`. Look in `ui/` and `components/` before writing a piece: it usually exists.
- **Tokens**: `src/ui/theme.rs`. The raw palette is private to that file; everything else uses `color::*`, `text::*` and `radius::*`. Corners come in three sizes by role: `radius::TAG` (badges), `radius::CONTROL` (buttons, bars, boxes inside the card) and `radius::CARD`; no other radius. A test fails on a raw colour (`Color32::from_*`, `WHITE`, `BLACK`) anywhere else.
- **Screens** draw only from the `Frame` they get and return `Action`s for the app to apply. They never read the poller, the disk or the clock on their own (`format::now` is pinned in tests).
- **Snapshots**: `src/screens/snapshots.rs` renders every screen state from fixtures into `native/tests/snapshots/*.png`, and the tests fail when a render drifts. They are the gallery: a new state gets a new case. After an intended visual change, `UPDATE_SNAPSHOTS=1 cargo test --release` rewrites them; review the PNGs before committing.
- **Icon**: `native/assets/icon.png` (1024 px, on the macOS template, generated in the pet's art style); `native/scripts/icon.sh` turns it into `native/assets/icon.icns`.
- **Bundle**: `native/scripts/bundle.sh` builds `native/target/Clauddy.app` (Info.plist, icon, no Dock icon, ad-hoc signed as `app.clauddy.dev`, so a development build never mixes with the installed app). `native/scripts/package.sh <version>` builds what a release ships on the current OS, as `app.clauddy`. Notifications go through UNUserNotificationCenter: at launch the app comes to the front and asks for permission, which macOS shows as its Allow / Don't Allow prompt until answered. The loose binary can't notify, so test them with `open native/target/Clauddy.app`. Never send through NSUserNotificationCenter as well: macOS refuses a bundle id that mixes the two APIs.
- **Art**: new pet art is generated (Codex `gpt-6-astra`), not drawn in code; Clauddy stays square and blocky like the classic sprite.

## Tests

- `cargo test --release` in `native/`: each reader (`usage.rs`, `auth.rs`, `codex.rs`, `cursor.rs`, `reminders.rs`, ...) has its unit tests, and `src/screens/snapshots.rs` is the snapshot gallery. A new state gets a new case.
- `bun run test` at the root: the launcher.

> `updates.rs`'s install path spawns `curl … | bash` — never call `Updater::install` from a test, or it installs over the running app.

## Data & secrets

- All user data lives in `~/.claude-usage-monitor/` (NOT in the repo): `auth.json` (OAuth token, mode 600), `config.json` (settings), `alerts.json` (which notifications are already armed, so restarts don't repeat them), `reset-reminders.json` (one-shot deadlines and recently delivered reminders), `accounts.json` (the account list + the active one), `native-ui.json`, `native-voice.json`, `limits-cache.json` (the app's own state), `bin/` (the launcher's downloads).
- Extra accounts nest their own `auth.json`/`alerts.json` under `accounts/<id>/`; the first account keeps the top-level paths, so existing installs are untouched. Switching rebinds the token, the Claude logs directory and the alert state — only the active account is polled, so only it can notify.
- **Never commit** `auth.json` or any token/credential. It's gitignored — keep it that way.

## Code style

- Rust: `cargo fmt` with `native/rustfmt.toml` and `cargo clippy` with no warnings; pull requests check both. `too_many_arguments` is allowed in `Cargo.toml`: drawing functions take the ui, where and what.
- The launcher: Biome, **single quotes**, **no semicolons**, **2-space** indent, LF. A pre-commit hook (`.githooks/pre-commit`) formats staged files and blocks on lint errors; `bun install` wires it up.
- Match the surrounding style; keep comments short and in English.

## Documentation

- `README.md` stays short: what Clauddy is, how to run and test it, and links. The depth lives in `docs/` ([`docs/README.md`](docs/README.md) indexes it).
- [`docs/design/`](docs/design/) has numbered notes that describe the code as it is today. A change to how something works updates its note in the same commit or PR.
- A decision that shapes the code (a new source of data, a storage rule, how a release is built) gets a record in [`docs/decisions/`](docs/decisions/), following the template in its README. Records are never edited; a reversal is a new record that supersedes the old one.
- Don't hard-wrap Markdown (README, AGENTS.md, CONTRIBUTING, docs, PR and issue bodies): each paragraph or list item goes on a single line, and the editor wraps it on display. Code blocks and tables keep their normal format. Commit messages are the exception: they still wrap at ~72 characters.

## Commits — Conventional Commits

Every commit message MUST follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(optional scope): <short, imperative description>
```

Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.

Rules:

- Subject in the imperative mood, lower-case, no trailing period, ≤ ~72 chars.
- One logical change per commit; use the body to explain the "why" when useful. Few commits per branch.

Examples:

- `feat: add confetti burst on session reset`
- `fix(auth): use platform.claude.com token endpoint`
- `chore: bump eframe to 0.37`
- `docs: document the ./pet simulator`

## Pull requests and commits

Never mention Claude Code (or any AI) as an author in PRs, commits or anything versioned in this repository. In particular, do **not** add:

- the `🤖 Generated with [Claude Code](https://claude.com/claude-code)` footer
- session links `https://claude.ai/code/session_...`
- the `Claude-Session:` trailer in commit messages
- `Co-Authored-By: Claude ...`

Claude Code as the product Clauddy tracks is fine to name. PR descriptions and commit messages carry only the technical content: the problem, the change and how it was validated.

## PR conventions

- The title follows Conventional Commits: `type(scope): subject` (scope when it makes sense), with a lowercase subject. PRs are squash-merged, so it becomes the commit and the release note: a `feat` bumps the minor version, a `fix` or `perf` the patch ([`docs/design/07-releases.md`](docs/design/07-releases.md)).
- The body follows `.github/PULL_REQUEST_TEMPLATE.md`: the **What?**, **Why?** and **Acceptance criteria** sections.
- The PR base is `main`.
- **Acceptance criteria** separate what was already validated (checked) from what only a person can confirm (unchecked). A PR that changes something visible needs visual QA items naming the size (expanded, compact, pet only, menu bar) and the state.
- If there is an issue, the PR closes it: `Closes #NN` on the first line of **What?**. When it solves only part of it, `Part of #NN` and what is still missing.

## Before finishing a change

1. `cargo fmt --check`, `cargo clippy --release --all-targets -- -D warnings` and `cargo test --release` pass in `native/`, and `native/scripts/bundle.sh` builds an app that launches.
2. A change to the launcher passes `bun run check` and `bun run test`.
3. The design note it touches is updated, and a structural decision has its record.
4. Commit follows the convention above.

## Companion behavior

- Widget sizes: expanded, compact, and pet-only; `native-ui.json` remembers the choice. Pet-only is floating-mode only; hovering over the pet shows a usage glance above it, drawn as the pet's own speech bubble, within the fixed window canvas, without resizing or moving the window. Dragging the pet moves the window; a click on it does not change size, and the expand button inside the glance opens compact.
- Session reset reminders are explicitly requested, separate from threshold notifications. They run while Clauddy is open and catch up after sleep/restart. They persist before firing, cancel on disconnect/removal, and are account-scoped.
- A deadline alone does not prove a new budget. Only a fresh, lower reading after that deadline earns “budget is back”; otherwise say the reset time arrived. Codex logs and Cursor fetches can be stale. Never substitute an expired reading with zero.
- Compact and pet-only moods follow activity across connected providers, independently of the selected usage tab. Logos identify the workers; simultaneous work uses a stable generic working animation. With no active sources, the hottest connected session sets the mood: on fire past the fire threshold, maxed out at 100%, otherwise rest. The logo of the service on fire (or maxed out) takes the status dot's place. Cursor's session is its billing month. An expired Codex or Cursor reading doesn't count. Stale activity and disconnected sources are ignored.
- Working means an open turn, not a recent write: Claude Code, Codex and Cursor can write nothing for minutes while a command runs or the model thinks. Claude's turn closes on a final answer (`end_turn`, then `system/turn_duration`) or `[Request interrupted by user`; Codex's task opens on `task_started` and closes on `task_complete` or `turn_aborted`, tracked as each rollout is read (its tasks run far past a tail); Cursor's on `turn_ended`. Slash commands, `!` shell input, task notices and meta lines open nothing. An open turn untouched for 30 minutes counts as abandoned; a log that says nothing falls back to the recent-write window.
- Pets: each is a pair of sprite sheets in the classic sprite's layout (`native/src/components/skins.rs`), with boxes that keep it the same size and baseline across moods ([design/06-pets.md](docs/design/06-pets.md)). Seasonal packs override the saved pick on their days (Halloween 10/31, Christmas 12/24–25), one pet per pack and year, picked by a hash and not stored.
- `AGENTS.md` is the single source of project instructions.
