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

/// What the screen hands back: the actions, and the areas that take the
/// pointer (the pet, and the glance while it's up), in window coordinates.
/// Everywhere else clicks fall through to what's behind the window.
pub struct Out {
    pub actions: Vec<Action>,
    pub hot: Vec<Rect>,
}

/// `over_pet`: whether the cursor is over the pet (or the glance); None
/// reads it from the pointer this window sees.
pub fn draw(ui: &Ui, ctx: &Context, f: &Frame, over_pet: Option<bool>) -> Out {
    let mut actions = vec![];
    let rect = f.area;
    let stage = Rect::from_min_size(Pos2::new(rect.right() - 21.0 - STAGE.x, rect.bottom() - 14.0 - STAGE.y), STAGE);

    // the pet is the handle
    let resp = ui.interact(stage, ui.scope_id().with("pet-drag"), Sense::click_and_drag());
    if resp.hovered() {
        ctx.set_cursor_icon(CursorIcon::Grab);
    }
    if resp.drag_started() {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }
    let body = f.pet.draw_fit(ui.painter(), stage.shrink2(Vec2::new(5.0, 10.0)), f.mood);

    // the glance, only while the cursor is over the pet (or the glance)
    let mut hot = vec![body];
    // where the glance was last frame, while it's up: the pet and the bubble
    // (and the gap between) all count as over, or the bubble would close on
    // the way up to its button
    let last_id = ui.scope_id().with("glance-rect");
    let last: Option<Rect> = ctx.data(|d| d.get_temp(last_id));
    let area = last.map_or(body, |b| body.union(b));
    let over = over_pet.unwrap_or_else(|| ctx.input(|i| i.pointer.hover_pos().is_some_and(|p| area.contains(p))));
    let shown = ctx.animate_bool_with_time(ui.scope_id().with("glance"), over, 0.16);
    // just above the head, but never off the top of the window
    let tip = (body.top() - 4.0).max(rect.top() + 4.0 + crate::components::glance::HEIGHT);
    if shown > 0.0 {
        let (clicked, bubble) = glance(ui, f.logos, body.center().x, tip, &compact::rows(f), shown);
        if clicked {
            actions.push(Action::Resize(Size::Compact));
        }
        // the gap between the bubble and the head counts too, so moving up
        // to the button doesn't close the bubble on the way
        hot = vec![body.union(bubble)];
        ctx.data_mut(|d| d.insert_temp(last_id, bubble));
    } else {
        ctx.data_mut(|d| d.remove::<Rect>(last_id));
    }
    if ctx.input(|i| i.key_pressed(Key::Enter)) {
        actions.push(Action::Resize(Size::Compact));
    }
    Out { actions, hot }
}
