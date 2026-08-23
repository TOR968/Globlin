# Development

Contributor-facing detail that doesn't belong on the front page of [`README.md`](../README.md): the
module layout, the environment quirks the code works around, how tests and lints are gated, the release
pipeline, and the self-update mechanics in full.

## Architecture

| Module | Responsibility |
|---|---|
| `main.rs` | the `tao` event loop and nothing else |
| `app.rs` | wiring: dispatches menu actions, spawns workers, owns the frame counter |
| `check.rs` | the read path — what is installed, what is behind |
| `update.rs` | the write path — runs updates, announces progress per package |
| `notice.rs` | decides whether an update set is worth a notification, and what it should say |
| `progress.rs` | the asymptotic creep, the batch's overall level and the 8-cell bar |
| `registry.rs` | dist-tags over HTTP |
| `selfupdate.rs` | the app's own release lookup, download, verification and swap |
| `source/` | `PackageSource` trait plus the npm and bun adapters |
| `tray/` | `mod.rs` owns the tray handle; `menu.rs` builds the menu and every label |
| `icon/` | `render.rs` draws the states, `ico.rs` writes the container |
| `platform/` | the OS-specific arm, selected by `cfg` |
| `diagnostics.rs` | the two files written for troubleshooting |
| `config.rs`, `model.rs` | settings and the types everything else speaks in |

The split follows one rule: anything with branch-worthy logic lives where it can be tested without a
running Win32 tray. That is why `notice.rs` exists as its own module rather than as methods on `App` — the
"only notify when the set changed" rule has real edge cases and `App` cannot be constructed in a test.

### The icon

The icon is an uppercase `G` — a ring with a gap on the right and a crossbar at shelf height closing that
gap — drawn from primitives in `src/icon/render.rs`. There are no image files as sources; `build.rs`
renders the same glyph into the `.exe` icon, so the two cannot drift apart. A lowercase `g` was tried
first and rejected: at 16 px it read as the numeral `9` no matter how the descender hook was shaped,
because `g` and `9` are topologically the same closed-loop-plus-descender shape and only a curvature that
tightens through the turn — something the constant-radius `Arc` primitive cannot express — tells them
apart.

While a job runs the letter becomes a vessel: a dim outline fills with water whose surface ripples every
120 ms, at a level driven by the batch-share arithmetic in `progress.rs`.

Every icon is supersampled 4× for antialiasing, at whatever size is asked for. The tray asks for 32 px
frames at runtime; `build.rs` renders 16/32/48/64/128 px into a real `.ico` (`src/icon/ico.rs` writes the
container by hand) for `winresource` to embed as the executable icon; every notification rewrites that
same file next to the config, so a build whose glyph changed cannot leave a stale toast artwork behind —
Windows reads the toast icon from the `IconUri` on disk, not from the running process. A rewrite can fail
— the shell can be holding the file open while a toast from the previous run is still on screen — and that
failure is deliberately swallowed: whichever artwork is already on disk stays registered, rather than
dropping the notification to the unbranded PowerShell fallback app id.

**The README's logo and per-state images (`docs/img/*.png`) are generated, not hand-drawn.** They come
from the same `render::rgba` the tray and the `.ico` writer use, encoded to PNG by a hand-written encoder
in `src/icon/png.rs` (stored/uncompressed zlib — no external crate). Regenerate them after any change to
`src/icon/render.rs`:

```
cargo test -- --ignored --exact icon::tests::dump_readme_images
```

That writes `docs/img/logo.png` (128 px, idle state) and `docs/img/icon-{idle,updates,error,busy}.png`
(32 px each). Commit the results alongside the render change.

### Environment workarounds

Two things about the environment the code has to work around, both verified rather than assumed:

- **`npm ls -g --json` exits non-zero when the global tree has problems** (an orphaned directory in
  `node_modules` is enough). The exit code is therefore ignored and stdout is parsed anyway; entries with
  no usable `version` are skipped instead of failing the whole listing.
- **`bun pm ls -g` does not list global packages.** It ignores `-g` and prints the tree for whatever
  directory it is run from, so it will happily report a project's dependencies as if they were global. The
  bun source instead reads `~/.bun/install/global/package.json` (honouring `BUN_INSTALL`) and resolves
  versions from that directory's `node_modules`. An empty global store reports zero packages, which is not
  an error.

## Self-update mechanics

The full mechanics behind the brief version in the [README](../README.md#keeping-itself-updated).

On the same schedule as the package check, the app makes one extra request per cycle to
`https://api.github.com/repos/TOR968/globlin/releases/latest`. The two are deliberately independent:
`check::run` looks the release up first and returns a `Report` carrying `packages: Result<Vec<Package>>`,
so a missing npm, a `PATH` without bun, or an unreachable registry cannot stop the app from finding — and
installing — its own update. That endpoint always resolves to the newest *published* release; GitHub
excludes pre-releases and drafts from it, so neither can reach a user by accident. A release is offered
only when its tag parses as semver strictly newer than the running build, and both `globlin.exe` and
`globlin.exe.sha256` are attached — a release missing either asset is skipped rather than half-offered.

Applying an update downloads both assets, hashes the `.exe` with SHA-256, and compares that against the
published `.sha256`; a mismatch discards the download and reports a failure, and the running binary is
never touched. Windows will not let a running `.exe` be overwritten, but it will let it be *renamed*, so
the swap is: write the new build as `globlin.exe.new` next to the running one, rename the running exe to
`globlin.exe.old`, then rename `.new` into the live name. If that last rename fails, the `.old` build is
renamed straight back and the installation is left exactly as it was. In the rare case where that rollback
also fails, the live path is left empty — but nothing is lost: the previous build is still intact at
`globlin.exe.old`, and the freshly verified new build is still intact at `globlin.exe.new`, because the
staged file is only deleted after a failed swap when the live executable is still there to replace it.
Renaming either file back to `globlin.exe` recovers the app. On the next clean start both
`globlin.exe.old` and `globlin.exe.new` are deleted, so a successful update leaves nothing extra behind
and a crash between the write and the swap does not strand a multi-megabyte staged binary. If a swap
completes but the new build fails to start, `globlin.exe.old` — the previous working build — is still
sitting next to it and can be renamed back to `globlin.exe`; that is the only recovery path in that case.

None of this works if the process cannot write next to itself — an install under `Program Files` without
elevation, or a directory an antivirus is holding a handle into, are the two cases seen in practice. Both
fail the same way: the check keeps finding the release, the rename keeps failing, and the app keeps
running the old build.

Three things keep that from becoming a silent loop. Every failure is written to `self-update.log` next to
the other diagnostics — last one wins, the same shape as `last-run.log` — and the `Globlin v<current>`
submenu grows an **Open Globlin-update log** row as soon as that file exists. The failure is toasted once
per version, tracked in `last_self_notice` in `globlin.json`, rather than on every check; clicking
**Update Globlin** clears that stamp first, so a deliberate retry always reports its outcome. And the
failed version is remembered in `App.blocked_self`, which gates only the *automatic* path
(`selfupdate::should_auto_apply`), so auto-update stops re-downloading a build that cannot be installed.
That block is in memory, not in `globlin.json`: most failures are transient — a lock, a dropped
connection — so a restart is meant to retry, and a newer release lifts the block on its own.

The other half of that loop is a swap that works but a relaunch that does not. The process is still
running the old image, so the next check would offer — and reinstall — the release already sitting on
disk. The installed version is therefore recorded in `App.pending_restart`; `selfupdate::supersedes`
filters it out of later checks, `start_self_update` refuses to run, and the menu reads `Restart to finish
the update to <version>` instead of offering it again.

A successful swap restarts the app itself: it spawns the new `.exe` with `--replaced` and exits. The new
process waits up to 10 seconds (50 attempts, 200 ms apart) for the single-instance mutex the old process is
still holding while it shuts down, then raises a `Globlin — updated / now running <version>` toast.

## Tests

```
cargo test
```

183 tests: 175 run by default (no network, no side effects), 8 `#[ignore]`d because they touch the real
system — the HKCU Run key, a real toast, a real `npm install -g`, two icon/PNG dump tests, and two that
hit `TOR968/globlin`'s real GitHub releases. Each carries its own exact invocation in its
`#[ignore = "…"]` message; `grep -rn "#\[ignore" src/` lists them. `updates_a_package_for_real` really
installs `$env:UPDATE_TARGET` globally, so point it at something harmless. To see the working UI on
demand, downgrade something disposable (`npm i -g npm-check-updates@23.0.0`) and hit *Check now*.

The update orchestration is exercised by pointing `npm_cmd` at a file that cannot be executed, which
drives the real failure paths without installing anything.

## Lints

`cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are CI gates. `Cargo.toml` enables
`clippy::pedantic`, so `-D warnings` denies that set too. `src/icon/render.rs` and `src/icon/ico.rs` carry
a module-level `#![allow(…)]` for the cast lints — both are arithmetic on small bounded integers, pixel
coordinates and `.ico` header fields, where the casts are the work rather than an accident. No other
module widens that allow — `src/icon/png.rs`, for instance, uses `u32::try_from`/`u16::try_from` instead
of `as` throughout. `clippy::nursery` is deliberately **not** enabled: its lints move between releases,
and a toolchain bump should not turn a green build red.

## CI and releases

Three workflows, all Windows-only, because that is the only platform arm this project implements. That
includes `release-plz.yml`, which is not obvious: computing a version number sounds platform-independent,
but release-plz runs `cargo package` with verification to compare the packaged files against the last
release, and verification *compiles* the crate. `tao` and `tray-icon` sit in plain `[dependencies]`, so on
a Linux runner they pull the GTK stack and the build dies on `The system library glib-2.0 ... was not
found`, taking the whole job with it — `failed to determine next versions: run cargo package`. There is no
config switch for this: `publish_no_verify` only reaches `cargo publish`, not the packaging done while
determining versions.

- **`ci.yml`** — every push to `master`/`main` and every pull request: fmt, clippy, test, then a release
  build whose `.exe` is attached to the run as an artifact. The `#[ignore]`d tests never run here.
- **`release.yml`** — a `v*` tag only. It first refuses to continue if the tag does not match the
  `version` in `Cargo.toml`, so a `v0.2.0` tag on a `0.1.0` manifest fails instead of publishing a
  mislabelled build. Then it tests, builds, and creates the GitHub Release with the bare `.exe` and a
  `.sha256` next to it.
- **`release-plz.yml`** — every push to `master`. [release-plz](https://release-plz.dev) keeps a
  **release pull request** open containing the version bump and the new `CHANGELOG.md` entries; nothing
  is published while it sits there. Merging it is the decision to release: release-plz creates the `v*`
  tag, which triggers `release.yml`. `release-plz.toml` sets `git_release_enable = false` so the two do
  not race to create the same release — release-plz stops at the tag and hands over. It also sets
  `git_only = true`: globlin is not published to a cargo registry, and without that flag release-plz
  looks for the previous release on crates.io, logs `Package globlin@*.*.* not found`, treats every run
  as an initial release, and proposes the version already in `Cargo.toml`. The job stays green while
  proposing nothing, which is the confusing part — no error, no release PR, no release.

To cut a version, write [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) —
`feat:` minor, `fix:` patch, `feat!:` or a `BREAKING CHANGE:` footer major. The subject becomes the
changelog line, so write it for a reader. Push to `master`, then merge the release PR when the version is
worth shipping. Tagging by hand works too; `release.yml` only cares that a `v*` tag appeared and agrees
with `Cargo.toml`.

The two jobs are ordered — `release-pr` declares `needs: tag` — and that ordering is load-bearing. Run
without it, both start at the same second, `release-pr` checks the repository out before `tag` has pushed
the new `v*` tag, and so it still believes the previous release is the latest one. It then opens a second
release pull request proposing the version that was just released, whose diff against `master` is empty.
Harmless, but it appears after every release and has to be closed by hand.

### The release token

Both release-plz jobs read `secrets.RELEASE_PLZ_TOKEN`, a fine-grained PAT scoped to this repository with
**Contents: read/write** and **Pull requests: read/write**. The default `secrets.GITHUB_TOKEN` is not
enough, and the reason is easy to lose an afternoon to: **GitHub does not start a workflow run for an
event raised by `GITHUB_TOKEN`**, a deliberate guard against a workflow triggering itself. A `v*` tag
pushed by release-plz under that token therefore lands in the repository without `release.yml` ever
noticing, and the release never gets built. Releases up to v0.2.0 worked only because those tags were
pushed by hand from a developer account.

If the token expires or is removed, the symptom is exactly that: the tag appears, no `Release` run
starts. Re-pushing the tag from a personal account (`git push origin :refs/tags/vX.Y.Z` then
`git push origin refs/tags/vX.Y.Z`) rebuilds it without moving the commit, and is the manual escape
hatch.

**Settings → Actions → General → Workflow permissions → "Allow GitHub Actions to create and approve pull
requests"** must also be enabled, or the `release-pr` job cannot open the pull request.

## Other platforms

Windows only in practice. `src/platform/unix.rs` returns an error from every call it cannot honour; the
rest of the code, and every dependency, already works on all three OSes — `tao` was chosen over `winit`
precisely because `tray-icon` needs a **GTK** event loop on Linux, which `tao` provides. Finishing macOS
or Linux means writing that one file: `notify-rust` for notifications (not used on Windows because it
offers no way to set the AppUserModelID, which would leave every toast attributed to PowerShell), a
`~/Library/LaunchAgents/*.plist` or `~/.config/autostart/*.desktop` entry for autostart, and a lock file
instead of a named mutex.

Two costs to know about first: on macOS, notifications require a signed `.app` bundle with an
`Info.plist`, so "portable, no install" does not survive the port. On Linux, `tray-icon` needs
`libgtk-3-dev`, `libxdo-dev` and `libayatana-appindicator3-dev` at build time, and GNOME shows no tray at
all without the AppIndicator extension.
