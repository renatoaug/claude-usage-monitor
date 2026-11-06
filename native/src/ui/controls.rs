// The small square buttons of the title band.

use eframe::egui::{CornerRadius, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use super::icon::{self, Glyph};
use super::theme::{color, radius};

pub const BUTTON: f32 = 19.0;

/// A 19 px icon button centred on `center`.
pub fn icon_button(ui: &Ui, center: Pos2, id: &str, glyph: Glyph) -> Response {
    let rect = Rect::from_center_size(center, Vec2::splat(BUTTON));
    let resp = ui.interact(rect, ui.scope_id().with(id), Sense::click());
    let (bg, fg) = if resp.hovered() { (color::CONTROL_HOVER, color::ICON_HOVER) } else { (color::CONTROL, color::ICON) };
    let p = ui.painter();
    p.rect(rect, CornerRadius::same(radius::CONTROL), bg, Stroke::new(1.0, color::CONTROL_EDGE), StrokeKind::Inside);
    icon::draw(p, center, glyph, 9.0, fg);
    resp
}

/// A row of icon buttons laid out from `right` leftwards, 4 px apart;
/// returns the index of the one clicked.
pub fn button_row(ui: &Ui, right: f32, y: f32, buttons: &[(&str, Glyph)]) -> Option<usize> {
    let mut clicked = None;
    for (i, (id, glyph)) in buttons.iter().enumerate() {
        let x = right - BUTTON / 2.0 - i as f32 * (BUTTON + 4.0);
        if icon_button(ui, Pos2::new(x, y), id, glyph).clicked() {
            clicked = Some(i);
        }
    }
    clicked
}
