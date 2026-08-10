A portable Windows tray app that watches your global npm and bun packages, notifies you when one falls
behind, and updates it on a click. It never updates anything on its own.

## Install

Download `npm-globals-tray.exe` and run it. There is no installer — it keeps its config in a
`npm-globals-tray.json` next to itself, so it can live on a USB stick or in any folder you like. Enable
*Run at startup* from the tray menu if you want it to come back after a reboot.

## Verify the download

```powershell
(Get-FileHash npm-globals-tray.exe -Algorithm SHA256).Hash.ToLower()
```

Compare the result with `npm-globals-tray.exe.sha256`.

## Notes

- `npm` and `@anthropic-ai/claude-code` are ignored by default, because the first rewrites its own shim
  mid-update on Windows and the second updates itself. Remove them from `ignore` in the config to manage
  them here.
- Checks run at startup and every six hours; a notification is only raised when the set of outdated
  packages actually changes.
- See the README for the full config reference and how to remove the app cleanly.
