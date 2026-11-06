// A small bordered label: the plan badge, the live tag.

use eframe::egui::{Align, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};

use super::text::text_mid;

use super::theme::{radius, text};

pub struct TagStyle {
    pub fg: Color32,
    pub edge: Color32,
    pub wash: Color32,
    pub pad: Vec2,
}

/// Draw `label` in a box centred on `mid`, its left (`Align::Min`) or right
/// (`Align::Max`) edge at `x`; returns the box.
pub fn tag(p: &Painter, x: f32, mid: f32, h: Align, label: &str, style: &TagStyle) -> Rect {
    let w = width_of(p, label) + 2.0 * style.pad.x;
    let left = if h == Align::Max { x - w } else { x };
    let frame = Rect::from_min_size(Pos2::new(left, mid - 6.0 - style.pad.y + 2.0), Vec2::new(w, 8.0 + 2.0 * style.pad.y));
    p.rect(frame, CornerRadius::same(radius::TAG), style.wash, Stroke::new(1.0, style.edge), StrokeKind::Inside);
    text_mid(p, frame.left() + style.pad.x, mid, Align::Min, label, text::TINY, style.fg);
    frame
}

fn width_of(p: &Painter, label: &str) -> f32 {
    p.layout_no_wrap(label.into(), FontId::proportional(text::TINY), style_fallback()).size().x
}

fn style_fallback() -> Color32 {
    Color32::PLACEHOLDER
}
