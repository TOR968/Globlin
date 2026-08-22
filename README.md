# Globlin

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

`target/release/globlin.exe` (~1.5 MB) is portable — copy it anywhere and run it; it keeps its
config next to itself.

## The menu

Right-click the tray icon:

```
Globlin — 3 updates available
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
Ticking `Ignore` writes the name into the `ignore` list in `globlin.json` and drops the
package out of `Update all` immediately, without touching the network. Unticking it starts a check
right away, because the registry was never asked about an ignored package — until that check lands the
row shows `?`, not `✓`. The list is keyed by name alone, so ignoring `typescript` silences it under
both npm and bun.

While an update runs, the batch is shown as a queue: finished packages carry `✓ done` (or `✗ failed`),
the package being worked on carries a spinner and a progress bar, and the rest carry `· queued`.
`npm install -g` reports progress only to a terminal, and the app runs it without a console, so the
bar is not a byte count: it rises asymptotically with how long the current package has been running,
and deliberately stops short of its last cell while the work is still in flight — the batch-share
arithmetic that combines elapsed time with `done`/`total` instead drives the tray icon's water level, not
this bar. The bar runs on a slower curve than the icon's water: the icon can be snapped forward the moment
a package lands, but the bar has nothing else to show while a package is running, so it needs to keep
visibly creeping across installs that take tens of seconds. Its last cell is reserved for the same reason
across both curves — a running package can never look finished. A package is only ever reported as landed
by its marker changing to `✓ done` (or `✗ failed`), never by the bar filling.

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

`globlin.json`, read from next to the exe first, otherwise from
`%LOCALAPPDATA%\globlin\`. Written back to the first of those that accepts a write. Missing keys
take their defaults, so a partial file is fine.

```json
{
  "check_interval_hours": 6,
  "sources": { "npm": true, "bun": true },
  "ignore": ["npm", "@anthropic-ai/claude-code"],
  "last_notified": [],
  "npm_cmd": null,
  "auto_update": false,
  "last_self_notice": null
}
```

- **`ignore`** — listed packages still appear in the menu with their version, but are never checked,
  flagged or notified about. `npm` is ignored by default because updating it on Windows means it rewrites
  its own shim mid-run; `@anthropic-ai/claude-code` because it self-updates. An explicit `[]` is respected.
- **`last_notified`** — bookkeeping for the "only notify on change" rule. Clear it to force the next check
  to notify again.
- **`npm_cmd`** — an override, only needed if `npm.cmd` is somewhere unusual. Left `null`, `PATH` is
  searched, then `%APPDATA%\npm\npm.cmd`.
- **`auto_update`** — when `true`, a newer published build of the app itself is installed and the app
  restarts on the next check that finds one, with no click needed. See [Updating itself](#updating-itself).
- **`last_self_notice`** — the version string of the last self-update failure a toast was already raised
  for, so retrying the same failing version does not notify twice; a differently-versioned release still
  toasts on its own first failure.

If the file is not valid JSON it is moved to `globlin.json.invalid`, defaults are used, and a
notification says so — your edits are never silently overwritten.

## Updating itself

On the same schedule as the package check — startup, then every `check_interval_hours` — the app makes one
extra request per cycle to `https://api.github.com/repos/TOR968/globlin/releases/latest`. That
endpoint always resolves to the newest *published* release; GitHub excludes pre-releases and drafts from
it, so neither one can reach a user by accident.

A release is offered only when its tag parses as semver strictly newer than the running build, and both
`globlin.exe` and `globlin.exe.sha256` are attached to it — a release missing either
asset is skipped rather than half-offered. When one is found, the menu grows an
`Update Globlin <current> → <new>` row next to `Update all`; the row disappears once the running
build catches up. Ticking `Auto-update this app` (config key `auto_update`, default `false`, right below
*Run at startup*) does the same job unattended: the next check that finds a newer release installs it
without a click. The job runs under its own `Activity`, so it cannot start while a package update, or
another self-update, is already in flight, and vice versa.

Applying an update downloads both assets, hashes the `.exe` with SHA-256, and compares that against the
published `.sha256`; a mismatch discards the download and reports a failure, and the running binary is
never touched. Windows will not let a running `.exe` be overwritten, but it will let it be *renamed*, so
the swap is: write the new build as `globlin.exe.new` next to the running one, rename the running
exe to `globlin.exe.old`, then rename `.new` into the live name. If that last rename fails, the
`.old` build is renamed straight back and the installation is left exactly as it was. In the rare case
where that rollback also fails, the live path is left empty — but nothing is lost: the previous build is
still intact at `globlin.exe.old`, and the freshly verified new build is still intact at
`globlin.exe.new`, because the staged file is only deleted after a failed swap when the live
executable is still there to replace it. Renaming either file back to `globlin.exe` recovers the
app. On the next clean start, `globlin.exe.old` is deleted, so a successful update leaves nothing
extra behind. If a swap completes but the new build fails to start, `globlin.exe.old` — the
previous working build — is still sitting next to it and can be renamed back to `globlin.exe`;
that is the only recovery path in that case.

None of this works if the process cannot write next to itself — an install under `Program Files` without
elevation, or a directory an antivirus is holding a handle into, are the two cases seen in practice. Both
fail the same way: the check keeps finding the release, the rename keeps failing, and the app keeps running
the old build. The failure is toasted once per version — tracked in `last_self_notice`, so a restart on the
same still-failing version does not toast again — rather than on every check.

A successful swap restarts the app itself: it spawns the new `.exe` with `--replaced` and exits. The new
process waits up to 10 seconds (50 attempts, 200 ms apart) for the single-instance mutex the old process is
still holding while it shuts down, then raises a `Globlin — updated / now running <version>` toast.

## Diagnostics

In `%LOCALAPPDATA%\globlin\`: **`last-check.txt`** (every package from the most recent check with
its state — the file to look at when the menu shows something surprising), **`last-run.log`** (stdout and
stderr of the most recent *failed* update, reachable from *Open last log*), **`self-update.log`** (the
error from the most recent *failed* self-update release lookup — a separate file so a GitHub outage or an
offline run cannot overwrite the package update log that *Open last log* reads), and **`app.ico`** (the
notification artwork).

## Removing it

Quit from the tray menu, delete the `.exe` and its `.json`, then delete `%LOCALAPPDATA%\globlin\`.
If a self-update was interrupted before its next clean start, `globlin.exe.old` may still be
sitting next to the `.exe` — see [Updating itself](#updating-itself) — and is safe to delete too.
Two `HKEY_CURRENT_USER` values are optional to clean:

- `Software\Microsoft\Windows\CurrentVersion\Run` → `globlin` — only if *Run at startup* was ever
  enabled; toggling it off removes it.
- `Software\Classes\AppUserModelId\Globlin.Tray` — created so notifications are attributed to
  "Globlin" rather than to PowerShell.

Nothing else is written. No installer, no service, no scheduled task.

### Upgrading from npm-globals-tray

This app was renamed from `npm-globals-tray` to Globlin. The new build does not migrate anything — it
starts fresh with default settings — so an existing install leaves three harmless leftovers behind that
can be deleted by hand once you have switched to `globlin.exe`:

- the old config file, `npm-globals-tray.json`
- the old data directory, `%LOCALAPPDATA%\npm-globals-tray\`
- the old `Software\Microsoft\Windows\CurrentVersion\Run` value named `npm-globals-tray`, if *Run at
  startup* was ever enabled

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
and `build.rs` renders the same glyph into the `.exe` icon, so the two cannot drift apart. The state is
carried by colour:

| State | Colour | Meaning |
| --- | --- | --- |
| `Idle` | emerald `#10b981` | everything installed is current — nothing to do |
| `Updates` | amber `#f59e0b` | one or more packages are behind |
| `Error` | red `#ef4444` | the last registry check failed |
| `Busy` | sky `#38bdf8` | a check or an update is running |

While a job runs the letter becomes a vessel: a dim outline fills with water whose surface ripples every
120 ms, at the level described above.

Every icon is supersampled 4× for antialiasing, at whatever size is asked for. The tray asks for 32 px
frames at runtime; `build.rs` renders 16/32/48/64/128 px into a real `.ico` (`src/icon/ico.rs` writes the
container by hand) for `winresource` to embed as the executable icon; every notification rewrites that
same file next to the config, so a build whose glyph changed cannot leave a stale toast artwork behind —
Windows reads the toast icon from the `IconUri` on disk, not from the running process. A rewrite can fail
— the shell can be holding the file open while a toast from the previous run is still on screen — and
that failure is deliberately swallowed: whichever artwork is already on disk stays registered, rather than
dropping the notification to the unbranded PowerShell fallback app id.

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

156 tests, no network and no side effects. The update orchestration is exercised by pointing `npm_cmd` at a
file that cannot be executed, which drives the real failure paths without installing anything.

Seven tests are `#[ignore]`d because they do touch the real system — the HKCU Run key, a real toast, a real
`npm install -g`, an icon dump, and two that hit `TOR968/globlin`'s real GitHub releases. Each
carries its own exact invocation in its `#[ignore = "…"]` message; `grep -rn "#\[ignore" src/` lists them.
`updates_a_package_for_real` really installs `$env:UPDATE_TARGET`
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
