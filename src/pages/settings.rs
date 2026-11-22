//! Settings page.

use super::monitor::MonitorHandle;
use super::strings;
use crate::core::{disk, settings};
use crate::i18n::{self, tr};
use crate::{AppWindow, Settings, Theme};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::rc::Rc;

/// Language setting values in combo box order: system, English, bundled languages.
fn language_codes() -> Vec<&'static str> {
    let mut v = vec!["", "en"];
    v.extend(i18n::LANGUAGES.iter().map(|(c, ..)| *c));
    v
}

pub fn set_page_labels(ui: &AppWindow) {
    let g = ui.global::<Settings>();
    g.set_page_labels(strings(super::nav_pages().into_iter().map(|(_, l)| l)));
    let mut langs = vec![tr("System default"), "English".to_string()];
    langs.extend(i18n::LANGUAGES.iter().map(|(_, name, _)| name.to_string()));
    g.set_languages(strings(langs));
}

pub fn setup(ui: &AppWindow, s: settings::Settings, monitor: MonitorHandle) {
    let g = ui.global::<Settings>();
    let disks: Vec<String> = disk::disks().into_iter().map(|d| d.name).collect();
    let page_ids: Vec<String> = super::nav_pages().into_iter().map(|(id, _)| id).collect();

    set_page_labels(ui);
    g.set_language_index(language_codes().iter().position(|c| *c == s.language).unwrap_or(0) as i32);
    g.set_theme_index(if s.theme == "light" { 1 } else { 0 });
    g.set_start_page_index(page_ids.iter().position(|p| *p == s.start_page).unwrap_or(0) as i32);
    g.set_disks(strings(disks.clone()));
    let disk_idx = disks.iter().position(|d| *d == s.disk_name).or_else(|| disks.iter().position(|d| d == "/")).unwrap_or(0);
    g.set_disk_index(disk_idx as i32);
    g.set_cpu_alert(s.cpu_alert_percent);
    g.set_memory_alert(s.memory_alert_percent);
    g.set_disk_alert(s.disk_alert_percent);
    g.set_gpu_alert(s.gpu_alert_percent);
    g.set_autostart(settings::is_self_autostart());

    let state = Rc::new(RefCell::new(s));
    let weak = ui.as_weak();
    g.on_changed(move || {
        let Some(ui) = weak.upgrade() else { return };
        let g = ui.global::<Settings>();
        let mut s = state.borrow_mut();
        let language = language_codes().get(g.get_language_index() as usize).copied().unwrap_or("").to_string();
        let language_changed = language != s.language;
        s.language = language;
        s.theme = if g.get_theme_index() == 1 { "light" } else { "dark" }.into();
        s.start_page = page_ids.get(g.get_start_page_index() as usize).cloned().unwrap_or_else(|| "dashboard".into());
        s.disk_name = disks.get(g.get_disk_index() as usize).cloned().unwrap_or_default();
        s.cpu_alert_percent = g.get_cpu_alert();
        s.memory_alert_percent = g.get_memory_alert();
        s.disk_alert_percent = g.get_disk_alert();
        s.gpu_alert_percent = g.get_gpu_alert();
        s.save();

        ui.global::<Theme>().set_dark(s.theme != "light");
        {
            let mut m = monitor.borrow_mut();
            m.disk_name = s.disk_name.clone();
            m.cpu_alert = s.cpu_alert_percent;
            m.memory_alert = s.memory_alert_percent;
            m.disk_alert = s.disk_alert_percent;
            m.gpu_alert = s.gpu_alert_percent;
        }
        if g.get_autostart() != settings::is_self_autostart() {
            let _ = settings::set_self_autostart(g.get_autostart());
        }
        if language_changed {
            i18n::set_language(&s.language);
            super::retranslate(&ui);
        }
    });
}
