//! Processes page: periodic `ps` snapshot, sort, filter, terminate.

use super::{spawn, table};
use crate::core::{format, process, system};
use crate::{App, AppWindow, Processes};
use slint::{ComponentHandle, Timer, TimerMode};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::sync::atomic::{AtomicBool, Ordering as AtOrd};
use std::time::Duration;

struct State {
    procs: Vec<process::Process>,
    shown: Vec<i32>, // pids in display order
    sort_col: i32,
    sort_asc: bool,
    user: String,
    timer: Timer,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        procs: vec![],
        shown: vec![],
        sort_col: 2,
        sort_asc: false,
        user: system::username(),
        timer: Timer::default(),
    });
}
static LOADING: AtomicBool = AtomicBool::new(false);

pub fn setup(ui: &AppWindow) {
    let g = ui.global::<Processes>();

    let weak = ui.as_weak();
    g.on_sort(move |col, asc| {
        STATE.with_borrow_mut(|s| {
            s.sort_col = col;
            s.sort_asc = asc;
        });
        if let Some(ui) = weak.upgrade() {
            render(&ui);
        }
    });

    let weak = ui.as_weak();
    g.on_select_row(move |row| {
        if let Some(ui) = weak.upgrade() {
            let pid = STATE.with_borrow(|s| s.shown.get(row as usize).copied().unwrap_or(-1));
            ui.global::<Processes>().set_selected_pid(pid);
        }
    });

    let weak = ui.as_weak();
    g.on_options_changed(move || {
        if let Some(ui) = weak.upgrade() {
            render(&ui);
            restart_timer(&ui);
        }
    });

    let weak = ui.as_weak();
    g.on_end_process(move || {
        let Some(ui) = weak.upgrade() else { return };
        let pid = ui.global::<Processes>().get_selected_pid();
        if pid <= 0 {
            return;
        }
        spawn(&ui, move || process::terminate(pid), |ui, res| {
            let g = ui.global::<Processes>();
            g.set_message(res.err().map(|e| crate::i18n::tr(&e)).unwrap_or_default().into());
            g.set_selected_pid(-1);
            refresh(&ui);
        });
    });

    restart_timer(ui);
}

fn restart_timer(ui: &AppWindow) {
    let secs = ui.global::<Processes>().get_refresh_seconds().clamp(1, 10) as u64;
    let weak = ui.as_weak();
    STATE.with_borrow(|s| {
        s.timer.start(TimerMode::Repeated, Duration::from_secs(secs), move || {
            if let Some(ui) = weak.upgrade() {
                if ui.global::<App>().get_current_page() == "processes" {
                    refresh(&ui);
                }
            }
        })
    });
}

/// Take a new snapshot in the background.
pub fn refresh(ui: &AppWindow) {
    if LOADING.swap(true, AtOrd::SeqCst) {
        return;
    }
    spawn(ui, process::list, |ui, procs| {
        LOADING.store(false, AtOrd::SeqCst);
        STATE.with_borrow_mut(|s| s.procs = procs);
        render(&ui);
    });
}

fn cmp(a: &process::Process, b: &process::Process, col: i32) -> Ordering {
    let f = |x: f64, y: f64| x.partial_cmp(&y).unwrap_or(Ordering::Equal);
    match col {
        0 => a.pid.cmp(&b.pid),
        1 => a.user.cmp(&b.user),
        2 => f(a.pcpu, b.pcpu),
        3 => f(a.pmem, b.pmem),
        4 => a.rss.cmp(&b.rss),
        5 => a.vsize.cmp(&b.vsize),
        6 => a.state.cmp(&b.state),
        7 => a.nice.cmp(&b.nice),
        8 => a.start_time.cmp(&b.start_time),
        _ => a.cmd.to_lowercase().cmp(&b.cmd.to_lowercase()),
    }
}

fn render(ui: &AppWindow) {
    let g = ui.global::<Processes>();
    let all = g.get_all_users();
    let filter = g.get_filter().to_lowercase();
    let selected = g.get_selected_pid();

    let (rows, shown) = STATE.with_borrow_mut(|s| {
        let mut list: Vec<&process::Process> = s
            .procs
            .iter()
            .filter(|p| all || p.user == s.user)
            .filter(|p| filter.is_empty() || p.cmd.to_lowercase().contains(&filter) || p.user.to_lowercase().contains(&filter) || p.pid.to_string() == filter)
            .collect();
        list.sort_by(|a, b| {
            let o = cmp(a, b, s.sort_col);
            if s.sort_asc { o } else { o.reverse() }
        });
        let shown: Vec<i32> = list.iter().map(|p| p.pid).collect();
        let rows: Vec<Vec<String>> = list
            .iter()
            .map(|p| {
                vec![
                    p.pid.to_string(),
                    p.user.clone(),
                    format!("{:.1}", p.pcpu),
                    format!("{:.1}", p.pmem),
                    format::bytes(p.rss),
                    format::bytes(p.vsize),
                    p.state.clone(),
                    p.nice.to_string(),
                    p.start_time.clone(),
                    p.cmd.clone(),
                ]
            })
            .collect();
        s.shown = shown.clone();
        (rows, shown)
    });

    g.set_count(rows.len() as i32);
    g.set_rows(table(rows));
    // keep the selected process highlighted after re-sorting
    match shown.iter().position(|&p| p == selected) {
        Some(i) => g.set_current_row(i as i32),
        None => {
            g.set_current_row(-1);
            g.set_selected_pid(-1);
        }
    }
}
