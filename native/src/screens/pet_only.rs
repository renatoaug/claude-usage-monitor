// Just the pet, floating. Dragging it moves the window; hovering shows the
// usage glance above its head, whose button (or Enter) opens the compact
// monitor. A click on the pet itself changes nothing.

use eframe::egui::{Context, CursorIcon, Key, Pos2, Rect, Sense, Ui, Vec2, ViewportCommand};

use super::{Action, Frame, MARGIN, Size, compact};
use crate::components::glance::glance;

const CARD: Vec2 = Vec2::new(144.0, 166.0);
// the pet sits low in the window; the glance rises above it
const STAGE: Vec2 = Vec2::new(100.0, 102.0);

pub fn size() -> [f32; 2] {
    [CARD.x + 2.0 * MARGIN, CARD.y + 2.0 * MARGIN]
}

/// `glance` forces the glance on or off (snapshots); None follows the pointer.
pub fn draw(ui: &Ui, ctx: &Context, f: &Frame, glance_override: Option<bool>) -> Vec<Action> {
    let mut actions = vec![];
    let rect = ctx.content_rect().shrink(MARGIN);
    let stage = Rect::from_min_size(Pos2::new(rect.right() - 21.0 - STAGE.x, rect.bottom() - 14.0 - STAGE.y), STAGE);

    // the pet is the handle
    let resp = ui.interact(stage, ui.scope_id().with("pet-drag"), Sense::click_and_drag());
    if resp.hovered() {
        ctx.set_cursor_icon(CursorIcon::Grab);
    }
    if resp.drag_started() {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }
    f.pet.draw_fit(ui.painter(), stage.shrink2(Vec2::new(5.0, 10.0)), f.mood);

    // the glance, while the pointer is over the window
    let over = glance_override.unwrap_or_else(|| ctx.input(|i| i.pointer.hover_pos().is_some()));
    let shown = ctx.animate_bool_with_time(ui.scope_id().with("glance"), over, 0.16);
    if shown > 0.0 && glance(ui, f.logos, stage.center().x, rect.top() + 10.0, &compact::rows(f), shown) {
        actions.push(Action::Resize(Size::Compact));
    }
    if ctx.input(|i| i.key_pressed(Key::Enter)) {
        actions.push(Action::Resize(Size::Compact));
    }
    actions
}
