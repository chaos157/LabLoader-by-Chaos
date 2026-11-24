# LabLoader by Chaos

BONELAB mod manager for Windows PC and Meta Quest 3 / 3S.

## Features

- Multiple mod profiles with per-profile enable/disable state
- Thunderstore browsing and downloads
- Deploy mods into the game, uninstall, and manage custom/local mods
- `.blmp` profile import and export
- Optional proxy setup on PC
- Quest support: manager data lives in `/storage/emulated/0/BLMP`, and top-level
  `MelonLoader` / `LemonLoader` folders are detected and given `Mods`, `Plugins`,
  `UserLibs` and `UserData` subfolders

## Download

Windows installer: see **Releases** → `labloader release v1`
(`LabLoader-release-v1-setup.exe`).

The installer is per-user and does not require administrator rights.

## Build (Windows)

Requires [Bun](https://bun.sh) and the Rust toolchain.

```bash
bun install
bun run tauri build
```

Installers are written to `src-tauri/target/release/bundle/`
(`nsis/` for the `.exe` setup, `msi/` for the `.msi`).

## Quest build

The Android project under `src-tauri/gen/android` targets arm64 with
minSdk 24 / compileSdk 36. Building it needs the Android SDK, NDK 27 and JDK 17,
plus an arm64 Rust target:

```bash
rustup target add aarch64-linux-android
bun run tauri android build --target aarch64
```

Quest mods must be ARM-compatible; most Thunderstore BONELAB packages are x64 PC mods.
