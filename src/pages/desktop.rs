//! Desktop (GNOME) settings page.

use super::{model, spawn, strings};
use crate::core::gsettings::{self, Kind, Setting};
use crate::{AppWindow, Desktop, DesktopSetting};
use crate::i18n::tr;
use slint::ComponentHandle;
use std::cell::RefCell;

#[derive(Default)]
struct State {
    loaded: bool,
    rows: Vec<Option<Setting>>, // None = group header
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// Show the page on GNOME-based sessions (Stacer only showed it on Ubuntu/Unity).
pub fn is_gnome() -> bool {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();
    gsettings::available() && (desktop.contains("gnome") || desktop.contains("ubuntu") || desktop.contains("unity"))
}

pub fn setup(ui: &AppWindow) {
    let g = ui.global::<Desktop>();

    let weak = ui.as_weak();
    g.on_set_bool(move |i, v| {
        if let Some(ui) = weak.upgrade() {
            apply(&ui, i, if v { "true".into() } else { "false".into() });
        }
    });

    let weak = ui.as_weak();
    g.on_set_int(move |i, v| {
        if let Some(ui) = weak.upgrade() {
            apply(&ui, i, v.to_string());
        }
    });

    let weak = ui.as_weak();
    g.on_set_option(move |i, idx| {
        let Some(ui) = weak.upgrade() else { return };
        let value = STATE.with_borrow(|s| match s.rows.get(i as usize) {
            Some(Some(Setting { kind: Kind::Enum(opts), .. })) => opts.get(idx as usize).map(|(_, v)| v.to_string()),
            _ => None,
        });
        if let Some(v) = value {
            apply(&ui, i, v);
        }
    });
}

fn apply(ui: &AppWindow, row: i32, value: String) {
    let Some(s) = STATE.with_borrow(|st| st.rows.get(row as usize).cloned().flatten()) else { return };
    let err = gsettings::set(s.schema, s.key, &value).err();
    ui.global::<Desktop>().set_message(err.unwrap_or_default().into());
}

/// Reload labels in the current language (only if the page was already loaded).
pub fn reload(ui: &AppWindow) {
    if STATE.with_borrow(|s| s.loaded) {
        STATE.with_borrow_mut(|s| s.loaded = false);
        load_once(ui);
    }
}

pub fn load_once(ui: &AppWindow) {
    if STATE.with_borrow(|s| s.loaded) {
        return;
    }
    STATE.with_borrow_mut(|s| s.loaded = true);
    ui.global::<Desktop>().set_loading(true);
    spawn(
        ui,
        || {
            gsettings::supported_settings()
                .into_iter()
                .map(|s| {
                    let v = gsettings::get(s.schema, s.key);
                    (s, v)
                })
                .collect::<Vec<_>>()
        },
        |ui, list| {
            let mut rows = Vec::new();
            let mut items = Vec::new();
            let mut group = "";
            for (s, value) in list {
                if s.group != group {
                    group = s.group;
                    rows.push(None);
                    items.push(DesktopSetting { kind: 0, label: tr(group).to_uppercase().into(), ..Default::default() });
                }
                let item = match &s.kind {
                    Kind::Bool => DesktopSetting { kind: 1, label: tr(s.label).into(), bool_value: value == "true", ..Default::default() },
                    Kind::Int { min, max } => DesktopSetting { kind: 2, label: tr(s.label).into(), int_value: value.parse().unwrap_or(*min), min: *min, max: *max, ..Default::default() },
                    Kind::Enum(opts) => DesktopSetting {
                        kind: 3,
                        label: tr(s.label).into(),
                        options: strings(opts.iter().map(|(l, _)| tr(l))),
                        option_index: opts.iter().position(|(_, v)| *v == value).map(|i| i as i32).unwrap_or(-1),
                        ..Default::default()
                    },
                };
                items.push(item);
                rows.push(Some(s));
            }
            STATE.with_borrow_mut(|s| s.rows = rows);
            let g = ui.global::<Desktop>();
            g.set_loading(false);
            g.set_settings(model(items));
        },
    );
}
