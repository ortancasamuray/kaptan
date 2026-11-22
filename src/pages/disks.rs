//! "All disks" dialog opened from the dashboard DISK gauge.

use super::{model, strings};
use crate::core::{blockdev, format};
use crate::i18n::{tr, trf};
use crate::{AppWindow, Dashboard, DiskRow, Settings};
use slint::{ComponentHandle, Model};

pub fn setup(ui: &AppWindow) {
    let g = ui.global::<Dashboard>();

    let weak = ui.as_weak();
    g.on_open_disks(move || {
        if let Some(ui) = weak.upgrade() {
            render(&ui);
            ui.global::<Dashboard>().set_disks_open(true);
        }
    });

    let weak = ui.as_weak();
    g.on_select_disk(move |mount| {
        let Some(ui) = weak.upgrade() else { return };
        // The gauge follows Settings → Dashboard disk, so select it there.
        let s = ui.global::<Settings>();
        let disks = s.get_disks();
        let mut names: Vec<String> = disks.iter().map(|d| d.to_string()).collect();
        let idx = match names.iter().position(|d| *d == mount.as_str()) {
            Some(i) => i,
            None => {
                // mounted after start (e.g. a USB stick)
                names.push(mount.to_string());
                s.set_disks(strings(names.clone()));
                names.len() - 1
            }
        };
        s.set_disk_index(idx as i32);
        s.invoke_changed();
        render(&ui);
    });
}

/// Mount point currently shown on the dashboard gauge.
fn current_disk(ui: &AppWindow) -> String {
    let s = ui.global::<Settings>();
    s.get_disks().row_data(s.get_disk_index().max(0) as usize).map(|d| d.to_string()).unwrap_or_else(|| "/".into())
}

fn render(ui: &AppWindow) {
    let current = current_disk(ui);
    let mut rows = Vec::new();
    for d in blockdev::list() {
        rows.push(DiskRow {
            is_disk: true,
            title: if d.model.is_empty() { d.name.clone() } else { d.model.clone() }.into(),
            subtitle: format!("/dev/{} · {}", d.name, d.kind).into(),
            size: format::bytes(d.size).into(),
            percent: -1.0,
            ..Default::default()
        });
        for p in &d.parts {
            let swap = p.fstype == "swap";
            // a real directory to select ("[SWAP]" is not one)
            let mount = p.mounts.iter().find(|m| m.starts_with('/')).cloned().unwrap_or_default();
            let title = if !mount.is_empty() {
                mount.clone()
            } else if swap {
                if p.mounts.is_empty() { tr("Swap (inactive)") } else { tr("Swap") }
            } else if p.fstype.is_empty() && !p.mounts.is_empty() {
                p.mounts.join(", ")
            } else {
                tr("Not mounted")
            };
            let mut sub = vec![format!("/dev/{}", p.name)];
            if !p.fstype.is_empty() {
                sub.push(p.fstype.clone());
            }
            if !p.label.is_empty() {
                sub.push(p.label.clone());
            }
            let (percent, detail) = match (p.used, p.fssize) {
                (Some(used), Some(total)) if total > 0 && !mount.is_empty() => (
                    (used as f64 / total as f64 * 100.0) as f32,
                    trf("{} used · {} free", &[&format::bytes(used), &format::bytes(total.saturating_sub(used))]),
                ),
                _ => (-1.0, String::new()),
            };
            rows.push(DiskRow {
                is_disk: false,
                selected: !mount.is_empty() && mount == current,
                title: title.into(),
                subtitle: sub.join(" · ").into(),
                size: format::bytes(p.size).into(),
                detail: detail.into(),
                percent,
                mount: mount.into(),
                depth: p.depth as i32,
            });
        }
    }
    ui.global::<Dashboard>().set_disk_rows(model(rows));
}
