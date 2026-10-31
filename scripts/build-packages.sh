#!/usr/bin/env bash
set -euo pipefail

project_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$project_dir/Cargo.toml" | head -n 1)
out_dir="$project_dir/dist"
build_dir="$out_dir/.build"
package_name="fiveplayonlinux-${version}-x86_64"

mkdir -p "$out_dir"
if [[ -d "$build_dir" ]]; then
  find "$build_dir" -mindepth 1 -delete
fi
mkdir -p "$build_dir"

cargo build --release --locked --manifest-path "$project_dir/Cargo.toml"
binary="$project_dir/target/release/fiveplayonlinux"

stage="$build_dir/$package_name"
install -Dm755 "$binary" "$stage/usr/bin/fiveplayonlinux"
install -Dm644 "$project_dir/io.github.Knownassa.FIveplayonlinux.desktop" \
  "$stage/usr/share/applications/io.github.Knownassa.FIveplayonlinux.desktop"
install -Dm644 "$project_dir/assets/fiveplayonlinux.svg" \
  "$stage/usr/share/icons/hicolor/scalable/apps/fiveplayonlinux.svg"
install -Dm644 "$project_dir/io.github.Knownassa.FIveplayonlinux.metainfo.xml" \
  "$stage/usr/share/metainfo/io.github.Knownassa.FIveplayonlinux.metainfo.xml"
install -Dm644 "$project_dir/assets/hud-extension/manifest.json" \
  "$stage/usr/share/fiveplayonlinux/hud-extension/manifest.json"
install -Dm644 "$project_dir/assets/hud-extension/hud.js" \
  "$stage/usr/share/fiveplayonlinux/hud-extension/hud.js"
install -Dm644 "$project_dir/LICENSE" "$stage/usr/share/licenses/fiveplayonlinux/LICENSE"
install -Dm644 "$project_dir/README.md" "$stage/README.md"
install -Dm755 "$project_dir/install.sh" "$stage/install.sh"
install -Dm755 "$project_dir/uninstall.sh" "$stage/uninstall.sh"
tar -C "$build_dir" -czf "$out_dir/${package_name}.tar.gz" "$package_name"

deb_root="$build_dir/deb"
mkdir -p "$deb_root/DEBIAN"
cp -a "$stage/usr" "$deb_root/usr"
sed -e "s/@VERSION@/$version/g" "$project_dir/packaging/debian/control" > "$deb_root/DEBIAN/control"
deb_work="$build_dir/deb-archive"
mkdir -p "$deb_work"
printf '2.0\n' > "$deb_work/debian-binary"
tar -C "$deb_root/DEBIAN" --owner=0 --group=0 -czf "$deb_work/control.tar.gz" .
tar -C "$deb_root" --exclude=DEBIAN --owner=0 --group=0 -czf "$deb_work/data.tar.gz" .
(cd "$deb_work" && ar r "$out_dir/fiveplayonlinux_${version}_amd64.deb" debian-binary control.tar.gz data.tar.gz)

if command -v rpmbuild >/dev/null; then
  rpm_root="$build_dir/rpmbuild"
  mkdir -p "$rpm_root/BUILD" "$rpm_root/BUILDROOT" "$rpm_root/RPMS" "$rpm_root/SOURCES" "$rpm_root/SPECS" "$rpm_root/SRPMS"
  install -Dm755 "$binary" "$rpm_root/SOURCES/fiveplayonlinux"
  install -Dm644 "$project_dir/io.github.Knownassa.FIveplayonlinux.desktop" "$rpm_root/SOURCES/io.github.Knownassa.FIveplayonlinux.desktop"
  install -Dm644 "$project_dir/assets/fiveplayonlinux.svg" "$rpm_root/SOURCES/fiveplayonlinux.svg"
  install -Dm644 "$project_dir/LICENSE" "$rpm_root/SOURCES/LICENSE"
  install -Dm644 "$project_dir/io.github.Knownassa.FIveplayonlinux.metainfo.xml" "$rpm_root/SOURCES/io.github.Knownassa.FIveplayonlinux.metainfo.xml"
  install -Dm644 "$project_dir/assets/hud-extension/manifest.json" "$rpm_root/SOURCES/hud-manifest.json"
  install -Dm644 "$project_dir/assets/hud-extension/hud.js" "$rpm_root/SOURCES/hud.js"
  sed "s/@VERSION@/$version/g" "$project_dir/packaging/fiveplayonlinux.spec" > "$rpm_root/SPECS/fiveplayonlinux.spec"
  rpmbuild -bb --define "_topdir $rpm_root" "$rpm_root/SPECS/fiveplayonlinux.spec"
  find "$rpm_root/RPMS" -type f -name '*.rpm' -exec cp {} "$out_dir/" \;
else
  echo "rpmbuild not found; RPM was skipped" >&2
fi

printf 'Packages written to %s\n' "$out_dir"
