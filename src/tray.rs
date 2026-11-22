//! System tray icon (StatusNotifierItem over D-Bus), like Stacer's tray menu.
//! Menu actions are sent back to the Slint event loop.

use ksni::blocking::TrayMethods;
use ksni::menu::StandardItem;
use ksni::{Icon, MenuItem, ToolTip, Tray};

/// What a tray click asks the UI to do.
#[derive(Clone, Debug)]
pub enum TrayEvent {
    /// Left click: show the window, or hide it when it is visible.
    Toggle,
    /// Show the window on a page ("" keeps the current page).
    Open(String),
    Quit,
}

type Sender = Box<dyn Fn(TrayEvent) + Send + 'static>;

pub struct KaptanTray {
    /// (page id, translated label)
    pages: Vec<(String, String)>,
    open_label: String,
    quit_label: String,
    icon: Vec<Icon>,
    send: Sender,
}

pub type TrayHandle = ksni::blocking::Handle<KaptanTray>;

impl KaptanTray {
    pub fn set_labels(&mut self, pages: Vec<(String, String)>, open_label: String, quit_label: String) {
        self.pages = pages;
        self.open_label = open_label;
        self.quit_label = quit_label;
    }
}

impl Tray for KaptanTray {
    fn id(&self) -> String {
        "kaptan".into()
    }

    fn title(&self) -> String {
        "Kaptan".into()
    }

    fn icon_name(&self) -> String {
        // used when the theme has it (installed packages), the pixmap otherwise
        "kaptan".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        self.icon.clone()
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip { title: "Kaptan".into(), icon_pixmap: self.icon.clone(), ..Default::default() }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        (self.send)(TrayEvent::Toggle);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut items: Vec<MenuItem<Self>> = vec![
            StandardItem {
                label: self.open_label.clone(),
                activate: Box::new(|t: &mut Self| (t.send)(TrayEvent::Open(String::new()))),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
        ];
        for (id, label) in &self.pages {
            let id = id.clone();
            items.push(
                StandardItem {
                    label: label.clone(),
                    activate: Box::new(move |t: &mut Self| (t.send)(TrayEvent::Open(id.clone()))),
                    ..Default::default()
                }
                .into(),
            );
        }
        items.push(MenuItem::Separator);
        items.push(
            StandardItem {
                label: self.quit_label.clone(),
                icon_name: "application-exit".into(),
                activate: Box::new(|t: &mut Self| (t.send)(TrayEvent::Quit)),
                ..Default::default()
            }
            .into(),
        );
        items
    }
}

/// Render the app icon (SVG) to ARGB32 pixmaps in a few sizes.
fn render_icon() -> Vec<Icon> {
    use resvg::{tiny_skia, usvg};
    let svg = include_bytes!("../ui/icons/kaptan.svg");
    let Ok(tree) = usvg::Tree::from_data(svg, &usvg::Options::default()) else { return vec![] };
    [22u32, 32, 48, 64]
        .iter()
        .filter_map(|&size| {
            let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
            let scale = size as f32 / tree.size().width().max(tree.size().height());
            resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
            let data = pixmap
                .pixels()
                .iter()
                .flat_map(|p| {
                    let c = p.demultiply();
                    [c.alpha(), c.red(), c.green(), c.blue()] // ARGB, network byte order
                })
                .collect();
            Some(Icon { width: size as i32, height: size as i32, data })
        })
        .collect()
}

/// Start the tray service. `None` when the desktop has no StatusNotifier host.
pub fn start(pages: Vec<(String, String)>, open_label: String, quit_label: String, send: Sender) -> Option<TrayHandle> {
    let tray = KaptanTray { pages, open_label, quit_label, icon: render_icon(), send };
    match tray.spawn() {
        Ok(handle) => Some(handle),
        Err(e) => {
            eprintln!("kaptan: system tray not available: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn icon_renders() {
        let icons = super::render_icon();
        assert_eq!(icons.len(), 4);
        let i = &icons[0];
        assert_eq!(i.data.len(), (i.width * i.height * 4) as usize);
        // not fully transparent
        assert!(i.data.chunks(4).any(|p| p[0] > 0));
    }
}
