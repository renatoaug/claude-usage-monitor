// The night behind the expanded pet: a dotted sky and ground, two blocky
// clouds and a scatter of stars. Static, so it costs nothing to keep.

use eframe::egui::{Painter, Pos2, Rect, Vec2};

use crate::ui::theme::color;

// drawn in a 216 x 135 box stretched to the pane, as the Electron SVG was
const BOX: Vec2 = Vec2::new(216.0, 135.0);
const STARS: [(f32, f32); 8] = [(100., 16.), (60., 22.), (150., 100.), (196., 56.), (205., 98.), (128., 26.), (182., 22.), (202., 14.)];

pub fn night_sky(p: &Painter, pane: Rect) {
    let (sx, sy) = (pane.width() / BOX.x, pane.height() / BOX.y);
    let at = |x: f32, y: f32| Pos2::new(pane.left() + x * sx, pane.top() + y * sy);
    let block = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(at(x, y), Vec2::new(w * sx, h * sy));

    let mut x = 4.0;
    while x <= 212.0 {
        p.circle_filled(at(x, 6.0), 1.3 * sx.min(sy), color::SKY_DOT);
        p.circle_filled(at(x, 130.0), 1.3 * sx.min(sy), color::SKY_DOT);
        x += 9.0;
    }
    for (cx, cy, bottom, middle, top) in [(16.0, 10.0, 58.0, 42.0, 22.0), (120.0, 50.0, 40.0, 28.0, 15.0)] {
        p.rect_filled(block(cx + 12.0, cy, top, 9.0), 0.0, color::CLOUD);
        p.rect_filled(block(cx + 5.0, cy + 7.0, middle, 9.0), 0.0, color::CLOUD);
        p.rect_filled(block(cx, cy + 14.0, bottom, 10.0), 0.0, color::CLOUD);
    }
    for (x, y) in STARS {
        p.rect_filled(block(x - 3.0, y - 0.7, 6.0, 1.4), 0.0, color::STAR);
        p.rect_filled(block(x - 0.7, y - 3.0, 1.4, 6.0), 0.0, color::STAR);
    }
}
