//! Kaptan — Linux system optimizer and monitor (Rust + Slint port of Stacer).

mod core;
mod i18n;
mod pages;
#[cfg(test)]
mod root_tests;
mod tray;

slint::include_modules!();

use slint::{ComponentHandle, Image, SharedString};

/// Pages shown in the sidebar: (id, label, icon file).
fn nav_items() -> Vec<(&'static str, &'static str, Image)> {
    macro_rules! icon {
        ($f:literal) => {
            Image::load_from_svg_data(include_bytes!(concat!("../ui/icons/", $f, ".svg"))).unwrap_or_default()
        };
    }
    let flatpak = core::system::is_flatpak();
    let mut v = vec![("dashboard", "Dashboard", icon!("dashboard")), ("startup", "Startup Apps", icon!("startup")), ("cleaner", "System Cleaner", icon!("cleaner")), ("search", "Search", icon!("search"))];
    if !flatpak {
        v.push(("services", "Services", icon!("services")));
        v.push(("processes", "Processes", icon!("processes")));
        v.push(("uninstaller", "Uninstaller", icon!("uninstaller")));
    }
    v.push(("resources", "Resources", icon!("resources")));
    v.push(("helpers", "Hosts", icon!("helpers")));
    if core::apt_source::supported() {
        v.push(("apt", "APT Repositories", icon!("apt")));
    }
    if pages::desktop::is_gnome() {
        v.push(("desktop", "Desktop", icon!("desktop")));
    }
    v.push(("settings", "Settings", icon!("settings")));
    v
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    let settings = core::settings::Settings::load();
    core::settings::migrate_self_autostart();

    i18n::set_language(&settings.language);

    let nav = nav_items();
    let app = ui.global::<App>();
    app.set_version(env!("CARGO_PKG_VERSION").into());
    pages::set_nav(&ui, nav.iter().map(|(id, label, icon)| (id.to_string(), label.to_string(), icon.clone())).collect());
    let start = if nav.iter().any(|(id, ..)| *id == settings.start_page) { settings.start_page.clone() } else { "dashboard".into() };
    app.set_current_page(SharedString::from(start.as_str()));
    ui.global::<Theme>().set_dark(settings.theme != "light");
    app.on_open_url(|url| {
        let _ = std::process::Command::new("xdg-open").arg(url.as_str()).spawn();
    });


    let monitor = pages::monitor::setup(&ui, &settings);
    pages::disks::setup(&ui);
    pages::processes::setup(&ui);
    pages::startup::setup(&ui);
    pages::services::setup(&ui);
    pages::uninstaller::setup(&ui);
    pages::cleaner::setup(&ui);
    pages::search::setup(&ui);
    pages::hosts::setup(&ui);
    pages::apt::setup(&ui);
    pages::desktop::setup(&ui);
    let close_action = settings.close_action.clone();
    pages::settings::setup(&ui, settings, monitor);
    let tray = pages::window::setup(&ui, &close_action);

    // Pages that load lazily get notified when they are first opened.
    let weak = ui.as_weak();
    app.on_page_changed(move |id| {
        if let Some(ui) = weak.upgrade() {
            pages::on_page_opened(&ui, id.as_str());
        }
    });
    pages::on_page_opened(&ui, &start);

    // `kaptan --hide` starts in the tray (only if there is one); autostart opens the window.
    let start_hidden = tray && std::env::args().any(|a| a == "--hide");
    if !start_hidden {
        ui.show()?;
    }
    // Keeps running while the window is hidden in the tray; ends on quit_event_loop().
    slint::run_event_loop_until_quit()?;
    pages::window::quit();
    Ok(())
}
