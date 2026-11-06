// Expanded: the account and controls, the pet's stage, the status line, the
// meters, and Details (tokens by model and by project, the 30-day map).

use eframe::egui::{self, Align2, Context, CornerRadius, FontId, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};

use super::{Action, Frame, MARGIN, Size, View, connected, shown};
use crate::components::logos::Provider;
use crate::components::meter::{Meter, codex_meters, cursor_meters};
use crate::ui::surface::card_tinted;
use crate::ui::text::text_mid;
use crate::usage::Labeled;
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
use crate::ui::controls::{button_row, click_area};
use crate::ui::icon;
use crate::ui::surface::{hairline, pane};
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

const TABS_H: f32 = 10.0 + 26.0;

/// What the panel shows for one service: its meters and colour, today's
/// tokens, the lists and the map, and whether it's working.
struct Service<'a> {
    provider: Provider,
    meters: Vec<Meter>,
    accent: egui::Color32,
    bar: egui::Color32,
    heat: &'static [egui::Color32; 5],
    today: Option<u64>,
    live: bool,
    working: bool,
    /// what it's doing, when its logs say
    activity: Option<&'a str>,
    /// tokens by model and project, and the map: Cursor has none
    details: bool,
    by_model: &'a [Labeled],
    by_project: &'a [Labeled],
    days30: &'a [u64],
    month: u64,
}

fn service<'a>(snap: &'a Snapshot, provider: Provider, empty: &'a crate::usage::Usage) -> Service<'a> {
    match provider {
        Provider::Cursor if let Some(c) = &snap.cursor => Service {
            provider,
            meters: cursor_meters(c),
            accent: color::CURSOR_BAR,
            bar: color::CURSOR_BAR,
            heat: &color::HEAT,
            today: None,
            live: false,
            working: c.active,
            activity: c.activity,
            details: false,
            by_model: &[],
            by_project: &[],
            days30: &[],
            month: 0,
        },
        Provider::Codex if let Some(c) = &snap.codex => Service {
            provider,
            meters: codex_meters(c),
            accent: color::CODEX_BAR,
            bar: color::CODEX_BAR,
            heat: &color::HEAT_CODEX,
            today: Some(c.tokens_today),
            live: false, // logs, not a live feed
            working: c.active,
            activity: None,
            details: true,
            by_model: &c.by_model,
            by_project: &c.by_project,
            days30: &c.days30,
            month: c.month_tokens,
        },
        _ => {
            let u = snap.usage.as_ref().unwrap_or(empty);
            Service {
                provider: Provider::Claude,
                meters: meters(snap),
                accent: color::ACCENT,
                bar: color::BREAKDOWN,
                heat: &color::HEAT,
                today: snap.loaded.then_some(u.today_tokens),
                live: snap.limits.is_some(),
                working: u.active,
                activity: u.activity.as_deref(),
                details: true,
                by_model: &u.by_model,
                by_project: &u.by_project,
                days30: &u.days30,
                month: u.month_tokens,
            }
        }
    }
}

/// The service shown, from the view's choice and what's connected.
fn shown_in(snap: &Snapshot, view: &View) -> Provider {
    if connected(snap).contains(&view.provider) { view.provider } else { Provider::Claude }
}

fn meters_h(count: usize) -> f32 {
    (0..count).map(|i| meter::height(i == 0)).sum()
}

fn height(meter_count: usize, view: &View, details: Option<(usize, usize)>, logging_in: bool) -> f32 {
    let mut h = STAGE_TOP + STAGE_H + STATUS_H;
    h += if logging_in {
        LOGIN_H
    } else if meter_count > 0 {
        meters_h(meter_count) + 12.0
    } else {
        CONNECT_H
    };
    let Some((models, projects)) = details else { return h + PAD_BOTTOM - 12.0 };
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
    let empty = crate::usage::Usage::empty();
    let s = service(snap, shown_in(snap, view), &empty);
    let m = if snap.loaded { s.meters.len() } else { 2 };
    let tabs = if connected(snap).len() > 1 { TABS_H } else { 0.0 };
    let login = s.provider == Provider::Claude && logging_in(snap);
    [CARD_W + 2.0 * MARGIN, height(m, view, s.details.then_some((s.by_model.len(), s.by_project.len())), login) + tabs + 2.0 * MARGIN]
}

pub fn draw(ui: &mut Ui, ctx: &Context, f: &Frame, code: &mut String) -> Vec<Action> {
    let mut actions = vec![];
    let empty = crate::usage::Usage::empty();
    let svc = service(f.snap, shown(f), &empty);
    let rect = f.area;
    card_tinted(ui, ctx, f.card, svc.provider.surface());
    let (left, right) = (rect.left() + SIDE, rect.right() - SIDE);
    let p = ui.painter().clone();
    let p = &p;

    let chip = title_band(ui, ctx, f, rect, &mut actions);

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

    // status line: the shown service's state and tokens
    let activity = svc.activity;
    let mood = match (svc.working, f.mood) {
        (true, _) => Mood::Working,
        (false, Mood::Working) => Mood::Idle, // the other service is the one working
        (false, m) => m,
    };
    status_line(p, left, right, stage.bottom() + 17.0, mood, activity, svc.today, svc.live);
    let mut y = stage.bottom() + STATUS_H;
    hairline(p, left, right, y);

    // with several services, tabs pick the one the panel shows
    let services = connected(f.snap);
    if services.len() > 1 {
        if let Some(pick) = service_tabs(ui, f, left, right, y + 10.0, &services, svc.provider) {
            actions.push(Action::SelectProvider(pick));
        }
        y += TABS_H;
    }

    // the meters, or the way in when no account is connected
    let list = &svc.meters;
    if svc.provider == Provider::Claude && logging_in(f.snap) {
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
        if click_area(ui, r, "connect").clicked() {
            actions.push(Action::BeginLogin);
        }
        y += CONNECT_H;
    } else {
        for (i, m) in list.iter().enumerate() {
            meter(p, left, right, y, m, i == 0, svc.accent);
            y += meter::height(i == 0);
        }
        y += 12.0;
    }

    // Details
    if !svc.details {
        account_menu_overlay(ui, ctx, f, rect, chip, &mut actions);
        return actions;
    }
    hairline(p, left, right, y);
    let toggle = Rect::from_min_size(Pos2::new(left, y), Vec2::new(right - left, TOGGLE_H));
    let resp = click_area(ui, toggle, "details");
    let c = if resp.hovered() { color::FG } else { color::FG_TOGGLE };
    p.text(Pos2::new(left, y + 20.0), Align2::LEFT_CENTER, "Details", FontId::proportional(text::BODY), c);
    let open_t = ctx.animate_bool_with_time(ui.scope_id().with("details-open"), f.view.details, 0.16);
    icon::disclosure(p, Pos2::new(right - 5.0, y + 20.0), open_t, c);
    if resp.clicked() {
        actions.push(Action::ToggleDetails);
    }
    y += TOGGLE_H;
    if f.view.details {
        details(ui, p, left, right, y, &svc, f, &mut actions);
    }

    account_menu_overlay(ui, ctx, f, rect, chip, &mut actions);
    actions
}

/// The title band shared by the expanded panel and Settings: the account
/// chip, then settings, the usage page, pet-only, minimize and close.
/// Returns the chip's box (the account menu hangs from it).
pub fn title_band(ui: &Ui, ctx: &Context, f: &Frame, rect: Rect, actions: &mut Vec<Action>) -> Rect {
    let p = ui.painter();
    let active = f.snap.accounts.iter().enumerate().find(|(_, a)| a.id == f.snap.active);
    let label = active
        .map(|(i, a)| account_label(a, i))
        .filter(|_| active.is_some_and(|(_, a)| a.label.is_some()))
        .or_else(|| f.snap.profile.as_ref().and_then(|pr| pr.email.clone()));
    let plan = f.snap.profile.as_ref().and_then(|pr| pr.plan.clone());
    let menu_t = ctx.animate_bool_with_time(ui.scope_id().with("menu-open"), f.view.account_menu, 0.16);
    let chip = account_chip(p, rect.left() + 10.0, rect.top() + 19.0, label.as_deref(), plan.as_deref(), false, menu_t);
    if click_area(ui, chip, "account-chip").clicked() {
        actions.push(Action::ToggleAccountMenu);
    }
    // in the menu bar the popover is the only size: no minimize, no pet-only
    let mut buttons = vec![("close", icon::CLOSE, Action::Close)];
    if !f.menu_bar {
        buttons.push(("minimize", icon::MINIMIZE, Action::Resize(Size::Compact)));
        buttons.push(("pet-only", icon::PET, Action::Resize(Size::PetOnly)));
    }
    buttons.push(("usage", icon::EXTERNAL, Action::OpenUsagePage));
    buttons.push(("settings", icon::GEAR, Action::ToggleSettings));
    let glyphs: Vec<_> = buttons.iter().map(|b| (b.0, b.1)).collect();
    if let Some(i) = button_row(ui, rect.right() - 8.0, rect.top() + 19.0, &glyphs) {
        actions.push(buttons.swap_remove(i).2);
    }
    chip
}

/// The account menu, over everything; a click anywhere else closes it.
pub fn account_menu_overlay(ui: &Ui, ctx: &Context, f: &Frame, rect: Rect, chip: Rect, actions: &mut Vec<Action>) {
    if !f.view.account_menu {
        return;
    }
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

// by model, by project, the 30-day map
fn details(ui: &Ui, p: &egui::Painter, left: f32, right: f32, mut y: f32, s: &Service, f: &Frame, actions: &mut Vec<Action>) {
    let models = &s.by_model[..s.by_model.len().min(MODEL_ROWS)];
    let projects = &s.by_project[..s.by_project.len().min(PROJECT_ROWS)];
    if section(ui, left, right, y, "models", "by model · 7 days", models, f.view.models, !f.snap.loaded, s.bar) {
        actions.push(Action::ToggleModels);
    }
    y += breakdown::height(models.len(), f.view.models);
    if section(ui, left, right, y, "projects", "by project · 7 days", projects, f.view.projects, !f.snap.loaded, s.bar) {
        actions.push(Action::ToggleProjects);
    }
    y += breakdown::height(projects.len(), f.view.projects);
    heat_map(p, left, right, y, s.days30, s.month, s.heat);
}

// the service tabs: logo, name and session % each; the chosen one on a
// plate tinted with its colour
fn service_tabs(ui: &Ui, f: &Frame, left: f32, right: f32, y: f32, services: &[Provider], chosen: Provider) -> Option<Provider> {
    let p = ui.painter();
    let gap = 6.0;
    let w = (right - left - gap * (services.len() - 1) as f32) / services.len() as f32;
    let mut pick = None;
    for (i, &prov) in services.iter().enumerate() {
        let r = Rect::from_min_size(Pos2::new(left + i as f32 * (w + gap), y), Vec2::new(w, 26.0));
        let resp = click_area(ui, r, ("service-tab", prov.name()));
        let on = prov == chosen;
        let (bg, edge) = match (on, prov) {
            (true, Provider::Codex) => (color::TAB_CODEX, color::TAB_CODEX_EDGE),
            (true, Provider::Cursor) => (color::TAB_CURSOR, color::TAB_CURSOR_EDGE),
            (true, _) => (color::TAB_CLAUDE, color::TAB_CLAUDE_EDGE),
            (false, _) => (color::WASH, if resp.hovered() { color::CONTROL_HOVER.gamma_multiply(0.7) } else { color::TAB_EDGE }),
        };
        p.rect(r, CornerRadius::same(radius::CONTROL), bg, Stroke::new(1.0, edge), StrokeKind::Inside);
        let fg = if on || resp.hovered() { color::FG } else { color::FG_MUTED };
        let mid = r.center().y;
        let pct = super::sessions(f.snap, f.session).into_iter().find(|s| s.0 == prov).and_then(|s| s.1);
        let urgent = pct.is_some_and(|v| v >= 80.0);
        let value = pct.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "—".into());
        let value_fg = if urgent { color::URGENT } else { fg };
        let tint = prov.tint().gamma_multiply(if on { 1.0 } else { 0.65 });
        if services.len() > 2 {
            // three across: no room for names, so logo and % sit centred
            let gap = 6.0;
            let value_w = p.layout_no_wrap(value.clone(), FontId::proportional(text::SMALL), value_fg).size().x;
            let x = r.center().x - (12.0 + gap + value_w) / 2.0;
            f.logos.draw(p, prov, Rect::from_center_size(Pos2::new(x + 6.0, mid), Vec2::splat(12.0)), tint);
            text_mid(p, x + 12.0 + gap, mid, egui::Align::Min, &value, text::SMALL, value_fg);
        } else {
            f.logos.draw(p, prov, Rect::from_center_size(Pos2::new(r.left() + 14.0, mid), Vec2::splat(12.0)), tint);
            text_mid(p, r.left() + 25.0, mid, egui::Align::Min, prov.name(), text::BODY, fg);
            text_mid(p, r.right() - 8.0, mid, egui::Align::Max, &value, text::SMALL, value_fg);
        }
        if resp.clicked() && !on {
            pick = Some(prov);
        }
    }
    pick
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn details_grow_the_panel_and_folding_shrinks_it() {
        let closed = View { details: false, ..View::default() };
        let open = View { details: true, ..View::default() };
        let folded = View { details: true, models: false, projects: false, ..View::default() };
        assert!(height(2, &open, Some((3, 5)), false) > height(2, &closed, Some((3, 5)), false));
        let _ = TABS_H;
        assert!(height(2, &folded, Some((3, 5)), false) < height(2, &open, Some((3, 5)), false));
    }
}
