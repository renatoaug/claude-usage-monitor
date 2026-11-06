// Expanded: the account and controls, the pet's stage, the status line, the
// meters, and Details (tokens by model and by project, the 30-day map).

use eframe::egui::{Align2, Context, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use super::{Action, Frame, MARGIN, Size, View};
use crate::poller::Snapshot;
use crate::components::account::account;
use crate::components::breakdown::{self, section};
use crate::components::heat_map::{HEAT_H, heat_map};
use crate::components::meter::{self, meter, meter_skeleton, meters};
use crate::components::night_sky::night_sky;
use crate::components::pet::Mood;
use crate::components::status::{STATUS_H, status_line};
use crate::ui::controls::button_row;
use crate::ui::icon::{self, CHEVRON_DOWN, CHEVRON_RIGHT};
use crate::ui::surface::{card, hairline, pane};
use crate::ui::theme::{color, radius, text};

const CARD_W: f32 = 280.0;
const SIDE: f32 = 18.0;
const STAGE_TOP: f32 = 38.0;
const STAGE_H: f32 = 132.0;
const CONNECT_H: f32 = 56.0;
const TOGGLE_H: f32 = 32.0;
const MODEL_ROWS: usize = 4;
const PROJECT_ROWS: usize = 6;
const PAD_BOTTOM: f32 = 14.0;

fn meters_h(count: usize) -> f32 {
    (0..count).map(|i| meter::height(i == 0)).sum()
}

fn height(meter_count: usize, view: &View, models: usize, projects: usize) -> f32 {
    let mut h = STAGE_TOP + STAGE_H + STATUS_H;
    h += if meter_count > 0 { meters_h(meter_count) + 12.0 } else { CONNECT_H };
    h += TOGGLE_H;
    if view.details {
        h += breakdown::height(models.min(MODEL_ROWS), view.models) + breakdown::height(projects.min(PROJECT_ROWS), view.projects) + HEAT_H;
    }
    h + PAD_BOTTOM
}

/// Window size for this content. Before the first reading, the skeleton's
/// shape: two meters and Details as it was left.
pub fn size(snap: &Snapshot, view: &View) -> [f32; 2] {
    let usage = snap.usage.as_ref();
    let m = if snap.loaded { meters(snap).len() } else { 2 };
    let models = usage.map(|u| u.by_model.len()).unwrap_or(0);
    let projects = usage.map(|u| u.by_project.len()).unwrap_or(0);
    [CARD_W + 2.0 * MARGIN, height(m, view, models, projects) + 2.0 * MARGIN]
}

pub fn draw(ui: &Ui, ctx: &Context, f: &Frame) -> Vec<Action> {
    let mut actions = vec![];
    let usage = f.snap.usage.as_ref();
    let rect = ctx.content_rect().shrink(MARGIN);
    card(ui, ctx, rect);
    let (left, right) = (rect.left() + SIDE, rect.right() - SIDE);
    let p = ui.painter();

    // title band: the account chip, then settings, the usage page, pet-only, minimize, close
    account(p, rect.left() + 10.0, rect.top() + 16.5, f.snap.profile.as_ref(), true);
    let buttons = [
        ("close", icon::CLOSE),
        ("minimize", icon::MINIMIZE),
        ("pet-only", icon::PET),
        ("usage", icon::EXTERNAL),
        ("settings", icon::GEAR),
    ];
    match button_row(ui, rect.right() - 8.0, rect.top() + 19.0, &buttons) {
        Some(0) => actions.push(Action::Close),
        Some(1) => actions.push(Action::Resize(Size::Compact)),
        Some(3) => actions.push(Action::OpenUsagePage),
        _ => {} // pet-only and settings: next
    }

    // the stage: a dark pane with the pet near its left edge
    let stage = Rect::from_min_size(Pos2::new(left, rect.top() + STAGE_TOP), Vec2::new(right - left, STAGE_H));
    pane(p, stage);
    let inside = p.with_clip_rect(stage);
    night_sky(&inside, stage);
    let pet_area = if f.mood == Mood::Working {
        Rect::from_min_max(Pos2::new(stage.left() + 14.0, stage.top() + 8.0), Pos2::new(stage.left() + 144.0, stage.bottom() - 2.0))
    } else {
        let cell = 8.5;
        Rect::from_min_max(
            Pos2::new(stage.left() + 26.0 - cell, stage.bottom() - 3.0 - 9.0 * cell),
            Pos2::new(stage.left() + 26.0 + 11.0 * cell, stage.bottom() - 3.0 + cell),
        )
    };
    f.pet.draw(&inside, pet_area, f.mood);

    // status line
    let activity = usage.and_then(|u| u.activity.as_deref());
    let today = if f.snap.loaded { usage.map(|u| u.today_tokens) } else { None };
    status_line(p, left, right, stage.bottom() + 17.0, f.mood, activity, today, f.snap.limits.is_some());
    let mut y = stage.bottom() + STATUS_H;
    hairline(p, left, right, y);

    // the meters, or the way in when no account is connected
    let list = meters(f.snap);
    if !f.snap.loaded {
        for i in 0..2 {
            meter_skeleton(p, left, right, y, i == 0);
            y += meter::height(i == 0);
        }
        y += 12.0;
    } else if list.is_empty() {
        let r = Rect::from_min_size(Pos2::new(left, y + 12.0), Vec2::new(right - left, CONNECT_H - 24.0));
        p.rect(r, CornerRadius::same(radius::CONTROL), color::WASH, Stroke::new(1.0, color::WASH_EDGE), StrokeKind::Inside);
        p.text(r.center() - Vec2::new(0.0, 6.0), Align2::CENTER_CENTER, "Connect your Claude account", FontId::proportional(text::BODY), color::FG);
        p.text(r.center() + Vec2::new(0.0, 8.0), Align2::CENTER_CENTER, "to see the real session and week %", FontId::proportional(text::SMALL), color::FG_MUTED);
        y += CONNECT_H;
    } else {
        for (i, m) in list.iter().enumerate() {
            meter(p, left, right, y, m, i == 0);
            y += meter::height(i == 0);
        }
        y += 12.0;
    }

    // Details
    hairline(p, left, right, y);
    let toggle = Rect::from_min_size(Pos2::new(left, y), Vec2::new(right - left, TOGGLE_H));
    let resp = ui.interact(toggle, ui.scope_id().with("details"), Sense::click());
    let c = if resp.hovered() { color::FG } else { color::FG_TOGGLE };
    p.text(Pos2::new(left, y + 20.0), Align2::LEFT_CENTER, "Details", FontId::proportional(text::BODY), c);
    icon::draw(p, Pos2::new(right - 5.0, y + 20.0), if f.view.details { CHEVRON_DOWN } else { CHEVRON_RIGHT }, 9.0, c);
    if resp.clicked() {
        actions.push(Action::ToggleDetails);
    }
    y += TOGGLE_H;
    if !f.view.details {
        return actions;
    }
    let Some(u) = usage else { return actions };
    let models = &u.by_model[..u.by_model.len().min(MODEL_ROWS)];
    let projects = &u.by_project[..u.by_project.len().min(PROJECT_ROWS)];
    if section(ui, left, right, y, "models", "by model · 7 days", models, f.view.models) {
        actions.push(Action::ToggleModels);
    }
    y += breakdown::height(models.len(), f.view.models);
    if section(ui, left, right, y, "projects", "by project · 7 days", projects, f.view.projects) {
        actions.push(Action::ToggleProjects);
    }
    y += breakdown::height(projects.len(), f.view.projects);
    heat_map(p, left, right, y, &u.days30, u.month_tokens);
    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn details_grow_the_panel_and_folding_shrinks_it() {
        let closed = View { details: false, ..View::default() };
        let open = View { details: true, ..View::default() };
        let folded = View { details: true, models: false, projects: false, ..View::default() };
        assert!(height(2, &open, 3, 5) > height(2, &closed, 3, 5));
        assert!(height(2, &folded, 3, 5) < height(2, &open, 3, 5));
    }
}
