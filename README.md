# Kaptan

Linux system optimizer and monitor written in Rust with a [Slint](https://slint.dev) UI.
Kaptan is a port of [Stacer](https://github.com/QuentiumYT/Stacer) (C++/Qt).

## Features

| Page | What it does |
|---|---|
| Dashboard | CPU, memory, disk and GPU gauges, network speed, fan speeds, system info, CPU and GPU temperature gauges (cold / hot / very hot zones) |
| Resources | 60-second history charts: per-core CPU, load averages, memory/swap, GPU/VRAM, network, disk I/O; file system usage |
| Processes | Sortable, filterable process list; end a process (falls back to `pkexec`) |
| Startup Apps | Enable, disable, add, edit and delete `~/.config/autostart` entries |
| Services | Enable/disable at boot and start/stop systemd services (`pkexec`) |
| Uninstaller | Remove apt/dnf/yum/zypper/pacman and snap packages |
| System Cleaner | Scan and clean package caches, crash reports, logs, app caches, trash |
| Search | `find`-based file search with name, type, time, size, permission and owner filters |
| Hosts | Edit `/etc/hosts` |
| APT Repositories | Toggle, edit, remove (`.list` and deb822 `.sources`) and add repositories |
| Desktop | GNOME tweaks via `gsettings` (only shown on GNOME sessions) |
| Settings | Theme (dark/light), start page, dashboard disk, usage alerts, start at login |

Like Stacer, Kaptan has a system tray icon (StatusNotifierItem: KDE, XFCE 4.16+, GNOME with the
AppIndicator extension). Its menu opens any page or quits; closing the window asks whether to keep
running in the tray (Settings → When the window is closed). Started at login (Settings → Start Kaptan
at login), the window opens; `kaptan --hide` starts hidden in the tray. Without a tray host, closing
the window quits.

GPU usage is read from sysfs on AMD and from `nvidia-smi` on NVIDIA. On hybrid laptops a sleeping
NVIDIA GPU is not woken up. Intel iGPUs expose no usage counter and are not shown.

Temperatures come from hwmon (`coretemp`, `k10temp`, `zenpower`, ARM `cpu_thermal`, thermal zones as a
fallback) and, for the GPU, from `nvidia-smi` or the card's hwmon sensor (amdgpu, nouveau, xe). Zone
borders are in `src/core/temps.rs`: CPU 60 / 85 °C, GPU 60 / 80 °C.

## Build

```sh
cargo build --release
./target/release/kaptan
```

Runtime tools used when present: `pkexec` (privileged actions), `nvidia-smi`, `notify-send` (alerts),
`zenity` or `kdialog` (directory picker), `gio` (move to trash), `add-apt-repository`, `gsettings`.

## Tests

```sh
cargo test                                                   # unit tests
cargo test root_ -- --ignored --test-threads=1 --nocapture   # privileged actions (pkexec prompts)
```

The `root_` tests (`src/root_tests.rs`) change `/etc/hosts`, an APT source and a file in `/tmp`
through pkexec, then check that everything is restored byte for byte.

## Debian package

```sh
cargo install cargo-deb   # once
cargo deb                 # -> target/debian/kaptan_<version>-1_amd64.deb
sudo apt install ./target/debian/kaptan_*_amd64.deb
```

Package metadata is in `[package.metadata.deb]` in `Cargo.toml`; `debian/` holds the changelog,
copyright and man page. Bump the version in both `Cargo.toml` and `debian/changelog`.

## Packages for Debian, Fedora and Arch

```sh
packaging/build-packages.sh            # all of them, into dist/
packaging/build-packages.sh deb rpm    # or a subset: deb rpm arch src
```

| Output | Built with |
|---|---|
| `kaptan_<ver>-1_amd64.deb` | `cargo deb` |
| `kaptan-<ver>-1.x86_64.rpm` | `rpmbuild` with `packaging/rpm/kaptan.spec` (repackages the release binary) |
| `kaptan-<ver>-1-x86_64.pkg.tar.zst` | `packaging/arch/mkpkg.py` (no `makepkg` needed; reads `packaging/arch/PKGBUILD`) |
| `kaptan-<ver>.tar.gz` | source tarball for `rpmbuild -ba kaptan.spec` or `makepkg -si` |

The script stops if the spec or PKGBUILD version differs from `Cargo.toml`. Binaries built on
Debian 13 need glibc ≥ 2.39 (Ubuntu 24.04+, Fedora 40+, current Arch); for older systems build from
source with the spec or PKGBUILD. `docs/tanitim/` is a short Turkish introduction page with screenshots.

## Install (system-wide)

```sh
cargo build --release
sudo install -Dm755 target/release/kaptan /usr/local/bin/kaptan
sudo install -Dm644 data/kaptan.desktop /usr/local/share/applications/kaptan.desktop
sudo install -Dm644 data/kaptan.svg /usr/local/share/icons/hicolor/scalable/apps/kaptan.svg
```

Uninstall: `sudo rm /usr/local/bin/kaptan /usr/local/share/applications/kaptan.desktop /usr/local/share/icons/hicolor/scalable/apps/kaptan.svg`

## Install (user)

```sh
cargo build --release
install -Dm755 target/release/kaptan ~/.local/bin/kaptan
install -Dm644 data/kaptan.svg ~/.local/share/icons/hicolor/scalable/apps/kaptan.svg
# absolute Exec path: menus don't always have ~/.local/bin in PATH
sed "s|^Exec=kaptan$|Exec=$HOME/.local/bin/kaptan|" data/kaptan.desktop > ~/.local/share/applications/kaptan.desktop
update-desktop-database ~/.local/share/applications
```

Uninstall:

```sh
rm ~/.local/bin/kaptan ~/.local/share/applications/kaptan.desktop \
   ~/.local/share/icons/hicolor/scalable/apps/kaptan.svg
rm -r ~/.config/kaptan   # settings, optional
```

## Layout

```
src/core/    platform logic, no UI (ported from stacer-core)
src/pages/   one module per page: connects core to the UI
src/i18n.rs  translations for strings built in Rust
lang/        gettext catalogs
tools/       translation sources and catalog generator
ui/          Slint UI: theme, widgets, globals (data contract), pages, icons
```

Settings are stored in `~/.config/kaptan/settings.json`.

## Differences from Stacer

- Alerts use desktop notifications (`notify-send`) instead of tray balloons.
- Stacer's Unity/Compiz settings were replaced with current GNOME settings.
- No update check against GitHub.

## Translations

Kaptan ships with English and 26 more languages, the same set as Stacer: Arabic, Basque, Catalan,
Chinese (Simplified and Traditional), Czech, Dutch, French, Galician, German, Hindi, Hungarian,
Italian, Kannada, Korean, Malayalam, Occitan, Polish, Portuguese, Romanian, Russian, Spanish,
Swedish, Turkish, Ukrainian and Vietnamese. The language follows the system locale and can be
changed live in Settings → Language.

Where Stacer had the same text, its translators' work is reused (13–39 % of the strings depending
on the language). The rest was translated for Kaptan and has **not been reviewed by native
speakers** yet — corrections are welcome in `tools/translations/<code>.json`.

Sources: `tools/translations/<code>.json` (`{"English": "translation"}`), compiled into
`lang/<code>/LC_MESSAGES/kaptan.po` (bundled into the Slint UI, `@tr(...)`, and read by
`src/i18n.rs` for strings built in Rust). After adding or changing texts, run:

```sh
python3 tools/i18n.py   # regenerates all catalogs and src/i18n_languages.rs
```

Turkish must be complete (the tool fails otherwise); other languages fall back to English for
missing strings. To add a language, create `tools/translations/<code>.json` and add the code to
`LANGUAGES` in `tools/i18n.py`. `tools/stacer_ts.py` reads Stacer's Qt `.ts` catalogs.

Korean, Chinese and the Indic scripts need a font that covers them (e.g. `fonts-noto-cjk` or
`fonts-nanum` for Korean); otherwise characters show as boxes.

## License

GPL-3.0-or-later, same as Stacer.
