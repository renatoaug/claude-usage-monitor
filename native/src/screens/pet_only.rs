// Just the pet, floating, with the usage glance always above its head.
// Dragging the pet moves the window; hovering the pet or the glance slides
// out the glance's button, which (or Enter) opens the compact monitor. A
// click on the pet itself changes nothing.

use eframe::egui::{Context, CursorIcon, Key, Pos2, Rect, Sense, Ui, Vec2, ViewportCommand};

use super::{Action, Frame, MARGIN, Size, compact, weeks};
use crate::components::glance::glance;

// wide enough for the widest glance (three services at 100%, the week beside
// each, the expand button out), centred on the pet; the window is see-through
const CARD: Vec2 = Vec2::new(268.0, 166.0);
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
    let stage = Rect::from_min_size(Pos2::new((rect.center().x - STAGE.x / 2.0).round(), rect.bottom() - 14.0 - STAGE.y), STAGE);

    // the pet is the handle
    let resp = ui.interact(stage, ui.scope_id().with("pet-drag"), Sense::click_and_drag());
    if resp.hovered() {
        ctx.set_cursor_icon(CursorIcon::Grab);
    }
    if resp.drag_started() {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }
    let body = f.pet.draw_fit(ui.painter(), stage.shrink2(Vec2::new(5.0, 10.0)), f.mood);

    // the glance, always up; its button only while the cursor is over the
    // pet or the glance. Where the glance was last frame counts as over, the
    // gap above the head included, or the button would hide on the way up
    let last_id = ui.scope_id().with("glance-rect");
    let last: Option<Rect> = ctx.data(|d| d.get_temp(last_id));
    let area = last.map_or(body, |b| body.union(b));
    let over = over_pet.unwrap_or_else(|| ctx.input(|i| i.pointer.hover_pos().is_some_and(|p| area.contains(p))));
    let hover = ctx.animate_bool_with_time(ui.scope_id().with("glance-hover"), over, 0.16);
    // just above the head, but never off the top of the window
    let tip = (body.top() - 4.0).max(rect.top() + 4.0 + crate::components::glance::HEIGHT);
    let (clicked, bubble) = glance(ui, f.logos, body.center().x, tip, &compact::rows(f), &weeks(f.snap), 1.0, hover);
    if clicked {
        actions.push(Action::Resize(Size::Compact));
    }
    ctx.data_mut(|d| d.insert_temp(last_id, bubble));
    let hot = vec![body.union(bubble)];
    if ctx.input(|i| i.key_pressed(Key::Enter)) {
        actions.push(Action::Resize(Size::Compact));
    }
    Out { actions, hot }
}
