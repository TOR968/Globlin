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
| `registry.rs` | every remote catalog: npm dist-tags, PyPI feeds, crates.io, NuGet and the PowerShell Gallery |
| `selfupdate.rs` | the app's own release lookup, download, verification and swap |
| `source/` | `PackageSource` trait plus one adapter per package manager, fourteen in all |
| `tray/` | `mod.rs` owns the tray handle; `menu.rs` builds the short menu, every label and every action id |
| `window/` | the package list: `window.rs` builds the snapshot, `shell.rs` owns the WebView2 window, `ui.html` is the page |
| `icon/` | `render.rs` draws the states, `ico.rs` writes the container |
| `platform/` | the OS-specific arm, selected by `cfg` |
| `diagnostics.rs` | the two files written for troubleshooting |
| `config.rs`, `model.rs` | settings and the types everything else speaks in |

The split follows one rule: anything with branch-worthy logic lives where it can be tested without a
running Win32 tray. That is why `notice.rs` exists as its own module rather than as methods on `App` — the
"only notify when the set changed" rule has real edge cases and `App` cannot be constructed in a test.
The same rule is why `window.rs` builds a `Snapshot` of plain serde structs and `window/ui.html` only
draws it: every decision the window makes about *what* a row is lands in a unit test, and the untestable
part — creating a WebView2 — is confined to `window/shell.rs`.

### Seven catalogs

`SourceKind::catalog` is the fork in the read path. It answers one question per source: *where does the
version to compare against come from, and what does its absence mean?*

| Catalog | Sources | `latest` comes from | Compared with |
|---|---|---|---|
| `Npm` | npm, bun, pnpm, yarn | `registry::npm_latest`, dist-tags per name | `semver` |
| `Crates` | cargo | `registry::crates_latest`, one `?ids[]=` request per 100 crates | `semver` |
| `PyPi` | pipx | `registry::pypi_latest`, the RSS release feed | `registry::numeric_is_newer` |
| `NuGet` | dotnet | `registry::nuget_latest`, the flat-container `index.json` | `registry::numeric_is_newer` |
| `PsGallery` | psgallery | `registry::psgallery_latest`, the package endpoint's redirect | `registry::numeric_is_newer` |
| `SelfResolved` | uv, scoop, go, gem, brew | the source fills `Installed.available` | `registry::is_newer`, falling back to inequality |
| `SelfReported` | winget, choco | the source fills `Installed.available` | presence |

The first five are remote: the source reports only what is installed, and `check.rs` asks the registry
for the union of names in that catalog. A name the registry did not answer for is `Unknown`, never
`Current`. `check.rs` must never send a winget id to registry.npmjs.org — `Git.Git` is not a package there.

**An unreachable registry is a skipped source, not a failed check.** Each `registry::*_latest` returns
`Err("<host> could not be reached")` only when it was asked about at least one name and *no* request got
any HTTP answer at all — a 404 counts as an answer, because a private package that npmjs has never heard
of must stay a quiet `Unknown`, not an alarm. `check.rs` moves that error into `Report.skipped` beside
the sources that could not be listed, so the icon turns red and one notification names the host, while
every other catalog's packages are still shown. Before this, `npm_latest` alone could fail the whole
check: one private npm global, or a blocked registry.npmjs.org, hid pipx, scoop, winget and the rest.

**Every child process has a deadline.** `process::output_within` runs a command with piped output and
kills it — on Windows with `taskkill /T`, so the `node` behind a `.cmd` shim goes too — once
`LISTING_LIMIT` (two minutes, for listings and `npm config`) or `CHANGE_LIMIT` (thirty minutes, for
updates and uninstalls, because `cargo install` compiles) runs out. `Activity` is the global busy lock and
only a worker's terminal message clears it, so one hung child used to disable Check, Update and Uninstall
until Globlin was restarted. The go source also sets `GIT_TERMINAL_PROMPT=0` and `GCM_INTERACTIVE=never`,
so `go list -m <private module>@latest` fails at once instead of waiting on a credential prompt.

**A failed listing is an error, not an empty source.** `source::listed` turns a listing whose command
exited non-zero *and* produced no rows into `"<command> listed nothing and <status>: <stderr>"`, which
`check.rs` reports as skipped. Rows from a command that exited badly are kept — `npm ls -g` and
`winget list` both exit non-zero on a usable reply.

**The last two differ only in what `None` means.** For `SelfReported`, `available: None` means the tool
looked and found no upgrade — `winget list` simply has no `Available` cell for that row. For
`SelfResolved`, the source always fills `available` with the version it compared against, the installed
one included, so `None` can only mean it could not find out — and that reads `Unknown`. uv and gem are
here rather than beside winget because `uv tool list --outdated` and `gem outdated` list *only* the
outdated packages: a failed run and an up-to-date machine would otherwise print the same empty report.
Both sources therefore run the plain listing and the outdated report separately, and a non-zero exit
from the report leaves every package unresolved. scoop resolves against the local bucket manifest and go
against `go list -m <module>@latest`; a missing manifest or an offline proxy is `None` for the same
reason. A resolved version that differs from the installed one is `Outdated` only when it is not older:
`registry::is_newer` compares as semver (a leading `v` stripped) and then numerically, and an install
that is ahead — a go pseudo-version built from `master`, an `-rc` — reads `Current`, because **Update**
would otherwise be a downgrade. Versions neither comparison understands (`7.1_1`, a cask's `latest`)
still read `Outdated` when they differ, since the tool itself reported them.

**A package installed from somewhere else is never looked up or updated.** `Installed.external` marks
an npm-family package that did not come from the registry Globlin would compare it against, and
`check.rs` leaves it out of the registry request and reads it as `Unknown`, which also means it never
gets an **Update** button. npm reads `<npm root -g>/.package-lock.json`: an entry marked `link`, or one
whose `resolved` URL is neither `https://registry.npmjs.org/` nor the configured `npm config get
registry`, is external (a scoped private registry, a git URL, a tarball, `npm link`, `npm i -g ./dir`);
with no lockfile entry, a symlink in the global `node_modules` is. bun and yarn read the spec in their
global `package.json` — anything containing `:` or `/` (`file:`, `link:`, `github:`, `npm:` aliases,
URLs, `user/repo`) is external. pnpm already drops such entries, because their version is not semver.
Before this, a public package squatting the name of a private or linked tool at a higher version showed
as an update, and one click installed it and ran its install scripts. Package names are also
percent-encoded into every registry URL, so a name cannot change the request's path or query, and the
PowerShell Gallery source refuses to build a command for a module name with anything but letters,
digits, `.`, `_` and `-`. `diagnostics::record_failures` runs a failed command's output through
`redacted` before it reaches `last-run.log`: the rest of a line after `_authToken=`, `_auth=`,
`_password=` or `Authorization:` and any `npm_…`, `ghp_…` or `github_pat_…` token of 30+ characters
become `<redacted>`, because a verbose package manager can echo its credentials into stderr.

**Why each remote endpoint is the one it is.** Every choice here is the smallest reply that still names
the newest stable release, because the check runs every six hours in the background.

- **PyPI**: `/rss/project/<name>/releases.xml`, about 10 KB, taking the newest entry that is a plain run
  of numbers. The JSON API is the obvious choice and the wrong one: `/pypi/ruff/json` is roughly 6 MB.
- **crates.io**: `/api/v1/crates?per_page=100&ids[]=a&ids[]=b`, about 1 KB per crate for *all* crates in
  one request, returning `max_stable_version`. Two traps: the default page size is 10, so `per_page`
  is not optional, and a request without a `User-Agent` is refused with 403, which is why the shared
  agent sends `globlin/<version> (+https://github.com/TOR968/Globlin)`. A single-crate
  `/api/v1/crates/<name>` is 90–440 KB, and `?include=` shrinks it but empties `max_stable_version`.
- **NuGet**: `https://api.nuget.org/v3-flatcontainer/<id>/index.json`, a few hundred bytes, sorted
  oldest first, so it is read from the end. The id must be lowercased; the mixed-case URL is a 404.
- **PowerShell Gallery**: there is no small JSON — the v3 flat container answers 403 and the OData feed
  is 48 KB of XML that ignores its own `IsLatestVersion` filter. But `/api/v2/package/<id>` answers 302
  to `…/packages/<id>.<version>.nupkg`, so `registry::gallery_newest_release` sends one request with
  `max_redirects(0)` and reads the version out of the `Location` header. No body is ever downloaded.

`numeric_is_newer` compares dot-separated segments as numbers, padding the shorter side with zeros so
`1.0` and `1.0.0` are equal in both directions, and answers `None` when either side has a non-numeric
segment. PEP 440 and NuGet versions are not semver (`2025.9.5`, `1.2.3.4`), and a prerelease
or post-release is reported as `Unknown` rather than guessed at.

That fork is also why package versions are `String` in `model.rs` rather than `semver::Version`. winget
and choco emit `1.2.3.4` and `2024.01.15`; neither parses as semver, and both must still render.

### Elevation: winget and choco

Every source updates and uninstalls; there is no read-only source any more. Earlier versions kept winget
and choco read-only (`SourceKind::read_only`, `removable`), because both can need an administrator and
Globlin runs unelevated. They now go through the same `update_command`/`uninstall_command` path as the
rest, and the elevation is left to Windows' own prompt:

- **winget runs as the user.** `winget upgrade|uninstall --id <id> --exact --silent
  --disable-interactivity --accept-source-agreements` (plus `--accept-package-agreements` on upgrade). A
  per-user package changes with no prompt; a machine-scope installer raises its own UAC prompt, which
  appears on the secure desktop even though winget's window is hidden. `--exact` matters: without it
  `--id Git.Git` also matches `Git.Git.Beta`-style ids and winget stops to ask which one. The id is passed
  as a plain argument, never through a shell, so the `ARP\Machine\X64\{GUID}` ids winget invents for
  registry-only entries need no escaping and uninstall like any other.
- **choco always needs an administrator**, so `Choco::elevated` wraps it in Windows PowerShell:
  `Start-Process -FilePath <choco> -ArgumentList '<verb>', '<name>', '-y', '--no-progress' -Verb RunAs
  -WindowStyle Hidden -Wait -PassThru`, one UAC prompt per package. `$ErrorActionPreference = 'Stop'` is
  what makes a declined prompt fail: without it `Start-Process` writes a non-terminating error, `$run` is
  `$null`, and the script exits 0 — a refusal that reads as a success. Exit codes 1641 and 3010 (a reboot
  is pending) count as success; any other non-zero code exits through with a pointer to choco's own log,
  because an elevated process's output cannot be captured by an unelevated parent. The package name is
  pasted into the script, so it must pass `source::plain_name` (`[A-Za-z0-9._-]`, the same rule
  psgallery uses) or no command is built at all.
- **One prompt per package**, so the queue's per-row progress keeps working for choco. A
  batch of five choco packages shows five prompts; saying *No* fails that one package and the queue moves
  on.
- **The window says so before the prompt appears.** `window::elevation` gives winget and choco rows and
  their Settings switches an `Elevation { label, detail }` — *may ask for admin* for winget, *asks for
  admin* for choco. `ui.html` draws the label
  as a tag beside the package name (outdated and current rows alike, since **Uninstall** prompts too),
  appends it to the group's note, and puts it beside the source's switch, with the detail as the
  tooltip. The tag sits where the old `read-only` tag was, so the text is decided in Rust and tested
  there, and the page only draws it.
- **Globlin never auto-approves the prompt.** An unelevated process that can skip UAC is a UAC bypass
  any other program running as the user could reuse. The only ways to avoid the prompts are the user's:
  run Globlin itself elevated, or change the system's UAC policy.
- **Globlin's own winget row is dropped.** `winget list` shows the Inno Setup install as
  `ARP\User\X64\{380BD341-…}_is1`, named `Globlin`; uninstalling that from inside the running app would
  pull the binary out from under it, and a winget-owned Globlin is upgraded by winget outside the app.
  `winget::to_installed` skips any row whose display name is exactly `Globlin`.

go is still the one source without an uninstall command: `Go::uninstall_command` is `None`, and `Go`
implements `PackageSource::remove_directly`, which `remove::apply` tries before the command path. It finds
the binary with `go version -m <bin dir>` (the untagged header line of each block carries the file path),
matches it by package path and deletes that file, refusing any file whose parent is not the Go bin
directory.

### The window

`tao` and `wry` are the same project, so the window rides the existing event loop: `App::open_window`
builds it lazily from the `EventLoopWindowTarget` the loop hands to its own closure, and closing it hides
rather than destroys, because re-creating a WebView2 is visibly slow. State flows one way. `App` renders
`window.globlin.render(<json>)` through `evaluate_script`; the page posts an id string back over
`with_ipc_handler`, which becomes `Message::Ipc` on the same queue menu clicks use, and is parsed by the
same `Action::from_key`. There is one action vocabulary for both surfaces; the only id the tray never
builds is `update-many:npm:a|pnpm:b`, the bulk form.

The window sets both of its icons from `icon::image`: the small one (`with_window_icon`, the title bar)
and, on Windows, the big one (`with_taskbar_icon`, the taskbar button and Alt-Tab). tao's window icon only
fills the small slot; without the big one Windows draws the taskbar button from the `.exe`'s icon through
its shell icon cache, which an in-place upgrade does not refresh — after 0.4.0 the title bar showed the
goblin while the taskbar kept the old `G`.

Animation is the one place that does not re-render: at 8 fps a full rebuild of a 200-row list is
noticeable, so `App::advance_animation` sends `window.globlin.tick(...)` with just the headline and the
batch rows, mirroring the `Tray::render` / `Tray::animate` split exactly. The page goes one step further:
a tick only rebuilds the hero card's queue when a batch row's state changed since the last draw, and
otherwise rewrites the title, the progress bar and the busy flags. The spinner on the running row is CSS,
so it turns without any tick at all. Batch rows reach the page as `name`, `source`, `from`, `to` and
`state` rather than the tray's preformatted line, because the page groups the queue by source.

`src/window/ui.html` is one file with no build step, no bundler and no CDN — it is `include_str!`d into
the binary. Everything it needs arrives in the snapshot, already in display order: `window.rs` puts enabled sources
that found packages first (most outdated, then most packages), empty ones next and switched-off ones
last, and orders the package list by that same source order with outdated packages first inside each
source. The page owns filtering, search, which groups and tiles are open, the switch between the
package view and Settings, and the two-click uninstall confirmation, and nothing else. Its layout puts
updates first: a hero card with the headline and **Update all**, the outdated packages grouped by source
(five rows per group until *Show N more*), then one tile per source for everything that is not outdated.
Settings shows only what the snapshot carries; there is no interval picker because `Snapshot` has no
interval.

Switching a source on or off, and un-ignoring a package, does not start a check. It applies locally at
once (a switched-off source's rows are dropped) and moves `next_check` to `now + SETTLE_DELAY` (2 s), so
every further click pushes it out again and a run of toggles costs one check, not one per click. The same
move covers a toggle made while a job is already running: the old code called `start_check`, which returned
early on a busy `activity` and left the new source unchecked until the next scheduled interval. Because
such an in-flight check was started with the old `Config`, `on_checked` drops rows from sources the
current config has switched off.

When an update finishes, `update::mark_updated` sets every package in `Outcome.updated` to `Current` at its
target version before the follow-up check starts. Without it the window spent that check showing the
same `old → new` rows under *Checking for updates*, which read as if nothing had been updated. The check
still runs and corrects any package whose manager installed something other than the target.

**The page is the only document the view will ever show.** `window::is_own_page` accepts `about:blank`
and `data:text/html…` — what WebView2 reports for `with_html` (navigation is the `data:` URL, IPC
arrives from `about:blank`; WKWebView and WebKitGTK report `about:blank`). The navigation handler
cancels everything else, new windows are denied, file and link drops are swallowed by
`with_drag_drop_handler`, and the IPC handler drops any message whose origin is not the page. Without
these a link or an `.html` file dragged onto the window became a page with `window.ipc` that could post
`auto-approve`, `update-all` or `remove:` ids. The no-comments rule covers it: no `<!-- -->`, no
`/* */`, no `//`.

That guard is also why the page has no `<a href>` to anything outside itself. *Support on Patreon* — in
the footer, under Settings › About and in the tray menu — posts the `open-patreon` id, and `App` hands
the fixed `PATREON_URL` in `app.rs` to `platform::open_in_shell` (`explorer`, `open` or `xdg-open`,
which pass a URL to the default browser). The page never names the URL, so no message it posts can make
Globlin open an address of its choosing.

### The icon

The icon is a chibi goblin — a play on the name — drawn from primitives (`Disc`, `Ellipse`, `Segment`,
`Polygon`, `Grown`) in `src/icon/render.rs`: a round head flattened from four quadratic curves, two
pointed ears, big dark eyes with white glints and an up-chevron mouth, the head in the state colour. It
has three levels of detail, picked by `Detail::for_size`:

- **Sticker** (48 px and up — the `.exe`/app icon, the macOS iconset, the Linux hicolor icons, the README
  and the site): a white outline (`Shape::Grown`, the silhouette dilated by 1.25 units) and two gold
  sparkles, like a laptop sticker.
- **Tray** (21–47 px, which is what the 32 px tray frames are): no outline or sparkles, smaller eyes, the
  mouth kept.
- **Tiny** (20 px and below): only the head, ears and big eyes — a 1-px mouth only blurs.

Error adds a sweat drop and greys the sparkles. There are no image files as sources; `build.rs` renders
the same drawing into the `.exe` icon, so the two cannot drift apart. It replaced an uppercase `G` in
2026-10, chosen on a design canvas from four directions, six goblin variations and six anime styles; the
coordinates are the canvas's 64-unit SVG halved into the 32-unit design space.

While a job runs the goblin becomes a vessel: a dim silhouette whose head fills with water — the ears
stay out of the fill so they cannot leave a ghost behind when they move — rippling every 120 ms at a
level driven by the batch-share arithmetic in `progress.rs`, while the ears twitch (rotated about their
base by up to 0.2 rad), the mouth hops and, at sticker size, the sparkles orbit the face. The other states
are still.

**The monochrome look is a second drawing of the same shapes, not a recolour.** `render::mono` paints
the silhouette in a single ink and *erases* the eyes and mouth (`Paint::Erase`, a destination-out
composite), so the face is a hole the tray shows through; the glints go back in as ink, so the goblin still
looks up the way the colour one does. With no colour left to say which state it is in, the state moves into
shape: Updates adds a badge disc at the lower right, set apart from the head by an erased 1.5-unit gap;
Error cuts an exclamation mark into that badge; Busy draws the silhouette at 35 % alpha and fills the head
with the same wave clip at full alpha. Every opaque pixel is exactly the ink colour, so on macOS the image
works as a template (`with_icon_as_template`/`set_icon_as_template`) and AppKit tints it to the menu bar.
Elsewhere the ink is chosen by `icon::look`: near-black when `platform::tray_is_light()` says the tray is
light — on Windows the `SystemUsesLightTheme` value under `HKCU\…\Themes\Personalize`, which is the
taskbar's half of the theme setting — white otherwise, and always white on Linux, whose panels are dark by
default. `Tray::render` re-reads it, `Tray::animate` reuses what render read, so a theme switch shows on
the next full render (a state change or a check), not mid-animation. The switch is `monochrome_icon` in
the config and the `monochrome-icon` id; it touches only the tray — the `.exe`, the toast and the window
keep the colour goblin.

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
in `src/icon/png.rs`. Each row is filtered — Sub for the first row, Up for every row after it — before the
encoder packs the result into a single fixed-Huffman DEFLATE block whose only match distance is 1, which
is exactly what a run of repeated bytes from those filters looks like, so it comes out as run-length
encoding in practice. It stays dependency-free and `#[cfg(test)]`-only; that filtering and compression is
what took the 1200×630 Open Graph card from about 3 MB to about 50 KB. Regenerate them after any change to
`src/icon/render.rs`:

```
cargo test -- --ignored --exact icon::tests::dump_readme_images
```

That writes `docs/img/logo.png` (128 px, idle state) and `docs/img/icon-{idle,updates,error,busy}.png`
(32 px each). Commit the results alongside the render change.

**The landing page's artwork (`site/img/*.png`) is generated the same way, by a second ignored test.**
Regenerate it after any change to `src/icon/render.rs`, or after any change to the Open Graph card layout
in `src/icon/tests.rs`:

```
cargo test -- --ignored --exact icon::tests::dump_site_images
```

That writes six files: `site/img/logo.png` (256 px, idle state), `site/img/icon-{idle,updates,error,busy}.png`
(64 px each), and `site/img/og.png`, the Open Graph card the landing page's meta tags point social
previews at (1200×630, the size Facebook, Twitter/X and Slack all expect without cropping). Commit the
results alongside the render change.

Both dump tests round-trip the encoder's output only against `inflate_fixed`, its own sibling in the same
`#[cfg(test)]` module — and that reader shares the `LENGTH_CODES` table with the encoder, so a wrong entry
there would round-trip cleanly without ever failing a test. Nothing checks the PNGs that actually get
committed against an independent implementation. After regenerating either set of images, inflate at
least one of them with a real zlib and confirm the byte count matches the dimensions in the `IHDR` chunk:

```
python -c "
import struct, zlib, sys
data = open(sys.argv[1], 'rb').read()
assert data[:8] == b'\x89PNG\r\n\x1a\n'
offset, idat, width, height = 8, b'', None, None
while offset < len(data):
    length = struct.unpack('>I', data[offset:offset + 4])[0]
    kind = data[offset + 4:offset + 8]
    body = data[offset + 8:offset + 8 + length]
    if kind == b'IHDR':
        width, height = struct.unpack('>II', body[:8])
    elif kind == b'IDAT':
        idat += body
    offset += 12 + length
raw = zlib.decompress(idat)
print(f'{sys.argv[1]}: {width}x{height}, inflated {len(raw)} bytes, IDAT {len(idat)} bytes')
" site/img/og.png
```

Verified against `site/img/og.png` on Python 3.14.4, which printed
`site/img/og.png: 1200x630, inflated 3024630 bytes, IDAT 50126 bytes` — `3024630` is exactly
`(1200 * 4 + 1) * 630`, the filtered-row stride times the height, which is what a correctly-shaped stream
inflates to regardless of what the encoder's own reader would have accepted.

### The window screenshots

The README's and the landing page's pictures of the window (`docs/img/window-{updates,batch,settings}.png`,
copied unchanged into `site/img/`) are generated too, in two steps. An ignored test builds three realistic
`Snapshot`s through the real `window::snapshot` — 23 packages across npm, bun, go and scoop with three
outdated; a 12-target batch with five done, one failed, one running and five queued; and the same
packages with a Globlin release waiting — and writes each as a self-contained copy of `ui.html` that
stubs `window.ipc` before the page's script runs and calls `window.globlin.render(...)` after it.
`settings.html` also sets the `#settings` fragment first: on its first render the page opens Settings
when `location.hash` is `#settings`. The app itself never sets a fragment, which the navigation guard
would refuse anyway (`about:blank#x` is not the page's own URL).

```
$env:GLOBLIN_SCREENSHOT_DIR = "$env:TEMP\globlin-screenshots"
cargo test -- --ignored --exact window::tests::dump_window_screenshots
```

Then headless Edge renders each page at the window's own 960×640, at 2× for sharp text.
`--force-dark-mode` pins the colour scheme: without it headless Edge follows the Windows setting, so a
machine in light mode would produce light screenshots, and these are meant to show the default dark
theme. (`--blink-settings=preferredColorScheme=1` renders the light theme instead, for checking it.) A
fresh `--user-data-dir` keeps it from attaching to a running Edge, and the `| Out-Null` is load-bearing:
`msedge.exe` is a GUI binary, so without a pipe PowerShell returns at once, the next run attaches to the
first one's profile, and only one of the three PNGs gets written:

```
$edge = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
foreach ($name in "updates", "batch", "settings") {
    $page = "$env:GLOBLIN_SCREENSHOT_DIR\$name.html" -replace '\\', '/'
    & $edge --headless=new --disable-gpu --hide-scrollbars --force-dark-mode `
        --user-data-dir="$env:TEMP\globlin-edge-profile" --force-device-scale-factor=2 `
        --window-size=960,640 --screenshot="$PWD\docs\img\window-$name.png" "file:///$page" | Out-Null
    Copy-Item "docs\img\window-$name.png" "site\img\window-$name.png"
}
```

Regenerate them after any visible change to `src/window/ui.html` or to the fixtures in
`src/window/tests.rs`, open each PNG to check it before committing, and commit them with the change.

### The version resource

`build.rs` fills the Win32 VERSIONINFO block, not only the icon. `winresource` derives `ProductName` from
the crate name and both version fields from `package.version`, and leaves `CompanyName`, `LegalCopyright`,
`OriginalFilename` and `InternalName` empty unless they are set; `FileDescription` did not pick up
`package.description` either, so all five are stated explicitly rather than derived.

The reason is the antivirus problem described under [the VirusTotal scan](#the-virustotal-scan). An empty
version resource is one of the cheap static features heuristic engines weigh, because legitimate software
fills it in and malware usually does not. It is a small weight next to *unsigned* and *replaces its own
executable*, and on its own it will not clear a `Wacatac!ml` verdict — but it is the only one of those
features that can be fixed without removing something Globlin exists to do.

`FileDescription` is the string Task Manager shows in its process list, so it reads as a phrase rather
than as the bare crate name.

### The release profile is an antivirus decision

`[profile.release]` is `opt-level = 3`, `lto = true`, `codegen-units = 1` — and deliberately *not*
`opt-level = "z"`, `panic = "abort"`, `strip = true`, which is what it used up to v0.2.7. The size-tuned
profile produced a binary that Microsoft, McAfee and SecureAge flagged; the conventional one does not.

This was measured, not guessed. Three builds of identical source were compared:

| profile | size | entropy | VirusTotal |
|---|---|---|---|
| `opt-level = "z"`, `strip`, `panic = "abort"` | 1,758,208 | 6.736 | 3 of 71 |
| the same, minus `strip` | 1,761,280 | 6.733 | not submitted |
| `opt-level = 3`, no `strip`, unwind | 2,885,120 | 6.597 | **0 of 71** |

The three that flagged the size-tuned build were Microsoft (`Trojan:Win32/Wacatac.C!ml`), McAfee
(`Trojan:Win/Malwarex.EFM`) and SecureAge (`Malicious`). The middle build was not submitted because it
sits 3 KB from a known-flagged one, which makes its verdict predictable and uninformative.

`strip` turned out to be a red herring: under MSVC the debug information lives in a separate `.pdb`, so
stripping removes 3 KB and changes nothing a scanner sees. `opt-level` is the variable that matters, and
it is also the one that sets the size — there is no setting that is both 1.7 MB and unflagged.

Two earlier theories were wrong and are recorded here so they are not retried. The verdict was never about
behaviour: VirusTotal does not run the file, so the `HKCU` `Run` key, the `npm` child process and the
self-replacing update never entered into it. And [filling the version resource](#the-version-resource),
tested on its own in v0.2.7, moved nothing — 3 of 71 before and after.

The mechanism is worth stating carefully, because the obvious reading of the table is wrong.
`opt-level = "z"` is not itself a red flag. [Claude-Code-Usage-Monitor][ccum], an unsigned Rust tray app
that ships on winget, uses the whole size-tuned set — `opt-level = "z"`, `lto`, `strip`,
`codegen-units = 1`, `panic = "abort"` — and is not flagged. Its binary is 7.3 MB. Ours was 1.7.

[ccum]: https://github.com/CodeZeno/Claude-Code-Usage-Monitor
What the models react to is a *small* dense executable, which is the shape of packed malware; the profile
matters only because it is what sets the size. Anything that shrinks this binary back toward 1.7 MB is
likely to bring the detections back, whatever setting achieves it.

The trade was 1.7 MB → 2.8 MB, measured on v0.2.8. The binary has grown since — the window links
WebView2's glue in, and v0.3.0 builds at about 3.4 MB — which moves it further from the dense shape that
was flagged, so the numbers in the table above are the record of that decision rather than today's size.
For a tray application that is not a cost worth a false positive. Do not
reintroduce the size-tuned settings without re-running the comparison; the numbers above are the baseline
to beat.

**`panic = "abort"` is not free to restore, for a reason unrelated to scanning.** `App` sets
`activity = Some(..)` on the main thread before spawning a worker, and clears it only when the worker's
terminal `Message` arrives (`app.rs:170`, cleared at `app.rs:312`). Under `abort` a panic in a worker
takes the process down — loud, and obvious to the user. Under unwind it kills only that thread: the
message is never sent, `activity` stays `Some` forever, `start_check` and `start_update` return early
from then on, and `next_wake` stays pinned at `now + 120 ms`, so the app spins at 8 fps doing nothing
until it is killed. A silent permanent hang is worse than a crash. The exposure is small — the only
`unwrap` outside tests and `icon/` writes to a `String` and cannot fail — but if `panic = "abort"` is ever
restored, or a fallible `unwrap` is added to a worker path, that interaction is what to think about.

`rust-toolchain.toml` pins the compiler for the same reason. v0.2.8 shipped at 2,878,976 bytes while the
build measured at 0 of 71 was 2,885,120 — 6 KB apart, and not because of the paths baked into the binary,
since the CI checkout path is *longer* than a typical local checkout and would have made the file
bigger, not smaller. Neither workflow installed a toolchain, so both took whatever rustc the
runner image happened to carry, and a different compiler lays out code differently. Without the pin the
artifact that gets scanned is not the artifact that was tested, and the shipped binary changes whenever
GitHub refreshes the image. Bumping the channel is a deliberate act: treat it as a change to the released
binary and watch the next scan.

### Environment workarounds

Things about the environment the code has to work around, all verified rather than assumed:

- **A batch updates brew and scoop last.** Both install the runtimes other sources run on. On the first
  real Mac, *Update all* upgraded Homebrew's `ruby` mid-batch, `brew link` failed on a conflicting `erb`,
  `/opt/homebrew/bin/gem` vanished, and every gem update queued after it failed with
  `could not run gem: No such file or directory`. `model::in_update_order` (applied in
  `App::start_update`) stable-sorts `SourceKind::installs_runtimes` targets to the end, so the gems, npm
  packages and pipx tools are updated while their runtime is still in place.
- **`npm ls -g --json` exits non-zero when the global tree has problems** (an orphaned directory in
  `node_modules` is enough). The exit code is therefore ignored and stdout is parsed anyway; entries with
  no usable `version` are skipped instead of failing the whole listing.
- **npm 12 blocks install scripts by default and still exits 0.** A package missing from the
  `allow-scripts` list has its `postinstall` skipped with only a warning. `@anthropic-ai/claude-code`
  is the case that surfaced it: its `postinstall` copies the native binary over a 500-byte stub in
  `bin/claude.exe`, so a "successful" update left a `claude` that Windows refuses to start. The update
  command therefore passes `--allow-scripts=<name>@<version>`, which approves the scripts of exactly the
  release the user chose to install — the one the check showed, which is also the one installed, as
  `<name>@<version>` rather than `@latest` — and `--strict-allow-scripts`, which turns any other blocked script (a
  dependency's) into `ESTRICTALLOWSCRIPTS` and a non-zero exit. That check runs before the old version
  is touched, so a refused update leaves the working install in place and reports as blocked (see the
  next bullet). A CLI `--allow-scripts` replaces the `.npmrc` list rather than extending it (npm warns
  `.npmrc allow-scripts setting is being ignored`), so the update first reads
  `npm config get allow-scripts -g` and passes that list with the pinned package appended. Without that
  merge a dependency the user approved in `.npmrc` could never be approved at all:
  `@deepseek-ai/dsh` needs `koffi`, `node-pty` and `protobufjs`, and failed every time. The fix for a
  refused update is to append the listed dependencies to the user-level list — npm's own hint,
  `npm config set allow-scripts=<deps> --location=user`, replaces the existing list, so keep the old
  entries in it. npm older than 12 does not know either flag and only warns.
- **A refused npm update opens an approval dialog instead of just failing.** `ESTRICTALLOWSCRIPTS`
  stderr is parsed by `npm::parse_blocked_scripts`: the header line `npm error --strict-allow-scripts:
  <N> package(s) have install scripts not covered by allowScripts:` names how many package lines to
  expect, and every line between it and `npm error Allow them with…` must parse, number exactly `N`,
  and name no package twice. npm prints script bodies raw, so a body containing a newline spills onto
  an unindented line inside the block; that line fails to parse and the whole refusal falls back to a
  plain failure rather than a dialog that shows only the first half of a script. The lines
  `npm error   <name>@<version> (<event>: <body>; …)` come from one place in npm
  (`@npmcli/arborist/lib/unreviewed-scripts.js`), and the version is split at the last `@` so scoped
  names survive. The target lands in `Outcome.blocked`, not `failed`, so there is no failure toast;
  `App` keeps it in `approvals` (memory only) and opens the window, whose modal lists each package and
  its scripts. A batch in which every target was blocked changed nothing, so it skips the usual
  post-update check (`Outcome::changed_packages`); otherwise **Allow and update** would sit disabled
  behind a full check of every source. **Allow and update** posts `approve:npm:<name>`; the worker reads
  `npm config get allow-scripts -g`, which is the list a global install actually applies (user + global,
  never the project layer — `--location=user` does not restrict what `config get` reads, verified on
  npm 12.1.0) — appends pinned `name@version` entries (npm's own `approve-scripts` default; npm older
  than 12 prints the bare tokens `undefined` or `null` here, which the merge drops instead of keeping as
  an entry), and runs `npm config set … --location=user`, so only the write is pinned to the user file.
  A failed read fails the approval instead of turning into an overwrite of the existing list. A retry
  that hits new dependencies comes back as blocked again. A failed `config set` fails the update without
  running it. Unparseable output falls back to a plain failure. The pinned name is the one npm prints
  (self-reported); for an alias dependency npm matches on the resolved identity, so that approval would
  not take and the dialog would show again.
- **`auto_approve_scripts` (off by default) skips the dialog, once.** When it is on, `App::on_updated`
  takes `update::auto_approvals(outcome, …)` and starts the approval run itself, for every blocked target
  in one batch, without opening the window. `update::run` marks any run that carried approvals as
  `Outcome.retried`, and `auto_approvals` returns nothing for a retried run. So a retry that is refused
  again (new dependencies, or the alias case above) lands in the dialog instead of approving in a loop.
  The decision lives in `update.rs` and not in `App` so that it can be tested.
- **`bun pm ls -g` does not list global packages.** It ignores `-g` and prints the tree for whatever
  directory it is run from, so it will happily report a project's dependencies as if they were global. The
  bun source instead reads the global manifest directly — see below for where that manifest actually
  lives — and resolves versions from that directory's `node_modules`. An empty global store reports zero
  packages, which is not an error.
- **bun does not keep its global manifest in one fixed place.** On bun 1.3.3 for Windows the global
  `package.json`, `bun.lock`, and `node_modules` sit directly in `%USERPROFILE%`, while
  `~/.bun/install/global` — the layout older builds used, and the one the app originally hard-coded —
  exists but stays empty. `bun pm bin -g` still points at `~/.bun/bin`, so the shim directory is no
  guide to where the manifest lives. `Bun::new` therefore probes `$BUN_INSTALL/install/global`, then
  `~/.bun/install/global`, then `~`, and takes the first candidate whose `package.json` parses and has a
  sibling `bun.lock` or `bun.lockb` next to it — a bare parse check would adopt any project's
  `package.json`, including a stale empty manifest an older bun left behind. If no candidate has a
  lockfile, it falls back to the first whose manifest parses and declares at least one dependency. When
  nothing matches either rule, the source reports zero packages — a bun install with no global packages
  is legitimate — and writes the probed paths to the log so an empty list is explained rather than
  silent.
- **pnpm prints an array, npm prints an object.** `pnpm ls -g --json` wraps the dependency map in a
  one-element array of projects, where `npm ls -g --json` returns the map at the top level. The pnpm
  source accepts either shape (`#[serde(untagged)]`) and de-duplicates by name, because a multi-project
  reply can repeat one.
- **`yarn global` only exists in yarn classic.** Yarn 2+ removed it, so there is no global install to
  list. Rather than parse `yarn global list`'s `info "pkg@1.2.3" has binaries:` chatter, the source asks
  `yarn global dir` and then reads that directory's `package.json` and `node_modules` — the same
  mechanism the bun source uses, shared as `source::node_modules_listing`. On Berry the command fails,
  the source reports zero packages, and the log says why. `yarn global dir` also prints its own banner
  and a `Done in 0.2s.` line around the path, so the parser drops known chatter prefixes rather than
  taking the last line.
- **`pipx list --short` is the whole listing.** It prints `<name> <version>` per line — the main
  package of each venv, not its injected dependencies — which is exactly the set a tools manager should
  watch. The "nothing has been installed with pipx" notice goes to stderr, so stdout stays clean and the
  parser only has to drop any line that is not two columns. `--json` carries the same two fields under
  four levels of nesting.
- **`uv tool list --outdated` hides current tools.** It is not the listing with a `[latest: X]` suffix
  added — it skips every tool that is not behind. So the source runs `uv tool list` for the set and
  `--outdated` for the upgrades, and merges by name. Both print entrypoints as `- <name>` lines under each
  tool; the name column is checked for a leading `-` because an entrypoint called `vim` would otherwise
  parse as a tool named `-` at version `im`.
- **scoop is read from disk, not from `scoop list`.** `scoop list` and `scoop status` are PowerShell
  `Format-Table` output with ANSI colour codes, and `scoop status` prints nothing but a warning when the
  buckets are stale. The same facts are plain JSON on disk: `apps\<name>\current\manifest.json` holds
  the installed `version`, `current\install.json` names the `bucket`, and
  `buckets\<bucket>\bucket\<name>.json` holds the version that bucket offers (older buckets keep
  manifests at their root, so that path is tried second). `current` is a junction, which `fs` follows.
  `apps\scoop` itself is a git checkout with no manifest and drops out on its own. The root is `$SCOOP`,
  else `%USERPROFILE%\scoop`.
- **`cargo install --list` marks git and path installs.** A registry crate prints `name vX.Y.Z:`; one
  installed from git prints `name vX.Y.Z (https://…#rev):`. The second is dropped, because crates.io
  would answer for an unrelated crate of the same name. The format was captured from the real cargo
  without installing anything: `cargo install --list --root <dir>` reads `<dir>\.crates.toml`, so a
  hand-written one is enough.
- **go names a tool by its package, but versions belong to its module.** `go version -m <bin dir>` scans
  the whole directory in one call and prints a `path` line (the package, what `go install` takes) and a
  `mod` line (the module and its version) per binary. They differ for anything under `cmd/` —
  `honnef.co/go/tools/cmd/staticcheck` is version `v0.4.7` of module `honnef.co/go/tools` — and only the
  module can be asked for `@latest`. The source keeps both, names the row by the package, and queries by
  the module. Binaries built locally report `(devel)` and are skipped.
- **`dotnet tool list` grew JSON in SDK 9.** `--format json` wraps rows in `{"version":1,"data":[…]}`;
  the serde structs accept camel and Pascal case because the SDK's serializer context decides which. An
  older SDK rejects the flag, prints nothing on stdout, and the source falls back to the table, reading
  rows after the dashed separator so the localised header is never a row. A machine with the `dotnet`
  host but no SDK (a runtime-only install) answers `tool list` with exit `0x80008091` and *No .NET SDKs
  were found*; that reads as no tools, not as a failed source.
- **Log entries carry a heading.** `last-run.log` and `self-update.log` start each entry with
  `== Globlin <version> · <UTC time> ==`, because notes are appended to an older failure report and a
  stale `docx@latest` failure from a previous build once looked fresh next to a new dotnet note.
- **psgallery quotes module names.** Every name reaches PowerShell inside `-Command`, so `source::quoted` wraps it
  in single quotes and doubles any quote inside. `Get-InstalledModule` lists only Gallery-installed
  modules, which is the set `Update-Module` can act on; the update is pinned to `-Scope CurrentUser` so
  it never needs elevation.
- **gem uninstall must not prompt.** A gem with several installed versions makes `gem uninstall` ask
  which to remove, and the process has no console to ask on, so it would wait forever. `--all
  --executables` removes every version and its binstubs without asking. `gem list` puts `default: ` in
  front of default gems and a platform label after native ones (`1.16.5 x64-mingw-ucrt`); both are
  stripped from the version.
- **`winget list` has no machine-readable output.** There is no `--json`; `winget export` omits anything
  without a manifest. So the fixed-width table is parsed: find the row of dashes, read column start
  offsets from the header line above it, and slice each row by those offsets. Column *positions* rather
  than column *names*, because the headers are localised. The `Available` column only appears when at
  least one package has an upgrade, so a five-column table means `Name Id Version Available Source` and
  a four-column one means `Name Id Version Source` — that rule holds in any language. The **Id** is
  taken as the key, not the display name, because the id is what `winget upgrade --id` takes. winget
  ellipsises cells that overflow their column in a console, but with stdout redirected winget 1.29 sized
  every column to its widest cell and printed no `…` across a long real listing. An id that did arrive
  truncated would carry the literal `…`, which `--exact` matches against nothing, so the update fails and
  lands in the log rather than touching some other package.
- **`choco list` changed meaning between v1 and v2.** In Chocolatey 1.x, `choco list -r` lists the
  *remote* community feed — thousands of packages — and the local set needs `--local-only`; in 2.x that
  flag is gone and `choco list -r` is local. Guessing wrong is not a small error, so the source reads
  `choco --version` first and picks the argument list from the major version. `choco outdated -r` is
  stable across both and supplies the upgrades, merged onto the listing by name.

## The landing page

`site/` is the marketing page at [globlin.pages.dev](https://globlin.pages.dev). Cloudflare Pages is
pointed at the `site` directory on the `master` branch with no build command — every push republishes
it verbatim. There is no bundler, no `package.json`, and no third-party request at render time; the CSS
and the one script are inline in `site/index.html`.

See [The icon](#the-icon) section above for how the landing page's artwork is regenerated.

The download button points at `releases/latest/download/globlin.exe`, a permanent GitHub redirect, so it
needs no maintenance when a version ships. The script only replaces the word `latest` with the resolved
tag, and the page is correct without it.

`site/_headers` carries the security headers and cache lifetimes. The Content-Security-Policy allows
scripts from the page itself and from `static.cloudflareinsights.com`, and network calls to
`api.github.com` and `cloudflareinsights.com` — nothing else. Those two Cloudflare hosts are there for
Web Analytics, which Pages injects into every `pages.dev` response on its own: with the policy that
shipped first, the beacon was blocked, the analytics panel showed zero visits with no error anywhere,
and Lighthouse marked the console error down to 92 on Best Practices. A blocked beacon is silent, so
the symptom to look for is missing data rather than a failure.

Images under `/img/` are cached for a day, and every reference to one in `index.html` carries a
`?v=<version>` query. They were cached for a year as `immutable` until the goblin replaced the `G` logo:
the files are regenerated in place under the same names, so browsers kept showing the old logo with no
way to refresh it. Bump the `?v=` value whenever the dump tests rewrite `site/img/`.

`site/llms.txt` describes the project for AI agents, in the format Lighthouse's agentic-browsing
category checks for: a title, a one-line summary, a few paragraphs of prose, and link sections pointing
at the releases, the README, the development notes and the repository. Keep it in step with the
README — it is a summary of the same facts, not a second source of truth.

Commits that touch only `site/` must be scoped `chore(site):` or `docs(site):`. `Cargo.toml` declares
the package at the repository root, so release-plz reads every commit in the repository, and by default
any commit at all bumps the version — the type only decides which changelog section it lands in. Two
settings in `release-plz.toml` do the work: `release_commits = "^(feat|fix)"` means only a `feat` or a
`fix` opens a release PR, and the first entry in `[changelog] commit_parsers` skips anything scoped
`(site)`, so the landing page never appears in a changelog that documents the application. Both match on
the scope, which is why it is not optional.

## How a launch decides whether to show the window

`main::launch_kind` sorts every start into three kinds. **Opened** — no flag: the user started it from
the Start menu, a pinned icon or the exe, so the window opens once the `App` exists. **Background** —
`--background`, which every autostart entry carries (the `Run` value `set_autostart` writes, the
installer's autostart task, the macOS LaunchAgent): tray only. **Replaced** — `--replaced`, self-update's
relaunch: tray only, plus the "updated" notification; it wins over `--background`.

An Opened start that loses the single-instance race does not just exit, which is what made a pinned icon
look dead while Globlin sat in the tray: on Windows it sets the auto-reset event `Local\globlin-show`,
which the running instance waits on in a `show-request` thread and turns into `Message::ShowWindow`. On
macOS Launch Services never starts a second copy of a running `.app`; it sends a reopen, which tao
delivers as `Event::Reopen` and `main.rs` maps to the same message. Older entries without the flag are
upgraded on the next start: `refresh_autostart` rewrites the entry whenever autostart is on, so only the
first sign-in after updating opens the window.

## Self-update mechanics

The full mechanics behind the brief version in the [README](../README.md#keeping-itself-updated).

On the same schedule as the package check, the app makes one extra request per cycle to
`https://api.github.com/repos/TOR968/globlin/releases/latest`. The two are deliberately independent:
`check::run` looks the release up first and returns a `Report` carrying `packages: Result<Vec<Package>>`,
so a missing npm, a `PATH` without bun, or an unreachable registry cannot stop the app from finding — and
installing — its own update. That endpoint always resolves to the newest *published* release; GitHub
excludes pre-releases and drafts from it, so neither can reach a user by accident. A release is offered
only when its tag parses as semver strictly newer than the running build, and both the binary for the
running platform (`globlin.exe` on x64, `globlin-arm64.exe` on ARM64, `globlin-macos-apple-silicon.zip`
on a Mac, `globlin-<arch>.AppImage` in an AppImage — `selfupdate::asset_for`), `SHA256SUMS` and `SHA256SUMS.minisig` are all attached — a release
missing any of them is skipped rather than half-offered.

Applying an update downloads all three, verifies the signature on `SHA256SUMS` (see
[The release signature](#the-release-signature)), hashes the download with SHA-256 and compares it with
its line in `SHA256SUMS`; any mismatch discards the download and reports a failure, and the running
binary is never touched. Windows will not let a running `.exe` be overwritten, but it will let it be *renamed*, so
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

### The release signature

A checksum published in the same release as the binary proves the download is intact, not that the
project published it: whoever can edit a release can edit both. So `SHA256SUMS` lists the
SHA-256 of all twelve release files, `SHA256SUMS.minisig` is a minisign signature over that list, and
self-update refuses a download unless the signature verifies against `RELEASE_PUBLIC_KEY` in
`selfupdate.rs` (`RWTTLQhyI4Bn/MclkJqsthxavfiFPGVos6b7/d9keFthKYcrAmvZ31V+`) and the download's hash
matches its line. `offer` does not even offer a release that lacks either file, and the download agent is
`https_only`. The same two files let anyone check any download with `minisign -Vm SHA256SUMS` and
`sha256sum -c --ignore-missing SHA256SUMS`.

Up to v0.3.15 each self-update binary carried its own `.sha256` and `.minisig`, and the Mac updated from a
bare `globlin-macos-arm64`. v0.4.0 dropped all of that in one step, on purpose: copies older than 0.4.0
look for names that no longer exist and stop finding updates, so they have to install 0.4.0 by hand once;
everything from 0.4.0 on updates itself through `SHA256SUMS`.

`.github/minisign-sign.py` writes the prehashed (`ED`, BLAKE2b-512) format `minisign -V` and the
`minisign-verify` crate read, from the `MINISIGN_SECRET_KEY` repository secret — base64 of an 8-byte key
id followed by the 32-byte Ed25519 seed, not minisign's own encrypted key file, so CI needs no password.
The publish job fails rather than publish unsigned files when the secret is missing. The private key
exists only in that secret and in the maintainer's offline backup; **losing it means shipping a build with
a new public key, which every installed copy will refuse until the user downloads it by hand**, so the
backup matters more than the secret. The unit tests sign with a fixed throwaway key
(`TEST_KEY`), never the real one.

### When winget owns the binary

`install::managed_by()` answers one question — is this `.exe` running from a directory a package
manager controls, and which one (`Manager::Winget`, or `Manager::SystemPackage` for a Linux binary under
`/usr`) — and two unrelated behaviours hang off it. The predicate is deliberately dumb: the exe
path contains the adjacent components `WinGet` and `Packages`, matched case-insensitively. That covers the
user-scope `%LOCALAPPDATA%\Microsoft\WinGet\Packages\` and the machine-scope
`%PROGRAMFILES%\WinGet\Packages\` without reading either environment variable, and it deliberately does
not match the shim in `WinGet\Links\`, which is not where the real binary lives. `is_winget_path` takes
the path as an argument so all of that is unit-testable; `managed_by` is the thin wrapper that calls
`current_exe`.

**The config would otherwise be destroyed on every upgrade.** Globlin is portable: `config::candidate_paths`
tries `globlin.json` next to the exe first, for both reading and writing, and falls back to
`%LOCALAPPDATA%\globlin\`. A winget package directory is user-writable, so the config would land inside it
— and `winget upgrade` deletes that directory and lays down a new one. The ignore list, `auto_update` and
`last_notified` would silently reset on every upgrade. `exe_dir` therefore returns `None` when the install
is winget-managed, which drops the first candidate and leaves `%LOCALAPPDATA%` as the only home.

**Self-update is switched off, not merely hidden.** Two installers racing for the same `.exe` is worse than
either alone: Globlin replaces itself with 0.3.0, winget still records 0.2.5 as installed, and the next
`winget upgrade --all` cheerfully overwrites the newer binary with the older one. So `check::run` skips the
release lookup entirely — no network call, no `Release`, nothing for the auto-update path to act on — and
the menu says `Installed with winget — run winget upgrade` in place of the update row.

That menu change is why `View` carries a `SelfUpdate` enum rather than another `bool`. Adding a fifth flag
to a struct that already had four tripped `clippy::struct_excessive_bools`, and the lint was right: the two
cases are alternatives, not independent switches. `SelfUpdate::Managed(Manager)` carries only which
manager to name, and
`SelfUpdate::Own { release, auto_update, log }` groups the three fields that only mean anything when
Globlin manages itself — so the auto-update checkbox cannot be rendered in a state where ticking it would
do nothing.

### When the installer owns the directory

`installer/globlin.iss` is an Inno Setup 6 script for a **per-user** install: `PrivilegesRequired=lowest`,
into `%LOCALAPPDATA%\Programs\Globlin`, with a Start menu entry, an entry in *Settings → Apps*, and two
tasks — a desktop shortcut (off by default) and *Run Globlin when I sign in* (on). The release builds it
twice, `globlin-setup-x64.exe` (`ArchitecturesAllowed=x64compatible`) and `globlin-setup-arm64.exe`
(`arm64`), from the same two binaries the portable downloads are, through `.github/build-installer.ps1`.
MSI was considered and declined: it is per-machine by habit, and an MSI repair would put back an exe that
self-update had replaced.

The fixed `AppId` `{380BD341-81C1-4AC3-AA2E-BC70BE5CC8F7}` is load-bearing twice. Re-running a newer setup
finds the existing install through it and upgrades in place. And Inno records the install under
`HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{380BD341-81C1-4AC3-AA2E-BC70BE5CC8F7}_is1`,
whose `InstallLocation` is how Globlin recognises an installed copy: `install::setup_managed()` is true
when `current_exe`'s directory equals it, compared component-wise and case-insensitively by
`install::is_setup_path`, so the trailing backslash Inno writes does not matter.

Unlike winget, the installer does not take self-update away — the directory is the user's and writable,
and a second installer cannot race the swap. Two things change for an installed copy instead:

- **Config lives in `%LOCALAPPDATA%\globlin\`**, as for winget (`config::exe_dir` returns `None`). The
  program directory holds only `globlin.exe` and Inno's uninstaller, and uninstalling leaves settings and
  logs behind for a reinstall, which is the convention.
- **`DisplayVersion` follows self-update.** After a successful swap, `selfupdate::apply` writes the new
  version into the uninstall key, so *Settings → Apps* does not keep showing the version that was
  installed. A failed write lands in `self-update.log`; it never fails the update.

The installer's *Launch Globlin* checkbox starts it through `explorer.exe`, not directly. A program
started straight from the setup process inherits setup's environment, and a manual upgrade showed what
that can mean: `npm ls -g` answered with an empty tree (no `APPDATA`, so npm looked for its global prefix
in the wrong place), scoop listed nothing, and scoop's `bun.exe` shim could not start bun — while the same
build started from the Start menu saw every package. Explorer hands it the same environment the Start menu
does. Globlin also repairs its own environment on Windows, whoever starts it — see
`platform::prepare_environment` under [Other platforms](#other-platforms).

The autostart task writes the same `HKCU\…\Run\globlin` = `"<exe>" --background` the tray toggle
writes, so the two read one value. A second `[Registry]` entry with `ValueType: none; Flags: uninsdeletevalue` deletes it on
uninstall whoever created it. Setup and uninstall both `taskkill /F /IM globlin.exe` first, because the
exe is locked while it runs; the single-instance mutex means there is at most one to stop.

`ci.yml` proves the round trip on every push: the Windows x64 job compiles the x64 setup, installs it
silently with the autostart task, checks the exe, `InstallLocation`, `DisplayVersion` and the Run value,
uninstalls silently, and checks that the exe, the Apps entry and the Run value are gone
(`.github/installer-smoke.ps1`). The uninstaller re-launches itself from `%TEMP%` and returns at once,
which is why the script polls for the removal instead of trusting `-Wait`.

## Tests

```
cargo test
```

571 tests on Windows: 541 run by default (no network, no side effects), 30 `#[ignore]`d because they
touch the real system — the HKCU Run key, a real toast, a real `npm install -g` (two of them), one that
runs the real `npm config get/set` against a temporary userconfig, five icon/PNG dump tests (the `.ico`,
the README images, the site images, the macOS iconset and the Linux hicolor icons), one that writes the
window screenshot pages (`dump_window_screenshots`), two that hit
`TOR968/globlin`'s real GitHub releases, one that builds a real WebView2 window
(`the_window_shell_starts_against_a_real_webview`, the only check that the `wry` shell actually starts on
this machine), four that hit a real remote catalog (the PyPI feed, crates.io, the NuGet index and the
Gallery redirect), and twelve that run the real `winget`, `choco`, `pnpm`, `pipx`, `uv`, `scoop`,
`cargo`, `go`, `dotnet`, PowerShell, `gem` and `brew`. Those last twelve exist because
the parsers below are pinned against captured output, and captured output cannot notice the day a tool
changes its format — run them after a winget or Chocolatey upgrade. Each carries its own exact invocation
in its
`#[ignore = "…"]` message; `grep -rn "#\[ignore" src/` lists them. `updates_a_package_for_real` really
installs `$env:UPDATE_TARGET` globally, so point it at something harmless. To see the working UI on
demand, downgrade something disposable (`npm i -g npm-check-updates@23.0.0`) and hit *Check now*.

The update orchestration is exercised by pointing `npm_cmd` at a file that cannot be executed, which
drives the real failure paths without installing anything. **Every such fake must be named `npm` or
`npm.cmd`, in a directory of its own.** An override that is refused is an error and never falls back to
the `npm` on `PATH` — the one time it did (while the name check was being added), the suite ran the
real `npm install -g` and `npm uninstall -g prettier` on the developer's machine. The source adapters are tested against
captured stdout — a real `winget list` table, a `choco list -r` listing, a `pnpm ls -g --json` reply —
rather than by running those tools, so the parsers are pinned without the suite needing them installed.

## Lints

`cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are CI gates. `Cargo.toml` enables
`clippy::pedantic`, so `-D warnings` denies that set too. `src/icon/render.rs` and `src/icon/ico.rs` carry
a module-level `#![allow(…)]` for the cast lints — both are arithmetic on small bounded integers, pixel
coordinates and `.ico` header fields, where the casts are the work rather than an accident. No other
module widens that allow — `src/icon/png.rs`, for instance, uses `u32::try_from`/`u16::try_from` instead
of `as` throughout. `clippy::nursery` is deliberately **not** enabled: its lints move between releases,
and a toolchain bump should not turn a green build red.

## CI and releases

Three workflows. `release-plz.yml` stays Windows-only, which is not obvious: computing a version number
sounds platform-independent, but release-plz runs `cargo package` with verification to compare the packaged files against the last
release, and verification *compiles* the crate. `tao` and `tray-icon` sit in plain `[dependencies]`, so on
a Linux runner they pull the GTK stack and the build dies on `The system library glib-2.0 ... was not
found`, taking the whole job with it — `failed to determine next versions: run cargo package`. There is no
config switch for this: `publish_no_verify` only reaches `cargo publish`, not the packaging done while
determining versions.

- **`ci.yml`** — every push to `master`/`main` and every pull request: fmt once, then clippy and the
  suite on five runners — Windows x64 and ARM64, macOS ARM64, Linux x64 and ARM64 (see
  [Other platforms](#other-platforms)). The two Windows runners also build the release binary and attach
  it to the run as an artifact (`globlin.exe-<sha>`, `globlin-arm64.exe-<sha>`). `windows-11-arm` ships
  no Rust, so that job installs it with `rustup-init` first. The x64 job also builds the installer and
  runs it through a silent install and uninstall (see
  [When the installer owns the directory](#when-the-installer-owns-the-directory)). The macOS job bundles
  `Globlin.app`, verifies its signature and attaches the zip (see [macOS](#macos)). The `#[ignore]`d
  tests never run here.
- **`release.yml`** — a `v*` tag only. A `build` matrix on `windows-latest` and `windows-11-arm` tests,
  builds, names the binary for its architecture (`globlin.exe`, `globlin-arm64.exe`), writes its
  `.sha256`, and uploads it to VirusTotal for a **Virus scan** section named after the file — see below.
  The `publish` job then refuses to continue if the tag does not match the `version` in `Cargo.toml`, so
  a `v0.2.0` tag on a `0.1.0` manifest fails instead of publishing a mislabelled build, builds
  `globlin-setup-x64.exe` and `globlin-setup-arm64.exe` from the two binaries (Inno Setup is preinstalled
  on `windows-latest`; `build-installer.ps1` falls back to `choco install innosetup`), scans them too,
  and creates one GitHub Release with fourteen files. Twelve are the release proper: the two installers
  and the two portable `.exe`s; the `build-macos` job's `globlin-macos-apple-silicon.zip`; the
  `build-linux` jobs' two `.deb`, two `.rpm` and two `.AppImage` files (no VirusTotal scan for the macOS
  and Linux files); and `globlin-bin.PKGBUILD`, rendered from the two debs' hashes. The last two are
  `SHA256SUMS`, one `<hex>  <file>` line (two spaces, the `sha256sum` format) for each of the twelve, and its
  `SHA256SUMS.minisig` (see [The release signature](#the-release-signature)), which is also what
  self-update checks. Up to v0.3.15 every file carried its own `.sha256` and `.minisig`, thirty-two files
  in all, and the asset list GitHub shows — alphabetical, with
  no way to hide or reorder an entry — was too long for anyone to find the file they needed. The release
  notes now open with a *Which file do I need?* table instead: `.github/release-notes.md` is a template,
  `@TAG@` becomes the tag so its links point at this release's files, and `@CHANGELOG@` becomes this
  version's `CHANGELOG.md` section (cut by `changelog-section.ps1`) under *What's changed*, so the table
  and the verify commands come first and the changes right after them. The step throws if the
  `@CHANGELOG@` line goes missing rather than silently dropping the changelog.
- **`release-plz.yml`** — every push to `master`. [release-plz](https://release-plz.dev) keeps a
  **release pull request** open containing the version bump and the new `CHANGELOG.md` entries; nothing
  is published while it sits there. Merging it is the decision to release: release-plz creates the `v*`
  tag, which triggers `release.yml`. `release-plz.toml` sets `git_release_enable = false` so the two do
  not race to create the same release — release-plz stops at the tag and hands over. It also sets
  `release_commits = "^(feat|fix)"`, because release-plz otherwise proposes a patch bump for *any*
  unreleased commit regardless of its type: the landing page arrived as sixteen `chore(site)` and
  `docs(site)` commits that changed no shipped code, and release-plz still opened a release PR for them.
  And `git_only = true`: globlin is not published to a cargo registry, and without that flag release-plz
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

`tag` is serialised across runs too (its own `concurrency` group, never cancelled). Merging the release
PR and another PR seconds apart starts two runs; without the group both check out before either has
tagged, the second tries to create the same `v*` tag, and GitHub answers `422 Reference already exists`
— the release itself is fine, but `master` shows a red run. Queued, the second run checks out after the
tag exists and has nothing to do.

### The VirusTotal scan

Every release uploads both binaries, `globlin.exe` and `globlin-arm64.exe`, and both installers to VirusTotal and links each
report from the release notes under a **Virus scan — <file>** heading, because
an unsigned binary that writes an autostart key, spawns `npm` and rewrites its own `.exe` is flagged by
machine-learning heuristics on a regular basis. Defender has quarantined the download as
`Trojan:Win32/Wacatac.B!ml` and as `Program:Win32/Wacapew.C!ml`; the README explains this to users, and the
per-release report is what turns "trust me" into a number they can check against seventy other engines.

`.github/virustotal-scan.ps1` does the work: `POST /api/v3/files`, then poll `GET /api/v3/analyses/{id}`
every 20 seconds until the status is `completed`, then emit a markdown section naming the flagged count.
The denominator counts only the four `stats` buckets that represent an actual verdict — `malicious`,
`suspicious`, `undetected` and `harmless`. `type-unsupported`, `timeout` and `failure` are engines that
did not answer, and VirusTotal leaves them out of the ratio its own report shows. The first release to
run this, v0.2.6, is what settled it: including `timeout` reported `3 of 71` against a report that read
`3 of 70`, and a number that disagrees with the link beside it undermines the one thing the section is
for.

Three things about it are deliberate:

- **It can never fail the release.** No `VIRUSTOTAL_API_KEY` secret, a failed upload, a failed poll, or an
  analysis still running after six minutes: each returns an empty string instead of throwing, the step
  carries `continue-on-error: true` on top of that, and the notes step only appends the section when
  `dist/virustotal.md` exists. A third-party API having a bad afternoon must not block a tagged build.
  Because the key is a secret, the step is also a no-op on a fork.
- **It runs before `gh release create`, not after.** That costs the release up to six minutes of polling,
  which buys a single-pass notes file rather than creating the release and editing it afterwards.
- **A timed-out analysis still publishes the link.** The report URL is `gui/file/<sha256>` and is valid
  the moment the upload lands, so the section is emitted saying the analysis was still running.

**The explanation follows the verdict.** A clean scan gets the bare report line; a flagged one gets a
paragraph, and which paragraph depends on `-Installer`, which the publish job passes for the two
`globlin-setup-*.exe` files. Until 2026-10 every section carried the binary's explanation, so a clean
`globlin.exe` was followed by a paragraph on why it trips heuristics, and the installers — the only files
actually flagged — were blamed on the autostart key and `npm` rather than on what the engines object to.

**The installers are flagged every release; the portable binaries usually are not.** From v0.4.0 to
v0.4.6 both `globlin-setup-*.exe` drew 3–4 of 71 every time (`obfuscatedpoly`: Arctic Wolf, DeepInstinct,
SecureAge, Skyhigh) while `globlin.exe` and `globlin-arm64.exe` drew none. VirusTotal tags the installer
`overlay`, which suggested Inno's LZMA2-compressed payload. A one-off A/B on 2026-10-11 built the same
exe into two installers in CI and scanned both: `Compression=lzma2` (3.5 MB) drew 4 of 71,
`Compression=none` (6.1 MB) drew 3 of 71, the bare exe 1 of 70. Compression is not the cause — the
unsigned Inno Setup wrapper is, a loader that unpacks and starts another program — so `globlin.iss` keeps
`lzma2`. What would change the verdict is an Authenticode signature, not an installer setting.

The key is a free VirusTotal Community API key (`VIRUSTOTAL_API_KEY` in repository secrets), whose 4
requests/minute and 500/day are far more than one upload plus a handful of polls per release. Note that
uploading publishes the sample: VirusTotal shares files with the antivirus industry and with Intelligence
subscribers, and there is no way to withdraw one. That is acceptable here only because the same binary is
already a public release asset.

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

### Workflow hardening

Every third-party action is pinned to a full commit SHA with its release in a trailing `# vX.Y.Z`, because
a tag can be moved and the release jobs hand actions a write token, the PAT and `AUR_SSH_KEY`.
`.github/dependabot.yml` proposes SHA bumps monthly with a `ci:` prefix, so they never open a release.
Workflows default to `permissions: contents: read`; only `release.yml`'s `publish` job and the two
release-plz jobs get `contents: write`. Every `actions/checkout` sets `persist-credentials: false`, so a
build script, proc-macro or `rustup-init` running during a release cannot read a token out of
`.git/config`. `release.yml` passes the tag to its scripts as the `TAG` environment variable rather than
interpolating `${{ github.ref_name }}` into `run:` text.

## Other platforms

Windows is the main platform; macOS and Linux work and are experimental. `ci.yml` runs clippy and the
suite on Windows x64 and ARM64, macOS ARM64, and Linux x64 and ARM64. The seams:

- `src/platform/` — `windows.rs`, `macos.rs` and `linux.rs`, each real. `platform/mod.rs` holds what
  every arm shares (the notification-click hook, `BACKGROUND_FLAG`); `platform/unix.rs` holds what macOS
  and Linux share (the `File::try_lock` single-instance lock and the login-shell `PATH`).
- `src/window/shell.rs` serves every platform — WebView2, WKWebView, WebKitGTK; only `attach` differs,
  because on Linux the page goes into the GTK box tao gives the window (`build_gtk`) rather than onto a
  raw window handle.
- `#[cfg_attr(not(windows), allow(dead_code))]` on `icon::{ico, write_app_icon}` — the `.ico` and the toast
  artwork only Windows uses.
- `source::find_on_path` takes a bare name. On Windows it tries `exe`, `cmd`, `bat` in that order in each
  `PATH` directory, so the first directory holding any of them wins, as in a shell — not `PATHEXT`,
  which can list `.JS` or `.PS1` files `Command::new` cannot spawn. On Unix it requires the execute bit.
  Only npm's `default_location` (`%APPDATA%\npm\npm.cmd`) and `hidden_command` (`CREATE_NO_WINDOW`) are
  still `cfg`-split.
- `SourceKind::available_on(os)` — scoop, winget and choco exist only on Windows, Homebrew everywhere
  but Windows. `source::enabled` skips the rest, and the window's Settings (`window::snapshot_on`) does
  not list them at all, so a Mac never shows a scoop row and Windows never shows brew. The config keeps
  every field on every platform, so one `globlin.json` still parses anywhere.
- `install::managed_by` — `Manager::Winget` for a winget package directory, `Manager::SystemPackage` for
  a Linux binary under `/usr` (the deb and rpm install to `/usr/bin`). Either one skips the release
  lookup and tells the user which tool updates Globlin instead.
- `selfupdate::asset_for(os, arch, appimage)` — `windows/x86_64` → `globlin.exe`, `windows/aarch64` →
  `globlin-arm64.exe`, `macos/aarch64` → `globlin-macos-apple-silicon.zip`, and on Linux, only when
  running from an AppImage, `globlin-x86_64.AppImage` or `globlin-aarch64.AppImage`; anything else — a
  Linux package or a binary built from source included — has no build, and the lookup logs why instead of
  offering one. `globlin.exe` stays the x64 name forever: every released version
  looks for it by that exact name, and an x64 build running under emulation on ARM keeps updating to x64.
- `build.rs` — embeds the Windows resource only when the target is Windows.
- `platform::prepare_environment` runs first in `main` on every OS. On macOS and Linux it adopts the login
  shell's `PATH`. On Windows it restores `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `HOMEDRIVE` and
  `HOMEPATH` from `HKCU\Volatile Environment` when the launcher dropped them, and appends the registered
  machine and user `Path` entries (variables expanded) that the inherited `PATH` lacks — never reordering
  what is already there. A launcher with a stale or stripped environment therefore cannot hide npm, scoop
  or a shim's target.

The `App` — and with it the tray — is created on `StartCause::Init`, on every OS, because macOS refuses
a status item before its event loop runs. Two test quirks off Windows: the five `tray::menu` tests that
build a real menu are ignored on macOS, because muda creates menus only on the main thread and libtest
runs tests on worker threads; and the `install` tests are Windows-only, because their fixtures are
Windows paths.

### macOS

Experimental, Apple Silicon only, signed ad hoc (no Apple Developer ID, so no notarisation — the README
walks users through *Open Anyway*).

- **Bundle.** `.github/bundle-macos.sh` builds `Globlin.app`: an `Info.plist` with the `Cargo.toml`
  version, `LSUIElement` (no Dock icon, no app menu) and bundle id `dev.globlin.app`; an `.icns` that
  `iconutil` makes from the PNGs the ignored `icon::tests::dump_macos_iconset` renders (so the icon is
  still code); `codesign --force --sign -`; `ditto` into `globlin-macos-apple-silicon.zip`. `main.rs`
  also sets `ActivationPolicy::Accessory`, because self-update relaunches the executable directly rather
  than going through Launch Services. CI attaches the zip to every run, for testing a branch on a real Mac.
- **Dock while the window is open.** A menu-bar app whose window has no Dock icon is invisible to ⌘-Tab
  and gets lost behind other windows, so `window::show_in_dock` switches the policy to `Regular` (Dock
  icon, ⌘-Tab, app menu) right before the window is shown and back to `Accessory` when it is closed —
  `set_activation_policy_at_runtime` overrides `LSUIElement`. A Dock click while the window is open
  arrives as `Event::Reopen` and shows it again. A no-op off macOS.
- **PATH.** A GUI app on macOS inherits launchd's minimal `PATH`, without `/opt/homebrew/bin`,
  `~/.cargo/bin` or nvm. `platform::prepare_environment` runs first in `main`: it asks `$SHELL -i -l -c`
  to print `PATH` between markers (5 s timeout; the markers survive a noisy rc file) and puts those
  entries in front of the inherited ones, deduplicated.
- **Single instance** is `File::try_lock` on `globlin.lock` in the data directory; the handle lives in a
  `OnceLock` for the life of the process.
- **Notifications** go through `mac-notification-sys` under bundle id `dev.globlin.app`, each on its own
  thread with `wait_for_click`; a click fires the shared hook that opens the window.
- **Autostart** is a LaunchAgent, `~/Library/LaunchAgents/dev.globlin.app.plist`, whose presence is the
  setting. `SMAppService` would be the modern route, but it needs a properly signed app.
- **Config** always lives in `~/Library/Application Support/globlin/` — never next to the exe, which is
  inside the bundle.
- **Self-update** replaces the whole bundle: it downloads `globlin-macos-apple-silicon.zip` (the same
  file people install from), checks it against the signed `SHA256SUMS`, unpacks it with `ditto -x -k` into
  `Globlin.app.update` beside the running bundle, and swaps the bundles with the same rename-and-rollback
  as the Windows exe (`Globlin.app` → `Globlin.app.old`, the unpacked bundle → `Globlin.app`), then
  relaunches the executable at the same path. Swapping the whole `.app` keeps its ad-hoc seal and its
  `Info.plist` version consistent, which replacing only `Contents/MacOS/globlin` broke. Files Globlin
  writes itself carry no quarantine flag, so Gatekeeper does not ask again. `selfupdate::bundle_of` finds
  the bundle from the executable path; outside a `.app` the Mac build does not update itself, and
  `clean_stale` deletes `Globlin.app.old` and any leftover `Globlin.app.update` on the next start.

### Linux

Experimental, x64 and ARM64, shipped as a deb and an rpm.

- **Window.** `wry` on WebKitGTK 4.1, attached to tao's GTK box. Build dependencies: `libgtk-3-dev`,
  `libayatana-appindicator3-dev`, `libwebkit2gtk-4.1-dev` — every Linux CI job installs exactly those.
- **No libxdo.** `tray-icon` is built with `default-features = false, features = ["gtk"]`. Its default
  `libxdo` feature only makes muda's predefined Copy/Cut/Paste/Select All items work, and the tray uses
  none of them — only separators. Linked in, it tied the binary to the build machine's soname: the
  Ubuntu 22.04 release linked `libxdo.so.3`, Arch's `xdotool` ships `libxdo.so.4`, and `globlin-bin`
  0.4.3 failed at load with `libxdo.so.3: cannot open shared object file` before `main` ran. The deb and
  the AUR smoke tests now run `ldd /usr/bin/globlin` and fail on any `not found`.
- **Tray.** `tray-icon` uses Ayatana/libappindicator, which reports no clicks — any click opens the menu,
  and **Open Globlin** is the way to the window. GNOME shows no tray at all without the AppIndicator
  extension.
- **Notifications** go through `notify-rust` over D-Bus with a `default` action, so clicking the body
  fires the shared hook that opens the window (on desktops whose notification server supports actions).
- **Single instance and "show the window"** — the shared `globlin.lock`, plus a Unix socket,
  `globlin.sock` in the data directory: the running copy listens on it, and a second copy started by hand
  connects, writes `show` and exits. The socket is chmodded `0600` right after `bind`, so its mode does not
  depend on the umask; `show` is the only thing a connection can ask for.
- **Autostart** is `~/.config/autostart/globlin.desktop` (`$XDG_CONFIG_HOME` honoured) with
  `Exec="<exe>" --background`, the path quoted and escaped per the Desktop Entry spec.
- **PATH** — a desktop session often lacks what `.bashrc` adds (nvm, `~/.cargo/bin`), so Linux adopts the
  login shell's `PATH` exactly as macOS does.
- **Config** always lives in `$XDG_DATA_HOME/globlin` (`~/.local/share/globlin`), never next to the exe.

**Packaging.** `.github/package-linux.sh` builds the release binary, renders the hicolor icons through the
ignored `icon::tests::dump_linux_icons` (so they are still code), and runs `cargo deb` and
`cargo generate-rpm` against `[package.metadata.deb]` / `[package.metadata.generate-rpm]` in
`Cargo.toml`: `/usr/bin/globlin`, `packaging/linux/globlin.desktop`, and the icons under
`/usr/share/icons/hicolor`. The deb's library dependencies come from `$auto` (dpkg-shlibdeps) plus an
explicit `libayatana-appindicator3-1 | libappindicator3-1`, because tray-icon loads that library at
runtime and shlibdeps cannot see it; the rpm requires `libayatana-appindicator3.so.1()(64bit)` by soname,
which every rpm distribution can resolve whatever it names the package. Releases are built on
`ubuntu-22.04` and `ubuntu-22.04-arm`, so the packages need glibc 2.35, not the 2.39 of the CI runners.
`ci.yml` installs the deb with `apt`, which proves its dependencies resolve, checks the files landed, and
removes it again.

**Arch Linux** gets `globlin-bin` in the AUR rather than a release file. `packaging/aur/PKGBUILD.in` is a
template: `.github/render-pkgbuild.sh <version> <sha-amd64> <sha-arm64> <dir>` fills it in, and the
PKGBUILD downloads the release's deb for the machine's architecture, checks its sha256, and unpacks its
`data.tar` into the package — the same binary, desktop file and icons, with Arch package names for the
dependencies. `ci.yml` proves it on every push: `.github/aur-smoke.sh` renders the PKGBUILD around the
freshly built deb and, in an `archlinux:base-devel` container, runs `makepkg --syncdeps --install`, checks
the files, and removes the package again. `release.yml`'s `aur` job renders it from the two deb lines
of the published `SHA256SUMS` — and fails rather than render an empty checksum if either is missing —
and pushes it to `ssh://aur@aur.archlinux.org/globlin-bin.git` with
`KSXGitHub/github-actions-deploy-aur` — but only when the `AUR_SSH_KEY` secret exists; without it the job
does nothing. To switch it on: create an account on aur.archlinux.org, add an SSH public key to it, store
the private key as the repository secret `AUR_SSH_KEY`, and optionally set the repository variable
`AUR_COMMIT_EMAIL`. The first push creates the package. Independently of the AUR, the `publish` job renders
the same PKGBUILD from the release's own deb checksums and attaches it as `globlin-bin.PKGBUILD`, so Arch
users can `makepkg -si` it while the AUR package does not exist yet. Like the deb and rpm it installs to `/usr/bin`, so
`install::managed_by` reports a system package and self-update stays off.

**AppImage.** `.github/package-appimage.sh`, run at the end of `package-linux.sh`, lays out an AppDir
from the same binary, desktop file and hicolor icons — `AppRun` is a symlink to `usr/bin/globlin` — and
packs it with `appimagetool` 1.9.1 and the static type2 runtime 20251108, both downloaded from their
GitHub releases and checked against pinned SHA-256s before they run (`APPIMAGE_EXTRACT_AND_RUN=1`, so the
runner needs no FUSE). It is deliberately thin: no GTK, no WebKitGTK, no AppIndicator inside, about the
size of the binary, depending on the same system libraries as the packages. A bundled WebKitGTK (the
linuxdeploy-plugin-gtk route Tauri takes) would add 80–100 MB and break wherever WebKit's helper
processes, found by absolute path, or the bundled EGL/Mesa stack disagree with the host. `ci.yml`
unpacks it with `--appimage-extract` and runs the same `ldd` check as the deb.

The AppImage is the one Linux build that updates itself. The runtime sets `$APPIMAGE` to the `.AppImage`
file, while `current_exe` is the binary inside the read-only mount, so `platform::program_path()` — the
`$APPIMAGE` path on Linux when it is set, `current_exe` everywhere else — is what self-update swaps,
`relaunch` starts and `clean_stale` tidies next to, and what the autostart entry's `Exec` names, so moving
the AppImage and starting it once repairs autostart. `selfupdate::asset_for` offers
`globlin-<arch>.AppImage` only when `$APPIMAGE` is set, so a binary built from source can never be
replaced by an AppImage. The swap needs the AppImage's folder to be writable; in a read-only one it fails
and lands in the self-update log. Notifications ask for the `globlin` icon by name, which an AppImage
does not install into the icon theme, so they show the generic icon unless an integration tool such as
AppImageLauncher or Gear Lever has registered it.

Decided and deliberately not done: Flatpak (its sandbox would have to be opened to the host for every
command Globlin runs) and self-update for the packages (the package manager owns `/usr/bin`; a newer
release is a new package — the AppImage is the self-updating option).
