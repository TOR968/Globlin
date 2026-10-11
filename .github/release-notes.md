## Which file do I need?

| For | x64 | ARM64 |
| --- | --- | --- |
| Windows installer — recommended | [`globlin-setup-x64.exe`](https://github.com/TOR968/Globlin/releases/download/@TAG@/globlin-setup-x64.exe) | [`globlin-setup-arm64.exe`](https://github.com/TOR968/Globlin/releases/download/@TAG@/globlin-setup-arm64.exe) |
| Windows portable — no install | [`globlin.exe`](https://github.com/TOR968/Globlin/releases/download/@TAG@/globlin.exe) | [`globlin-arm64.exe`](https://github.com/TOR968/Globlin/releases/download/@TAG@/globlin-arm64.exe) |
| macOS 11+ (experimental) | — | [`Globlin-macos-arm64.zip`](https://github.com/TOR968/Globlin/releases/download/@TAG@/Globlin-macos-arm64.zip) |
| Debian, Ubuntu (experimental) | [`globlin-amd64.deb`](https://github.com/TOR968/Globlin/releases/download/@TAG@/globlin-amd64.deb) | [`globlin-arm64.deb`](https://github.com/TOR968/Globlin/releases/download/@TAG@/globlin-arm64.deb) |
| Fedora, openSUSE (experimental) | [`globlin-x86_64.rpm`](https://github.com/TOR968/Globlin/releases/download/@TAG@/globlin-x86_64.rpm) | [`globlin-aarch64.rpm`](https://github.com/TOR968/Globlin/releases/download/@TAG@/globlin-aarch64.rpm) |
| Arch Linux (experimental) | [`globlin-bin.PKGBUILD`](https://github.com/TOR968/Globlin/releases/download/@TAG@/globlin-bin.PKGBUILD) | same file |

Everything else in the list below is not for downloading by hand. `globlin-macos-arm64`, with no extension,
is the bare binary the installed Mac app updates itself from — macOS will not run it on its own. The
`.sha256` and `.minisig` files beside `globlin.exe`, `globlin-arm64.exe` and `globlin-macos-arm64` are what
the app checks before it installs an update of itself.

## Verify the download

`SHA256SUMS` lists the SHA-256 of every release file, and `SHA256SUMS.minisig` is its
[minisign](https://jedisct1.github.io/minisign/) signature by the Globlin release key — the same key the app
checks before it updates itself. Download both next to your file, then check the signature and the hash:

```sh
minisign -Vm SHA256SUMS -P RWTTLQhyI4Bn/MclkJqsthxavfiFPGVos6b7/d9keFthKYcrAmvZ31V+
sha256sum -c --ignore-missing SHA256SUMS
```

On macOS use `shasum -a 256 -c --ignore-missing SHA256SUMS` for the second line. On Windows without those
tools, this prints `True` when the file matches its line in `SHA256SUMS`:

```powershell
$file = 'globlin-setup-x64.exe'
(Get-Content SHA256SUMS) -contains "$((Get-FileHash $file -Algorithm SHA256).Hash.ToLower())  $file"
```

@CHANGELOG@

**Beta.** Globlin is still in beta on every platform — the installer, the ARM64 builds and the macOS and
Linux builds are new. Please [open an issue](https://github.com/TOR968/Globlin/issues) when something misbehaves.

A tray app for Windows, macOS and Linux that watches your global packages — npm, bun, pnpm, yarn, pipx,
uv, cargo, go, dotnet, gem, Homebrew, scoop, winget, choco and the PowerShell Gallery — notifies you when
one falls behind, and updates it on a click. It never updates a package on its own.

## Install

**Windows:** the installer sets Globlin up for your user without an administrator prompt and adds Start
menu and *Settings → Apps* entries. The portable exe keeps its config in a `globlin.json` next to itself,
so it can live on a USB stick or in any folder you like. Both update themselves. Enable *Run at startup*
from the tray menu if you want it to come back after a reboot.

**macOS:** unzip, move Globlin.app to Applications and open it. Choose **Done** when macOS says it could not
verify Globlin, then allow it once under *System Settings → Privacy & Security → Open Anyway* — it is signed
ad hoc, not notarised. If the button is missing, run `xattr -dr com.apple.quarantine /Applications/Globlin.app`.

**Linux:** `sudo apt install ./globlin-amd64.deb` or `sudo dnf install ./globlin-x86_64.rpm`. GNOME needs the
AppIndicator extension to show the tray icon. On Arch, save `globlin-bin.PKGBUILD` as `PKGBUILD` and run
`makepkg -si`. Packages do not update themselves — install the next one.

## Notes

- Click the tray icon, or pick *Open Globlin* from its right-click menu, for the window: every package in one list, filtered
  by status or by source, with bulk updates.
- npm, bun, pnpm and yarn are checked by default. winget and choco are switched off until you turn them
  on, and are reported rather than updated — both need an elevated process, and Globlin runs unelevated.
- `npm` and `@anthropic-ai/claude-code` are ignored by default, because the first rewrites its own shim
  mid-update on Windows and the second updates itself. Remove them from `ignore` in the config to manage
  them here.
- Checks run at startup and every six hours; a notification is only raised when the set of outdated
  packages actually changes.
- See the README for the full config reference and how to remove the app cleanly.
