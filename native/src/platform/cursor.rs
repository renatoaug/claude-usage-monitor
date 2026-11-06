// Where the cursor is, even when it isn't over a window that hears it.
//
// Just-the-pet lets clicks fall through its transparent window to whatever
// is behind, so the window can't see the cursor coming back. A light thread
// asks the OS for the cursor's screen position every 40 ms (only while it's
// asked to watch) and wakes the UI only when the cursor enters or leaves one
// of the hot areas: the pet, and its glance while it's up.

use eframe::egui::{Context, Rect};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
pub struct Watch {
    hot: Arc<Mutex<Vec<Rect>>>,
    inside: Arc<AtomicBool>,
    on: Arc<AtomicBool>,
}

impl Watch {
    pub fn new(ctx: &Context) -> Self {
        let w = Watch { hot: Arc::default(), inside: Arc::default(), on: Arc::default() };
        #[cfg(target_os = "macos")]
        {
            let height = mac::primary_height();
            let (t, ctx) = (w.clone(), ctx.clone());
            std::thread::Builder::new()
                .name("cursor".into())
                .spawn(move || loop {
                    if !t.on.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(200));
                        continue;
                    }
                    let at = mac::cursor(height);
                    let inside = t.hot.lock().map(|h| h.iter().any(|r| r.contains(at))).unwrap_or(false);
                    if t.inside.swap(inside, Ordering::Relaxed) != inside {
                        ctx.request_repaint();
                    }
                    std::thread::sleep(Duration::from_millis(40));
                })
                .expect("cursor thread");
        }
        #[cfg(not(target_os = "macos"))]
        let _ = ctx;
        w
    }

    /// Watch these areas (screen coordinates), or stop watching (None).
    pub fn watch(&self, hot: Option<Vec<Rect>>) {
        self.on.store(hot.is_some(), Ordering::Relaxed);
        if let (Some(h), Ok(mut slot)) = (hot, self.hot.lock()) {
            *slot = h;
        }
    }

    /// Whether the cursor is over a hot area. Without an OS answer (not
    /// macOS), `fallback` is what the window itself saw.
    pub fn inside(&self, fallback: bool) -> bool {
        if cfg!(target_os = "macos") { self.inside.load(Ordering::Relaxed) } else { fallback }
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use eframe::egui::Pos2;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSEvent, NSScreen};

    /// The primary screen's height: AppKit counts y up from its bottom edge,
    /// windows (and egui) count down from its top.
    pub fn primary_height() -> f64 {
        MainThreadMarker::new()
            .and_then(|mtm| NSScreen::screens(mtm).firstObject())
            .map(|s| s.frame().size.height)
            .unwrap_or(0.0)
    }

    pub fn cursor(height: f64) -> Pos2 {
        let p = NSEvent::mouseLocation();
        Pos2::new(p.x as f32, (height - p.y) as f32)
    }
}

