// A number with − and + on either side: "− 80% +".

use eframe::egui::{Align, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};

use super::controls::click_area;
use super::text::text_mid;
use super::theme::{color, radius, text};

const BTN: f32 = 22.0;
const VAL: f32 = 40.0;
const H: f32 = 24.0;

/// A stepper with its right edge at `right`, centred on `mid`; returns -1 or
/// +1 when a side is clicked. A disabled one is faded and inert.
pub fn stepper(ui: &Ui, right: f32, mid: f32, id: &str, value: i32, unit: &str, enabled: bool) -> Option<i32> {
    let rect = Rect::from_min_size(Pos2::new(right - (2.0 * BTN + VAL), mid - H / 2.0), Vec2::new(2.0 * BTN + VAL, H));
    let fade = if enabled { 1.0 } else { 0.4 };
    let p = ui.painter();
    p.rect(rect, CornerRadius::same(radius::CONTROL), color::WASH, Stroke::new(1.0, color::HAIR), StrokeKind::Inside);
    let mut step = None;
    for (i, (sign, glyph)) in [(-1, "−"), (1, "+")].into_iter().enumerate() {
        let b = Rect::from_min_size(Pos2::new(if i == 0 { rect.left() } else { rect.right() - BTN }, rect.top()), Vec2::new(BTN, H));
        let hovered = enabled && {
            let r = click_area(ui, b, (id, sign));
            if r.clicked() {
                step = Some(sign);
            }
            r.hovered()
        };
        if hovered {
            p.rect_filled(b.shrink(1.0), CornerRadius::same(radius::CONTROL), color::MENU_HOVER);
        }
        text_mid(p, b.center().x, mid, Align::Center, glyph, text::BODY, color::FG_MUTED.gamma_multiply(fade));
    }
    text_mid(p, rect.center().x, mid, Align::Center, &format!("{value}{unit}"), text::BODY, color::FG.gamma_multiply(fade));
    step
}
