# FIveplayonlinux

FIveplayonlinux is a native Linux launch client for **Play V2 Fiveplay**. It
provides a polished GTK 4 interface, dedicated-GPU selection, WebGPU launch
tuning, an isolated browser profile, and lightweight host-level ad blocking.

The project deliberately contains no Fiveplay site link. On first run, paste
your secure launch address into the client. It is stored only in your local
configuration and is never sent anywhere except to the selected browser.

## Features

- Native GTK 4 launch panel with a compact dark interface.
- Custom draggable title bar with working minimize, maximize, restore, and close.
- Vulkan preflight checks and automatic compatible-GPU selection.
- NVIDIA PRIME and Mesa device routing with an integrated-GPU fallback.
- WebGPU, GPU rasterization, and zero-copy launch tuning.
- Balanced, high-performance, and memory-saver session profiles.
- Optional direct-to-fullscreen launch.
- Advertising hosts blocked before their scripts can connect.
- Separate browser profile without personal cookies, logins, tabs, or add-ons.
- No resident launcher overhead: the Rust process is replaced when play starts.

## Install

Release builds provide:

- `.deb` for Debian and Ubuntu based systems.
- `.rpm` for Fedora, RHEL, and compatible RPM systems.
- `.tar.gz` with a user-local installer for other x86_64 distributions.
- An Arch Linux `PKGBUILD` in `packaging/arch`.

For a source checkout, run:

```sh
./install.sh
```

The installer uses `~/.local` and does not need root access. GTK 4 and one
supported Chromium-family browser must be installed by the distribution.

## Build from source

Required development packages are Rust, Cargo, `pkg-config`, and GTK 4 headers.

```sh
cargo build --release --locked
cargo test --locked
```

Build the generic archive, Debian package, and RPM package:

```sh
./scripts/build-packages.sh
```

Artifacts are written to `dist/`.

## Command line

```text
fiveplayonlinux --play
fiveplayonlinux --graphics-check
fiveplayonlinux --fullscreen
fiveplayonlinux --performance
fiveplayonlinux --memory-saver
fiveplayonlinux --integrated
fiveplayonlinux --no-adblock
fiveplayonlinux --url <secure-launch-address>
fiveplayonlinux --dry-run
```

The saved launch address lives under the XDG configuration directory. Browser
data lives under the XDG data directory in a separate `fiveplayonlinux`
profile.

## License

MIT
