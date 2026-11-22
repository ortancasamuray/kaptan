//! Search page: `find` with filters, results table and file actions.

use super::{spawn, strings, table};
use crate::core::search::{self, Found, Query};
use crate::core::{command, format, fsutil, system};
use crate::{AppWindow, Search};
use crate::i18n::{tr, trf};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::path::Path;

thread_local! {
    static RESULTS: RefCell<Vec<Found>> = const { RefCell::new(Vec::new()) };
}

const TYPES: [&str; 4] = ["", "f", "d", "l"];
const TIME_TYPES: [&str; 4] = ["", "-amin", "-mmin", "-cmin"];
const CMP: [&str; 3] = ["-", "", "+"];
const SIZE_CMP: [&str; 4] = ["", "-", "=", "+"];
const UNITS: [&str; 4] = ["c", "k", "M", "G"];

pub fn setup(ui: &AppWindow) {
    let g = ui.global::<Search>();
    g.set_directory(fsutil::home().to_string_lossy().into_owned().into());
    set_lists(ui);

    let weak = ui.as_weak();
    g.on_run_search(move || {
        if let Some(ui) = weak.upgrade() {
            run(&ui);
        }
    });

    let weak = ui.as_weak();
    g.on_browse(move || {
        let Some(ui) = weak.upgrade() else { return };
        spawn(&ui, pick_directory, |ui, dir| {
            let g = ui.global::<Search>();
            match dir {
                Ok(Some(d)) => g.set_directory(d.into()),
                Ok(None) => {}
                Err(e) => {
                    g.set_message(tr(&e).into());
                    g.set_message_error(true);
                }
            }
        });
    });

    let weak = ui.as_weak();
    g.on_sort(move |col, asc| {
        let Some(ui) = weak.upgrade() else { return };
        RESULTS.with_borrow_mut(|r| {
            r.sort_by(|a, b| {
                let o = match col {
                    0 => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    1 => a.path.cmp(&b.path),
                    2 => a.size.cmp(&b.size),
                    3 => a.user.cmp(&b.user),
                    _ => a.modified.cmp(&b.modified),
                };
                if asc { o } else { o.reverse() }
            })
        });
        render(&ui);
    });

    let weak = ui.as_weak();
    g.on_open_folder(move || {
        let Some(ui) = weak.upgrade() else { return };
        if let Some(f) = selected(&ui) {
            let dir = if f.path.is_dir() { f.path.clone() } else { f.path.parent().unwrap_or(Path::new("/")).to_path_buf() };
            let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
        }
    });

    let weak = ui.as_weak();
    g.on_move_to_trash(move || {
        let Some(ui) = weak.upgrade() else { return };
        if let Some(f) = selected(&ui) {
            let p = f.path.to_string_lossy().into_owned();
            let res = command::exec_checked("gio", &["trash", "--", &p]).map(|_| ());
            after_action(&ui, &f, res, "{} moved to trash.");
        }
    });

    let weak = ui.as_weak();
    g.on_delete_file(move || {
        let Some(ui) = weak.upgrade() else { return };
        if let Some(f) = selected(&ui) {
            let res = if f.path.is_dir() && !f.path.is_symlink() { std::fs::remove_dir_all(&f.path) } else { std::fs::remove_file(&f.path) };
            after_action(&ui, &f, res.map_err(|e| e.to_string()), "{} deleted.");
        }
    });
}

pub fn set_lists(ui: &AppWindow) {
    let g = ui.global::<Search>();
    g.set_users(strings(std::iter::once(tr("Any")).chain(system::users())));
    g.set_groups(strings(std::iter::once(tr("Any")).chain(system::groups())));
}

fn selected(ui: &AppWindow) -> Option<Found> {
    let row = ui.global::<Search>().get_current_row();
    RESULTS.with_borrow(|r| r.get(row as usize).cloned())
}

fn after_action(ui: &AppWindow, f: &Found, res: Result<(), String>, done_fmt: &str) {
    let g = ui.global::<Search>();
    match res {
        Ok(()) => {
            RESULTS.with_borrow_mut(|r| r.retain(|x| x.path != f.path));
            g.set_message(trf(done_fmt, &[&f.path.display()]).into());
            g.set_message_error(false);
            g.set_current_row(-1);
            render(ui);
        }
        Err(e) => {
            g.set_message(tr(&e).into());
            g.set_message_error(true);
        }
    }
}

/// Native directory picker through zenity or kdialog.
fn pick_directory() -> Result<Option<String>, String> {
    let home = fsutil::home().to_string_lossy().into_owned();
    let out = if command::is_executable("zenity") {
        std::process::Command::new("zenity").args(["--file-selection", "--directory", "--filename", &format!("{home}/")]).output()
    } else if command::is_executable("kdialog") {
        std::process::Command::new("kdialog").args(["--getexistingdirectory", &home]).output()
    } else {
        return Err("Install zenity or kdialog to browse, or type the directory path.".into());
    };
    let out = out.map_err(|e| e.to_string())?;
    let dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok((out.status.success() && !dir.is_empty()).then_some(dir))
}

fn run(ui: &AppWindow) {
    let g = ui.global::<Search>();
    let pick = |list: &[&str], i: i32| list.get(i as usize).copied().unwrap_or("").to_string();
    let q = Query {
        directory: g.get_directory().trim().to_string(),
        pattern: g.get_pattern().trim().to_string(),
        case_insensitive: g.get_case_insensitive(),
        regex: g.get_regex(),
        invert: g.get_invert(),
        empty: g.get_empty(),
        file_type: pick(&TYPES, g.get_type_index()),
        time_type: pick(&TIME_TYPES, g.get_time_type_index()),
        time_cmp: pick(&CMP, g.get_time_cmp_index()),
        time_minutes: g.get_time_minutes(),
        readable: g.get_readable(),
        writable: g.get_writable(),
        executable: g.get_executable(),
        size_cmp: pick(&SIZE_CMP, g.get_size_cmp_index()),
        size: g.get_size(),
        size_unit: pick(&UNITS, g.get_size_unit_index()),
        user: if g.get_user_index() > 0 { system::users().get(g.get_user_index() as usize - 1).cloned().unwrap_or_default() } else { String::new() },
        group: if g.get_group_index() > 0 { system::groups().get(g.get_group_index() as usize - 1).cloned().unwrap_or_default() } else { String::new() },
        as_root: g.get_as_root(),
    };
    g.set_searching(true);
    g.set_message("".into());
    g.set_advanced_open(false);
    spawn(ui, move || search::run(&q), |ui, res| {
        let g = ui.global::<Search>();
        g.set_searching(false);
        g.set_current_row(-1);
        match res {
            Ok((total, found)) => {
                g.set_info(if total > found.len() {
                    trf("{} found, showing {}", &[&total, &found.len()])
                } else {
                    trf("{} found", &[&total])
                }.into());
                RESULTS.with_borrow_mut(|r| *r = found);
            }
            Err(e) => {
                g.set_message(tr(&e).into());
                g.set_message_error(true);
                RESULTS.with_borrow_mut(Vec::clear);
                g.set_info("".into());
            }
        }
        render(&ui);
    });
}

fn render(ui: &AppWindow) {
    let rows: Vec<Vec<String>> = RESULTS.with_borrow(|r| {
        r.iter()
            .map(|f| {
                vec![
                    f.name.clone(),
                    f.path.parent().map(|p| p.display().to_string()).unwrap_or_default(),
                    format::bytes(f.size),
                    f.user.clone(),
                    search::format_time(f.modified),
                ]
            })
            .collect()
    });
    ui.global::<Search>().set_rows(table(rows));
}
