// Clauddy in the menu bar: a template icon (macOS tints it for the theme)
// with the session % beside it, a left click that opens the panel under it,
// and a right-click menu. On Windows and Linux it's a tray icon with the %
// in its tooltip.

use eframe::egui::{Context, Pos2, Rect, Vec2};
use std::sync::{Arc, Mutex};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

static ICON: &[u8] = include_bytes!("../../assets/tray-template.png");

/// What happened in the menu bar since the last frame.
#[derive(Debug, Clone, PartialEq)]
pub enum TrayEvent {
    /// a left click, with the icon's box in physical pixels
    Clicked(Rect),
    Open,
    Floating,
    Quit,
}

pub struct Tray {
    icon: TrayIcon,
    title: String,
    events: Arc<Mutex<Vec<TrayEvent>>>,
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
        let (open, floating, quit) = (MenuItem::new("Open Clauddy", true, None), MenuItem::new("Floating widget", true, None), MenuItem::new("Quit", true, None));
        let menu = Menu::new();
        menu.append_items(&[&open, &PredefinedMenuItem::separator(), &floating, &PredefinedMenuItem::separator(), &quit]).ok()?;
        let icon = TrayIconBuilder::new()
            .with_icon_templated(decode(ICON)?)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("Clauddy")
            .build()
            .ok()?;

        // the menu bar answers on its own thread: queue, then wake the UI
        let (q, c) = (events.clone(), ctx.clone());
        TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
            if let TrayIconEvent::Click { rect, button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                let r = Rect::from_min_size(
                    Pos2::new(rect.position.x as f32, rect.position.y as f32),
                    Vec2::new(rect.size.width as f32, rect.size.height as f32),
                );
                if let Ok(mut v) = q.lock() {
                    v.push(TrayEvent::Clicked(r));
                }
                c.request_repaint();
            }
        }));
        let (q, c) = (events.clone(), ctx.clone());
        let ids = (open.id().clone(), floating.id().clone(), quit.id().clone());
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            let ev = if e.id == ids.0 {
                TrayEvent::Open
            } else if e.id == ids.1 {
                TrayEvent::Floating
            } else if e.id == ids.2 {
                TrayEvent::Quit
            } else {
                return;
            };
            if let Ok(mut v) = q.lock() {
                v.push(ev);
            }
            c.request_repaint();
        }));
        Some(Tray { icon, title: String::new(), events })
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
        TrayIconEvent::set_event_handler(None::<fn(TrayIconEvent)>);
        MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
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

#[cfg(test)]
mod tests {
    #[test]
    fn the_menu_bar_reads_the_session_and_catches_fire() {
        assert_eq!(super::title(Some(42.4), 90.0), "42%");
        assert_eq!(super::title(Some(93.0), 90.0), "🔥 93%");
        assert_eq!(super::title(None, 90.0), "");
    }

    #[test]
    fn the_template_icon_decodes() {
        assert!(super::decode(super::ICON).is_some());
    }
}
