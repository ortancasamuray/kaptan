//! Live monitoring: Dashboard gauges, Resources history charts and usage alerts.

use super::{model, strings, sync};
use crate::core::{cpu, disk, format, gpu, memory, network, settings, system};
use crate::i18n::{tr, trf};
use crate::{App, AppWindow, ChartData, ChartSeries, Dashboard, DiskUsage, Resources};
use slint::{Color, ComponentHandle, ModelRc, SharedString, StandardListViewItem, Timer, TimerMode, VecModel};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const HISTORY: usize = 61;

/// Settings the monitor needs; updated from the Settings page.
#[derive(Clone, Default)]
pub struct MonitorConfig {
    pub disk_name: String,
    pub cpu_alert: i32,
    pub memory_alert: i32,
    pub disk_alert: i32,
    pub gpu_alert: i32,
}

pub type MonitorHandle = Rc<RefCell<MonitorConfig>>;

/// Fixed-size sample buffer, newest first.
#[derive(Default)]
struct History(VecDeque<f64>);

impl History {
    fn push(&mut self, v: f64) {
        self.0.push_front(v);
        self.0.truncate(HISTORY);
    }

    /// SVG path in a 60 x 100 viewbox; newest sample on the right.
    fn path(&self, y_max: f64) -> SharedString {
        if self.0.is_empty() || y_max <= 0.0 {
            return "M 0 100".into();
        }
        let mut s = String::with_capacity(self.0.len() * 14);
        for (i, v) in self.0.iter().enumerate() {
            let x = 60.0 - i as f64;
            let y = 100.0 - (v / y_max * 100.0).clamp(0.0, 100.0);
            s.push_str(if i == 0 { "M " } else { " L " });
            s.push_str(&format!("{x:.1} {y:.2}"));
        }
        s.into()
    }

    fn max(&self) -> f64 {
        self.0.iter().copied().fold(0.0, f64::max)
    }
}

#[derive(Default)]
struct Alert(bool);

impl Alert {
    /// Notify once when `value` crosses `limit`, re-arm when it drops below.
    fn check(&mut self, value: i32, limit: i32, title: &str, body: String) {
        if limit <= 0 {
            return;
        }
        if value > limit && !self.0 {
            settings::notify(&tr(title), &body);
            self.0 = true;
        } else if value < limit {
            self.0 = false;
        }
    }
}

struct State {
    cpu: cpu::CpuInfo,
    cores: usize,
    cpu_hist: Vec<History>, // per core
    load_hist: [History; 3],
    mem_hist: History,
    swap_hist: History,
    net: (u64, u64),
    net_hist: [History; 2],
    net_max: (u64, u64),
    io: (u64, u64),
    io_hist: [History; 2],
    gpu: Arc<gpu::GpuInfo>,
    gpu_busy: Arc<AtomicBool>,
    gpu_last: Arc<Mutex<Option<gpu::GpuStatus>>>,
    gpu_hist: [History; 2],
    disks: Vec<disk::Disk>,
    ticks: u64,
    alerts: [Alert; 4], // cpu, memory, disk, gpu
    charts_model: Rc<VecModel<ChartData>>,
    disks_model: Rc<VecModel<DiskUsage>>,
}

const PALETTE: [u32; 20] = [
    0x2ecc71, 0xe74c3c, 0x3498db, 0xf1c40f, 0xe67e22, 0x1abc9c, 0x9b59b6, 0x95a5a6, 0xd35400, 0xc0392b,
    0x8e44ad, 0xff8f00, 0x16a085, 0x5499c7, 0x58d68d, 0xcd6155, 0xf5b041, 0x566573, 0x27ae60, 0x2980b9,
];

fn color(i: usize) -> Color {
    if i < PALETTE.len() {
        Color::from_argb_encoded(0xff000000 | PALETTE[i])
    } else {
        // more cores than palette entries: spread hues
        let h = (i as f32 * 47.0) % 360.0;
        Color::from_hsva(h, 0.65, 0.85, 1.0)
    }
}

fn percent_labels() -> slint::ModelRc<SharedString> {
    strings(["100%", "75%", "50%", "25%", "0%"])
}

fn byte_labels(max: f64) -> slint::ModelRc<SharedString> {
    strings((0..5).map(|i| {
        let v = max * (4 - i) as f64 / 4.0;
        if i == 4 { "0".to_string() } else { format!("{}/s", format::bytes(v as u64)) }
    }))
}

fn series(h: &History, max: f64, c: Color, label: String) -> ChartSeries {
    ChartSeries { commands: h.path(max), color: c, label: label.into() }
}

pub fn set_system_info(ui: &AppWindow) {
    static INFO: std::sync::OnceLock<system::SystemInfo> = std::sync::OnceLock::new();
    let info = INFO.get_or_init(system::read);
    let lines = [
        trf("Hostname: {}", &[&info.hostname]),
        trf("Platform: {}", &[&info.platform]),
        trf("Distribution: {}", &[&info.distribution]),
        trf("Kernel Release: {}", &[&info.kernel]),
        trf("CPU Model: {}", &[&info.cpu_model]),
        trf("CPU Cores: {}", &[&info.cpu_cores]),
        trf("CPU Speed: {}", &[&info.cpu_speed]),
    ];
    ui.global::<Dashboard>().set_system_info(model(lines.into_iter().map(|l| StandardListViewItem::from(SharedString::from(l))).collect()));
}

pub fn setup(ui: &AppWindow, s: &settings::Settings) -> MonitorHandle {
    let config: MonitorHandle = Rc::new(RefCell::new(MonitorConfig {
        disk_name: s.disk_name.clone(),
        cpu_alert: s.cpu_alert_percent,
        memory_alert: s.memory_alert_percent,
        disk_alert: s.disk_alert_percent,
        gpu_alert: s.gpu_alert_percent,
    }));

    // Static system info
    set_system_info(ui);

    let gpu_info = Arc::new(gpu::GpuInfo::detect());
    ui.global::<Dashboard>().set_gpu_visible(false);
    ui.global::<crate::Settings>().set_gpu_available(gpu_info.has_gpu());

    let cores = cpu::core_count();
    let mut cpu_info = cpu::CpuInfo::default();
    cpu_info.percents(); // prime deltas
    let state = Rc::new(RefCell::new(State {
        cpu: cpu_info,
        cores,
        cpu_hist: (0..cores).map(|_| History::default()).collect(),
        load_hist: Default::default(),
        mem_hist: History::default(),
        swap_hist: History::default(),
        net: network::rx_tx_bytes(),
        net_hist: Default::default(),
        net_max: (1 << 20, 1 << 20),
        io: disk::io_bytes(),
        io_hist: Default::default(),
        gpu: gpu_info,
        gpu_busy: Arc::new(AtomicBool::new(false)),
        gpu_last: Arc::new(Mutex::new(None)),
        gpu_hist: Default::default(),
        disks: disk::disks(),
        ticks: 0,
        alerts: Default::default(),
        charts_model: Rc::new(VecModel::default()),
        disks_model: Rc::new(VecModel::default()),
    }));
    {
        let st = state.borrow();
        ui.global::<Resources>().set_charts(ModelRc::from(st.charts_model.clone()));
        ui.global::<Resources>().set_disks(ModelRc::from(st.disks_model.clone()));
    }

    let timer = Timer::default();
    let weak = ui.as_weak();
    let cfg = config.clone();
    tick(ui, &mut state.borrow_mut(), &cfg.borrow());
    timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
        if let Some(ui) = weak.upgrade() {
            tick(&ui, &mut state.borrow_mut(), &cfg.borrow());
        }
    });
    std::mem::forget(timer); // lives for the whole app
    config
}

fn tick(ui: &AppWindow, st: &mut State, cfg: &MonitorConfig) {
    st.ticks += 1;
    let dash = ui.global::<Dashboard>();

    // CPU
    let percents = st.cpu.percents();
    let total = percents.first().copied().unwrap_or(0);
    for (i, h) in st.cpu_hist.iter_mut().enumerate() {
        h.push(percents.get(i + 1).copied().unwrap_or(0) as f64);
    }
    let ghz = cpu::avg_clock_mhz() / 1000.0;
    dash.set_cpu_value(total as f32);
    dash.set_cpu_text(format!("{ghz:.2} GHz\n{total}%").into());
    st.alerts[0].check(total, cfg.cpu_alert, "High CPU Usage", trf("The amount of CPU used is over {}%.", &[&cfg.cpu_alert]));

    let loads = cpu::load_avgs();
    for (h, v) in st.load_hist.iter_mut().zip(loads) {
        h.push(v);
    }

    // Memory
    let m = memory::read();
    let mem_pct = if m.mem_total > 0 { m.mem_used as f64 / m.mem_total as f64 * 100.0 } else { 0.0 };
    let swap_pct = if m.swap_total > 0 { m.swap_used as f64 / m.swap_total as f64 * 100.0 } else { 0.0 };
    st.mem_hist.push(mem_pct);
    st.swap_hist.push(swap_pct);
    dash.set_mem_value(mem_pct as f32);
    dash.set_mem_text(format!("{} / {}", format::bytes(m.mem_used), format::bytes(m.mem_total)).into());
    st.alerts[1].check(mem_pct as i32, cfg.memory_alert, "High Memory Usage", trf("The amount of memory used is over {}%.", &[&cfg.memory_alert]));

    // Disk (every 5 s)
    if st.ticks % 5 == 1 {
        st.disks = disk::disks();
    }
    if let Some(d) = disk::pick(&st.disks, &cfg.disk_name) {
        let pct = if d.size > 0 { d.used as f64 / d.size as f64 * 100.0 } else { 0.0 };
        dash.set_disk_value(pct as f32);
        dash.set_disk_text(format!("{} / {}", format::bytes(d.used), format::bytes(d.size)).into());
        st.alerts[2].check(pct as i32, cfg.disk_alert, "High Disk Usage", trf("The amount of disk used is over {}%.", &[&cfg.disk_alert]));
    }

    // Network
    let (rx, tx) = network::rx_tx_bytes();
    let (drx, dtx) = (rx.saturating_sub(st.net.0), tx.saturating_sub(st.net.1));
    st.net = (rx, tx);
    st.net_max = (st.net_max.0.max(drx), st.net_max.1.max(dtx));
    st.net_hist[0].push(drx as f64);
    st.net_hist[1].push(dtx as f64);
    dash.set_down_value((drx as f64 / st.net_max.0 as f64 * 100.0) as f32);
    dash.set_down_text(format!("{}/s", format::bytes(drx)).into());
    dash.set_down_total(trf("Total: {}", &[&format::bytes(rx)]).into());
    dash.set_up_value((dtx as f64 / st.net_max.1 as f64 * 100.0) as f32);
    dash.set_up_text(format!("{}/s", format::bytes(dtx)).into());
    dash.set_up_total(trf("Total: {}", &[&format::bytes(tx)]).into());

    // Disk I/O
    let (r, w) = disk::io_bytes();
    let (dr, dw) = (r.saturating_sub(st.io.0), w.saturating_sub(st.io.1));
    st.io_hist[0].push(dr as f64);
    st.io_hist[1].push(dw as f64);

    // GPU: query on a worker thread, use the latest finished result.
    if st.gpu.has_gpu() {
        if let Some(g) = st.gpu_last.lock().ok().and_then(|mut l| l.take()) {
            apply_gpu(ui, st, &g, cfg);
        }
        if !st.gpu_busy.swap(true, Ordering::SeqCst) {
            let (info, busy, last) = (st.gpu.clone(), st.gpu_busy.clone(), st.gpu_last.clone());
            std::thread::spawn(move || {
                let status = info.status();
                if let Ok(mut l) = last.lock() {
                    *l = Some(status);
                }
                busy.store(false, Ordering::SeqCst);
            });
        }
    }

    let page = ui.global::<App>().get_current_page();
    if page == "resources" {
        update_resources(st, (dr, dw), (drx, dtx), &m);
    }
}

fn apply_gpu(ui: &AppWindow, st: &mut State, g: &gpu::GpuStatus, cfg: &MonitorConfig) {
    let dash = ui.global::<Dashboard>();
    dash.set_gpu_visible(g.available);
    if !g.available {
        return;
    }
    let vram_pct = if g.mem_total > 0 { g.mem_used as f64 / g.mem_total as f64 * 100.0 } else { 0.0 };
    st.alerts[3].check(g.usage, cfg.gpu_alert, "High GPU Usage", trf("The amount of GPU used is over {}%.", &[&cfg.gpu_alert]));
    st.gpu_hist[0].push(g.usage as f64);
    st.gpu_hist[1].push(vram_pct);
    dash.set_gpu_value(g.usage as f32);
    dash.set_gpu_name(g.name.clone().into());
    dash.set_gpu_text(if g.mem_total > 0 {
        format!("{} / {}\n{}%", format::bytes(g.mem_used), format::bytes(g.mem_total), g.usage)
    } else {
        format!("{}%", g.usage)
    }.into());
}

fn update_resources(st: &State, io: (u64, u64), net: (u64, u64), m: &memory::MemoryInfo) {
    let mut charts = Vec::new();

    // CPU per core
    charts.push(ChartData {
        title: tr("CPU History").into(),
        series: model(
            st.cpu_hist
                .iter()
                .enumerate()
                .map(|(i, h)| series(h, 100.0, color(i), format!("CPU{}: {:.0}%", i + 1, h.0.front().unwrap_or(&0.0))))
                .collect(),
        ),
        y_labels: percent_labels(),
    });

    // Load averages
    let load_max = st.load_hist.iter().map(History::max).fold(st.cores as f64, f64::max).ceil();
    let loads = cpu::load_avgs();
    charts.push(ChartData {
        title: tr("CPU Load Averages").into(),
        series: model(
            [1, 5, 15]
                .iter()
                .enumerate()
                .map(|(i, min)| series(&st.load_hist[i], load_max, color(i), trf("{} minute average: {}", &[min, &format!("{:.2}", loads[i])])))
                .collect(),
        ),
        y_labels: strings((0..5).map(|i| format!("{:.1}", load_max * (4 - i) as f64 / 4.0))),
    });

    // Memory
    charts.push(ChartData {
        title: tr("Memory History").into(),
        series: model(vec![
            series(&st.mem_hist, 100.0, color(0), trf("Memory: {} of {}", &[&format::bytes(m.mem_used), &format::bytes(m.mem_total)])),
            series(&st.swap_hist, 100.0, color(1), trf("Swap: {} of {}", &[&format::bytes(m.swap_used), &format::bytes(m.swap_total)])),
        ]),
        y_labels: percent_labels(),
    });

    // GPU
    if st.gpu.has_gpu() && !st.gpu_hist[0].0.is_empty() {
        charts.push(ChartData {
            title: tr("GPU History").into(),
            series: model(vec![
                series(&st.gpu_hist[0], 100.0, color(5), format!("GPU: {:.0}%", st.gpu_hist[0].0.front().unwrap_or(&0.0))),
                series(&st.gpu_hist[1], 100.0, color(6), format!("VRAM: {:.1}%", st.gpu_hist[1].0.front().unwrap_or(&0.0))),
            ]),
            y_labels: percent_labels(),
        });
    }

    // Network
    let net_max = st.net_hist.iter().map(History::max).fold(1024.0 * 100.0, f64::max);
    charts.push(ChartData {
        title: tr("Network History").into(),
        series: model(vec![
            series(&st.net_hist[0], net_max, color(0), trf("Download: {}/s  Total: {}", &[&format::bytes(net.0), &format::bytes(st.net.0)])),
            series(&st.net_hist[1], net_max, color(2), trf("Upload: {}/s  Total: {}", &[&format::bytes(net.1), &format::bytes(st.net.1)])),
        ]),
        y_labels: byte_labels(net_max),
    });

    // Disk I/O
    let io_max = st.io_hist.iter().map(History::max).fold(1024.0 * 100.0, f64::max);
    charts.push(ChartData {
        title: tr("Disk Read / Write").into(),
        series: model(vec![
            series(&st.io_hist[0], io_max, color(3), trf("Read: {}/s", &[&format::bytes(io.0)])),
            series(&st.io_hist[1], io_max, color(1), trf("Write: {}/s", &[&format::bytes(io.1)])),
        ]),
        y_labels: byte_labels(io_max),
    });

    sync(&st.charts_model, charts);
    sync(
        &st.disks_model,
        st.disks
            .iter()
            .map(|d| DiskUsage {
                name: d.name.clone().into(),
                device: d.device.clone().into(),
                fs_type: d.fs_type.clone().into(),
                text: trf("{} / {}  ({} free)", &[&format::bytes(d.used), &format::bytes(d.size), &format::bytes(d.free)]).into(),
                percent: if d.size > 0 { (d.used as f64 / d.size as f64 * 100.0) as f32 } else { 0.0 },
            })
            .collect(),
    );
}
