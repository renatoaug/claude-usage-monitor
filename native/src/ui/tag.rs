// A small bordered label: the plan badge, the live tag.

use eframe::egui::{Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};

use super::theme::{radius, text};

pub struct TagStyle {
    pub fg: Color32,
    pub edge: Color32,
    pub wash: Color32,
    pub pad: Vec2,
}

/// Draw `label` anchored by `align` at `pos`; returns the tag's box.
pub fn tag(p: &Painter, pos: Pos2, align: Align2, label: &str, style: &TagStyle) -> Rect {
    let r = p.text(pos, align, label, FontId::proportional(text::TINY), style.fg);
    let frame = r.expand2(style.pad);
    p.rect(frame, CornerRadius::same(radius::TAG), style.wash, Stroke::new(1.0, style.edge), StrokeKind::Inside);
    frame
}
