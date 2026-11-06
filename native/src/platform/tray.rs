// Clauddy in the menu bar: a template icon (macOS tints it for the theme)
// with the session % beside it. A click opens its menu: the limits first
// (session, week, per-model weeks, each with its reset), then Open, back to
// floating, and Quit. On Windows and Linux it's a tray icon with the % in
// its tooltip.

use eframe::egui::{Context, Pos2, Rect, Vec2};

use std::sync::{Arc, Mutex};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

static ICON: &[u8] = include_bytes!("../../assets/tray-template.png");

/// What was picked in the menu since the last frame.
#[derive(Debug, Clone, PartialEq)]
pub enum TrayEvent {
    Open,
    Floating,
}

pub struct Tray {
    icon: TrayIcon,
    title: String,
    /// the limit lines at the top of the menu, as last shown
    lines: Vec<String>,
    open: MenuItem,
    floating: MenuItem,
    quit: MenuItem,
    events: Arc<Mutex<Vec<TrayEvent>>>,
}

// the menu: the limit lines (read-only), then the actions
fn build_menu(lines: &[String], open: &MenuItem, floating: &MenuItem, quit: &MenuItem) -> Option<Menu> {
    let menu = Menu::new();
    for l in lines {
        menu.append(&MenuItem::new(l, false, None)).ok()?;
    }
    if !lines.is_empty() {
        menu.append(&PredefinedMenuItem::separator()).ok()?;
    }
    menu.append_items(&[open, floating, &PredefinedMenuItem::separator(), quit]).ok()?;
    Some(menu)
}

fn decode(bytes: &[u8]) -> Option<tray_icon::Icon> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut r = dec.read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()?];
    let info = r.next_frame(&mut buf).ok()?;
    tray_icon::Icon::from_rgba(buf[..info.buffer_size()].to_vec(), info.width, info.height).ok()
}

impl Tray {
    /// Put the icon in the menu bar. Call on the main thread.
    pub fn new(ctx: &Context) -> Option<Self> {
        let events: Arc<Mutex<Vec<TrayEvent>>> = Arc::default();
        let (open, floating, quit) = (MenuItem::new("Open Clauddy", true, None), MenuItem::new("Floating widget", true, None), MenuItem::new("Quit Clauddy", true, None));
        let menu = build_menu(&[], &open, &floating, &quit)?;
        let icon = TrayIconBuilder::new()
            .with_icon_templated(decode(ICON)?)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(true)
            .with_tooltip("Clauddy")
            // macOS keeps where the icon was dragged to across launches
            .with_autosave_name("clauddy")
            .build()
            .ok()?;
        let (q, c) = (events.clone(), ctx.clone());
        let ids = (open.id().clone(), floating.id().clone(), quit.id().clone());
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            let ev = if e.id == ids.0 {
                TrayEvent::Open
            } else if e.id == ids.1 {
                TrayEvent::Floating
            } else if e.id == ids.2 {
                // no frame may come to answer it while the window is hidden
                std::process::exit(0);
            } else {
                return;
            };
            // a hidden window gets no frames: show it so the next one answers
            crate::platform::wake();
            if let Ok(mut v) = q.lock() {
                v.push(ev);
            }
            c.request_repaint();
        }));
        Some(Tray { icon, title: String::new(), lines: vec![], open, floating, quit, events })
    }

    /// The limit lines at the top of the menu; rebuilt only when they change.
    pub fn set_lines(&mut self, lines: Vec<String>) {
        if lines == self.lines {
            return;
        }
        if let Some(menu) = build_menu(&lines, &self.open, &self.floating, &self.quit) {
            self.icon.set_menu(Some(Box::new(menu)));
            self.lines = lines;
        }
    }

    /// The icon's box, in physical pixels, where the OS says.
    pub fn rect(&self) -> Option<Rect> {
        self.icon.rect().map(|r| {
            Rect::from_min_size(Pos2::new(r.position.x as f32, r.position.y as f32), Vec2::new(r.size.width as f32, r.size.height as f32))
        })
    }

    pub fn take_events(&self) -> Vec<TrayEvent> {
        self.events.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default()
    }

    /// The text beside the icon (macOS) or in its tooltip; set only on change.
    pub fn set_title(&mut self, title: &str) {
        if title == self.title {
            return;
        }
        self.title = title.to_string();
        #[cfg(target_os = "macos")]
        self.icon.set_title(Some(format!(" {title}")).filter(|_| !title.is_empty()));
        let _ = self.icon.set_tooltip(Some(if title.is_empty() { "Clauddy".to_string() } else { format!("Clauddy · {title}") }));
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
    }
}

/// A limit's line in the menu: "Session 39% · resets 4:40 PM".
pub fn line(label: &str, pct: Option<f64>, reset_ms: Option<i64>) -> String {
    let value = pct.map(|p| format!("{p:.0}%")).unwrap_or_else(|| "—".into());
    match reset_ms {
        Some(ms) if pct.is_some() => format!("{label} {value} · resets {}", crate::format::clock(ms)),
        _ => format!("{label} {value}"),
    }
}

/// The menu bar's text: the session % (🔥 past the fire threshold), or
/// nothing while there's no reading.
pub fn title(session_pct: Option<f64>, fire: f64) -> String {
    match session_pct {
        Some(p) if p >= fire => format!("🔥 {p:.0}%"),
        Some(p) => format!("{p:.0}%"),
        None => String::new(),
    }
}

/// Several services: one number, the session closest to its limit. A
/// title per service grew wide enough to land behind the notch, where
/// macOS hides it; the menu lists each one.
pub fn title_hottest(sessions: &[Option<f64>], fire: f64) -> String {
    title(sessions.iter().flatten().copied().reduce(f64::max), fire)
}

#[cfg(test)]
mod tests {
    #[test]
    fn several_services_show_the_hottest() {
        assert_eq!(super::title_hottest(&[Some(64.0), Some(28.0), None], 90.0), "64%");
        assert_eq!(super::title_hottest(&[Some(20.0), Some(93.0)], 90.0), "🔥 93%");
        assert_eq!(super::title_hottest(&[None, None], 90.0), "");
    }

    #[test]
    fn the_menu_bar_reads_the_session_and_catches_fire() {
        assert_eq!(super::title(Some(42.4), 90.0), "42%");
        assert_eq!(super::title(Some(93.0), 90.0), "🔥 93%");
        assert_eq!(super::title(None, 90.0), "");
    }

    #[test]
    fn a_limit_line_names_its_reset() {
        assert!(super::line("Session", Some(39.2), Some(60_000)).starts_with("Session 39% · resets "));
        assert_eq!(super::line("Weekly", None, Some(60_000)), "Weekly —");
    }

    #[test]
    fn the_template_icon_decodes() {
        assert!(super::decode(super::ICON).is_some());
    }
}
