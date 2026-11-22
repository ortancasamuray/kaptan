//! APT Repositories page.

use super::{model, spawn};
use crate::core::apt_source::{self, AptSource};
use crate::{Apt, AptItem, AppWindow};
use crate::i18n::tr;
use slint::ComponentHandle;
use std::cell::RefCell;

thread_local! {
    static SOURCES: RefCell<Vec<AptSource>> = const { RefCell::new(Vec::new()) };
}

pub fn setup(ui: &AppWindow) {
    let g = ui.global::<Apt>();

    let weak = ui.as_weak();
    g.on_reload(move || {
        if let Some(ui) = weak.upgrade() {
            reload(&ui);
        }
    });

    let weak = ui.as_weak();
    g.on_toggle(move |i, on| {
        if let Some(ui) = weak.upgrade() {
            if let Some(src) = get(i) {
                run(&ui, move || apt_source::set_active(&src, on), if on { "Repository enabled." } else { "Repository disabled." });
            }
        }
    });

    let weak = ui.as_weak();
    g.on_remove(move |i| {
        if let Some(ui) = weak.upgrade() {
            if let Some(src) = get(i) {
                run(&ui, move || apt_source::remove(&src), "Repository removed.");
            }
        }
    });

    let weak = ui.as_weak();
    g.on_edit(move |i| {
        let Some(ui) = weak.upgrade() else { return };
        if let Some(src) = get(i) {
            let g = ui.global::<Apt>();
            g.set_editor_index(i);
            g.set_f_types(src.types.into());
            g.set_f_uri(src.uri.into());
            g.set_f_suites(src.suites.into());
            g.set_f_components(src.components.into());
            g.set_editor_open(true);
        }
    });

    let weak = ui.as_weak();
    g.on_save_edit(move || {
        let Some(ui) = weak.upgrade() else { return };
        let g = ui.global::<Apt>();
        let Some(src) = get(g.get_editor_index()) else { return };
        let mut new = src.clone();
        new.types = g.get_f_types().trim().to_string();
        new.uri = g.get_f_uri().trim().to_string();
        new.suites = g.get_f_suites().trim().to_string();
        new.components = g.get_f_components().trim().to_string();
        if new.types.is_empty() || new.uri.is_empty() || new.suites.is_empty() {
            g.set_message(tr("Types, URI and suites are required.").into());
            g.set_message_error(true);
            return;
        }
        g.set_editor_open(false);
        run(&ui, move || apt_source::change(&src, Some(&new)), "Repository updated.");
    });

    let weak = ui.as_weak();
    g.on_add(move || {
        let Some(ui) = weak.upgrade() else { return };
        let g = ui.global::<Apt>();
        let repo = g.get_new_repo().trim().to_string();
        let with_source = g.get_new_with_source();
        if repo.is_empty() {
            return;
        }
        g.set_new_repo("".into());
        run(&ui, move || apt_source::add_repository(&repo, with_source), "Repository added.");
    });
}

fn get(i: i32) -> Option<AptSource> {
    SOURCES.with_borrow(|s| s.get(i as usize).cloned())
}

fn run(ui: &AppWindow, op: impl FnOnce() -> Result<(), String> + Send + 'static, ok_msg: &'static str) {
    let g = ui.global::<Apt>();
    g.set_busy(true);
    spawn(ui, op, move |ui, res| {
        let g = ui.global::<Apt>();
        g.set_busy(false);
        match res {
            Ok(()) => {
                g.set_message(tr(ok_msg).into());
                g.set_message_error(false);
            }
            Err(e) => {
                g.set_message(tr(&e).into());
                g.set_message_error(true);
            }
        }
        reload(&ui);
    });
}

pub fn reload(ui: &AppWindow) {
    let list = apt_source::list();
    let items = list
        .iter()
        .map(|s| AptItem { display: s.display().into(), file: s.file.to_string_lossy().into_owned().into(), active: s.active })
        .collect();
    SOURCES.with_borrow_mut(|s| *s = list);
    ui.global::<Apt>().set_sources(model(items));
}
