//! Startup Apps page: ~/.config/autostart entries.

use super::model;
use crate::core::startup::{self, StartupAppFields};
use crate::{AppWindow, Startup, StartupItem};
use crate::i18n::tr;
use slint::ComponentHandle;
use std::path::Path;

pub fn setup(ui: &AppWindow) {
    let g = ui.global::<Startup>();

    let weak = ui.as_weak();
    g.on_toggle(move |path, on| {
        if let Some(ui) = weak.upgrade() {
            let err = startup::set_enabled(Path::new(path.as_str()), on).err();
            ui.global::<Startup>().set_message(err.map(|e| e.to_string()).unwrap_or_default().into());
            reload(&ui);
        }
    });

    let weak = ui.as_weak();
    g.on_remove(move |path| {
        if let Some(ui) = weak.upgrade() {
            let err = startup::remove(Path::new(path.as_str())).err();
            ui.global::<Startup>().set_message(err.map(|e| e.to_string()).unwrap_or_default().into());
            reload(&ui);
        }
    });

    let weak = ui.as_weak();
    g.on_open_editor(move |path| {
        let Some(ui) = weak.upgrade() else { return };
        let g = ui.global::<Startup>();
        let f = if path.is_empty() { StartupAppFields::default() } else { startup::read_fields(Path::new(path.as_str())) };
        g.set_editor_path(path);
        g.set_f_name(f.name.into());
        g.set_f_generic(f.generic_name.into());
        g.set_f_comment(f.comment.into());
        g.set_f_icon(f.icon.into());
        g.set_f_command(f.command.into());
        g.set_editor_error("".into());
        g.set_editor_open(true);
    });

    let weak = ui.as_weak();
    g.on_save(move || {
        let Some(ui) = weak.upgrade() else { return };
        let g = ui.global::<Startup>();
        let fields = StartupAppFields {
            name: g.get_f_name().trim().to_string(),
            generic_name: g.get_f_generic().trim().to_string(),
            comment: g.get_f_comment().trim().to_string(),
            icon: g.get_f_icon().trim().to_string(),
            command: g.get_f_command().trim().to_string(),
        };
        let path = g.get_editor_path();
        let target = (!path.is_empty()).then(|| Path::new(path.as_str()).to_path_buf());
        match startup::save(target.as_deref(), &fields) {
            Ok(()) => {
                g.set_editor_open(false);
                reload(&ui);
            }
            Err(e) => g.set_editor_error(tr(&e).into()),
        }
    });

    reload(ui);
}

pub fn reload(ui: &AppWindow) {
    let apps = startup::list()
        .into_iter()
        .map(|a| StartupItem { name: a.name.into(), enabled: a.enabled, path: a.path.to_string_lossy().into_owned().into() })
        .collect();
    ui.global::<Startup>().set_apps(model(apps));
}
