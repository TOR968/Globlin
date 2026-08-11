# Changelog

All notable changes to this project are documented here. From 0.1.0 onward the entries below the header
are written by [release-plz](https://release-plz.dev) from [Conventional
Commits](https://www.conventionalcommits.org/en/v1.0.0/), so the commit message is the changelog entry.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-08-11

Housekeeping only — the binary behaves exactly as 0.1.0 does.

### Other

- Enable `clippy::pedantic` in `Cargo.toml` so the existing `-D warnings` CI gate covers it, and clear
  every finding. Only the rasteriser and the `.ico` writer keep a module-level allow, for cast lints on
  small bounded integers.
- Trim the README from 308 lines to 215 and add `CLAUDE.md` describing the architecture that takes several
  files to see.
- Widen `.gitignore` to cover the diagnostics files, editor and OS leftovers, and machine-local Claude
  settings.

## [0.1.0] - 2026-08-11

First release.

### Added

- A tray icon that watches every globally installed npm and bun package, checking the registry's `latest`
  dist-tag at startup and every six hours.
- A context menu listing the whole inventory: `↑` outdated with both versions, `✓` current, `·` ignored,
  `?` not answered by the registry. Clicking an outdated row updates that package; *Update all* takes the
  set.
- Live progress while working: the header names the package and both versions with a `[2/3]` counter and
  cycling dots, the row being updated spins, and the tray icon becomes a rotating ring of dots.
- Desktop notifications, raised only when the set of outdated packages changes, attributed to
  "npm globals" through a registered AppUserModelID and its own icon.
- *Run at startup* as a menu toggle, backed by the `HKCU` `Run` key.
- Icons drawn procedurally at any size, shared by the tray, the executable resource and the notification.
- A portable configuration file kept next to the executable, with an `ignore` list defaulting to `npm` and
  `@anthropic-ai/claude-code`, and quarantine of an unparseable file rather than silent overwriting.
- `last-check.txt` and `last-run.log` in `%LOCALAPPDATA%` for troubleshooting.
- A single-instance guard, so launching twice leaves one tray icon.
