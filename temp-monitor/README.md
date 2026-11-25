# temp-monitor

Small Windows desktop app (Rust + egui) that shows CPU and GPU temperatures, refreshed every second.

- **CPU**: per-core and package temperature plus TjMax, read straight from Intel's digital thermal
  sensors (the same registers Core Temp reads). P-cores and E-cores are labelled on hybrid CPUs.
- **GPU**: temperature of every GPU whose driver reports it, read from the WDDM adapter performance data,
  the same vendor-neutral source Task Manager uses. Integrated GPUs usually don't report one.

## Requirements

On most machines Windows doesn't expose CPU temperature: it lives in per-core CPU registers (MSRs) that
only a kernel driver can read. The app uses [PawnIO](https://pawnio.eu), a signed driver that only runs
signed modules restricted to specific registers, so no other monitoring tool needs to be running.

```powershell
winget install namazso.PawnIO
```

- **Intel CPUs only** for now (PawnIO's `IntelMSR` module).
- **Runs as administrator.** PawnIO only accepts calls from elevated processes, so the exe embeds a
  `requireAdministrator` manifest and Windows shows a UAC prompt on launch. Run `cargo` commands from an
  elevated terminal.
- The GUI renders on the low-power GPU so a discrete GPU can stay asleep in hybrid graphics mode.

`pawnio/IntelMSR.bin` is the signed module from
[PawnIO.Modules](https://github.com/namazso/PawnIO.Modules) 0.2.12, licensed under LGPL-2.1
(`pawnio/COPYING`).

## Run

```powershell
mise run run      # or: cargo run --release
mise run build    # target/release/temp-monitor.exe
mise run lint
mise run test
```
