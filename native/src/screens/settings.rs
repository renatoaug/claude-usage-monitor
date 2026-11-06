// Settings: the title band stays; below it, three tabs (Accounts, Alerts,
// Appearance), Cancel / Save, and the version. Edits go to a draft and reach
// config.json only on Save.

use eframe::egui::{Align, Context, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};

use super::expanded::{account_menu_overlay, title_band};
use super::{Action, Frame, MARGIN};
use crate::components::login::{LoginPick, login_panel};
use crate::components::logos::Provider;
use crate::poller::{Login, Snapshot};
use crate::prefs::Prefs;
use crate::ui::controls::click_area;
use crate::ui::group::{GAP, block_h, group, group_label, row_label};
use crate::ui::icon;
use crate::ui::segmented::segmented;
use crate::ui::stepper::stepper;
use crate::ui::surface::card;
use crate::ui::tag::{TagStyle, tag};
use crate::ui::text::{bold_mid, text_mid};
use crate::ui::theme::{color, radius, text};
use crate::ui::toggle::toggle;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Accounts,
    Alerts,
    Appearance,
}

const TABS: [(Tab, &str); 3] = [(Tab::Accounts, "Accounts"), (Tab::Alerts, "Alerts"), (Tab::Appearance, "Appearance")];

const CARD_W: f32 = 280.0;
const SIDE: f32 = 16.0;
const BODY_TOP: f32 = 38.0;
const TABS_H: f32 = 4.0 + 30.0 + 12.0;
const TILES_H: f32 = 62.0;
const CONN_H: f32 = 64.0;
const LOGIN_H: f32 = 112.0;
const ACTIONS_H: f32 = 14.0 + 30.0;
const VERSION_H: f32 = 12.0 + 14.0;
const PAD_BOTTOM: f32 = 14.0;

fn logging_in(snap: &Snapshot) -> bool {
    matches!(snap.login, Login::AwaitingCode | Login::Failed(_))
}

// the service whose card Accounts shows: the one whose tile was picked
fn account_service(view: &super::View) -> Provider {
    view.provider
}

fn pane_h(tab: Tab, snap: &Snapshot, view: &super::View) -> f32 {
    let login = account_service(view) == Provider::Claude && logging_in(snap);
    match tab {
        Tab::Accounts => TILES_H + 10.0 + CONN_H + if login { LOGIN_H } else { 0.0 },
        Tab::Alerts => block_h(3) + GAP + block_h(1),
        Tab::Appearance => block_h(1) + GAP + block_h(2) + GAP + block_h(2),
    }
}

/// Window size for Settings on `tab`.
pub fn size(snap: &Snapshot, tab: Tab, view: &super::View) -> [f32; 2] {
    let h = BODY_TOP + 12.0 + TABS_H + pane_h(tab, snap, view) + ACTIONS_H + VERSION_H + PAD_BOTTOM;
    [CARD_W + 2.0 * MARGIN, h + 2.0 * MARGIN]
}

/// `draft` is the edited copy; `saved` what config.json holds.
pub fn draw(ui: &mut Ui, ctx: &Context, f: &Frame, tab: Tab, draft: &Prefs, saved: &Prefs, code: &mut String) -> Vec<Action> {
    let mut actions = vec![];
    let rect = f.area;
    card(ui, ctx, f.card);
    let chip = title_band(ui, ctx, f, rect, &mut actions);

    // the settings sheet: its own shade, from the title band to the bottom
    let p = ui.painter().clone();
    let sheet = Rect::from_min_max(Pos2::new(rect.left() + 1.0, rect.top() + BODY_TOP), rect.right_bottom() - Vec2::new(1.0, 1.0));
    let r = radius::CARD - 1;
    p.rect_filled(sheet, CornerRadius { nw: 0, ne: 0, sw: r, se: r }, color::SETTINGS);
    let (left, right) = (rect.left() + SIDE, rect.right() - SIDE);
    let mut y = sheet.top() + 12.0;

    let tabs = Rect::from_min_size(Pos2::new(left, y + 4.0), Vec2::new(right - left, 30.0));
    let labels: Vec<&str> = TABS.iter().map(|t| t.1).collect();
    let selected = TABS.iter().position(|t| t.0 == tab).unwrap_or(0);
    if let Some(i) = segmented(ui, tabs, "settings-tabs", &labels, selected, text::BODY, &[]) {
        actions.push(Action::SettingsTab(TABS[i].0));
    }
    y += TABS_H;

    match tab {
        Tab::Accounts => accounts(ui, f, left, right, y, code, saved, &mut actions),
        Tab::Alerts => alerts(ui, left, right, y, draft, &mut actions),
        Tab::Appearance => appearance(ui, f, left, right, y, draft, &mut actions),
    }
    y += pane_h(tab, f.snap, f.view);

    // Cancel, and Save once there's something to save
    let dirty = draft != saved;
    let row = Rect::from_min_size(Pos2::new(left, y + 14.0), Vec2::new(right - left, 30.0));
    let (cancel, save) = if dirty {
        let w = (row.width() - 8.0) / 2.0;
        (Rect::from_min_size(row.min, Vec2::new(w, 30.0)), Some(Rect::from_min_size(row.min + Vec2::new(w + 8.0, 0.0), Vec2::new(w, 30.0))))
    } else {
        (row, None)
    };
    if action_button(ui, cancel, "cancel", if dirty { "Cancel" } else { "Done" }, false) {
        actions.push(Action::ToggleSettings);
    }
    if let Some(s) = save {
        save_glow(ui, ctx, s);
        if action_button(ui, s, "save", "Save changes", true) {
            actions.push(Action::SaveSettings);
        }
    }
    y += ACTIONS_H;

    // the version, and the update check (not wired yet)
    let vy = y + 12.0 + 6.0;
    text_mid(&p, left, vy, Align::Min, &format!("clauddy v{}", env!("CARGO_PKG_VERSION")), text::CAPTION, color::FG_MUTED);
    text_mid(&p, right, vy, Align::Max, "Check for updates", text::CAPTION, color::ADD.gamma_multiply(0.4));

    account_menu_overlay(ui, ctx, f, rect, chip, &mut actions);
    actions
}

// unsaved changes: a soft glow breathes round Save, every 1.6 s, so they
// aren't forgotten (layers of fading rounded boxes stand in for a blur)
fn save_glow(ui: &Ui, ctx: &Context, rect: Rect) {
    let t = ctx.input(|i| i.time) as f32;
    let breath = 0.5 - 0.5 * (t * std::f32::consts::TAU / 1.6).cos();
    let p = ui.painter();
    for k in 1..=6 {
        let spread = k as f32 * 1.5;
        let a = breath * (1.0 - k as f32 / 7.0) * 0.35;
        p.rect_filled(rect.expand(spread), CornerRadius::same(radius::CONTROL + 2 + k as u8), color::PRIMARY_GLOW.gamma_multiply(a));
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(50));
}

fn action_button(ui: &Ui, rect: Rect, id: impl std::hash::Hash + std::fmt::Debug, label: &str, primary: bool) -> bool {
    let resp = click_area(ui, rect, id);
    let (bg, fg) = match (primary, resp.hovered()) {
        (true, true) => (color::PRIMARY_HOVER, color::ON_PRIMARY),
        (true, false) => (color::PRIMARY, color::ON_PRIMARY),
        (false, true) => (color::CONTROL_HOVER, color::FG),
        (false, false) => (color::CONTROL, color::FG),
    };
    let p = ui.painter();
    p.rect(rect, CornerRadius::same(radius::CONTROL + 2), bg, Stroke::new(1.0, color::CONTROL_EDGE), StrokeKind::Inside);
    text_mid(p, rect.center().x, rect.center().y, Align::Center, label, text::BODY, fg);
    resp.clicked()
}

fn accounts(ui: &mut Ui, f: &Frame, left: f32, right: f32, y: f32, code: &mut String, saved: &Prefs, actions: &mut Vec<Action>) {
    let p = ui.painter().clone();
    // one tile per service; a click opens its card below
    let picked = account_service(f.view);
    let w = (right - left - 12.0) / 3.0;
    for (i, provider) in [Provider::Claude, Provider::Codex, Provider::Cursor].into_iter().enumerate() {
        let tile = Rect::from_min_size(Pos2::new(left + i as f32 * (w + 6.0), y), Vec2::new(w, TILES_H));
        let ready = true;
        let on = provider == picked;
        let hovered = ready && {
            let r = click_area(ui, tile, ("service-tile", provider.name()));
            if r.clicked() && !on {
                actions.push(Action::SelectProvider(provider));
            }
            r.hovered()
        };
        let (bg, edge) = match (on, hovered) {
            (true, _) => (color::TILE_ON, color::TILE_EDGE_ON),
            (false, true) => (color::TILE, color::CONTROL_HOVER.gamma_multiply(0.6)),
            (false, false) => (color::TILE, color::HAIR),
        };
        p.rect(tile, CornerRadius::same(radius::CONTROL + 4), bg, Stroke::new(1.0, edge), StrokeKind::Inside);
        let linked = match provider {
            Provider::Claude => f.snap.connected,
            Provider::Codex => saved.codex,
            Provider::Cursor => saved.cursor,
        };
        let fade = if !ready { 0.4 } else if linked || on { 1.0 } else { 0.7 };
        let logo = Rect::from_center_size(Pos2::new(tile.center().x, tile.top() + 19.0), Vec2::splat(18.0));
        f.logos.draw(&p, provider, logo, if ready && (linked || on) { provider.tint() } else { color::FG_MUTED.gamma_multiply(fade) });
        text_mid(&p, tile.center().x, tile.top() + 38.0, Align::Center, provider.name(), text::SMALL, color::FG.gamma_multiply(fade));
        let state = match (ready, linked) {
            (false, _) => "Soon",
            (true, true) => "Connected",
            (true, false) => "Connect",
        };
        text_mid(&p, tile.center().x, tile.top() + 51.0, Align::Center, state, text::CAPTION, color::FG_MUTED.gamma_multiply(fade));
    }

    if picked != Provider::Claude {
        ext_card(ui, f, picked, left, right, y + TILES_H + 10.0, saved, actions);
        return;
    }

    // the Claude card: its avatar, who's connected and on what plan, and
    // the way out (or in)
    let card = Rect::from_min_size(Pos2::new(left, y + TILES_H + 10.0), Vec2::new(right - left, CONN_H));
    p.rect(card, CornerRadius::same(radius::CONTROL + 2), color::GROUP, Stroke::new(1.0, color::HAIR), StrokeKind::Inside);
    let avatar = Rect::from_min_size(Pos2::new(card.left() + 10.0, card.center().y - 14.0), Vec2::splat(28.0));
    p.rect_filled(avatar, CornerRadius::same(radius::CONTROL + 1), color::AVATAR_CLAUDE);
    f.logos.draw(&p, Provider::Claude, Rect::from_center_size(avatar.center(), Vec2::splat(18.0)), color::CLAUDE);
    let x = avatar.right() + 10.0;
    let (head_y, caption_y) = (card.center().y - 8.0, card.center().y + 9.0);
    let name = bold_mid(&p, x, head_y, Align::Min, "Claude", text::BODY, color::FG);
    if let Some(plan) = f.snap.profile.as_ref().and_then(|pr| pr.plan.as_deref()).filter(|_| f.snap.connected) {
        let style = TagStyle { fg: color::PLAN, edge: color::PLAN_EDGE, wash: color::PLAN_WASH, pad: Vec2::new(4.0, 3.0) };
        tag(&p, name.right() + 6.0, head_y, Align::Min, &plan.to_lowercase(), &style);
    }
    if !f.snap.loaded && !f.snap.connected {
        crate::ui::skeleton::sk_line(&p, x, caption_y, 120.0, 8.0);
    } else if f.snap.connected {
        // who is connected, not just that someone is
        let who = f.snap.profile.as_ref().and_then(|pr| pr.email.as_deref()).unwrap_or("Connected");
        let clip = p.with_clip_rect(Rect::from_min_max(Pos2::new(x, card.top()), Pos2::new(card.right() - 44.0, card.bottom())));
        text_mid(&clip, x, caption_y, Align::Min, who, text::SMALL, color::CAPTION);
        let out = Rect::from_center_size(Pos2::new(card.right() - 22.0, card.center().y), Vec2::splat(24.0));
        let resp = click_area(ui, out, "logout");
        if resp.hovered() {
            p.rect_filled(out, CornerRadius::same(radius::CONTROL), color::MENU_HOVER);
        }
        icon::draw(&p, out.center(), icon::LOG_OUT, 13.0, if resp.hovered() { color::REMOVE_HOVER } else { color::REMOVE });
        if resp.clicked() {
            actions.push(Action::Logout);
        }
    } else {
        text_mid(&p, x, caption_y, Align::Min, "Connect via browser to track your usage.", text::SMALL, color::CAPTION);
        if !logging_in(f.snap) {
            let btn = Rect::from_min_size(Pos2::new(card.right() - 76.0, card.center().y - 12.0), Vec2::new(64.0, 24.0));
            if action_button(ui, btn, "settings-connect", "Connect", true) {
                actions.push(Action::BeginLogin);
            }
        }
    }
    if logging_in(f.snap) {
        let error = match &f.snap.login {
            Login::Failed(e) => Some(e.as_str()),
            _ => None,
        };
        match login_panel(ui, left, right, card.bottom() - 2.0, code, error) {
            Some(LoginPick::Submit) => actions.push(Action::SubmitCode),
            Some(LoginPick::Reopen) => actions.push(Action::BeginLogin),
            None => {}
        }
    }
}

// Codex and Cursor: read from what their own apps keep on this Mac, so
// there's no login here, only on or off; it takes effect at once. Codex
// needs a session on disk, Cursor its app signed in.
#[allow(clippy::too_many_arguments)]
fn ext_card(ui: &Ui, f: &Frame, provider: Provider, left: f32, right: f32, y: f32, saved: &Prefs, actions: &mut Vec<Action>) {
    let p = ui.painter().clone();
    let card = Rect::from_min_size(Pos2::new(left, y), Vec2::new(right - left, CONN_H));
    p.rect(card, CornerRadius::same(radius::CONTROL + 2), color::GROUP, Stroke::new(1.0, color::HAIR), StrokeKind::Inside);
    let avatar = Rect::from_min_size(Pos2::new(card.left() + 10.0, card.center().y - 14.0), Vec2::splat(28.0));
    let cursor = provider == Provider::Cursor;
    p.rect_filled(avatar, CornerRadius::same(radius::CONTROL + 1), if cursor { color::AVATAR_CURSOR } else { color::AVATAR_CODEX });
    f.logos.draw(&p, provider, Rect::from_center_size(avatar.center(), Vec2::splat(18.0)), provider.tint());
    let x = avatar.right() + 10.0;
    let (head_y, caption_y) = (card.center().y - 8.0, card.center().y + 9.0);
    let name = bold_mid(&p, x, head_y, Align::Min, provider.name(), text::BODY, color::FG);
    let (on, found, plan) = if cursor {
        let (found, plan) = f.snap.cursor_found.clone().unwrap_or((true, None));
        (saved.cursor, found, f.snap.cursor.as_ref().and_then(|c| c.plan.clone()).or(plan))
    } else {
        let (found, plan) = f.snap.codex_found.clone().unwrap_or((true, None));
        (saved.codex, found, f.snap.codex.as_ref().and_then(|c| c.plan.clone()).or(plan))
    };
    let set = |v: bool| if cursor { Action::SetCursor(v) } else { Action::SetCodex(v) };
    if let Some(plan) = plan.filter(|_| on) {
        let style = TagStyle { fg: color::PLAN, edge: color::PLAN_EDGE, wash: color::PLAN_WASH, pad: Vec2::new(4.0, 3.0) };
        tag(&p, name.right() + 6.0, head_y, Align::Min, &plan.to_lowercase(), &style);
    }
    if on {
        text_mid(&p, x, caption_y, Align::Min, "Connected on this device", text::SMALL, color::CAPTION);
        let out = Rect::from_center_size(Pos2::new(card.right() - 22.0, card.center().y), Vec2::splat(24.0));
        let resp = click_area(ui, out, (provider.name(), "off"));
        if resp.hovered() {
            p.rect_filled(out, CornerRadius::same(radius::CONTROL), color::MENU_HOVER);
        }
        icon::draw(&p, out.center(), icon::LOG_OUT, 13.0, if resp.hovered() { color::REMOVE_HOVER } else { color::REMOVE });
        if resp.clicked() {
            actions.push(set(false));
        }
    } else {
        let hint = match (found, cursor) {
            (true, _) => "Track it beside Claude.",
            (false, false) => "No Codex sessions on this Mac yet.",
            (false, true) => "Sign in to the Cursor app first.",
        };
        let clip = p.with_clip_rect(Rect::from_min_max(Pos2::new(x, card.top()), Pos2::new(card.right() - if found { 84.0 } else { 12.0 }, card.bottom())));
        text_mid(&clip, x, caption_y, Align::Min, hint, text::SMALL, color::CAPTION);
        if found {
            let btn = Rect::from_min_size(Pos2::new(card.right() - 76.0, card.center().y - 12.0), Vec2::new(64.0, 24.0));
            if action_button(ui, btn, (provider.name(), "on"), "Connect", true) {
                actions.push(set(true));
            }
        }
    }
}

fn alerts(ui: &Ui, left: f32, right: f32, y: f32, draft: &Prefs, actions: &mut Vec<Action>) {
    let p = ui.painter().clone();
    let mut edit = |next: Prefs| actions.push(Action::EditPrefs(next.clamped()));

    let gy = group_label(&p, left, y, "notifications");
    let rows = group(&p, left, right, gy, 3);
    row_label(&p, left, rows[0], "Gentle notifications", false);
    if toggle(ui, right - 10.0, rows[0], "alerts", draft.alerts, true).clicked() {
        edit(Prefs { alerts: !draft.alerts, ..draft.clone() });
    }
    row_label(&p, left, rows[1], "Heads-up at", true);
    if let Some(d) = stepper(ui, right - 10.0, rows[1], "heads-up", draft.heads_up, "%", draft.alerts) {
        edit(Prefs { heads_up: draft.heads_up + d, ..draft.clone() });
    }
    row_label(&p, left, rows[2], "Near limit at", true);
    if let Some(d) = stepper(ui, right - 10.0, rows[2], "near-limit", draft.near_limit, "%", draft.alerts) {
        edit(Prefs { near_limit: draft.near_limit + d, ..draft.clone() });
    }

    let gy = group_label(&p, left, gy + 3.0 * crate::ui::group::ROW + GAP, "mood");
    let rows = group(&p, left, right, gy, 1);
    row_label(&p, left, rows[0], "Catch fire at", false);
    if let Some(d) = stepper(ui, right - 10.0, rows[0], "fire", draft.fire, "%", true) {
        edit(Prefs { fire: draft.fire + d, ..draft.clone() });
    }
}

// Appearance: what the native app can't change yet is shown, faded.
fn appearance(ui: &Ui, f: &Frame, left: f32, right: f32, y: f32, draft: &Prefs, actions: &mut Vec<Action>) {
    let p = ui.painter().clone();
    let row = crate::ui::group::ROW;

    let gy = group_label(&p, left, y, "pet");
    let rows = group(&p, left, right, gy, 1);
    let thumb = Rect::from_center_size(Pos2::new(left + 22.0, rows[0]), Vec2::splat(22.0));
    let _ = f.pet.draw_fit(&p, thumb, crate::components::pet::Mood::Idle);
    text_mid(&p, left + 40.0, rows[0], Align::Min, "Clauddy", text::BODY, color::ROW_LABEL);
    text_mid(&p, right - 10.0, rows[0], Align::Max, "Change", text::BODY, color::ADD.gamma_multiply(0.4));

    let gy = group_label(&p, left, gy + row + GAP, "display");
    let rows = group(&p, left, right, gy, 2);
    row_label(&p, left, rows[0], "Lives in", false);
    let seg = Rect::from_min_size(Pos2::new(right - 10.0 - 140.0, rows[0] - 12.0), Vec2::new(140.0, 24.0));
    if let Some(i) = segmented(ui, seg, "lives-in", &["Floating", "Menu bar"], draft.menu_bar as usize, text::SMALL, &[]) {
        actions.push(Action::EditPrefs(Prefs { menu_bar: i == 1, ..draft.clone() }));
    }
    row_label(&p, left, rows[1], "Zoom", false);
    // applied as you step it, so the widget shows the size at once
    if let Some(d) = stepper(ui, right - 10.0, rows[1], "zoom", draft.zoom, "%", true) {
        actions.push(Action::EditPrefs(Prefs { zoom: draft.zoom + 25 * d, ..draft.clone() }.clamped()));
    }

    let gy = group_label(&p, left, gy + 2.0 * row + GAP, "voice");
    let rows = group(&p, left, right, gy, 2);
    row_label(&p, left, rows[0], "Speech bubbles", false);
    toggle(ui, right - 10.0, rows[0], "talk", false, false);
    row_label(&p, left, rows[1], "Chiptune voice", true);
    toggle(ui, right - 10.0, rows[1], "sound", false, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_tab_sizes_the_window_to_its_own_content() {
        let snap = Snapshot::default();
        let view = crate::screens::View::default();
        let h = |t| size(&snap, t, &view)[1];
        assert!(h(Tab::Appearance) > h(Tab::Alerts));
        let logging = Snapshot { login: Login::AwaitingCode, ..Default::default() };
        assert!(size(&logging, Tab::Accounts, &view)[1] > h(Tab::Accounts));
    }
}
