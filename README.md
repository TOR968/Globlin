<div align="center">
  <img src="docs/img/logo.png" width="96" height="96" alt="Globlin logo">

  # Globlin

  A tray icon that watches your global npm and bun packages and tells you when one falls behind.

  [![CI](https://github.com/TOR968/globlin/actions/workflows/ci.yml/badge.svg)](https://github.com/TOR968/globlin/actions/workflows/ci.yml)
  [![Release](https://img.shields.io/github/v/release/TOR968/globlin)](https://github.com/TOR968/globlin/releases/latest)
</div>

**[Install](#install)** · **[Using it](#using-it)** · **[Icon states](#icon-states)** ·
**[Keeping itself updated](#keeping-itself-updated)** · **[More](#more)** · **[License](#license)**

## What it does

Global npm CLIs rot silently — nothing tells you `@salesforce/cli` is six versions behind until a
command breaks in a way that turns out to be "oh, I'm ancient." Globlin sits in the tray, checks the
registry on a schedule, and shows you exactly what's behind. Click a package to update it, or update
everything at once. It never touches anything on its own unless you turn that on.

Single portable `.exe`, ~1.7 MB, no installer, no runtime dependencies, Windows only.

## Install

1. Download `globlin.exe` from the [latest release](https://github.com/TOR968/globlin/releases/latest).
2. Run it. That's it — no installer, nothing to unzip.

It keeps its config next to itself, so it's portable: move the `.exe` anywhere and it keeps working.
Right-click the tray icon and tick **Run at startup** if you want it running every time you log in.

## Using it

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
─────────────────────────────────────
Globlin v0.2.3                        ▸  Update Globlin 0.2.3 → 0.3.0
                                         ☐ Auto-update Globlin
                                         Open Globlin-update log
─────────────────────────────────────
Quit
```

- `↑` outdated · `✓` current · `·` ignored · `?` the registry hasn't answered yet (never treated as "fine").
- Every row opens a submenu: **Update** runs `npm install -g <name>@latest` (or the bun equivalent) with
  no console window, then re-checks. **Ignore** removes it from checks and from `Update all` without
  touching the network — untick it and it's re-checked immediately.
- **Update all** updates every outdated package as a queue: the header shows a `[2/3]` counter, the
  active row spins with a progress bar, finished rows show `✓ done` or `✗ failed`.
- **Check now** re-checks immediately instead of waiting for the schedule (every 6 hours by default).
- A package name suffixed `(bun)` means it's the bun copy — the same name can be installed under both
  npm and bun and stay distinguishable.
- A desktop notification only fires when the set of outdated packages *changes*, so it won't nag about
  the same three packages every six hours.

## Icon states

The tray icon is drawn from code, not an image file, so its colour always matches what it's telling you:

| | State | Meaning |
|---|---|---|
| <img src="docs/img/icon-idle.png" width="24" height="24" alt="idle icon"> | **Idle** — emerald | everything installed is current |
| <img src="docs/img/icon-updates.png" width="24" height="24" alt="updates icon"> | **Updates** — amber | one or more packages are behind |
| <img src="docs/img/icon-error.png" width="24" height="24" alt="error icon"> | **Error** — red | the last registry check failed |
| <img src="docs/img/icon-busy.png" width="24" height="24" alt="busy icon"> | **Busy** — sky blue | a check or an update is running, filling like a glass |

## Keeping itself updated

Globlin also checks its own [GitHub releases](https://github.com/TOR968/globlin/releases) on the same
schedule as the package check. The `Globlin v<current>` submenu names the build that is running, so you
can always tell which one is live. When a newer one is published the submenu grows an
`Update Globlin <current> → <new>` row; ticking **Auto-update Globlin** installs it unattended the next
time one is found, no click needed.

Before installing anything, the download is verified against a published SHA-256 checksum — a mismatch
is discarded and the running app is never touched. The one real failure mode: if Globlin can't write next
to itself (installed under `Program Files` without elevation, or a directory an antivirus is holding
open), the install fails. You get one toast about it, not a repeat nag; auto-update stops retrying that
build rather than re-downloading it every few hours, and the reason lands in a log you can open from the
menu. The row stays, so you can still retry by hand. See
[`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md#self-update-mechanics) for exactly how the swap, rollback and
restart work.

## More

<details>
<summary><strong>Configuration</strong> — <code>globlin.json</code></summary>

<br>

Read from next to the exe first, otherwise from `%LOCALAPPDATA%\globlin\`. Written back to whichever of
those accepts a write. Missing keys take their defaults, so a partial file is fine. If the file isn't
valid JSON, it's moved to `globlin.json.invalid`, defaults are used, and a notification says so — your
edits are never silently overwritten.

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
- **`last_notified`** — bookkeeping for the "only notify on change" rule. Clear it to force the next
  check to notify again.
- **`npm_cmd`** — an override, only needed if `npm.cmd` is somewhere unusual. Left `null`, `PATH` is
  searched, then `%APPDATA%\npm\npm.cmd`.
- **`auto_update`** — see [Keeping itself updated](#keeping-itself-updated).
- **`last_self_notice`** — the version string of the last self-update failure a toast was already raised
  for, so retrying the same failing version doesn't notify twice.

</details>

<details>
<summary><strong>Diagnostics &amp; removing it</strong></summary>

<br>

In `%LOCALAPPDATA%\globlin\`: **`last-check.txt`** (every package from the most recent check with its
state — look here first when the menu shows something surprising), **`last-run.log`** (stdout/stderr of
the most recent *failed* package update, reachable from *Open last log*), **`self-update.log`** (the
error from the most recent *failed* self-update lookup, kept separate so an offline run doesn't overwrite
the log *Open last log* reads), and **`app.ico`** (the notification artwork).

To remove Globlin: quit from the tray menu, delete the `.exe` and its `.json`, then delete
`%LOCALAPPDATA%\globlin\`. If a self-update was interrupted, `globlin.exe.old` may still be sitting next
to the `.exe` — safe to delete too. Two optional `HKEY_CURRENT_USER` cleanups:

- `Software\Microsoft\Windows\CurrentVersion\Run` → `globlin`, only if *Run at startup* was ever enabled.
- `Software\Classes\AppUserModelId\Globlin.Tray`, created so notifications are attributed to "Globlin"
  rather than to PowerShell.

Nothing else is written — no installer, no service, no scheduled task.

**Upgrading from `npm-globals-tray`:** this app was renamed from `npm-globals-tray` to Globlin. The new
build starts fresh with default settings, leaving three harmless leftovers you can delete once you've
switched to `globlin.exe`: the old config file (`npm-globals-tray.json`), the old data directory
(`%LOCALAPPDATA%\npm-globals-tray\`), and the old `Run` key value named `npm-globals-tray`.

</details>

<details>
<summary><strong>Building from source</strong></summary>

<br>

Requires a Rust toolchain (MSVC on Windows) plus Microsoft C++ Build Tools and the Windows SDK.

```
cargo build --release
```

`target/release/globlin.exe` is portable — copy it anywhere and run it; it keeps its config next to
itself.

</details>

For architecture notes, the verified environment workarounds, tests, lints, and the CI/release pipeline,
see [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md).

## License

Copyright (C) 2026 TOR968.

Globlin is free software: you can redistribute it and/or modify it under the terms of the GNU General
Public License as published by the Free Software Foundation, either version 3 of the License, or (at your
option) any later version.

It is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY — without even the implied
warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the [GNU General Public
License](LICENSE) for details, or <https://www.gnu.org/licenses/>.
