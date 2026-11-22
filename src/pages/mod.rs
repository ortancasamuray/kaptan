//! Glue between the Slint UI and `core`. One module per page.

pub mod apt;
pub mod cleaner;
pub mod desktop;
pub mod hosts;
pub mod monitor;
pub mod processes;
pub mod search;
pub mod services;
pub mod settings;
pub mod startup;
pub mod uninstaller;

use crate::i18n::tr;
use crate::{App, AppWindow, NavItem};
use slint::{ComponentHandle, Image, ModelRc, SharedString, StandardListViewItem, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    /// Sidebar pages: (id, English label, icon)
    static NAV: RefCell<Vec<(String, String, Image)>> = const { RefCell::new(Vec::new()) };
}

pub fn set_nav(ui: &AppWindow, nav: Vec<(String, String, Image)>) {
    NAV.with_borrow_mut(|n| *n = nav);
    render_nav(ui);
}

fn render_nav(ui: &AppWindow) {
    let items = NAV.with_borrow(|n| n.iter().map(|(id, l, icon)| NavItem { id: id.into(), label: tr(l).into(), icon: icon.clone() }).collect());
    ui.global::<App>().set_nav(model(items));
}

/// (id, translated label) of the sidebar pages.
pub fn nav_pages() -> Vec<(String, String)> {
    NAV.with_borrow(|n| n.iter().map(|(id, l, _)| (id.clone(), tr(l))).collect())
}

/// Re-apply strings produced in Rust after the language changed (`@tr` updates itself).
pub fn retranslate(ui: &AppWindow) {
    render_nav(ui);
    settings::set_page_labels(ui);
    monitor::set_system_info(ui);
    cleaner::set_titles(ui);
    search::set_lists(ui);
    desktop::reload(ui);
}

/// Run `work` on a background thread, then `done` on the UI thread with the result.
pub fn spawn<T, W, D>(ui: &AppWindow, work: W, done: D)
where
    T: Send + 'static,
    W: FnOnce() -> T + Send + 'static,
    D: FnOnce(AppWindow, T) + Send + 'static,
{
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let result = work();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak.upgrade() {
                done(ui, result);
            }
        });
    });
}

pub fn model<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> {
    ModelRc::from(Rc::new(VecModel::from(v)))
}

pub fn strings(v: impl IntoIterator<Item = impl Into<SharedString>>) -> ModelRc<SharedString> {
    model(v.into_iter().map(Into::into).collect())
}

/// Rows for a StandardTableView.
pub fn table(rows: impl IntoIterator<Item = Vec<String>>) -> ModelRc<ModelRc<StandardListViewItem>> {
    model(
        rows.into_iter()
            .map(|r| model(r.into_iter().map(|c| StandardListViewItem::from(SharedString::from(c))).collect()))
            .collect(),
    )
}

/// Update rows in place when the length is unchanged, so lists keep their scroll position.
pub fn sync<T: Clone + 'static>(m: &VecModel<T>, items: Vec<T>) {
    use slint::Model;
    if m.row_count() == items.len() {
        for (i, it) in items.into_iter().enumerate() {
            m.set_row_data(i, it);
        }
    } else {
        m.set_vec(items);
    }
}

pub fn on_page_opened(ui: &AppWindow, id: &str) {
    match id {
        "services" => services::load_once(ui),
        "uninstaller" => uninstaller::load_once(ui),
        "apt" => apt::reload(ui),
        "desktop" => desktop::load_once(ui),
        "startup" => startup::reload(ui),
        "helpers" => hosts::load_once(ui),
        "processes" => processes::refresh(ui),
        _ => {}
    }
}
