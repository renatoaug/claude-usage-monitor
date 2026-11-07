// The pet's speech bubble: a remark in the same pixel-art bubble as the
// pet-only glance (dark outline, two-tone paper, a stepped tail), its tail
// touching the point it speaks from. A click anywhere on it dismisses it.

use eframe::egui::{FontFamily, FontId, Pos2, Rect, Ui, Vec2};

use super::glance::{O, TAIL, pixel_box, tail};
use crate::ui::controls::click_area;
use crate::ui::theme::color;

const PAD: Vec2 = Vec2::new(7.0, 5.0);

fn font() -> FontId {
    FontId::new(10.5, FontFamily::Monospace)
}

/// Draw `text` wrapped to `max_w`, the tail's tip at `tip`, the tail `tail_x`
/// in from the bubble's left (clamped to stay on it), kept inside `bounds`.
/// `shown` fades it (0..1). Returns true when it was clicked.
pub fn bubble(ui: &Ui, text: &str, tip: Pos2, tail_x: f32, max_w: f32, bounds: Rect, shown: f32) -> bool {
    let mut p = ui.painter().clone();
    p.multiply_opacity(shown);
    let galley = p.layout(text.to_string(), font(), color::INK, max_w - 2.0 * PAD.x);
    let size = galley.size() + 2.0 * PAD + Vec2::new(0.0, 3.0);
    // rises 4 px in three steps as it appears, like the glance
    let slide = ((1.0 - shown) * 3.0).ceil() / 3.0 * 4.0;
    // kept inside `bounds`: a tall remark sits lower and its tail is shorter
    let top = (tip.y - TAIL - O - size.y).max(bounds.top() + O) + slide;
    let left = (tip.x - tail_x).clamp(bounds.left() + O, (bounds.right() - O - size.x).max(bounds.left() + O));
    let r = Rect::from_min_size(Pos2::new(left.round(), top.round()), Vec2::new(size.x.round(), size.y.round()));
    pixel_box(&p, r.translate(Vec2::new(0.0, O)), color::BUBBLE_SHADOW);
    pixel_box(&p, r, color::INK);
    p.rect_filled(r, 0.0, color::PAPER);
    p.rect_filled(Rect::from_min_size(r.left_bottom() - Vec2::new(0.0, 4.0), Vec2::new(r.width(), 4.0)), 0.0, color::PAPER_SHADE);
    p.rect_filled(Rect::from_min_size(r.min, Vec2::new(r.width(), 2.0)), 0.0, color::PAPER_LIGHT);
    tail(&p, tip.x.clamp(r.left() + 12.0, r.right() - 12.0), r.bottom() + O);
    p.galley(r.min + PAD, galley, color::INK);
    shown > 0.5 && click_area(ui, r.expand(O), "bubble").clicked()
}
