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
● npm globals — 3 updates
─────────────────────────────
↑  @google/gemini-cli   0.53.1 → 0.54.4
↑  @salesforce/cli      2.145.6 → 2.146.3
↑  vercel               58.4.4 → 58.9.1
─────────────────────────────
✓  prettier             3.9.6
✓  typescript           7.0.2
·  npm                  12.0.2      (ignored)
?  some-package         1.0.0       (not checked)
─────────────────────────────
Update all (3)
Check now
☑ Run at startup
Open last log
Quit
```

Clicking an `↑` row runs `npm install -g <name>@latest` (or `bun add -g …`) with no console window, then
re-checks. Packages from bun are suffixed ` (bun)` so a name installed in both places stays distinguishable.

The icon colour is the state: grey = up to date, amber with an arrow = updates available, blue = working,
red = the last check failed.

Markers: `↑` outdated, `✓` current, `·` ignored, `?` the registry did not answer for it. `?` is
deliberately **not** the same as `✓` — a network failure must never look like "everything is fine".

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
- **`last-run.log`** — stdout and stderr of the most recent *failed* update. Reachable from the menu via
  *Open last log*.

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

The tray icon is drawn pixel by pixel at runtime, so there are no image assets and no image-decoding
dependency. The exe itself has no custom icon yet — Explorer shows the default.

## Tests

```
cargo test
```

Three tests are `#[ignore]`d because they touch the real system. Run them deliberately:

```
cargo test -- --ignored --exact platform::windows::tests::autostart_round_trips_through_the_run_key
cargo test -- --ignored --exact check::tests::a_failed_update_is_recorded_in_the_log
$env:UPDATE_TARGET="npm-check-updates"; cargo test -- --ignored --exact check::tests::updates_a_package_for_real
```

The autostart test saves and restores whatever the Run key held before it ran. The last one really installs
`$UPDATE_TARGET@latest` globally — point it at something harmless.

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
