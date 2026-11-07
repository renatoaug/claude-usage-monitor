// Placeholders that hold a reading's place while it loads, so the card keeps
// its shape instead of flashing a wrong message.

use eframe::egui::{CornerRadius, Painter, Pos2, Rect, Vec2};

use super::theme::{color, radius};

/// A placeholder block. It breathes gently, so a load reads as under way.
pub fn sk(p: &Painter, rect: Rect) {
    let t = p.ctx().input(|i| i.time) as f32;
    let breath = 0.7 + 0.3 * (t * std::f32::consts::TAU / 1.4).cos();
    p.rect_filled(rect, CornerRadius::same(radius::TAG), color::SKELETON.gamma_multiply(breath));
    p.ctx().request_repaint_after(std::time::Duration::from_millis(60));
}

/// A line of text's placeholder, `w` wide and `h` tall, centred on `y`.
pub fn sk_line(p: &Painter, left: f32, y: f32, w: f32, h: f32) {
    sk(p, Rect::from_min_size(Pos2::new(left, y - h / 2.0), Vec2::new(w, h)));
}
