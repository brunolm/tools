# tools launcher

Tiny Windows launcher panel (Rust + plain Win32, ~200 KB) with one button per app in this repo.
Pin `target/release/tools.exe` to the taskbar and start any tool from there.

A tool whose binary isn't built yet shows as `(not built)` and is disabled; the list re-checks
whenever the launcher window gets focus. Tools are listed in [`src/tools.rs`](src/tools.rs):

| Tool | Binary | How to build it |
| ---- | ------ | --------------- |
| Disk Cleanup | `disk-cleanup/target/release/disk-cleanup.exe` | `mise run build` |
| Extract Images from Video | `extract-images-from-video/src-tauri/target/release/extract-images-from-video.exe` | `npm install; npx tauri build --no-bundle` |
| File Finder | `filefinder/node_modules/electron/dist/electron.exe .` | `npm install; npm run build:rust` |
| Temp Monitor | `temp-monitor/target/release/temp-monitor.exe` | `mise run build` |

The repo root is baked in at compile time, so the launcher must run from this checkout.
The window reopens where it was last closed (saved under `HKCU\Software\brunolm\tools`).

## Run

```powershell
mise run run      # or: cargo run --release
mise run build    # target/release/tools.exe
mise run lint
```

Building needs `rc.exe` from the Windows SDK to embed the icon and manifest.
