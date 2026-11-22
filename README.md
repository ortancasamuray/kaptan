# Kaptan

Linux system optimizer and monitor written in Rust with a [Slint](https://slint.dev) UI.
Kaptan is a port of [Stacer](https://github.com/QuentiumYT/Stacer) (C++/Qt).

## Features

| Page | What it does |
|---|---|
| Dashboard | CPU, memory, disk and GPU gauges, network speed, system info |
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

GPU usage is read from sysfs on AMD and from `nvidia-smi` on NVIDIA. On hybrid laptops a sleeping
NVIDIA GPU is not woken up. Intel iGPUs expose no usage counter and are not shown.

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
tools/       translation catalog generator
ui/          Slint UI: theme, widgets, globals (data contract), pages, icons
```

Settings are stored in `~/.config/kaptan/settings.json`.

## Differences from Stacer

- No tray icon (Slint has no tray support); alerts use desktop notifications.
- Stacer's Unity/Compiz settings were replaced with current GNOME settings.
- No update check against GitHub.

## Translations

Kaptan ships with English and Turkish. The language follows the system locale and can be changed
live in Settings → Language.

`lang/<code>/LC_MESSAGES/kaptan.po` is bundled into the Slint UI at build time (`@tr(...)`) and is also
read by `src/i18n.rs` for strings built in Rust (`tr()` / `trf()`). After adding or changing texts, run:

```sh
python3 tools/tr_catalog.py   # regenerates the Turkish catalog, fails on missing translations
```

To add a language, copy the Turkish catalog to `lang/<code>/LC_MESSAGES/kaptan.po`, translate it and
add the code to `LANGUAGES` in `src/i18n.rs`.

## License

GPL-3.0-or-later, same as Stacer.
