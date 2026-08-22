# npm-globals-tray

A tray-resident utility that watches your **global** npm (and bun) packages, notifies you when one falls
behind, and updates it on a click. Single portable `.exe`, no installer, no runtime dependencies.

It never updates anything on its own. It reports; you click.

Windows only in practice. The crates and module seams are cross-platform, but the macOS and Linux arms of
`src/platform/` are stubs that return an error — see [Other platforms](#other-platforms).

## Build

Requires a Rust toolchain (MSVC on Windows) plus Microsoft C++ Build Tools and the Windows SDK.

```
cargo build --release
```

`target/release/npm-globals-tray.exe` (~1.5 MB) is portable — copy it anywhere and run it; it keeps its
config next to itself.

## The menu

Right-click the tray icon:

```
npm globals — 3 updates available
─────────────────────────────────────
↑  @google/gemini-cli   0.53.1 → 0.54.4      ▸  Update · ☐ Ignore
↑  @salesforce/cli      2.145.6 → 2.146.3    ▸  Update · ☐ Ignore
↑  vercel               58.4.4 → 58.9.1      ▸  Update · ☐ Ignore
─────────────────────────────────────
✓  prettier             3.9.6                ▸  ☐ Ignore
✓  typescript           7.0.2                ▸  ☐ Ignore
·  npm                  12.0.2      (ignored) ▸ ☑ Ignore
?  some-package         1.0.0   (not checked) ▸ ☐ Ignore
─────────────────────────────────────
Update all (3)
Check now
☑ Run at startup
Open last log
Quit
```

Opening a row's submenu and picking `Update` runs `npm install -g <name>@latest` (or `bun add -g …`) with
no console window, then re-checks. Packages from bun are suffixed ` (bun)` so a name installed in both
places stays distinguishable.

Markers: `↑` outdated, `✓` current, `·` ignored, `?` the registry did not answer for it. `?` is
deliberately **not** the same as `✓` — a network failure must never look like "everything is fine".

Every package row is a submenu. Outdated packages offer `Update`; every package offers `Ignore`.
Ticking `Ignore` writes the name into the `ignore` list in `npm-globals-tray.json` and drops the
package out of `Update all` immediately, without touching the network. Unticking it starts a check
right away, because the registry was never asked about an ignored package — until that check lands the
row shows `?`, not `✓`. The list is keyed by name alone, so ignoring `typescript` silences it under
both npm and bun.

While an update runs, the batch is shown as a queue: finished packages carry `✓ done` (or `✗ failed`),
the package being worked on carries a spinner and a progress bar, and the rest carry `· queued`.
`npm install -g` reports progress only to a terminal, and the app runs it without a console, so the
bar is not a byte count: it rises asymptotically towards the share of the batch that the current
package represents, and only completes when the package actually lands. A failure is therefore never
shown as a success.

The header line and the tray icon animate off the same frame counter, ticking every 120 ms, and every
clickable row is disabled while a job runs so a second job cannot start on top of the first. Idle costs
nothing: the event loop waits until the next scheduled check.

## Checking

On startup, then every `check_interval_hours`. For each package it reads only the `latest` dist-tag:

```
GET https://registry.npmjs.org/-/package/<name>/dist-tags
```

That is ~100 bytes per package instead of a multi-megabyte packument, and it deliberately ignores other
tags — `@salesforce/cli`, for instance, also publishes `latest-rc` and `nightly`, which you do not want
installed. Scoped names have their `/` encoded as `%2f`. Six threads, 10 s timeout. Versions are compared
with `semver`, not as strings, so `2.9.0 < 2.10.0` holds.

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

- **`ignore`** — listed packages still appear in the menu with their version, but are never checked,
  flagged or notified about. `npm` is ignored by default because updating it on Windows means it rewrites
  its own shim mid-run; `@anthropic-ai/claude-code` because it self-updates. An explicit `[]` is respected.
- **`last_notified`** — bookkeeping for the "only notify on change" rule. Clear it to force the next check
  to notify again.
- **`npm_cmd`** — an override, only needed if `npm.cmd` is somewhere unusual. Left `null`, `PATH` is
  searched, then `%APPDATA%\npm\npm.cmd`.

If the file is not valid JSON it is moved to `npm-globals-tray.json.invalid`, defaults are used, and a
notification says so — your edits are never silently overwritten.

## Diagnostics

In `%LOCALAPPDATA%\npm-globals-tray\`: **`last-check.txt`** (every package from the most recent check with
its state — the file to look at when the menu shows something surprising), **`last-run.log`** (stdout and
stderr of the most recent *failed* update, reachable from *Open last log*), and **`app.ico`** (the
notification artwork).

## Removing it

Quit from the tray menu, delete the `.exe` and its `.json`, then delete `%LOCALAPPDATA%\npm-globals-tray\`.
Two `HKEY_CURRENT_USER` values are optional to clean:

- `Software\Microsoft\Windows\CurrentVersion\Run` → `npm-globals-tray` — only if *Run at startup* was ever
  enabled; toggling it off removes it.
- `Software\Classes\AppUserModelId\NpmGlobals.Tray` — created so notifications are attributed to
  "npm globals" rather than to PowerShell.

Nothing else is written. No installer, no service, no scheduled task.

## Development

| Module | Responsibility |
|---|---|
| `main.rs` | the `tao` event loop and nothing else |
| `app.rs` | wiring: dispatches menu actions, spawns workers, owns the frame counter |
| `check.rs` | the read path — what is installed, what is behind |
| `update.rs` | the write path — runs updates, announces progress per package |
| `notice.rs` | decides whether an update set is worth a notification, and what it should say |
| `progress.rs` | the asymptotic creep, the batch's overall level and the 8-cell bar |
| `registry.rs` | dist-tags over HTTP |
| `source/` | `PackageSource` trait plus the npm and bun adapters |
| `tray/` | `mod.rs` owns the tray handle; `menu.rs` builds the menu and every label |
| `icon/` | `render.rs` draws the states, `ico.rs` writes the container |
| `platform/` | the OS-specific arm, selected by `cfg` |
| `diagnostics.rs` | the two files written for troubleshooting |
| `config.rs`, `model.rs` | settings and the types everything else speaks in |

The split follows one rule: anything with branch-worthy logic lives where it can be tested without a
running Win32 tray. That is why `notice.rs` exists as its own module rather than as methods on `App` — the
"only notify when the set changed" rule has real edge cases and `App` cannot be constructed in a test.

The icon is a lowercase `n`, drawn from primitives in `src/icon/render.rs` — there are no image files,
and `build.rs` renders the same glyph into the `.exe` icon, so the two cannot drift apart. The state
is carried by colour: slate when everything is current, amber when updates are waiting, red when the
last check failed. While a job runs the letter becomes a vessel: a dim outline fills with water whose
surface ripples every 120 ms, at the level described above.

Every icon is supersampled 4× for antialiasing, at whatever size is asked for. The tray asks for 32 px
frames at runtime; `build.rs` renders 16/32/48/64/128 px into a real `.ico` (`src/icon/ico.rs` writes the
container by hand) for `winresource` to embed as the executable icon; the first notification writes that
same file next to the config.

Two things about the environment that the code has to work around, both verified rather than assumed:

- **`npm ls -g --json` exits non-zero when the global tree has problems** (an orphaned directory in
  `node_modules` is enough). The exit code is therefore ignored and stdout is parsed anyway; entries with
  no usable `version` are skipped instead of failing the whole listing.
- **`bun pm ls -g` does not list global packages.** It ignores `-g` and prints the tree for whatever
  directory it is run from, so it will happily report a project's dependencies as if they were global. The
  bun source reads `~/.bun/install/global/package.json` (honouring `BUN_INSTALL`) and resolves versions
  from that directory's `node_modules`. An empty global store reports zero packages, which is not an error.

### Tests

```
cargo test
```

118 tests, no network and no side effects. The update orchestration is exercised by pointing `npm_cmd` at a
file that cannot be executed, which drives the real failure paths without installing anything.

Five tests are `#[ignore]`d because they do touch the real system — the HKCU Run key, a real toast, a real
`npm install -g`, an icon dump. Each carries its own exact invocation in its `#[ignore = "…"]` message;
`grep -rn "#\[ignore" src/` lists them. `updates_a_package_for_real` really installs `$env:UPDATE_TARGET`
globally, so point it at something harmless. To see the working UI on demand, downgrade something
disposable (`npm i -g npm-check-updates@23.0.0`) and hit *Check now*.

### Lints

`cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are CI gates. `Cargo.toml` enables
`clippy::pedantic`, so `-D warnings` denies that set too. `src/icon/render.rs` and `src/icon/ico.rs` carry
a module-level `#![allow(…)]` for the cast lints — both are arithmetic on small bounded integers, pixel
coordinates and `.ico` header fields, where the casts are the work rather than an accident.
`clippy::nursery` is deliberately **not** enabled: its lints move between releases, and a toolchain bump
should not turn a green build red.

## CI and releases

Three workflows, all Windows-only, because that is the only platform arm this project implements.

- **`ci.yml`** — every push to `master`/`main` and every pull request: fmt, clippy, test, then a release
  build whose `.exe` is attached to the run as an artifact. The `#[ignore]`d tests never run here.
- **`release.yml`** — a `v*` tag only. It first refuses to continue if the tag does not match the `version`
  in `Cargo.toml`, so a `v0.2.0` tag on a `0.1.0` manifest fails instead of publishing a mislabelled build.
  Then it tests, builds, and creates the GitHub Release with the bare `.exe` and a `.sha256` next to it.
- **`release-plz.yml`** — every push to `master`. [release-plz](https://release-plz.dev) keeps a **release
  pull request** open containing the version bump and the new `CHANGELOG.md` entries; nothing is published
  while it sits there. Merging it is the decision to release: release-plz creates the `v*` tag, which
  triggers `release.yml`. `release-plz.toml` sets `git_release_enable = false` so the two do not race to
  create the same release — release-plz stops at the tag and hands over.

To cut a version, write [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) — `feat:`
minor, `fix:` patch, `feat!:` or a `BREAKING CHANGE:` footer major. The subject becomes the changelog line,
so write it for a reader. Push to `master`, then merge the release PR when the version is worth shipping.
Tagging by hand works too; `release.yml` only cares that a `v*` tag appeared and agrees with `Cargo.toml`.

**Settings → Actions → General → Workflow permissions → "Allow GitHub Actions to create and approve pull
requests"** must be enabled, or the `release-pr` job fails with a permissions error while everything else
keeps working.

## Other platforms

`src/platform/unix.rs` returns an error from every call it cannot honour; the rest of the code, and every
dependency, already works on all three OSes — `tao` was chosen over `winit` precisely because `tray-icon`
needs a **GTK** event loop on Linux, which `tao` provides. Finishing macOS or Linux means writing that one
file: `notify-rust` for notifications (not used on Windows because it offers no way to set the
AppUserModelID, which would leave every toast attributed to PowerShell), a `~/Library/LaunchAgents/*.plist`
or `~/.config/autostart/*.desktop` entry for autostart, and a lock file instead of a named mutex.

Two costs to know about first: on macOS, notifications require a signed `.app` bundle with an `Info.plist`,
so "portable, no install" does not survive the port. On Linux, `tray-icon` needs `libgtk-3-dev`,
`libxdo-dev` and `libayatana-appindicator3-dev` at build time, and GNOME shows no tray at all without the
AppIndicator extension.
