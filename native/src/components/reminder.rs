// "Notify at reset": a one-shot reminder for the shown service's session
// reset, offered once usage is high enough to care, and while the reading
// is fresh enough to trust the reset time.

use eframe::egui::{Align, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};
use std::time::Duration;

use crate::components::logos::Provider;
use crate::poller::Snapshot;
use crate::ui::controls::click_area;
use crate::ui::text::text_mid;
use crate::ui::theme::{color, radius, text};

/// The room it takes under the session meter: the button, and the line
/// that says why an ask was refused.
pub const HEIGHT: f32 = 30.0;
pub const ERROR_H: f32 = 16.0;
/// a reading older than this can't be trusted for the reset time
const FRESH: Duration = Duration::from_secs(15 * 60);
/// how long a refusal stays on screen
pub const ERROR_FOR: Duration = Duration::from_secs(6);

/// Whether the button shows for this service, and if so whether it's on.
pub fn state(snap: &Snapshot, provider: Provider, claude: Option<(f64, Option<i64>)>, heads_up: f64) -> Option<bool> {
    let armed = snap.reminders.iter().any(|r| r.provider == provider && (provider != Provider::Claude || r.key == crate::reminders::key(provider, &snap.active)));
    if armed {
        return Some(true);
    }
    let now = crate::usage::now_ms();
    let ext = |w: Option<&crate::codex::Window>, at: Option<i64>| (w.and_then(|w| w.pct), w.and_then(|w| w.reset_ms), at.is_some_and(|t| now - t < FRESH.as_millis() as i64));
    let (pct, reset, fresh) = match provider {
        Provider::Claude => (claude.map(|s| s.0), claude.and_then(|s| s.1), snap.limits_at.is_some_and(|t| t.elapsed() < FRESH)),
        Provider::Codex => snap.codex.as_ref().map_or((None, None, false), |c| ext(c.session.as_ref(), c.limits_at)),
        Provider::Cursor => snap.cursor.as_ref().map_or((None, None, false), |c| ext(c.session.as_ref(), c.limits_at)),
    };
    (fresh && reset.is_some_and(|r| r > 0) && pct.is_some_and(|p| p >= heads_up)).then_some(false)
}

/// The refusal to show under the button, while it's recent.
pub fn error(snap: &Snapshot) -> Option<&str> {
    snap.reminder_error.as_ref().filter(|(_, at)| at.elapsed() < ERROR_FOR).map(|(e, _)| e.as_str())
}

/// The button at (left, y). Returns true when clicked.
pub fn button(ui: &Ui, left: f32, y: f32, on: bool) -> bool {
    let p = ui.painter();
    let label = if on { "Reminder on" } else { "Notify at reset" };
    // on: a drawn check before the words (the UI font has no ✓)
    let mark = if on { 12.0 } else { 0.0 };
    let w = p.layout_no_wrap(label.into(), eframe::egui::FontId::proportional(text::SMALL), color::FG).size().x + 14.0 + mark;
    let r = Rect::from_min_size(Pos2::new(left, y), Vec2::new(w, 22.0));
    let resp = click_area(ui, r, "reminder");
    let bg = if resp.hovered() { color::TILE_ON } else { color::TILE };
    let (fg, edge) = if on { (color::LIVE, color::LIVE_EDGE) } else { (color::FG_MUTED, color::HAIR) };
    p.rect(r, CornerRadius::same(radius::CONTROL), bg, Stroke::new(1.0, edge), StrokeKind::Inside);
    let fg = if resp.hovered() { color::FG } else { fg };
    if on {
        let (x, y) = (r.left() + 7.0, r.center().y);
        p.line_segment([Pos2::new(x, y), Pos2::new(x + 3.0, y + 3.0)], Stroke::new(1.4, fg));
        p.line_segment([Pos2::new(x + 3.0, y + 3.0), Pos2::new(x + 8.5, y - 3.5)], Stroke::new(1.4, fg));
    }
    text_mid(p, r.left() + 7.0 + mark, r.center().y, Align::Min, label, text::SMALL, fg);
    resp.clicked()
}

/// Seconds to keep repainting so a refusal fades out on time.
pub fn error_left(snap: &Snapshot) -> Option<Duration> {
    snap.reminder_error.as_ref().map(|(_, at)| ERROR_FOR.saturating_sub(at.elapsed())).filter(|d| !d.is_zero())
}

