# tools

A collection of small desktop tools. Each one lives in its own repository and is included here as a
git submodule.

| Tool | Description |
| ---- | ----------- |
| [disk-cleanup](https://github.com/brunolm/disk-cleanup) | Fast Windows disk usage analyzer (Rust): see which folders and files take the most space |
| [filefinder](https://github.com/brunolm/filefinder) | Simple family file search app (Electron + Rust) |
| [extract-images-from-video](https://github.com/brunolm/extract-images-from-video) | Desktop app (Tauri) for extracting images from a video |

## Clone

```powershell
git clone --recurse-submodules https://github.com/brunolm/tools.git
```

Already cloned without submodules:

```powershell
git submodule update --init --recursive
```

## Update every tool to its latest `main`

```powershell
git submodule update --remote --merge
git commit -am "Update tools"
```

## Add a tool

```powershell
git submodule add -b main https://github.com/brunolm/<repo>.git <repo>
```
