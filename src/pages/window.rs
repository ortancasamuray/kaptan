//! Window and tray behaviour: hide to tray on close (asking first, like Stacer), tray menu actions.

use crate::i18n::tr;
use crate::tray::{self, TrayEvent, TrayHandle};
use crate::{App, AppWindow, Settings};
use slint::{CloseRequestResponse, ComponentHandle};
use std::cell::RefCell;

/// Values of the close action setting, in combo box order.
pub const CLOSE_ACTIONS: [&str; 3] = ["ask", "tray", "quit"];

thread_local! {
    static TRAY: RefCell<Option<TrayHandle>> = const { RefCell::new(None) };
    static CLOSE_ACTION: RefCell<String> = RefCell::new("ask".into());
}

fn tray_running() -> bool {
    TRAY.with_borrow(|t| t.is_some())
}

/// Start the tray and install the close handler. Returns whether the tray is running.
pub fn setup(ui: &AppWindow, close_action: &str) -> bool {
    set_close_action(close_action);

    let weak = ui.as_weak();
    let send = move |ev: TrayEvent| {
        let weak = weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak.upgrade() {
                handle_event(&ui, ev);
            }
        });
    };
    let handle = tray::start(super::nav_pages(), tr("Open Kaptan"), tr("Quit"), Box::new(send));
    let running = handle.is_some();
    TRAY.with_borrow_mut(|t| *t = handle);
    ui.global::<App>().set_tray_available(running);
    ui.global::<Settings>().set_tray_available(running);

    let weak = ui.as_weak();
    ui.window().on_close_requested(move || {
        if !tray_running() {
            quit();
            return CloseRequestResponse::HideWindow;
        }
        match CLOSE_ACTION.with_borrow(|a| a.clone()).as_str() {
            "tray" => CloseRequestResponse::HideWindow,
            "quit" => {
                quit();
                CloseRequestResponse::HideWindow
            }
            _ => {
                if let Some(ui) = weak.upgrade() {
                    ui.global::<App>().set_quit_dialog_open(true);
                }
                CloseRequestResponse::KeepWindowShown
            }
        }
    });

    // Answer of the "keep running in the tray?" dialog
    let weak = ui.as_weak();
    ui.global::<App>().on_quit_choice(move |choice, remember| {
        let Some(ui) = weak.upgrade() else { return };
        ui.global::<App>().set_quit_dialog_open(false);
        if remember {
            set_close_action(&choice);
            let mut s = crate::core::settings::Settings::load();
            s.close_action = choice.to_string();
            s.save();
            if let Some(i) = CLOSE_ACTIONS.iter().position(|a| *a == choice.as_str()) {
                ui.global::<Settings>().set_close_action_index(i as i32);
            }
        }
        if choice == "quit" {
            quit();
        } else {
            let _ = ui.hide();
        }
    });

    running
}

pub fn set_close_action(action: &str) {
    let a = if CLOSE_ACTIONS.contains(&action) { action } else { "ask" };
    CLOSE_ACTION.with_borrow_mut(|c| *c = a.to_string());
}

fn handle_event(ui: &AppWindow, ev: TrayEvent) {
    match ev {
        TrayEvent::Toggle => {
            if ui.window().is_visible() {
                let _ = ui.hide();
            } else {
                let _ = ui.show();
            }
        }
        TrayEvent::Open(page) => {
            if !page.is_empty() {
                ui.global::<App>().set_current_page(page.as_str().into());
                super::on_page_opened(ui, &page);
            }
            let _ = ui.show();
        }
        // let the tray answer the D-Bus menu call before it is shut down
        TrayEvent::Quit => slint::Timer::single_shot(std::time::Duration::from_millis(100), quit),
    }
}

pub fn quit() {
    if let Some(handle) = TRAY.with_borrow_mut(Option::take) {
        handle.shutdown().wait();
    }
    let _ = slint::quit_event_loop();
}

/// Tray menu labels follow the UI language.
pub fn retranslate() {
    let pages = super::nav_pages();
    TRAY.with_borrow(|t| {
        if let Some(h) = t {
            h.update(|tray| tray.set_labels(pages, tr("Open Kaptan"), tr("Quit")));
        }
    });
}
