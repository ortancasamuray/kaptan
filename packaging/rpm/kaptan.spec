# Kaptan RPM spec (Fedora / openSUSE / RHEL-like).
# Build from source:     rpmbuild -ba kaptan.spec   (Source0 = kaptan-%%{version}.tar.gz)
# Repackage a binary:    rpmbuild -bb --define "_kaptan_prebuilt 1" --define "_sourcedir <tree>" kaptan.spec
#                        (packaging/build-packages.sh does this with target/release/kaptan)

Name:           kaptan
Version:        0.1.3
Release:        1%{?dist}
Summary:        Linux system optimizer and monitor (Rust port of Stacer)
License:        GPL-3.0-or-later
URL:            http://192.168.1.201:3100/os/kaptan
Source0:        %{name}-%{version}.tar.gz

%if 0%{!?_kaptan_prebuilt:1}
BuildRequires:  cargo >= 1.82
BuildRequires:  rust >= 1.82
BuildRequires:  gcc
BuildRequires:  pkgconfig(fontconfig)
BuildRequires:  pkgconfig(freetype2)
%endif

# Loaded at runtime with dlopen by the windowing / OpenGL backend (invisible to find-requires).
Requires:       libX11
Requires:       libX11-xcb
Requires:       libXcursor
Requires:       libXi
Requires:       libxkbcommon-x11
Requires:       (libglvnd-glx or Mesa-libGL1)
Requires:       procps-ng
Requires:       systemd
Recommends:     polkit
Recommends:     libnotify
Recommends:     libwayland-client
Recommends:     libwayland-egl
Recommends:     (libglvnd-egl or Mesa-libEGL1)
Suggests:       zenity
Suggests:       google-noto-sans-cjk-fonts

%global debug_package %{nil}
%global _build_id_links none

%description
Kaptan is a Linux system optimizer and monitor, a Rust and Slint port of
Stacer. It shows CPU, memory, disk, network and GPU usage with history
charts, manages processes, systemd services, startup applications and
packages, edits /etc/hosts, and cleans package caches, logs, crash reports,
application caches and the trash. Privileged actions use pkexec.

%prep
%if 0%{!?_kaptan_prebuilt:1}
%autosetup -n %{name}-%{version}
%endif

%build
%if 0%{!?_kaptan_prebuilt:1}
cargo build --release --locked
%endif

%install
%if 0%{?_kaptan_prebuilt:1}
cd %{_sourcedir}
%endif
install -Dm755 target/release/kaptan   %{buildroot}%{_bindir}/kaptan
install -Dm644 data/kaptan.desktop     %{buildroot}%{_datadir}/applications/kaptan.desktop
install -Dm644 data/kaptan.svg         %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/kaptan.svg
install -Dm644 debian/kaptan.1         %{buildroot}%{_mandir}/man1/kaptan.1
install -Dm644 README.md               %{buildroot}%{_docdir}/%{name}/README.md
install -Dm644 LICENSE                 %{buildroot}%{_licensedir}/%{name}/LICENSE

%files
%license %{_licensedir}/%{name}/LICENSE
%doc %{_docdir}/%{name}/README.md
%{_bindir}/kaptan
%{_datadir}/applications/kaptan.desktop
%{_datadir}/icons/hicolor/scalable/apps/kaptan.svg
%{_mandir}/man1/kaptan.1*

%changelog
* Mon Nov 23 2026 os <4129330+ortancasamuray@users.noreply.github.com> - 0.1.3-1
- About: links to the GitHub project and www.anadolupanteri.org.tr.

* Mon Nov 23 2026 os <4129330+ortancasamuray@users.noreply.github.com> - 0.1.2-1
- Dashboard: CPU and GPU temperature gauges with cold / hot / very hot zones.
- Dashboard layout: System Info and Fans end on the same line.
- Gauges no longer overstate small values; CPU usage is rounded correctly.
- Start at login opens the window.
- Packages for Debian, Fedora/openSUSE and Arch.

* Mon Nov 23 2026 os <4129330+ortancasamuray@users.noreply.github.com> - 0.1.1-1
- System tray icon, fan speeds, all-disks dialog, GPU usage alert.
- Uninstaller previews what the package manager will remove.
- 26 bundled translations.

* Sun Nov 22 2026 os <4129330+ortancasamuray@users.noreply.github.com> - 0.1.0-1
- Initial release: Rust + Slint port of Stacer.
