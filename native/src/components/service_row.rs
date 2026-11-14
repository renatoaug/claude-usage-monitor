// One service on the compact card: logo, name and % on top, the bar beneath,
// and the week as a thinner bar with its % under that.

use eframe::egui::{Align2, FontId, Painter, Pos2, Rect, Vec2};

use super::logos::{Logos, Provider};
use crate::ui::bar::bar;
use crate::ui::skeleton::sk_line;
use crate::ui::theme::{color, text};

// room under the bar for the week's thinner one
pub const ROW_H: f32 = 29.0;
pub const ROW_GAP: f32 = 5.0;
// a reading this high marks the row urgent
const URGENT_AT: f64 = 80.0;

pub struct ServiceRow {
    pub provider: Provider,
    pub pct: Option<f64>,
    pub reset_ms: Option<i64>,
    /// the week's %; None when the session already is the week (weekly-only
    /// plans) or there is no week (Cursor)
    pub week: Option<f64>,
    pub week_reset_ms: Option<i64>,
}

/// Draws the row; returns the week's area (its % and bar), if it has one.
pub fn service_row(p: &Painter, logos: &Logos, top_left: Pos2, width: f32, row: &ServiceRow, selected: bool) -> Option<Rect> {
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
    let name = if urgent && row.week.is_none() { format!("{} !", row.provider.name()) } else { row.provider.name().into() };
    let name_end = p.text(Pos2::new(x + 17.0, y), Align2::LEFT_CENTER, name, FontId::proportional(text::SMALL), fg).right();
    let right = top_left.x + width;
    let value = row.pct.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "—".into());
    let mut fill = if urgent { color::HOT } else { row.provider.bar() };
    if !selected && !urgent {
        fill = fill.gamma_multiply(0.55);
    }
    let left = x + 17.0;
    let ty = top_left.y + 23.5;
    let Some(w) = row.week else {
        p.text(Pos2::new(right, y), Align2::RIGHT_CENTER, value, FontId::proportional(text::DISPLAY), fg);
        bar(p, Rect::from_min_size(Pos2::new(left, ty), Vec2::new(right - left, 2.0)), row.pct.map(|v| (v / 100.0) as f32), fill, color::TRACK_THIN);
        return None;
    };
    // the week on the right, smaller and dimmer: its % over its own bar; the
    // session's % and bar move left to make room
    let ink = if w >= URGENT_AT { color::URGENT } else { color::FG_MUTED };
    let label = format!("{w:.0}%");
    let ww = p.layout_no_wrap(label.clone(), FontId::proportional(text::TINY), ink).size().x.max(18.0);
    let split = right - ww - 7.0;
    // a wide reading (100%) steps down a size rather than run into the name
    let wide = p.layout_no_wrap(value.clone(), FontId::proportional(text::DISPLAY), fg).size().x;
    let size = if split - wide < name_end + 4.0 { text::VALUE } else { text::DISPLAY };
    p.text(Pos2::new(split, y), Align2::RIGHT_CENTER, value, FontId::proportional(size), fg);
    bar(p, Rect::from_min_size(Pos2::new(left, ty), Vec2::new(split - left, 2.0)), row.pct.map(|v| (v / 100.0) as f32), fill, color::TRACK_THIN);
    crate::ui::text::text_mid(p, right, y + 3.0, eframe::egui::Align::Max, &label, text::TINY, ink);
    let wf = if w >= URGENT_AT { color::HOT } else { fill.gamma_multiply(0.45) };
    bar(p, Rect::from_min_size(Pos2::new(right - ww, ty + 0.25), Vec2::new(ww, 1.5)), Some((w / 100.0) as f32), wf, color::TRACK_THIN);
    Some(Rect::from_min_max(Pos2::new(right - ww, top_left.y), Pos2::new(right, top_left.y + ROW_H)))
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
