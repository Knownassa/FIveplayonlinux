#!/usr/bin/env bash
set -euo pipefail

project_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
install_root=${XDG_DATA_HOME:-"$HOME/.local/share"}
bin_root=${HOME}/.local/bin
application_root=${install_root}/applications
icon_root=${install_root}/icons/hicolor/scalable/apps
metainfo_root=${install_root}/metainfo

if ! pkg-config --exists gtk4 2>/dev/null; then
  echo "GTK 4 is required. Install the GTK 4 runtime for your distribution." >&2
  exit 1
fi
if command -v ldconfig >/dev/null; then
  vulkan_cache=$(ldconfig -p 2>/dev/null || true)
  if [[ "$vulkan_cache" != *libvulkan.so.1* ]]; then
    echo "The Vulkan loader is required. Install it for your distribution." >&2
    exit 1
  fi
fi

binary=""
for candidate in \
  "$project_dir/target/release/fiveplayonlinux" \
  "$project_dir/usr/bin/fiveplayonlinux"
do
  if [[ -x "$candidate" ]]; then
    binary=$candidate
    break
  fi
done

if [[ -z "$binary" ]]; then
  if ! command -v cargo >/dev/null; then
    echo "No packaged binary or Cargo toolchain was found." >&2
    exit 1
  fi
  cargo build --release --locked --manifest-path "$project_dir/Cargo.toml"
  binary="$project_dir/target/release/fiveplayonlinux"
fi

desktop_source="$project_dir/io.github.Knownassa.FIveplayonlinux.desktop"
icon_source="$project_dir/assets/fiveplayonlinux.svg"
metainfo_source="$project_dir/io.github.Knownassa.FIveplayonlinux.metainfo.xml"
hud_source="$project_dir/assets/hud-extension"
if [[ -f "$project_dir/usr/share/applications/io.github.Knownassa.FIveplayonlinux.desktop" ]]; then
  desktop_source="$project_dir/usr/share/applications/io.github.Knownassa.FIveplayonlinux.desktop"
  icon_source="$project_dir/usr/share/icons/hicolor/scalable/apps/fiveplayonlinux.svg"
  metainfo_source="$project_dir/usr/share/metainfo/io.github.Knownassa.FIveplayonlinux.metainfo.xml"
  hud_source="$project_dir/usr/share/fiveplayonlinux/hud-extension"
fi

install -Dm755 "$binary" "$bin_root/fiveplayonlinux"
install -Dm644 "$desktop_source" "$application_root/io.github.Knownassa.FIveplayonlinux.desktop"
install -Dm644 "$icon_source" "$icon_root/fiveplayonlinux.svg"
install -Dm644 "$metainfo_source" "$metainfo_root/io.github.Knownassa.FIveplayonlinux.metainfo.xml"
install -Dm644 "$hud_source/manifest.json" "$install_root/fiveplayonlinux/hud-extension/manifest.json"
install -Dm644 "$hud_source/hud.js" "$install_root/fiveplayonlinux/hud-extension/hud.js"

command -v update-desktop-database >/dev/null && update-desktop-database "$application_root" || true
command -v gtk4-update-icon-cache >/dev/null && gtk4-update-icon-cache -f -t "${install_root}/icons/hicolor" >/dev/null || true

echo "Installed Play V2 Fiveplay for the current user."
