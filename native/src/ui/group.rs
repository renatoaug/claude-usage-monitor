// The settings pattern: a muted label over a rounded group of equal rows,
// a label on the left and a compact control on the right.

use eframe::egui::{Align, CornerRadius, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};

use super::surface::hairline;
use super::text::text_mid;
use super::theme::{color, radius, text};

pub const ROW: f32 = 36.0;
pub const LABEL_H: f32 = 16.0;
pub const GAP: f32 = 14.0;

/// The label above a group, at `y`; returns where the group starts.
pub fn group_label(p: &Painter, left: f32, y: f32, label: &str) -> f32 {
    text_mid(p, left, y + 5.0, Align::Min, label, text::SMALL, color::FG_MUTED);
    y + LABEL_H
}

/// A group of `rows` rows at `y`; returns each row's middle, for the
/// caller to place its label and control.
pub fn group(p: &Painter, left: f32, right: f32, y: f32, rows: usize) -> Vec<f32> {
    let rect = Rect::from_min_size(Pos2::new(left, y), Vec2::new(right - left, rows as f32 * ROW));
    p.rect(rect, CornerRadius::same(radius::CONTROL + 4), color::GROUP, Stroke::new(1.0, color::HAIR), StrokeKind::Inside);
    (0..rows)
        .map(|i| {
            let top = y + i as f32 * ROW;
            if i > 0 {
                hairline(p, left + 10.0, right - 10.0, top);
            }
            top + ROW / 2.0
        })
        .collect()
}

/// A row's label; `sub` rows (the heads-up thresholds) read quieter.
pub fn row_label(p: &Painter, left: f32, mid: f32, label: &str, sub: bool) -> Rect {
    text_mid(p, left + 10.0, mid, Align::Min, label, text::BODY, if sub { color::FG_MUTED } else { color::ROW_LABEL })
}

/// The height of a label plus a group of `rows`.
pub fn block_h(rows: usize) -> f32 {
    LABEL_H + rows as f32 * ROW
}
