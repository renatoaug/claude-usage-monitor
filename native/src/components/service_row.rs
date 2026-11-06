// One service on the compact card: logo, name and % on top, the bar beneath.

use eframe::egui::{Align2, FontId, Painter, Pos2, Rect, Vec2};

use super::logos::{Logos, Provider};
use crate::ui::bar::bar;
use crate::ui::skeleton::sk_line;
use crate::ui::theme::{color, text};

pub const ROW_H: f32 = 29.0;
pub const ROW_GAP: f32 = 5.0;
// a reading this high marks the row urgent
const URGENT_AT: f64 = 80.0;

pub struct ServiceRow {
    pub provider: Provider,
    pub pct: Option<f64>,
    pub reset_ms: Option<i64>,
}

pub fn service_row(p: &Painter, logos: &Logos, top_left: Pos2, width: f32, row: &ServiceRow, selected: bool) {
    let urgent = row.pct.is_some_and(|v| v >= URGENT_AT);
    let fg = if urgent {
        color::URGENT
    } else if selected {
        color::FG_STRONG
    } else {
        color::FG_SECTION
    };
    let x = top_left.x + 5.0;
    let y = top_left.y + 11.0;
    let logo = Rect::from_center_size(Pos2::new(x + 6.0, y), Vec2::splat(12.0));
    logos.draw(p, row.provider, logo, if selected { row.provider.tint() } else { fg });
    let name = if urgent { format!("{} !", row.provider.name()) } else { row.provider.name().into() };
    p.text(Pos2::new(x + 17.0, y), Align2::LEFT_CENTER, name, FontId::proportional(text::SMALL), fg);
    let right = top_left.x + width;
    let value = row.pct.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "—".into());
    p.text(Pos2::new(right, y), Align2::RIGHT_CENTER, value, FontId::proportional(text::DISPLAY), fg);
    let track = Rect::from_min_size(Pos2::new(x + 17.0, top_left.y + 23.5), Vec2::new(right - (x + 17.0), 2.0));
    let mut fill = if urgent { color::HOT } else { row.provider.bar() };
    if !selected && !urgent {
        fill = fill.gamma_multiply(0.55);
    }
    bar(p, track, row.pct.map(|v| (v / 100.0) as f32), fill, color::TRACK_THIN);
}

/// A row's placeholder while the first reading loads.
pub fn service_row_skeleton(p: &Painter, logos: &Logos, top_left: Pos2, width: f32, provider: Provider) {
    let x = top_left.x + 5.0;
    let y = top_left.y + 11.0;
    logos.draw(p, provider, Rect::from_center_size(Pos2::new(x + 6.0, y), Vec2::splat(12.0)), color::FG_SECTION);
    p.text(Pos2::new(x + 17.0, y), Align2::LEFT_CENTER, provider.name(), FontId::proportional(text::SMALL), color::FG_SECTION);
    let right = top_left.x + width;
    sk_line(p, right - 30.0, y, 30.0, 12.0);
    bar(p, Rect::from_min_size(Pos2::new(x + 17.0, top_left.y + 23.5), Vec2::new(right - (x + 17.0), 2.0)), None, color::TRACK_THIN, color::TRACK_THIN);
}
