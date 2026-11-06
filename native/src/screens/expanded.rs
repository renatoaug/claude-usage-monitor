// Expanded: the account and controls, the pet's stage, the status line, the
// meters, and Details (tokens by model and by project, the 30-day map).

use eframe::egui::{self, Align2, Context, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use super::{Action, Frame, MARGIN, Size, View};
use crate::poller::Snapshot;
use crate::components::account::account_chip;
use crate::components::account_menu::{MenuPick, account_label, account_menu, menu_rect};
use crate::components::login::{LOGIN_H, LoginPick, login_panel};
use crate::poller::Login;
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

fn height(meter_count: usize, view: &View, models: usize, projects: usize, logging_in: bool) -> f32 {
    let mut h = STAGE_TOP + STAGE_H + STATUS_H;
    h += if logging_in {
        LOGIN_H
    } else if meter_count > 0 {
        meters_h(meter_count) + 12.0
    } else {
        CONNECT_H
    };
    h += TOGGLE_H;
    if view.details {
        h += breakdown::height(models.min(MODEL_ROWS), view.models) + breakdown::height(projects.min(PROJECT_ROWS), view.projects) + HEAT_H;
    }
    h + PAD_BOTTOM
}

fn logging_in(snap: &Snapshot) -> bool {
    matches!(snap.login, Login::AwaitingCode | Login::Failed(_))
}

/// Window size for this content. Before the first reading, the skeleton's
/// shape: two meters and Details as it was left.
pub fn size(snap: &Snapshot, view: &View) -> [f32; 2] {
    let usage = snap.usage.as_ref();
    let m = if snap.loaded { meters(snap).len() } else { 2 };
    let models = usage.map(|u| u.by_model.len()).unwrap_or(0);
    let projects = usage.map(|u| u.by_project.len()).unwrap_or(0);
    [CARD_W + 2.0 * MARGIN, height(m, view, models, projects, logging_in(snap)) + 2.0 * MARGIN]
}

pub fn draw(ui: &mut Ui, ctx: &Context, f: &Frame, code: &mut String) -> Vec<Action> {
    let mut actions = vec![];
    let usage = f.snap.usage.as_ref();
    let rect = ctx.content_rect().shrink(MARGIN);
    card(ui, ctx, rect);
    let (left, right) = (rect.left() + SIDE, rect.right() - SIDE);
    let p = ui.painter().clone();
    let p = &p;

    // title band: the account chip, then settings, the usage page, pet-only, minimize, close
    let active = f.snap.accounts.iter().enumerate().find(|(_, a)| a.id == f.snap.active);
    let label = active
        .map(|(i, a)| account_label(a, i))
        .filter(|_| active.is_some_and(|(_, a)| a.label.is_some()))
        .or_else(|| f.snap.profile.as_ref().and_then(|pr| pr.email.clone()));
    let plan = f.snap.profile.as_ref().and_then(|pr| pr.plan.clone());
    let chip = account_chip(p, rect.left() + 10.0, rect.top() + 16.5, label.as_deref(), plan.as_deref(), false);
    let chip_resp = ui.interact(chip, ui.scope_id().with("account-chip"), Sense::click());
    if chip_resp.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if chip_resp.clicked() {
        actions.push(Action::ToggleAccountMenu);
    }
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
    if logging_in(f.snap) {
        let error = match &f.snap.login {
            Login::Failed(e) => Some(e.as_str()),
            _ => None,
        };
        match login_panel(ui, left, right, y, code, error) {
            Some(LoginPick::Submit) => actions.push(Action::SubmitCode),
            Some(LoginPick::Reopen) => actions.push(Action::BeginLogin),
            None => {}
        }
        y += LOGIN_H;
    } else if !f.snap.loaded {
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
        let connect = ui.interact(r, ui.scope_id().with("connect"), Sense::click());
        if connect.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if connect.clicked() {
            actions.push(Action::BeginLogin);
        }
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
    if f.view.details {
        if let Some(u) = usage {
            details(ui, p, left, right, y, u, f, &mut actions);
        }
    }

    // the account menu, over everything; a click anywhere else closes it
    if f.view.account_menu {
        let at = Pos2::new(rect.left() + 8.0, rect.top() + 33.0);
        let menu = menu_rect(at, f.snap.accounts.len());
        match account_menu(ui, at, &f.snap.accounts, &f.snap.active, f.view.pending_remove.as_deref()) {
            Some(MenuPick::Switch(id)) => actions.push(Action::SwitchAccount(id)),
            Some(MenuPick::AskRemove(id)) => actions.push(Action::AskRemoveAccount(id)),
            Some(MenuPick::Remove(id)) => actions.push(Action::RemoveAccount(id)),
            Some(MenuPick::Add) => actions.push(Action::AddAccount),
            None => {
                let outside = ctx.input(|i| {
                    i.key_pressed(egui::Key::Escape)
                        || (i.pointer.any_click() && i.pointer.interact_pos().is_some_and(|pos| !menu.contains(pos) && !chip.contains(pos)))
                });
                if outside {
                    actions.push(Action::CloseAccountMenu);
                }
            }
        }
    }
    actions
}

// by model, by project, the 30-day map
fn details(ui: &Ui, p: &egui::Painter, left: f32, right: f32, mut y: f32, u: &crate::usage::Usage, f: &Frame, actions: &mut Vec<Action>) {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn details_grow_the_panel_and_folding_shrinks_it() {
        let closed = View { details: false, ..View::default() };
        let open = View { details: true, ..View::default() };
        let folded = View { details: true, models: false, projects: false, ..View::default() };
        assert!(height(2, &open, 3, 5, false) > height(2, &closed, 3, 5, false));
        assert!(height(2, &folded, 3, 5, false) < height(2, &open, 3, 5, false));
    }
}
