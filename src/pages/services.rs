//! Services page: systemd units, enable/disable and start/stop through pkexec.

use super::{model, spawn};
use crate::core::service::{self, Service};
use crate::{AppWindow, ServiceItem, Services};
use crate::i18n::trf;
use slint::ComponentHandle;
use std::cell::RefCell;

thread_local! {
    static LIST: RefCell<Option<Vec<Service>>> = const { RefCell::new(None) };
}

pub fn setup(ui: &AppWindow) {
    let g = ui.global::<Services>();

    let weak = ui.as_weak();
    g.on_filter_changed(move || {
        if let Some(ui) = weak.upgrade() {
            render(&ui);
        }
    });

    let weak = ui.as_weak();
    g.on_refresh(move || {
        if let Some(ui) = weak.upgrade() {
            load(&ui);
        }
    });

    let weak = ui.as_weak();
    g.on_set_enabled(move |name, on| {
        if let Some(ui) = weak.upgrade() {
            change(&ui, name.to_string(), move |n| service::set_enabled(n, on), if on { "{} enabled." } else { "{} disabled." });
        }
    });

    let weak = ui.as_weak();
    g.on_set_running(move |name, on| {
        if let Some(ui) = weak.upgrade() {
            change(&ui, name.to_string(), move |n| service::set_running(n, on), if on { "{} started." } else { "{} stopped." });
        }
    });
}

fn change(ui: &AppWindow, name: String, op: impl FnOnce(&str) -> Result<(), String> + Send + 'static, done_fmt: &'static str) {
    spawn(
        ui,
        move || {
            let res = op(&name);
            (name.clone(), res, service::is_enabled(&name), service::is_active(&name))
        },
        move |ui, (name, res, enabled, active)| {
            let g = ui.global::<Services>();
            match res {
                Ok(()) => {
                    g.set_message(trf(done_fmt, &[&name]).into());
                    g.set_message_error(false);
                }
                Err(e) => {
                    g.set_message(format!("{name}: {e}").into());
                    g.set_message_error(true);
                }
            }
            LIST.with_borrow_mut(|l| {
                if let Some(s) = l.as_mut().and_then(|l| l.iter_mut().find(|s| s.name == name)) {
                    s.enabled = enabled;
                    s.active = active;
                }
            });
            render(&ui); // fresh model also resets switches when the action was cancelled
        },
    );
}

pub fn load_once(ui: &AppWindow) {
    if LIST.with_borrow(|l| l.is_none()) {
        load(ui);
    }
}

fn load(ui: &AppWindow) {
    ui.global::<Services>().set_loading(true);
    spawn(ui, service::list, |ui, list| {
        LIST.with_borrow_mut(|l| *l = Some(list));
        ui.global::<Services>().set_loading(false);
        render(&ui);
    });
}

fn render(ui: &AppWindow) {
    let g = ui.global::<Services>();
    let filter = g.get_filter().to_lowercase();
    let state = g.get_state_filter();
    let (items, total) = LIST.with_borrow(|l| {
        let l = l.as_deref().unwrap_or_default();
        let items: Vec<ServiceItem> = l
            .iter()
            .filter(|s| filter.is_empty() || s.name.to_lowercase().contains(&filter) || s.description.to_lowercase().contains(&filter))
            .filter(|s| match state {
                1 => s.active,
                2 => !s.active,
                3 => s.enabled,
                4 => !s.enabled,
                _ => true,
            })
            .map(|s| ServiceItem { name: s.name.clone().into(), description: s.description.clone().into(), enabled: s.enabled, active: s.active })
            .collect();
        (items, l.len())
    });
    g.set_total(total as i32);
    g.set_services(model(items));
}
