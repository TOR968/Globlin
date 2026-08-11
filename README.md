# npm-globals-tray

A tray-resident utility that watches your **global** npm (and bun) packages, notifies you when one falls
behind, and updates it on a click. Single portable `.exe`, no installer, no runtime dependencies.

It never updates anything on its own. It reports; you click.

## Status

Windows is implemented and verified. The crates and module seams are cross-platform, but the macOS and
Linux arms of `src/platform/` are stubs that return an error — see [Other platforms](#other-platforms).

## Build

Requires a Rust toolchain (MSVC on Windows) — `scoop install rustup` or
`winget install Rustlang.Rustup`, plus Microsoft C++ Build Tools and the Windows SDK.

```
cargo build --release
```

The result is `target/release/npm-globals-tray.exe` (~1.5 MB). Copy it anywhere and run it; it keeps its
config next to itself.

## The menu

Right-click the tray icon:

```
npm globals — 3 updates available
─────────────────────────────────────
↑  @google/gemini-cli   0.53.1 → 0.54.4
↑  @salesforce/cli      2.145.6 → 2.146.3
↑  vercel               58.4.4 → 58.9.1
─────────────────────────────────────
✓  prettier             3.9.6
✓  typescript           7.0.2
·  npm                  12.0.2      (ignored)
?  some-package         1.0.0       (not checked)
─────────────────────────────────────
Update all (3)
Check now
☑ Run at startup
Open last log
Quit
```

Clicking an `↑` row runs `npm install -g <name>@latest` (or `bun add -g …`) with no console window, then
re-checks. Packages from bun are suffixed ` (bun)` so a name installed in both places stays distinguishable.

Markers: `↑` outdated, `✓` current, `·` ignored, `?` the registry did not answer for it. `?` is
deliberately **not** the same as `✓` — a network failure must never look like "everything is fine".

### While it is working

The first line always says what is happening right now, and the row being worked on is marked too, so
"something is happening" is never ambiguous:

```
Updating @salesforce/cli 2.145.6 → 2.146.3  [2/3]..
─────────────────────────────────────
↑  @google/gemini-cli   0.53.1 → 0.54.4
◓  @salesforce/cli      2.145.6 → 2.146.3..
↑  vercel               58.4.4 → 58.9.1
```

The trailing dots cycle through `` → `.` → `..` → `...`, the row marker spins through `◐◓◑◒`, and the tray
icon becomes a ring of orbiting dots — all driven by one frame counter that ticks every 120 ms and only
while work is in flight. Idle costs nothing: the event loop simply waits until the next scheduled check.

Every clickable row and *Update all* / *Check now* are disabled while an update runs, so a second job
cannot be started on top of the first.

## Icons

There are no image files in the repository. Every icon is drawn from primitives (discs, thick segments,
triangles) in `src/icon/render.rs`, supersampled 4× for antialiasing, and rendered at whatever size is
asked for:

| State | Look |
|---|---|
| Idle | slate badge, white check — everything current |
| Updates | amber badge, white up-arrow |
| Busy | ring of eight sky-blue dots, one bright head fading to a tail, rotating |
| Error | red badge, white exclamation mark |

The same drawing code has three consumers. The tray asks for 32 px frames at runtime. `build.rs` renders
the idle badge at 16/32/48/64/128 px and packs it into a real `.ico` (`src/icon/ico.rs` writes the
container by hand) which `winresource` embeds as the executable's icon. And the first notification writes
that same `.ico` next to the config so the toast can show it.

## Checking

On startup, then every `check_interval_hours`. For each package it reads only the `latest` dist-tag:

```
GET https://registry.npmjs.org/-/package/<name>/dist-tags
```

That is ~100 bytes per package instead of a multi-megabyte packument, and it deliberately ignores other
tags — `@salesforce/cli`, for instance, also publishes `latest-rc` and `nightly`, which you do not want
installed. Scoped names have their `/` encoded as `%2f`. Requests run on 6 threads with a 10 s timeout.
Versions are compared with `semver`, not as strings, so `2.9.0 < 2.10.0` holds.

A notification is raised only when the set of outdated packages **changes**, so a six-hourly check does not
nag about the same three packages forever.

## Config

`npm-globals-tray.json`, read from next to the exe first, otherwise from
`%LOCALAPPDATA%\npm-globals-tray\`. Written back to the first of those that accepts a write. Missing keys
take their defaults, so a partial file is fine.

```json
{
  "check_interval_hours": 6,
  "sources": { "npm": true, "bun": true },
  "ignore": ["npm", "@anthropic-ai/claude-code"],
  "last_notified": [],
  "npm_cmd": null
}
```

- **`ignore`** — listed packages still appear in the menu with their version, but are never checked, never
  flagged, and never notified about. `npm` is ignored by default because updating it on Windows means it
  rewrites its own shim mid-run; `@anthropic-ai/claude-code` because it self-updates. Remove either entry
  if you want them managed here. `"ignore": []` is respected — an explicit empty list is not overridden by
  the defaults.
- **`last_notified`** — bookkeeping for the "only notify on change" rule. Clear it to force the next check
  to notify again.
- **`npm_cmd`** — an override, only needed if `npm.cmd` is somewhere unusual. Left `null`, `PATH` is
  searched, then `%APPDATA%\npm\npm.cmd`.

If the file is not valid JSON it is moved to `npm-globals-tray.json.invalid`, defaults are used, and a
notification says so — your edits are never silently overwritten.

## Diagnostics

In `%LOCALAPPDATA%\npm-globals-tray\`:

- **`last-check.txt`** — every package from the most recent check, one per line, with its state. This is
  the file to look at when the menu shows something surprising.
- **`last-run.log`** — stdout and stderr of the most recent *failed* update, with the package name and both
  versions on the first line. Reachable from the menu via *Open last log*.
- **`app.ico`** — the notification artwork, written once so the toast can point at it.

## Removing it

1. Quit from the tray menu and delete the `.exe` and its `.json`.
2. Delete `%LOCALAPPDATA%\npm-globals-tray\`.
3. Two registry values, both under `HKEY_CURRENT_USER`, both optional to clean:
   - `Software\Microsoft\Windows\CurrentVersion\Run` → `npm-globals-tray` (only if *Run at startup* was
     ever enabled; toggling it off removes it)
   - `Software\Classes\AppUserModelId\NpmGlobals.Tray` (created so notifications are attributed to
     "npm globals" rather than to PowerShell)

Nothing else is written. There is no installer, no service, and no scheduled task.

## Implementation notes

Two things about the environment that the code has to work around, both verified rather than assumed:

- **`npm ls -g --json` exits non-zero when the global tree has problems** (an orphaned directory in
  `node_modules` is enough). The exit code is therefore ignored and stdout is parsed anyway; entries with
  no usable `version` are skipped instead of failing the whole listing.
- **`bun pm ls -g` does not list global packages.** It ignores `-g` and prints the tree for whatever
  directory it is run from, walking up to find a `package.json` — so it will happily report a project's
  dependencies as if they were global. The bun source reads
  `~/.bun/install/global/package.json` (honouring `BUN_INSTALL`) and resolves versions from that
  directory's `node_modules`. With an empty global store it reports zero packages, which is not an error.

Drawing the icons rather than shipping them keeps the dependency list at seven crates: no image decoder, no
asset pipeline, and the executable icon cannot drift out of sync with the tray icon because both come from
the same function.

## Layout

| Module | Responsibility |
|---|---|
| `main.rs` | the `tao` event loop and nothing else |
| `app.rs` | wiring: dispatches menu actions, spawns workers, owns the frame counter |
| `check.rs` | the read path — what is installed, what is behind |
| `update.rs` | the write path — runs updates, announces progress per package |
| `notice.rs` | decides whether an update set is worth a notification, and what it should say |
| `registry.rs` | dist-tags over HTTP |
| `source/` | `PackageSource` trait plus the npm and bun adapters |
| `tray/` | `mod.rs` owns the tray handle; `menu.rs` builds the menu and every label |
| `icon/` | `render.rs` draws the states, `ico.rs` writes the container |
| `platform/` | the OS-specific arm, selected by `cfg` |
| `diagnostics.rs` | the two files written for troubleshooting |
| `config.rs`, `model.rs` | settings and the types everything else speaks in |

The split follows one rule: anything with branch-worthy logic lives where it can be tested without a
running Win32 tray. That is why `notice.rs` exists as its own module rather than as methods on `App` —
the "only notify when the set changed" rule has real edge cases (first sighting, a newer target version,
everything becoming current) and `App` cannot be constructed in a test.

## Tests

```
cargo test
```

93 tests, no network and no side effects. The update orchestration is exercised by pointing `npm_cmd` at a
file that cannot be executed, which drives the real failure paths without installing anything.

Five tests are `#[ignore]`d because they do touch the real system. Run them deliberately:

```
cargo test -- --ignored --exact platform::windows::tests::autostart_round_trips_through_the_run_key
cargo test -- --ignored --exact platform::windows::tests::raises_a_real_notification
cargo test -- --ignored --exact update::tests::a_real_npm_failure_lands_in_the_log
cargo test -- --ignored --exact icon::tests::dump_every_state_for_visual_review
$env:UPDATE_TARGET="npm-check-updates"; cargo test -- --ignored --exact update::tests::updates_a_package_for_real
```

The autostart test saves and restores whatever the Run key held before it ran. `dump_every_state_for_visual_review`
writes one `.ico` per state and per spinner frame to `%TEMP%\npm-globals-tray-icons` so the artwork can be
eyeballed. `updates_a_package_for_real` really installs `$UPDATE_TARGET@latest` globally — point it at
something harmless.

To see the working UI on demand, downgrade something disposable and hit *Check now*:

```
npm i -g npm-check-updates@23.0.0
```

## CI and releases

Two workflows, both Windows-only, because that is the only platform arm this project implements.

**`.github/workflows/ci.yml`** runs on every push to `master`/`main` and on pull requests:
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, then a release build whose
`.exe` is attached to the run as an artifact and whose size is printed to the log. Superseded runs are
cancelled. The `#[ignore]`d tests never run here — that is the point of ignoring them.

**`.github/workflows/release.yml`** runs only on a `v*` tag. It first refuses to continue if the tag does
not match the `version` in `Cargo.toml`, so a `v0.2.0` tag on a `0.1.0` manifest fails instead of
publishing a mislabelled build. Then it tests, builds, and creates the GitHub Release with two assets: the
bare `npm-globals-tray.exe` and a `.sha256` next to it.

**`.github/workflows/release-plz.yml`** removes the manual version bookkeeping. On every push to `master`,
[release-plz](https://release-plz.dev) reads the commits since the last tag and keeps a **release pull
request** open containing the version bump in `Cargo.toml`/`Cargo.lock` and the new `CHANGELOG.md` entries.
Nothing is published while that PR sits there. Merging it is the decision to release: release-plz then
creates the `v*` tag, which triggers `release.yml`, which builds and publishes the release.

### Cutting a version

1. Write [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) — `feat:` for a minor bump,
   `fix:` for a patch, `feat!:` or a `BREAKING CHANGE:` footer for a major. The commit subject becomes the
   changelog line, so write it for a reader.
2. Push to `master`. Check the release PR release-plz opens or updates.
3. Merge that PR when the version is worth shipping. The tag, the build and the release follow on their own.

Nothing stops you from tagging by hand instead — `release.yml` only cares that a `v*` tag appeared and that
it agrees with `Cargo.toml`.

### Division of labour

`release-plz.toml` keeps the two halves from fighting over the same job:

```toml
[workspace]
publish = false            # not a crates.io crate, so never run cargo publish
git_tag_enable = true      # release-plz owns the tag
git_release_enable = false # release.yml owns the GitHub release and its assets
changelog_update = true
```

Without `git_release_enable = false` both release-plz and `release.yml` would race to create the same
release. As configured, release-plz stops at the tag and hands over.

`release.yml` builds its notes by pulling the matching section out of `CHANGELOG.md`
(`.github/changelog-section.ps1`) and appending the download and checksum instructions from
`.github/release-notes.md`. If no section matches the tag, the static notes are used alone rather than
failing the release.

### Required repository setting

release-plz opens pull requests, so **Settings → Actions → General → Workflow permissions → "Allow GitHub
Actions to create and approve pull requests"** must be enabled. Without it the `release-pr` job fails with a
permissions error; everything else keeps working.

Release PRs opened with the default `GITHUB_TOKEN` do not trigger other workflows, so CI will not run on the
release PR itself. That is a deliberate GitHub restriction; supply a PAT or a GitHub App token instead if
you want CI on those PRs.

`release-plz/action` is the only third-party action here — `actions/checkout`, `actions/upload-artifact` and
the `gh` CLI already on the runner cover the rest. Rust comes preinstalled on both runner images, so there
is no toolchain step and no cache to invalidate; a cold build takes about a minute.

## Other platforms

`src/platform/` is selected by `cfg`: `windows.rs` is implemented, `unix.rs` returns an error from every
call it cannot honour. The rest of the code, and every dependency, already works on all three OSes —
`tao` was chosen over `winit` precisely because `tray-icon` needs a **GTK** event loop on Linux, which
`tao` provides. Finishing macOS or Linux means writing that one file plus:

- **Notifications** — `notify-rust` (XDG D-Bus / NSUserNotification). It is not used on Windows because it
  offers no way to set the AppUserModelID, which would leave every toast attributed to PowerShell.
- **Autostart** — `~/Library/LaunchAgents/*.plist` on macOS, `~/.config/autostart/*.desktop` on Linux.
- **Single instance** — a lock file instead of a named mutex.

Two costs to know about before starting: on macOS, notifications require a signed `.app` bundle with an
`Info.plist`, so "portable, no install" does not survive the port. On Linux, `tray-icon` needs
`libgtk-3-dev`, `libxdo-dev` and `libayatana-appindicator3-dev` at build time, and GNOME shows no tray at
all without the AppIndicator extension.
