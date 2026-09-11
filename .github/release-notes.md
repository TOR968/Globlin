A portable Windows tray app that watches your global packages — npm, bun, pnpm, yarn, winget and choco —
notifies you when one falls behind, and updates it on a click. It never updates anything on its own.

## Install

Download `globlin.exe` and run it. There is no installer — it keeps its config in a
`globlin.json` next to itself, so it can live on a USB stick or in any folder you like. Enable
*Run at startup* from the tray menu if you want it to come back after a reboot.

## Verify the download

```powershell
(Get-FileHash globlin.exe -Algorithm SHA256).Hash.ToLower()
```

Compare the result with `globlin.exe.sha256`.

## Notes

- Double-click the tray icon, or pick *Open Globlin*, for the window: every package in one list, filtered
  by status or by source, with bulk updates.
- npm, bun, pnpm and yarn are checked by default. winget and choco are switched off until you turn them
  on, and are reported rather than updated — both need an elevated process, and Globlin runs unelevated.
- `npm` and `@anthropic-ai/claude-code` are ignored by default, because the first rewrites its own shim
  mid-update on Windows and the second updates itself. Remove them from `ignore` in the config to manage
  them here.
- Checks run at startup and every six hours; a notification is only raised when the set of outdated
  packages actually changes.
- See the README for the full config reference and how to remove the app cleanly.
