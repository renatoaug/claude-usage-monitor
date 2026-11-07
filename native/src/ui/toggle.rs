// An on/off switch: a pill with a knob that slides.

use eframe::egui::{CornerRadius, Pos2, Rect, Response, Ui, Vec2};

use super::controls::click_area;
use super::theme::{color, mix, radius};

pub const SIZE: Vec2 = Vec2::new(29.0, 16.0);

/// A switch with its right edge at `right`, centred on `mid`. A disabled one
/// is drawn faded and ignores clicks.
pub fn toggle(ui: &Ui, right: f32, mid: f32, id: &str, on: bool, enabled: bool) -> Response {
    let rect = Rect::from_min_size(Pos2::new(right - SIZE.x, mid - SIZE.y / 2.0), SIZE);
    let resp = if enabled { click_area(ui, rect, id) } else { ui.interact(rect, ui.scope_id().with(id), eframe::egui::Sense::hover()) };
    let t = ui.ctx().animate_bool_with_time(ui.scope_id().with((id, "on")), on, 0.18);
    let fade = if enabled { 1.0 } else { 0.4 };
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(radius::CONTROL + 2), mix(color::SWITCH_OFF, color::SWITCH_ON, t).gamma_multiply(fade));
    let knob = Pos2::new(rect.left() + 8.0 + 13.0 * t, mid);
    p.circle_filled(knob, 6.0, color::KNOB.gamma_multiply(fade));
    resp
}
