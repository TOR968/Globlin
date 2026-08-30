# Changelog

All notable changes to this project are documented here. From 0.1.0 onward the entries below the header
are written by [release-plz](https://release-plz.dev) from [Conventional
Commits](https://www.conventionalcommits.org/en/v1.0.0/), so the commit message is the changelog entry.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.6](https://github.com/TOR968/Globlin/compare/v0.2.5...v0.2.6) - 2026-08-30

### Other

- *(site)* word the commit-scope note for the repository, not the branch
- *(site)* tighten the install copy, the card alt text and the menu footer
- *(site)* correct the encoder and site notes in the development docs
- *(site)* link the landing page from the README and development notes
- *(site)* keep the install steps and state labels legible to assistive tech
- *(site)* add the features, states, install and FAQ sections
- *(site)* name every footer action in the menu illustration label
- *(site)* recreate the tray menu as markup instead of a screenshot
- *(site)* add the header, hero and download call to action
- *(site)* use sameAs for the repository link in the JSON-LD
- *(site)* add the search and social metadata
- *(site)* scaffold the static landing page directory
- *(icon)* correct the test counts in CLAUDE.md
- *(icon)* tighten the PNG bit writer and the Open Graph card tests
- *(icon)* compress the generated PNGs with fixed-Huffman deflate
- *(site)* render the landing-page artwork from the icon code

## [0.2.5](https://github.com/TOR968/Globlin/compare/v0.2.4...v0.2.5) - 2026-08-29

### Added

- uninstall a global package from its tray row
- offer a two-step Uninstall in every package row

### Fixed

- satisfy the clippy 1.98 lints CI runs
- keep the bun note from clobbering the failure log and tighten the global-root probe
- find bun global packages when the manifest lives in the home directory

## [0.2.4](https://github.com/TOR968/Globlin/compare/v0.2.3...v0.2.4) - 2026-08-23

### Added

- group the self-update controls under a Globlin v<version> submenu

### Other

- move unit tests into sibling tests.rs modules

## [0.2.3](https://github.com/TOR968/Globlin/compare/v0.2.2...v0.2.3) - 2026-08-23

### Fixed

- order the release-plz jobs so no empty release PR is opened

## [0.2.2](https://github.com/TOR968/Globlin/compare/v0.2.1...v0.2.2) - 2026-08-23

### Fixed

- run release-plz on Windows so cargo package can compile

### Other

- run release-plz under a PAT so its tags trigger the build

## [0.2.1](https://github.com/TOR968/Globlin/compare/v0.2.0...v0.2.1) - 2026-08-23

### Fixed

- make self-update reliable when a check or an install fails

### Other

- license Globlin under the GNU GPL v3 or later
- compare releases against git tags instead of crates.io

## [0.2.0] - 2026-08-23

### Added

- Self-update from GitHub Releases: the tray menu offers the app's own newer build, verifies it against
  its published checksum, swaps it in by rename with a rollback on failure, and restarts it — opt-in
  via `auto_update`, with repeated failure notices throttled and failed lookups logged to their own
  diagnostics file (`self-update.log`).
- The update batch shows as a queue with per-package progress bars, per-target finish announcements,
  and packages can be toggled ignored straight from the tray menu.
- New tray glyph — an uppercase G for the rename to Globlin — with an emerald glow when everything is
  current and a rising-water fill while a job runs.

### Fixed

- The single-instance mutex handle is closed after a failed claim, and a replaced launch that never
  claims it still raises its toast; a double rename failure keeps the verified new build instead of
  emptying the live path.
- The toast artwork is rewritten so a changed glyph actually reaches notifications, and the row bar
  and water level no longer overshoot or misreport what has landed.

### Changed

- The app is renamed from `npm-globals-tray` to Globlin end to end: binary slug, AppUserModelId, config
  file, data directory, autostart value, single-instance mutex, self-update endpoint and release assets.
  See the README for the leftover files an old install can delete by hand.

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
