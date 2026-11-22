//! Uninstaller page: system and snap packages.

use super::{model, spawn, strings};
use crate::core::package;
use crate::{AppWindow, PackageItem, Uninstaller};
use crate::i18n::{tr, trf};
use slint::{ComponentHandle, Model};
use std::cell::RefCell;
use std::collections::BTreeSet;

#[derive(Default)]
struct State {
    loaded: bool,
    system: Vec<String>,
    snaps: Vec<String>,
    checked_system: BTreeSet<String>,
    checked_snaps: BTreeSet<String>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

pub fn setup(ui: &AppWindow) {
    let g = ui.global::<Uninstaller>();
    g.set_snap_available(package::snap_available());

    let weak = ui.as_weak();
    g.on_filter_changed(move || {
        if let Some(ui) = weak.upgrade() {
            render(&ui);
        }
    });

    let weak = ui.as_weak();
    g.on_toggle(move |name, on| {
        let Some(ui) = weak.upgrade() else { return };
        let snap = ui.global::<Uninstaller>().get_snap_tab();
        STATE.with_borrow_mut(|s| {
            let set = if snap { &mut s.checked_snaps } else { &mut s.checked_system };
            if on { set.insert(name.to_string()) } else { set.remove(name.as_str()) };
        });
        let packages = ui.global::<Uninstaller>().get_packages();
        if let Some(i) = packages.iter().position(|p| p.name == name) {
            packages.set_row_data(i, PackageItem { name, checked: on });
        }
        update_count(&ui);
    });

    // Step 1: dry run, show exactly what would be removed.
    let weak = ui.as_weak();
    g.on_uninstall(move || {
        let Some(ui) = weak.upgrade() else { return };
        let snap = ui.global::<Uninstaller>().get_snap_tab();
        let pkgs = selected(snap);
        if pkgs.is_empty() {
            return;
        }
        ui.global::<Uninstaller>().set_busy(true);
        spawn(
            &ui,
            move || {
                let res = if snap { Ok(pkgs.clone()) } else { package::simulate_removal(&pkgs) };
                (pkgs, res)
            },
            |ui, (pkgs, res)| {
                let g = ui.global::<Uninstaller>();
                g.set_busy(false);
                match res {
                    Ok(list) if !list.is_empty() => {
                        // "libfoo:amd64" (dpkg selection) and "libfoo" (apt output) are the same package
                        let base = |p: &str| p.split(':').next().unwrap_or(p).to_string();
                        let chosen: Vec<String> = pkgs.iter().map(|p| base(p)).collect();
                        let extra = list.iter().filter(|p| !chosen.contains(&base(p))).count();
                        g.set_extra_count(extra as i32);
                        g.set_to_remove(strings(list));
                        g.set_confirm_open(true);
                    }
                    Ok(_) => {
                        g.set_message(tr("Nothing to remove.").into());
                        g.set_message_error(false);
                    }
                    Err(e) => {
                        g.set_message(tr(&e).into());
                        g.set_message_error(true);
                    }
                }
            },
        );
    });

    // Step 2: the user confirmed the list.
    let weak = ui.as_weak();
    g.on_confirm_uninstall(move || {
        let Some(ui) = weak.upgrade() else { return };
        let g = ui.global::<Uninstaller>();
        g.set_confirm_open(false);
        let snap = g.get_snap_tab();
        let pkgs = selected(snap);
        if pkgs.is_empty() {
            return;
        }
        g.set_busy(true);
        g.set_message(trf("Uninstalling {} package(s)…", &[&pkgs.len()]).into());
        g.set_message_error(false);
        spawn(
            &ui,
            move || {
                let r = if snap { package::uninstall_snaps(&pkgs) } else { package::uninstall(&pkgs) };
                (r, pkgs.len())
            },
            move |ui, (res, n)| {
                let g = ui.global::<Uninstaller>();
                g.set_busy(false);
                match res {
                    Ok(()) => {
                        g.set_message(trf("{} package(s) removed.", &[&n]).into());
                        g.set_message_error(false);
                        STATE.with_borrow_mut(|s| if snap { s.checked_snaps.clear() } else { s.checked_system.clear() });
                    }
                    Err(e) => {
                        g.set_message(tr(&e).into());
                        g.set_message_error(true);
                    }
                }
                load(&ui);
            },
        );
    });
}

fn selected(snap: bool) -> Vec<String> {
    STATE.with_borrow(|s| if snap { &s.checked_snaps } else { &s.checked_system }.iter().cloned().collect())
}

pub fn load_once(ui: &AppWindow) {
    if !STATE.with_borrow(|s| s.loaded) {
        load(ui);
    }
}

fn load(ui: &AppWindow) {
    STATE.with_borrow_mut(|s| s.loaded = true);
    let g = ui.global::<Uninstaller>();
    g.set_loading(true);
    spawn(ui, || (package::packages(), package::snap_packages()), |ui, (system, snaps)| {
        STATE.with_borrow_mut(|s| {
            s.checked_system.retain(|p| system.contains(p));
            s.checked_snaps.retain(|p| snaps.contains(p));
            s.system = system;
            s.snaps = snaps;
        });
        ui.global::<Uninstaller>().set_loading(false);
        render(&ui);
    });
}

fn update_count(ui: &AppWindow) {
    let g = ui.global::<Uninstaller>();
    let snap = g.get_snap_tab();
    g.set_selected_count(STATE.with_borrow(|s| if snap { s.checked_snaps.len() } else { s.checked_system.len() }) as i32);
}

fn render(ui: &AppWindow) {
    let g = ui.global::<Uninstaller>();
    let snap = g.get_snap_tab();
    let filter = g.get_filter().to_lowercase();
    let items = STATE.with_borrow(|s| {
        g.set_system_count(s.system.len() as i32);
        g.set_snap_count(s.snaps.len() as i32);
        let (list, checked) = if snap { (&s.snaps, &s.checked_snaps) } else { (&s.system, &s.checked_system) };
        list.iter()
            .filter(|p| filter.is_empty() || p.to_lowercase().contains(&filter))
            .map(|p| PackageItem { name: p.clone().into(), checked: checked.contains(p) })
            .collect::<Vec<_>>()
    });
    g.set_packages(model(items));
    update_count(ui);
}
