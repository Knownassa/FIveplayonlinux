Name:           fiveplayonlinux
Version:        @VERSION@
Release:        1%{?dist}
Summary:        Native Linux client for Play V2 Fiveplay
License:        MIT
BuildArch:      x86_64
Requires:       gtk4
Requires:       vulkan-loader
Recommends:     chromium
Source0:        fiveplayonlinux
Source1:        io.github.Knownassa.FIveplayonlinux.desktop
Source2:        fiveplayonlinux.svg
Source3:        LICENSE
Source4:        io.github.Knownassa.FIveplayonlinux.metainfo.xml
Source5:        hud-manifest.json
Source6:        hud.js

%description
Dedicated-GPU selection, WebGPU tuning, a collapsible performance HUD,
an isolated profile, and ad blocking in a compact GTK 4 interface.

%prep

%build

%install
install -Dm755 %{SOURCE0} %{buildroot}%{_bindir}/fiveplayonlinux
install -Dm644 %{SOURCE1} %{buildroot}%{_datadir}/applications/io.github.Knownassa.FIveplayonlinux.desktop
install -Dm644 %{SOURCE2} %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/fiveplayonlinux.svg
install -Dm644 %{SOURCE3} %{buildroot}%{_licensedir}/fiveplayonlinux/LICENSE
install -Dm644 %{SOURCE4} %{buildroot}%{_datadir}/metainfo/io.github.Knownassa.FIveplayonlinux.metainfo.xml
install -Dm644 %{SOURCE5} %{buildroot}%{_datadir}/fiveplayonlinux/hud-extension/manifest.json
install -Dm644 %{SOURCE6} %{buildroot}%{_datadir}/fiveplayonlinux/hud-extension/hud.js

%files
%license %{_licensedir}/fiveplayonlinux/LICENSE
%{_bindir}/fiveplayonlinux
%{_datadir}/applications/io.github.Knownassa.FIveplayonlinux.desktop
%{_datadir}/icons/hicolor/scalable/apps/fiveplayonlinux.svg
%{_datadir}/metainfo/io.github.Knownassa.FIveplayonlinux.metainfo.xml
%{_datadir}/fiveplayonlinux/hud-extension/manifest.json
%{_datadir}/fiveplayonlinux/hud-extension/hud.js

%changelog
* Sat Oct 31 2026 FIveplayonlinux contributors - @VERSION@-1
- Initial package
