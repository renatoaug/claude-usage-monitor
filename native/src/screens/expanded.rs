// Expanded: the account and controls, the pet's stage, the status line, the
// meters, and Details (tokens by model and by project, the 30-day map).

use eframe::egui::{self, Align2, Context, CornerRadius, FontId, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};

use super::{Action, Frame, MARGIN, Size, View, connected, shown};
use crate::components::account::account_chip;
use crate::components::account_menu::{MenuPick, account_label, account_menu, menu_rect};
use crate::components::breakdown::{self, Hit, Section, section};
use crate::components::heat_map::{HEAT_H, heat_map};
use crate::components::login::{LOGIN_H, LoginPick, login_panel};
use crate::components::logos::Provider;
use crate::components::meter::{self, meter, meter_skeleton, meters};
use crate::components::meter::{Meter, codex_meters, cursor_meters};
use crate::components::more_menu::{MorePick, menu_rect as more_menu_rect, more_button, more_menu};
use crate::components::night_sky::night_sky;
use crate::components::pet::Mood;
use crate::components::reminder;
use crate::components::status::{STATUS_H, status_line};
use crate::format::tokens;
use crate::poller::Login;
use crate::poller::Snapshot;
use crate::ui::controls::BUTTON;
use crate::ui::controls::{button_row, click_area};
use crate::ui::icon;
use crate::ui::segmented::segmented;
use crate::ui::surface::card_tinted;
use crate::ui::surface::{hairline, pane};
use crate::ui::text::text_mid;
use crate::ui::theme::{color, radius, text};
use crate::usage::{Labeled, Period, Span};

const CARD_W: f32 = 280.0;
const SIDE: f32 = 18.0;
const STAGE_TOP: f32 = 38.0;
const STAGE_H: f32 = 132.0;
const CONNECT_H: f32 = 64.0;
const TOGGLE_H: f32 = 32.0;
const MODEL_ROWS: usize = 4;
const PRODUCT_ROWS: usize = 4;
const PROJECT_ROWS: usize = 6;
const CONTROLS_H: f32 = 34.0;
const BACK_H: f32 = 30.0;
const NOTE_H: f32 = 20.0;
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
    /// the lists over each span; none for Codex, which has the week's only
    periods: &'a [Period],
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
            periods: &[],
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
            periods: &[],
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
                periods: &u.periods,
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

fn height(meter_count: usize, view: &View, details: Option<f32>, logging_in: bool) -> f32 {
    let mut h = STAGE_TOP + STAGE_H + STATUS_H;
    h += if logging_in {
        LOGIN_H
    } else if meter_count > 0 {
        meters_h(meter_count) + 12.0
    } else {
        CONNECT_H
    };
    let Some(details) = details else { return h + PAD_BOTTOM - 12.0 };
    h += TOGGLE_H;
    if view.details {
        h += details;
    }
    h + PAD_BOTTOM
}

/// One piece of Details, top to bottom. The panel's height and its drawing
/// both come from this list, so they can't disagree.
enum Block {
    /// what the lists cover: today, 7 days or 30 days
    Controls,
    Rows {
        id: &'static str,
        title: String,
        rows: Vec<Labeled>,
        open: bool,
        value: fn(u64) -> String,
        /// what its head does; None for a section that doesn't fold
        fold: Option<Action>,
        /// a row opens its project
        pick: bool,
    },
    /// the project the lists are opened on, and the way back
    Back(String),
    Note(String),
    Heat,
}

impl Block {
    fn height(&self) -> f32 {
        match self {
            Block::Controls => CONTROLS_H,
            Block::Rows { rows, open, .. } => breakdown::height(rows.len(), *open),
            Block::Back(_) => BACK_H,
            Block::Note(_) => NOTE_H,
            Block::Heat => HEAT_H,
        }
    }
}

/// The first `most` rows.
fn rows(list: &[Labeled], most: usize) -> Vec<Labeled> {
    list.iter().take(most).cloned().collect()
}

/// Details for the service shown: Claude's lists over the chosen span (or
/// one project's), Codex's week.
fn blocks(snap: &Snapshot, view: &View, s: &Service) -> Vec<Block> {
    let value = breakdown::tokens;
    let list = |id, title: String, rows, open, fold| Block::Rows { id, title, rows, open, value, fold, pick: false };
    let Some(period) = s.periods.get(view.span.index()) else {
        return vec![
            list("models", "by model · 7 days".into(), rows(s.by_model, MODEL_ROWS), view.models, Some(Action::ToggleModels)),
            list("projects", "by project · 7 days".into(), rows(s.by_project, PROJECT_ROWS), view.projects, Some(Action::ToggleProjects)),
            Block::Heat,
        ];
    };
    let span = view.span.label();
    let mut out = vec![Block::Controls];
    if let Some(p) = view.project.as_ref().and_then(|name| period.projects.iter().find(|p| &p.label == name)) {
        out.push(Block::Back(format!("{} · {} tokens", p.label, tokens(p.tokens))));
        out.push(list("project-models", format!("by model · {span}"), rows(&p.by_model, MODEL_ROWS), true, None));
        out.push(list("project-days", "by day · 7 days".into(), rows(&p.days, p.days.len()), true, None));
        if !p.worktrees.is_empty() {
            out.push(list("project-worktrees", format!("by worktree · {span}"), rows(&p.worktrees, PROJECT_ROWS), true, None));
        }
        if p.input > 0 {
            out.push(Block::Note(format!("{:.0}% of what it sent was read from the cache", p.cache_read as f64 / p.input as f64 * 100.0)));
        }
    } else {
        let products = products(snap, s.provider);
        if !products.is_empty() {
            out.push(Block::Rows {
                id: "products",
                title: "by product · this week".into(),
                rows: products,
                open: view.products,
                value: breakdown::percent,
                fold: Some(Action::ToggleProducts),
                pick: false,
            });
        }
        out.push(list("models", format!("by model · {span}"), rows(&period.by_model, MODEL_ROWS), view.models, Some(Action::ToggleModels)));
        out.push(Block::Rows {
            id: "projects",
            title: format!("by project · {span}"),
            rows: rows(&period.by_project, PROJECT_ROWS),
            open: view.projects,
            value,
            fold: Some(Action::ToggleProjects),
            pick: true,
        });
        let sessions = &period.sessions;
        out.push(list("sessions", format!("top sessions · {span}"), rows(sessions, sessions.len()), view.sessions, Some(Action::ToggleSessions)));
    }
    out.push(Block::Heat);
    out
}

fn logging_in(snap: &Snapshot) -> bool {
    matches!(snap.login, Login::AwaitingCode | Login::Failed(_))
}

// the room the reset reminder takes under the session meter
fn reminder_h(snap: &Snapshot, provider: Provider, session: Option<(f64, Option<i64>)>, heads_up: f64) -> f32 {
    match reminder::state(snap, provider, session, heads_up) {
        None => 0.0,
        Some(_) => reminder::HEIGHT + if reminder::error(snap).is_some() { reminder::ERROR_H } else { 0.0 },
    }
}

/// Window size for this content. Before the first reading, the skeleton's
/// shape: two meters and Details as it was left.
pub fn size(snap: &Snapshot, view: &View, session: Option<(f64, Option<i64>)>, heads_up: f64) -> [f32; 2] {
    let empty = crate::usage::Usage::empty();
    let s = service(snap, shown_in(snap, view), &empty);
    let login = s.provider == Provider::Claude && logging_in(snap);
    let extra = if snap.loaded && !login && !s.meters.is_empty() { reminder_h(snap, s.provider, session, heads_up) } else { 0.0 };
    let m = if snap.loaded { s.meters.len() } else { 2 };
    let tabs = if connected(snap).len() > 1 { TABS_H } else { 0.0 };
    let details = s.details.then(|| blocks(snap, view, &s).iter().map(Block::height).sum());
    [CARD_W + 2.0 * MARGIN, height(m, view, details, login) + tabs + extra + 2.0 * MARGIN]
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
    // a remark floats over the scene, beside the pet's head
    if let Some((said, shown)) = f.bubble {
        let tip = Pos2::new(stage.left() + 100.0, pet_area.top() + 20.0);
        if crate::components::bubble::bubble(ui, said, tip, 14.0, 170.0, stage.shrink(4.0), shown) {
            actions.push(Action::DismissBubble);
        }
    }

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
        // the two lines centred on their ink (capitals to baseline), not their line boxes
        p.text(r.center() - Vec2::new(0.0, 6.5), Align2::CENTER_CENTER, "Connect your Claude account", FontId::proportional(text::BODY), color::FG);
        p.text(
            r.center() + Vec2::new(0.0, 7.5),
            Align2::CENTER_CENTER,
            "to see the real session and week %",
            FontId::proportional(text::SMALL),
            color::FG_MUTED,
        );
        if click_area(ui, r, "connect").clicked() {
            actions.push(Action::BeginLogin);
        }
        y += CONNECT_H;
    } else {
        for (i, m) in list.iter().enumerate() {
            meter(p, left, right, y, m, i == 0, svc.accent);
            y += meter::height(i == 0);
            // under the session: a reminder for when it resets
            if i == 0
                && let Some(on) = reminder::state(f.snap, svc.provider, f.session, f.heads_up)
            {
                if reminder::button(ui, left, y + 2.0, on) {
                    actions.push(Action::Remind(svc.provider, !on));
                }
                y += reminder::HEIGHT;
                if let Some(e) = reminder::error(f.snap) {
                    text_mid(p, left, y + 4.0, egui::Align::Min, e, text::TINY, color::FG_MUTED);
                    y += reminder::ERROR_H;
                }
            }
        }
        y += 12.0;
    }

    // Details
    if !svc.details {
        band_overlays(ui, ctx, f, rect, chip, &mut actions);
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

    band_overlays(ui, ctx, f, rect, chip, &mut actions);
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
    // in the menu bar the popover is the only size: minimize tucks it back
    // into the bar (quitting is in the bar's menu), and there's no pet-only.
    // The rest (the pet, Settings, the usage page, updates) is in ⋯
    let mut buttons = vec![if f.menu_bar { ("minimize", icon::MINIMIZE, Action::HidePopover) } else { ("close", icon::CLOSE, Action::Close) }];
    if !f.menu_bar {
        buttons.push(("minimize", icon::MINIMIZE, Action::Resize(Size::Compact)));
        buttons.push(("pet-only", icon::PET, Action::Resize(Size::PetOnly)));
    }
    let glyphs: Vec<_> = buttons.iter().map(|b| (b.0, b.1)).collect();
    if let Some(i) = button_row(ui, rect.right() - 8.0, rect.top() + 19.0, &glyphs) {
        actions.push(buttons.swap_remove(i).2);
    }
    if more_button(ui, more_slot(f, rect).center(), update_out(f)).clicked() {
        actions.push(Action::ToggleMoreMenu);
    }
    chip
}

/// The ⋯ button's box in the expanded title band: left of the others.
fn more_slot(f: &Frame, rect: Rect) -> Rect {
    let i = if f.menu_bar { 1.0 } else { 3.0 };
    let x = rect.right() - 8.0 - BUTTON / 2.0 - i * (BUTTON + 4.0);
    Rect::from_center_size(Pos2::new(x, rect.top() + 19.0), Vec2::splat(BUTTON))
}

/// A newer release is out (or going in): ⋯ calls for attention, unless
/// its menu is open and already shows it.
pub fn update_out(f: &Frame) -> bool {
    matches!(f.update, crate::updates::Update::Available(_)) && !f.view.more_menu
}

/// The title band's menus, over everything: the account's and ⋯.
pub fn band_overlays(ui: &Ui, ctx: &Context, f: &Frame, rect: Rect, chip: Rect, actions: &mut Vec<Action>) {
    account_menu_overlay(ui, ctx, f, rect, chip, actions);
    let button = more_slot(f, rect);
    more_menu_overlay(ui, ctx, f, button, Pos2::new(button.right(), button.bottom() + 4.0), actions);
}

/// The ⋯ menu under its button. A click anywhere else closes it.
pub fn more_menu_overlay(ui: &Ui, ctx: &Context, f: &Frame, button: Rect, top_right: Pos2, actions: &mut Vec<Action>) {
    let open = f.view.more_menu;
    match more_menu(ui, ctx, top_right, open, &f.update) {
        // checking answers in the menu itself: it stays open
        Some(MorePick::CheckUpdates) => actions.push(Action::CheckUpdates),
        Some(pick) => {
            actions.push(Action::CloseMoreMenu);
            actions.push(match pick {
                MorePick::Settings => Action::ToggleSettings,
                MorePick::Pet => Action::QuickPet,
                MorePick::UsagePage => Action::OpenUsagePage(super::shown(f)),
                MorePick::CheckUpdates => Action::CheckUpdates,
                MorePick::InstallUpdate => Action::InstallUpdate,
            });
        }
        None if open => {
            let area = more_menu_rect(top_right, &f.update);
            let outside = ctx.input(|i| {
                i.key_pressed(egui::Key::Escape)
                    || (i.pointer.any_click() && i.pointer.interact_pos().is_some_and(|pos| !area.contains(pos) && !button.contains(pos)))
            });
            if outside {
                actions.push(Action::CloseMoreMenu);
            }
        }
        None => {}
    }
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
/// Claude's week by product (Claude Code, Cowork, Chats), from the account:
/// none for the other services, or before a reading.
fn products(snap: &Snapshot, provider: Provider) -> Vec<crate::usage::Labeled> {
    let rows = snap.limits.as_ref().filter(|_| provider == Provider::Claude).map(|l| l.by_product.as_slice()).unwrap_or_default();
    rows.iter().take(PRODUCT_ROWS).map(|r| crate::usage::Labeled { label: r.label.clone(), tokens: r.pct.round() as u64 }).collect()
}

fn details(ui: &Ui, p: &egui::Painter, left: f32, right: f32, mut y: f32, s: &Service, f: &Frame, actions: &mut Vec<Action>) {
    for block in blocks(f.snap, f.view, s) {
        let h = block.height();
        match block {
            Block::Controls => {
                let spans = Rect::from_min_size(Pos2::new(left, y + 5.0), Vec2::new(152.0, 24.0));
                let labels = Span::ALL.map(Span::label);
                if let Some(i) = segmented(ui, spans, "span", &labels, f.view.span.index(), text::SMALL, &[]) {
                    actions.push(Action::SetSpan(Span::ALL[i]));
                }
            }
            Block::Rows { id, title, rows, open, value, fold, pick } => {
                let sec = Section { id, title: &title, rows: &rows, open, loading: !f.snap.loaded, bar: s.bar, value, foldable: fold.is_some(), pick };
                match section(ui, left, right, y, &sec) {
                    Some(Hit::Head) => actions.extend(fold),
                    Some(Hit::Row(i)) => actions.push(Action::OpenProject(Some(rows[i].label.clone()))),
                    None => {}
                }
            }
            Block::Back(name) => {
                hairline(p, left, right, y);
                let row = Rect::from_min_size(Pos2::new(left, y), Vec2::new(right - left, BACK_H));
                let resp = click_area(ui, row, "project-back");
                let mid = y + BACK_H / 2.0 + 1.0;
                let back = text_mid(p, right, mid, egui::Align::Max, "all projects", text::SMALL, if resp.hovered() { color::FG } else { color::ADD });
                let clip = p.with_clip_rect(Rect::from_min_max(row.min, Pos2::new(back.left() - 8.0, row.bottom())));
                crate::ui::text::bold_mid(&clip, left, mid, egui::Align::Min, &name, text::BODY, color::FG);
                if resp.clicked() {
                    actions.push(Action::OpenProject(None));
                }
            }
            Block::Note(note) => {
                text_mid(p, left, y + NOTE_H / 2.0 - 2.0, egui::Align::Min, &note, text::CAPTION, color::FG_MUTED);
            }
            Block::Heat => {
                heat_map(p, left, right, y, s.days30, s.month, s.heat);
            }
        }
        y += h;
    }
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
        let snap = Snapshot::default();
        let h = |view: View| size(&snap, &view, None, 80.0)[1];
        let closed = h(View { details: false, ..View::default() });
        let open = h(View { details: true, ..View::default() });
        let folded = h(View { details: true, models: false, projects: false, sessions: false, ..View::default() });
        assert!(open > closed);
        assert!(folded < open && folded > closed);
    }
}
