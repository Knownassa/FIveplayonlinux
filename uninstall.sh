#!/usr/bin/env bash
set -euo pipefail

install_root=${XDG_DATA_HOME:-"$HOME/.local/share"}
targets=(
  "$HOME/.local/bin/fiveplayonlinux"
  "$install_root/applications/io.github.Knownassa.FIveplayonlinux.desktop"
  "$install_root/icons/hicolor/scalable/apps/fiveplayonlinux.svg"
  "$install_root/metainfo/io.github.Knownassa.FIveplayonlinux.metainfo.xml"
)

for target in "${targets[@]}"; do
  if [[ -e "$target" ]]; then
    find "$target" -maxdepth 0 -delete
  fi
done

command -v update-desktop-database >/dev/null && update-desktop-database "$install_root/applications" || true
echo "Removed the application files. Local settings and browser data were kept."
