//! Hosts page: edit /etc/hosts, save through pkexec.

use super::{spawn, table};
use crate::core::hosts;
use crate::{AppWindow, Hosts};
use crate::i18n::{tr, trf};
use slint::ComponentHandle;
use std::cell::RefCell;

#[derive(Default)]
struct State {
    loaded: bool,
    lines: Vec<String>,
    items: Vec<hosts::HostItem>,
    editing: Option<usize>, // line index, None = new
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

pub fn setup(ui: &AppWindow) {
    let g = ui.global::<Hosts>();

    let weak = ui.as_weak();
    g.on_reload(move || {
        if let Some(ui) = weak.upgrade() {
            load(&ui);
            ui.global::<Hosts>().set_message("".into());
        }
    });

    let weak = ui.as_weak();
    g.on_new_item(move || {
        let Some(ui) = weak.upgrade() else { return };
        STATE.with_borrow_mut(|s| s.editing = None);
        open_editor(&ui, "", "", "");
    });

    let weak = ui.as_weak();
    g.on_edit_item(move |row| {
        let Some(ui) = weak.upgrade() else { return };
        let item = STATE.with_borrow_mut(|s| {
            let it = s.items.get(row as usize).cloned();
            s.editing = it.as_ref().map(|i| i.line);
            it
        });
        if let Some(it) = item {
            open_editor(&ui, &it.ip, &it.hostname, &it.aliases);
        }
    });

    let weak = ui.as_weak();
    g.on_delete_item(move |row| {
        let Some(ui) = weak.upgrade() else { return };
        STATE.with_borrow_mut(|s| {
            if let Some(it) = s.items.get(row as usize) {
                s.lines.remove(it.line);
            }
        });
        ui.global::<Hosts>().set_dirty(true);
        ui.global::<Hosts>().set_current_row(-1);
        render(&ui);
    });

    let weak = ui.as_weak();
    g.on_save_item(move || {
        let Some(ui) = weak.upgrade() else { return };
        let g = ui.global::<Hosts>();
        let (ip, host, aliases) = (g.get_f_ip(), g.get_f_host(), g.get_f_aliases());
        if ip.trim().is_empty() || host.trim().is_empty() {
            g.set_editor_error(tr("The IP address and hostname fields are required.").into());
            return;
        }
        if ip.trim().parse::<std::net::IpAddr>().is_err() {
            g.set_editor_error(trf("'{}' is not a valid IP address.", &[&ip.trim()]).into());
            return;
        }
        let line = hosts::format_line(&ip, &host, &aliases);
        STATE.with_borrow_mut(|s| match s.editing {
            Some(i) if i < s.lines.len() => s.lines[i] = line,
            _ => s.lines.push(line),
        });
        g.set_editor_open(false);
        g.set_dirty(true);
        render(&ui);
    });

    let weak = ui.as_weak();
    g.on_save_file(move || {
        let Some(ui) = weak.upgrade() else { return };
        let lines = STATE.with_borrow(|s| s.lines.clone());
        spawn(&ui, move || hosts::save(&lines), |ui, res| {
            let g = ui.global::<Hosts>();
            match res {
                Ok(()) => {
                    g.set_message(tr("/etc/hosts saved.").into());
                    g.set_message_error(false);
                    load(&ui);
                }
                Err(e) => {
                    g.set_message(tr(&e).into());
                    g.set_message_error(true);
                }
            }
        });
    });
}

fn open_editor(ui: &AppWindow, ip: &str, host: &str, aliases: &str) {
    let g = ui.global::<Hosts>();
    g.set_f_ip(ip.into());
    g.set_f_host(host.into());
    g.set_f_aliases(aliases.into());
    g.set_editor_error("".into());
    g.set_editor_open(true);
}

pub fn load_once(ui: &AppWindow) {
    if !STATE.with_borrow(|s| s.loaded) {
        load(ui);
    }
}

fn load(ui: &AppWindow) {
    STATE.with_borrow_mut(|s| {
        s.loaded = true;
        s.lines = hosts::load();
    });
    ui.global::<Hosts>().set_dirty(false);
    render(ui);
}

fn render(ui: &AppWindow) {
    let rows = STATE.with_borrow_mut(|s| {
        s.items = hosts::items(&s.lines);
        s.items.iter().map(|i| vec![i.ip.clone(), i.hostname.clone(), i.aliases.clone()]).collect::<Vec<_>>()
    });
    let g = ui.global::<Hosts>();
    g.set_count(rows.len() as i32);
    g.set_rows(table(rows));
}
