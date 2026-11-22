//! System Cleaner page.

use super::{spawn, strings, sync};
use crate::core::cleaner::{self, Category, ScanGroup};
use crate::core::format;
use crate::i18n::{tr, trf};
use crate::{AppWindow, Cleaner, CleanerRow};
use slint::{ComponentHandle, Model, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct State {
    groups: Vec<ScanGroup>,
    checked: Vec<Vec<bool>>,
    expanded: Vec<bool>,
    rows: Vec<(usize, Option<usize>)>, // (group, item) per displayed row
    model: Rc<VecModel<CleanerRow>>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

pub fn setup(ui: &AppWindow) {
    let g = ui.global::<Cleaner>();
    g.set_rows(STATE.with_borrow(|s| s.model.clone()).into());
    set_titles(ui);
    let cats = Rc::new(VecModel::from(vec![true; Category::ALL.len()]));
    g.set_categories(cats.clone().into());

    g.on_set_category(move |i, on| cats.set_row_data(i as usize, on));

    let weak = ui.as_weak();
    g.on_scan(move || {
        if let Some(ui) = weak.upgrade() {
            scan(&ui);
        }
    });

    let weak = ui.as_weak();
    g.on_back(move || {
        if let Some(ui) = weak.upgrade() {
            let g = ui.global::<Cleaner>();
            g.set_has_results(false);
            g.set_message("".into());
        }
    });

    let weak = ui.as_weak();
    g.on_expand(move |group| {
        let Some(ui) = weak.upgrade() else { return };
        STATE.with_borrow_mut(|s| {
            if let Some(e) = s.expanded.get_mut(group as usize) {
                *e = !*e;
            }
        });
        render(&ui);
    });

    let weak = ui.as_weak();
    g.on_toggle_row(move |row, on| {
        let Some(ui) = weak.upgrade() else { return };
        STATE.with_borrow_mut(|s| match s.rows.get(row as usize).copied() {
            Some((g, None)) => s.checked[g].iter_mut().for_each(|c| *c = on),
            Some((g, Some(i))) => s.checked[g][i] = on,
            None => {}
        });
        render(&ui);
    });

    let weak = ui.as_weak();
    g.on_select_all(move |on| {
        let Some(ui) = weak.upgrade() else { return };
        STATE.with_borrow_mut(|s| s.checked.iter_mut().flatten().for_each(|c| *c = on));
        render(&ui);
    });

    let weak = ui.as_weak();
    g.on_clean(move || {
        if let Some(ui) = weak.upgrade() {
            clean(&ui);
        }
    });
}

pub fn set_titles(ui: &AppWindow) {
    ui.global::<Cleaner>().set_category_titles(strings(Category::ALL.iter().map(|c| tr(c.title()))));
}

fn selected_categories(ui: &AppWindow) -> Vec<Category> {
    let cats = ui.global::<Cleaner>().get_categories();
    Category::ALL.iter().enumerate().filter(|(i, _)| cats.row_data(*i).unwrap_or(false)).map(|(_, c)| *c).collect()
}

fn scan(ui: &AppWindow) {
    let cats = selected_categories(ui);
    let g = ui.global::<Cleaner>();
    if cats.is_empty() {
        g.set_message(tr("Select at least one category.").into());
        g.set_message_error(true);
        return;
    }
    g.set_scanning(true);
    g.set_message("".into());
    spawn(ui, move || cleaner::scan(&cats), |ui, groups| {
        STATE.with_borrow_mut(|s| {
            s.checked = groups.iter().map(|g| vec![false; g.items.len()]).collect();
            s.expanded = vec![false; groups.len()];
            s.groups = groups;
        });
        let g = ui.global::<Cleaner>();
        g.set_scanning(false);
        g.set_has_results(true);
        render(&ui);
    });
}

fn clean(ui: &AppWindow) {
    let paths: Vec<_> = STATE.with_borrow(|s| {
        s.groups
            .iter()
            .zip(&s.checked)
            .flat_map(|(g, c)| g.items.iter().zip(c).filter(|(_, on)| **on).map(|(it, _)| it.path.clone()))
            .collect()
    });
    let g = ui.global::<Cleaner>();
    if paths.is_empty() {
        g.set_message(tr("Nothing selected.").into());
        g.set_message_error(true);
        return;
    }
    g.set_cleaning(true);
    let cats: Vec<Category> = STATE.with_borrow(|s| s.groups.iter().map(|g| g.category).collect());
    spawn(
        ui,
        move || {
            let res = cleaner::clean(&paths);
            (res, cleaner::scan(&cats))
        },
        |ui, (res, groups)| {
            let g = ui.global::<Cleaner>();
            g.set_cleaning(false);
            match res {
                Ok(freed) => {
                    g.set_message(trf("{} cleaned.", &[&format::bytes(freed)]).into());
                    g.set_message_error(false);
                }
                Err(e) => {
                    g.set_message(tr(&e).into());
                    g.set_message_error(true);
                }
            }
            STATE.with_borrow_mut(|s| {
                s.checked = groups.iter().map(|g| vec![false; g.items.len()]).collect();
                s.expanded.resize(groups.len(), false);
                s.groups = groups;
            });
            render(&ui);
        },
    );
}

fn render(ui: &AppWindow) {
    let (rows, total) = STATE.with_borrow_mut(|s| {
        let mut rows = Vec::new();
        let mut map = Vec::new();
        for (gi, g) in s.groups.iter().enumerate() {
            let checked = &s.checked[gi];
            rows.push(CleanerRow {
                is_group: true,
                group: gi as i32,
                item: -1,
                title: format!("{} ({})", tr(g.category.title()), g.items.len()).into(),
                size: format::bytes(g.total()).into(),
                checked: !checked.is_empty() && checked.iter().all(|c| *c),
                expanded: s.expanded[gi],
            });
            map.push((gi, None));
            if s.expanded[gi] {
                for (ii, it) in g.items.iter().enumerate() {
                    rows.push(CleanerRow {
                        is_group: false,
                        group: gi as i32,
                        item: ii as i32,
                        title: it.name.clone().into(),
                        size: format::bytes(it.size).into(),
                        checked: checked[ii],
                        expanded: false,
                    });
                    map.push((gi, Some(ii)));
                }
            }
        }
        s.rows = map;
        let selected: u64 = s.groups.iter().zip(&s.checked).flat_map(|(g, c)| g.items.iter().zip(c).filter(|(_, on)| **on).map(|(i, _)| i.size)).sum();
        let all: u64 = s.groups.iter().map(ScanGroup::total).sum();
        (rows, trf("Total {} · selected {}", &[&format::bytes(all), &format::bytes(selected)]))
    });
    STATE.with_borrow(|s| sync(&s.model, rows));
    ui.global::<Cleaner>().set_total(total.into());
}
