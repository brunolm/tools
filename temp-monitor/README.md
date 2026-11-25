# temp-monitor

Small Windows desktop app (Rust + egui) that shows CPU and GPU temperatures, refreshed every second.

- **CPU**: per-core temperature, load, and TjMax, read from [Core Temp](https://www.alcpu.com/CoreTemp/)'s
  shared-memory plugin interface. Core Temp must be running (the app offers a button to start it).
- **GPU**: temperature of every GPU whose driver reports it, read from the WDDM adapter performance data —
  the same vendor-neutral source Task Manager uses. Integrated GPUs usually don't report one.

The app itself needs no admin rights and no driver of its own.

## Why Core Temp

On most machines Windows doesn't expose CPU temperature: the values live in per-core CPU registers (MSRs)
that only a kernel driver can read. Core Temp ships a signed driver that does this, so the app reads its
results instead of installing a driver itself.

```powershell
winget install ALCPU.CoreTemp
```

## Run

```powershell
mise run run      # or: cargo run --release
mise run build    # target/release/temp-monitor.exe
mise run lint
mise run test
```
