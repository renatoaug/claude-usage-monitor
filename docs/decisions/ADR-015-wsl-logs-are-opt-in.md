# ADR-015: Claude Code's logs in WSL are read only when switched on

- **Status**: Accepted
- **Date**: 2026-11-15
- **Related**: [design/04-usage.md](../design/04-usage.md#two-sources-two-jobs), [ADR-004](ADR-004-codex-and-cursor-are-opt-in.md), [ADR-006](ADR-006-working-means-an-open-turn.md)

## Context

On Windows, Claude Code is often run inside WSL. Its logs then live in the distro (`/home/<user>/.claude/projects`), which Windows reaches only as `\\wsl.localhost\<distro>\...`. Clauddy read only `%USERPROFILE%\.claude`, so the % (from the account) showed, but the tokens stayed at zero and the pet slept through every turn. Finding the distros means running `wsl.exe`, and opening a stopped distro's share boots it.

## Decision

On Windows, Clauddy reads Claude Code's logs inside WSL only when **Settings → Accounts → Claude → Claude Code in WSL** is on (`config.wsl.enabled`, off by default). Then, for the default account only, it asks `wsl.exe --list --running` at most once a minute (no console window, 3 s timeout) and reads `home/*/.claude/projects` and `root/.claude/projects` of each running distro beside the Windows logs. A stopped distro is never opened. Other systems never show the switch and never run `wsl.exe`.

## Alternatives considered

- **Always on for Windows**: no setup, but spawns `wsl.exe` every minute for everyone, WSL user or not.
- **Every installed distro, running or not**: keeps the counts steady, but boots stopped distros just to read them.
- **A path typed in Settings**: one more thing to get wrong, and it breaks when the distro or user changes.

## Consequences

- A clean install behaves as before on every system.
- With the switch on, a distro's tokens leave the counts while it is stopped and come back when it runs again; the pet only needs it while Claude Code runs, and then the distro is running.
- Extra accounts with their own Claude dir, and Codex inside WSL, are not covered.
