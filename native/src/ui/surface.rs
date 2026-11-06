// Surfaces: the widget's card, a pane inside it, the hairline between parts.

use eframe::egui::{Context, CornerRadius, Painter, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, ViewportCommand};

use super::theme::{color, radius};

/// The card: the panel colour, its border and the warm glow from the top
/// left. The whole card drags the window; anything drawn later sits on top.
pub fn card(ui: &Ui, ctx: &Context, rect: Rect) {
    let p = ui.painter();
    p.rect(rect, CornerRadius::same(radius::CARD), color::SURFACE, Stroke::new(1.0, color::BORDER), StrokeKind::Inside);
    let glow = p.with_clip_rect(rect.shrink(1.0));
    let center = rect.left_top() + Vec2::new(rect.width() * 0.2, 0.0);
    for i in 0..16 {
        glow.circle_filled(center, 140.0 - i as f32 * 8.0, color::SURFACE_GLOW.gamma_multiply(0.18));
    }
    let drag = ui.interact(rect, ui.scope_id().with("card-drag"), Sense::click_and_drag());
    if drag.drag_started() {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }
}

/// A dark pane inside the card (the expanded pet's stage).
pub fn pane(p: &Painter, rect: Rect) {
    p.rect(rect, CornerRadius::same(radius::CONTROL), color::PANE, Stroke::new(1.0, color::PANE_EDGE), StrokeKind::Inside);
}

/// A soft round glow behind something (the compact pet).
pub fn halo(p: &Painter, rect: Rect, tint: eframe::egui::Color32) {
    for (r, a) in [(30.0, 0.5), (22.0, 0.8), (14.0, 1.0)] {
        p.circle_filled(rect.center(), r, tint.gamma_multiply(a));
    }
}

/// The 1 px rule between parts of the panel.
pub fn hairline(p: &Painter, left: f32, right: f32, y: f32) {
    p.hline(left..=right, y, Stroke::new(1.0, color::HAIR));
}
