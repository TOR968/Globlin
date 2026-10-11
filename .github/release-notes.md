**Beta.** Globlin is still in beta on every platform — the installer, the ARM64 builds and the macOS and
Linux builds are new. Please [open an issue](https://github.com/TOR968/Globlin/issues) when something misbehaves.

A tray app for Windows, macOS and Linux that watches your global packages — npm, bun, pnpm, yarn, pipx,
uv, cargo, go, dotnet, gem, Homebrew, scoop, winget, choco and the PowerShell Gallery — notifies you when
one falls behind, and updates it on a click. It never updates a package on its own.

## Install

Either run the installer — `globlin-setup-x64.exe`, or `globlin-setup-arm64.exe` on Windows on ARM —
which installs for your user without an administrator prompt and adds Start menu and *Settings → Apps*
entries; or download the portable `globlin.exe` (or `globlin-arm64.exe`) and run it. The portable one
keeps its config in a `globlin.json` next to itself, so it can live on a USB stick or in any folder you
like. Enable *Run at startup* from the tray menu if you want it to come back after a reboot.

**macOS (experimental, Apple Silicon):** unzip `Globlin-macos-arm64.zip` and move Globlin.app to
Applications. Open it, choose **Done** when macOS says it could not verify Globlin, then allow it once under
*System Settings → Privacy & Security → Open Anyway* — it is signed ad hoc, not notarised. If the button is
missing, run `xattr -dr com.apple.quarantine /Applications/Globlin.app`. Do not open `globlin-macos-arm64` itself — it is the bare binary the installed
app updates itself from, and macOS refuses to run it on its own.

**Linux (experimental):** `sudo apt install ./globlin-amd64.deb` (or `globlin-arm64.deb`) on Debian and
Ubuntu, `sudo dnf install ./globlin-x86_64.rpm` (or `globlin-aarch64.rpm`) on Fedora. GNOME needs the
AppIndicator extension to show the tray icon. On Arch, download `globlin-bin.PKGBUILD`, save it as
`PKGBUILD` and run `makepkg -si`. Packages do not update themselves — install the next one.

## Verify the download

```powershell
(Get-FileHash globlin.exe -Algorithm SHA256).Hash.ToLower()
```

Compare the result with the `.sha256` published beside the file you downloaded. Every file also has a
[minisign](https://jedisct1.github.io/minisign/) signature beside it, made with the Globlin release key —
the same key the app checks before it installs an update of itself:

```sh
minisign -Vm globlin.exe -P RWTTLQhyI4Bn/MclkJqsthxavfiFPGVos6b7/d9keFthKYcrAmvZ31V+
```

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
