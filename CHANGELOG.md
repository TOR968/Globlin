# Changelog

All notable changes to this project are documented here. From 0.1.0 onward the entries below the header
are written by [release-plz](https://release-plz.dev) from [Conventional
Commits](https://www.conventionalcommits.org/en/v1.0.0/), so the commit message is the changelog entry.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.17](https://github.com/TOR968/Globlin/compare/v0.4.16...v0.4.17) - 2026-10-24

### Fixed

- winget updates that skipped the admin prompt, Steam rows stuck at not checked, and Close & update for programs in use ([#120](https://github.com/TOR968/Globlin/pull/120))

## [0.4.16](https://github.com/TOR968/Globlin/compare/v0.4.15...v0.4.16) - 2026-10-24

### Added

- link the GitHub repository and the Patreon membership page ([#118](https://github.com/TOR968/Globlin/pull/118))

## [0.4.15](https://github.com/TOR968/Globlin/compare/v0.4.14...v0.4.15) - 2026-10-21

### Added

- update global skills installed with skills.sh ([#115](https://github.com/TOR968/Globlin/pull/115))

## [0.4.14](https://github.com/TOR968/Globlin/compare/v0.4.13...v0.4.14) - 2026-10-21

### Fixed

- skip a winget portable update whose program is running instead of failing it

## [0.4.13](https://github.com/TOR968/Globlin/compare/v0.4.12...v0.4.13) - 2026-10-15

### Fixed

- find package managers listed after a stray quote in PATH

## [0.4.12](https://github.com/TOR968/Globlin/compare/v0.4.11...v0.4.12) - 2026-10-14

### Fixed

- report inapplicable winget updates under one prompt as skipped, not failed

## [0.4.11](https://github.com/TOR968/Globlin/compare/v0.4.10...v0.4.11) - 2026-10-14

### Added

- optionally update every winget package under one administrator prompt

## [0.4.10](https://github.com/TOR968/Globlin/compare/v0.4.9...v0.4.10) - 2026-10-14

### Added

- optionally update every choco package under one administrator prompt
- update and uninstall winget and choco packages

### Fixed

- check once after a run of source toggles instead of on every click
- show updated packages as current while the follow-up check runs

### Other

- render each window screenshot with its own Edge profile

## [0.4.9](https://github.com/TOR968/Globlin/compare/v0.4.8...v0.4.9) - 2026-10-12

### Fixed

- keep the monochrome tray icon a macOS template image so the menu bar tints it ([#107](https://github.com/TOR968/Globlin/pull/107))

### Other

- state the builds are unsigned after SignPath Foundation declined ([#106](https://github.com/TOR968/Globlin/pull/106))

## [0.4.8](https://github.com/TOR968/Globlin/compare/v0.4.7...v0.4.8) - 2026-10-11

### Fixed

- explain VirusTotal detections only where they happen, and name the installer's real cause ([#103](https://github.com/TOR968/Globlin/pull/103))

### Other

- add a code signing policy and a privacy section ([#104](https://github.com/TOR968/Globlin/pull/104))

## [0.4.7](https://github.com/TOR968/Globlin/compare/v0.4.6...v0.4.7) - 2026-10-11

### Added

- link to Patreon from the window and the tray menu ([#101](https://github.com/TOR968/Globlin/pull/101))

## [0.4.6](https://github.com/TOR968/Globlin/compare/v0.4.5...v0.4.6) - 2026-10-11

### Fixed

- treat a machine without package managers as nothing found, not a failed check ([#99](https://github.com/TOR968/Globlin/pull/99))

## [0.4.5](https://github.com/TOR968/Globlin/compare/v0.4.4...v0.4.5) - 2026-10-11

### Added

- ship a self-updating AppImage for Linux ([#97](https://github.com/TOR968/Globlin/pull/97))

## [0.4.4](https://github.com/TOR968/Globlin/compare/v0.4.3...v0.4.4) - 2026-10-11

### Fixed

- start on Linux without libxdo ([#95](https://github.com/TOR968/Globlin/pull/95))

## [0.4.3](https://github.com/TOR968/Globlin/compare/v0.4.2...v0.4.3) - 2026-10-11

### Added

- add a monochrome tray icon ([#93](https://github.com/TOR968/Globlin/pull/93))

## [0.4.2](https://github.com/TOR968/Globlin/compare/v0.4.1...v0.4.2) - 2026-10-11

### Fixed

- draw the taskbar button with the goblin instead of a cached old icon ([#91](https://github.com/TOR968/Globlin/pull/91))

## [0.4.1](https://github.com/TOR968/Globlin/compare/v0.4.0...v0.4.1) - 2026-10-11

### Fixed

- hide Update all when nothing can be updated ([#88](https://github.com/TOR968/Globlin/pull/88))

## [0.4.0](https://github.com/TOR968/Globlin/compare/v0.3.15...v0.4.0) - 2026-10-11

### Added

- [**breaking**] update through one signed SHA256SUMS and ship the Mac as one zip ([#85](https://github.com/TOR968/Globlin/pull/85))
- a redesigned package window that puts updates first ([#84](https://github.com/TOR968/Globlin/pull/84))
- draw the goblin as an anime sticker ([#82](https://github.com/TOR968/Globlin/pull/82))

## [0.3.15](https://github.com/TOR968/Globlin/compare/v0.3.14...v0.3.15) - 2026-10-11

### Added

- a goblin for the logo and the tray icon ([#80](https://github.com/TOR968/Globlin/pull/80))

## [0.3.14](https://github.com/TOR968/Globlin/compare/v0.3.13...v0.3.14) - 2026-10-11

### Added

- uninstall go tools by deleting their binary ([#79](https://github.com/TOR968/Globlin/pull/79))

### Fixed

- hide sources a platform cannot have and treat dotnet without an SDK as empty ([#77](https://github.com/TOR968/Globlin/pull/77))

## [0.3.13](https://github.com/TOR968/Globlin/compare/v0.3.12...v0.3.13) - 2026-10-11

### Fixed

- show the Mac window in the Dock, keep the tray menu short and open the log mid-batch ([#75](https://github.com/TOR968/Globlin/pull/75))

### Other

- bump the actions group with 2 updates ([#74](https://github.com/TOR968/Globlin/pull/74))

## [0.3.12](https://github.com/TOR968/Globlin/compare/v0.3.11...v0.3.12) - 2026-10-11

### Added

- sign every release file and refuse an unsigned self-update ([#72](https://github.com/TOR968/Globlin/pull/72))

### Fixed

- never offer a public package as the update of a private, linked or git install ([#70](https://github.com/TOR968/Globlin/pull/70))
- recheck after an auto-approved retry and save the config atomically ([#69](https://github.com/TOR968/Globlin/pull/69))
- show every source even when a registry or a package manager fails ([#68](https://github.com/TOR968/Globlin/pull/68))

### Other

- pin actions to commit SHAs and drop write access where it is not needed ([#71](https://github.com/TOR968/Globlin/pull/71))
- bring the README and developer notes up to date with the ports ([#67](https://github.com/TOR968/Globlin/pull/67))

## [0.3.11](https://github.com/TOR968/Globlin/compare/v0.3.10...v0.3.11) - 2026-10-10

### Fixed

- find every package when the installer is what started Globlin ([#65](https://github.com/TOR968/Globlin/pull/65))

## [0.3.10](https://github.com/TOR968/Globlin/compare/v0.3.9...v0.3.10) - 2026-10-10

### Fixed

- list the other sources when one of them fails ([#63](https://github.com/TOR968/Globlin/pull/63))

### Other

- keep the suite out of the real data directory ([#62](https://github.com/TOR968/Globlin/pull/62))
- attach the Arch PKGBUILD to every release ([#60](https://github.com/TOR968/Globlin/pull/60))
- build an AUR package for Arch Linux and publish it on release ([#59](https://github.com/TOR968/Globlin/pull/59))

## [0.3.9](https://github.com/TOR968/Globlin/compare/v0.3.8...v0.3.9) - 2026-10-10

### Added

- run on Linux (experimental) as a deb and an rpm ([#56](https://github.com/TOR968/Globlin/pull/56))

### Other

- run release-plz tagging one push at a time ([#55](https://github.com/TOR968/Globlin/pull/55))

## [0.3.8](https://github.com/TOR968/Globlin/compare/v0.3.7...v0.3.8) - 2026-10-10

### Added

- ship a per-user Windows installer and an experimental macOS build ([#52](https://github.com/TOR968/Globlin/pull/52))

## [0.3.7](https://github.com/TOR968/Globlin/compare/v0.3.6...v0.3.7) - 2026-10-10

### Added

- publish a Windows ARM64 build ([#48](https://github.com/TOR968/Globlin/pull/48))

## [0.3.6](https://github.com/TOR968/Globlin/compare/v0.3.5...v0.3.6) - 2026-10-10

### Added

- *(tray)* open the window on left click and on notification click ([#46](https://github.com/TOR968/Globlin/pull/46))

## [0.3.5](https://github.com/TOR968/Globlin/compare/v0.3.4...v0.3.5) - 2026-10-03

### Added

- *(window)* order sources by activity and add an auto-approve setting for npm scripts ([#44](https://github.com/TOR968/Globlin/pull/44))

## [0.3.4](https://github.com/TOR968/Globlin/compare/v0.3.3...v0.3.4) - 2026-09-27

### Added

- ask before running a dependency's blocked npm install scripts ([#42](https://github.com/TOR968/Globlin/pull/42))

## [0.3.3](https://github.com/TOR968/Globlin/compare/v0.3.2...v0.3.3) - 2026-09-26

### Fixed

- run a package's own install scripts when npm 12 blocks them ([#40](https://github.com/TOR968/Globlin/pull/40))

## [0.3.2](https://github.com/TOR968/Globlin/compare/v0.3.1...v0.3.2) - 2026-09-12

### Added

- watch pipx, uv, scoop, cargo, go, dotnet, PowerShell Gallery and gem ([#38](https://github.com/TOR968/Globlin/pull/38))

## [0.3.1](https://github.com/TOR968/Globlin/compare/v0.3.0...v0.3.1) - 2026-09-12

### Fixed

- keep the sidebar source rows on one line ([#36](https://github.com/TOR968/Globlin/pull/36))

## [0.3.0](https://github.com/TOR968/Globlin/compare/v0.2.9...v0.3.0) - 2026-09-11

### Added

- watch four more package managers and list them all in one window ([#33](https://github.com/TOR968/Globlin/pull/33))

### Other

- name the real cause of the antivirus detection ([#32](https://github.com/TOR968/Globlin/pull/32))

## [0.2.9](https://github.com/TOR968/Globlin/compare/v0.2.8...v0.2.9) - 2026-09-01

### Fixed

- pin the Rust toolchain so the scanned build is the tested build ([#30](https://github.com/TOR968/Globlin/pull/30))

## [0.2.8](https://github.com/TOR968/Globlin/compare/v0.2.7...v0.2.8) - 2026-09-01

### Fixed

- drop the size-tuned release profile that antivirus reads as packed ([#28](https://github.com/TOR968/Globlin/pull/28))

## [0.2.7](https://github.com/TOR968/Globlin/compare/v0.2.6...v0.2.7) - 2026-08-31

### Added

- fill the version resource so the binary identifies itself ([#26](https://github.com/TOR968/Globlin/pull/26))

### Other

- count only engines that returned a verdict in the VirusTotal ratio ([#25](https://github.com/TOR968/Globlin/pull/25))

## [0.2.6](https://github.com/TOR968/Globlin/compare/v0.2.5...v0.2.6) - 2026-08-31

### Added

- stand down when winget owns the binary ([#23](https://github.com/TOR968/Globlin/pull/23))

### Other

- Microsoft removed the Defender detection ([#22](https://github.com/TOR968/Globlin/pull/22))
- scan each release with VirusTotal and link the report from the notes
- explain the antivirus false positive in the README
- stop landing-page commits from proposing a release
- *(icon)* correct the test counts in CLAUDE.md
- *(icon)* tighten the PNG bit writer and the Open Graph card tests
- *(icon)* compress the generated PNGs with fixed-Huffman deflate

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
