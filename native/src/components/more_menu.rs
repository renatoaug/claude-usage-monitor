// The title band's ⋯: its button, whose dots ripple in coral while a newer
// release is out, and the menu that drops from it: Change pet, Settings, the
// usage page, and at the foot the version with the update.

use eframe::egui::{Align, Context, CornerRadius, FontId, Painter, Pos2, Rect, Response, Stroke, StrokeKind, Ui, Vec2};

use crate::ui::controls::{BUTTON, click_area};
use crate::ui::icon::{self, Glyph};
use crate::ui::surface::hairline;
use crate::ui::text::text_mid;
use crate::ui::theme::{color, radius, text};
use crate::updates::Update;

pub const W: f32 = 184.0;
const ROW: f32 = 24.0;
const PAD: f32 = 5.0;
const ROWS: usize = 3;
const FOOT: f32 = 28.0;
const FOOT_UPDATE: f32 = 50.0;
/// how long the menu takes to drop in, and to go
const OPEN_S: f32 = 0.18;

#[derive(Clone, Copy)]
pub enum MorePick {
    Settings,
    Pet,
    UsagePage,
    CheckUpdates,
    InstallUpdate,
}

/// The ⋯ button centred on `center`. With an update out, its dots turn coral
/// and, now and then, ripple once, quietly.
pub fn more_button(ui: &Ui, center: Pos2, update: bool) -> Response {
    let rect = Rect::from_center_size(center, Vec2::splat(BUTTON));
    let resp = click_area(ui, rect, "more");
    let p = ui.painter();
    let t = ui.ctx().input(|i| i.time) as f32;
    if update {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }
    let (bg, fg) = match (update, resp.hovered()) {
        (true, _) => (if resp.hovered() { color::CONTROL_HOVER } else { color::CONTROL }, color::ACCENT),
        (false, true) => (color::CONTROL_HOVER, color::ICON_HOVER),
        (false, false) => (color::CONTROL, color::ICON),
    };
    p.rect(rect, CornerRadius::same(radius::CONTROL), bg, Stroke::new(1.0, color::CONTROL_EDGE), StrokeKind::Inside);
    for i in 0..3 {
        // one ripple across the dots every 2.4 s, then a rest
        let k = ((t % 2.4) - i as f32 * 0.12) / 0.45;
        let lift = if update && (0.0..1.0).contains(&k) { (k * std::f32::consts::PI).sin() * 1.1 } else { 0.0 };
        p.circle_filled(center + Vec2::new((i as f32 - 1.0) * 3.4, -lift), 1.05, fg);
    }
    resp
}

/// The menu's box when fully open, its top-right corner at `top_right`.
pub fn menu_rect(top_right: Pos2, update: &Update) -> Rect {
    let foot = if matches!(update, Update::Available(_) | Update::Updating) { FOOT_UPDATE } else { FOOT };
    Rect::from_min_size(Pos2::new(top_right.x - W, top_right.y), Vec2::new(W, PAD * 2.0 + ROWS as f32 * ROW + 1.0 + foot))
}

/// The menu, dropping in while `open` and fading out after; returns what
/// was picked.
pub fn more_menu(ui: &Ui, ctx: &Context, top_right: Pos2, open: bool, update: &Update) -> Option<MorePick> {
    let raw = ctx.animate_bool_with_time(ui.scope_id().with("more-menu-open"), open, OPEN_S);
    if raw <= 0.0 {
        return None;
    }
    let e = 1.0 - (1.0 - raw).powi(3);
    let full = menu_rect(top_right, update);
    // drops a few points from under the button as it fades in
    let rect = full.translate(Vec2::new(0.0, -6.0 * (1.0 - e)));
    let mut p = ui.painter().clone();
    p.set_opacity(e);

    for k in 1..=6 {
        let a = (1.0 - k as f32 / 7.0) * 0.22;
        p.rect_filled(
            rect.translate(Vec2::new(0.0, 3.0)).expand(k as f32 * 1.3),
            CornerRadius::same(radius::CONTROL + 2 + k as u8),
            color::MENU_SHADOW.gamma_multiply(a),
        );
    }
    p.rect(rect, CornerRadius::same(radius::CONTROL + 2), color::MENU, Stroke::new(1.0, color::MENU_EDGE), StrokeKind::Inside);
    let live = open && raw > 0.6;
    if live {
        // the menu swallows clicks that miss its rows
        let _ = click_area(ui, rect, "more-menu");
    }

    let mut pick = None;
    let rows: [(&str, Glyph, MorePick); ROWS] =
        [("Change pet", icon::PAW, MorePick::Pet), ("Settings", icon::GEAR, MorePick::Settings), ("Usage page", icon::EXTERNAL, MorePick::UsagePage)];
    let (left, right) = (rect.left() + PAD, rect.right() - PAD);
    let mut y = rect.top() + PAD;
    for (label, glyph, what) in rows {
        let row = Rect::from_min_size(Pos2::new(left, y), Vec2::new(right - left, ROW));
        let hovered = live && {
            let r = click_area(ui, row, ("more-row", label));
            if r.clicked() {
                pick = Some(what);
            }
            r.hovered()
        };
        if hovered {
            p.rect_filled(row, CornerRadius::same(radius::CONTROL), color::MENU_HOVER);
        }
        let cy = row.center().y;
        // a touch up: the text's capitals, not its descenders, set where its middle reads
        icon::draw(&p, Pos2::new(row.left() + 11.0, cy - 1.0), glyph, 10.5, if hovered { color::ACCENT } else { color::ICON });
        text_mid(&p, row.left() + 24.0, cy, Align::Min, label, text::SMALL, if hovered { color::FG } else { color::FG_ROW });
        y += ROW;
    }
    hairline(&p, left + 4.0, right - 4.0, y + 0.5);
    y += 1.0;

    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let foot = Rect::from_min_max(Pos2::new(left, y), Pos2::new(right, rect.bottom() - PAD));
    if let Some(p2) = footer(ui, &p, foot, &version, update, live) {
        pick = Some(p2);
    }
    if open && raw < 1.0 || !open {
        ctx.request_repaint();
    }
    pick
}

/// The version, and the update: a quiet link to check, or, with a release
/// out, a coral card with the jump and an Update button.
fn footer(ui: &Ui, p: &Painter, foot: Rect, version: &str, update: &Update, live: bool) -> Option<MorePick> {
    let mut pick = None;
    match update {
        Update::Available(_) | Update::Updating => {
            let card = foot.shrink2(Vec2::new(0.0, 4.0)).translate(Vec2::new(0.0, 1.0));
            p.rect(
                card,
                CornerRadius::same(radius::CONTROL),
                color::ACCENT.gamma_multiply(0.14),
                Stroke::new(1.0, color::ACCENT.gamma_multiply(0.35)),
                StrokeKind::Inside,
            );
            let (title, sub) = match update {
                Update::Available(tag) => (format!("{tag} is out"), format!("you have {version}")),
                _ => ("Updating…".to_string(), version.to_string()),
            };
            text_mid(p, card.left() + 9.0, card.center().y - 6.5, Align::Min, &title, text::SMALL, color::FG);
            text_mid(p, card.left() + 9.0, card.center().y + 7.0, Align::Min, &sub, text::CAPTION, color::FG_MUTED);
            if matches!(update, Update::Available(_)) {
                let btn = Rect::from_min_size(Pos2::new(card.right() - 7.0 - 52.0, card.center().y - 10.0), Vec2::new(52.0, 20.0));
                let hovered = live && {
                    let r = click_area(ui, btn, "more-update");
                    if r.clicked() {
                        pick = Some(MorePick::InstallUpdate);
                    }
                    r.hovered()
                };
                p.rect_filled(btn, CornerRadius::same(radius::CONTROL), if hovered { color::PRIMARY_HOVER } else { color::PRIMARY });
                // centred on its capitals, not on the line box
                text_mid(p, btn.center().x, btn.center().y + 1.0, Align::Center, "Update", text::SMALL, color::ON_PRIMARY);
            }
        }
        _ => {
            let cy = foot.center().y + 1.0;
            text_mid(p, foot.left() + 6.0, cy, Align::Min, &format!("Clauddy {version}"), text::CAPTION, color::FG_MUTED);
            let (label, act) = match update {
                Update::Checking => ("Checking…", None),
                Update::UpToDate => ("Up to date · check", Some(MorePick::CheckUpdates)),
                Update::Failed(_) => ("Couldn't check · retry", Some(MorePick::CheckUpdates)),
                _ => ("Check for updates", Some(MorePick::CheckUpdates)),
            };
            let w = p.layout_no_wrap(label.to_string(), FontId::proportional(text::CAPTION), color::ADD).size().x;
            let hit = Rect::from_min_max(Pos2::new(foot.right() - 6.0 - w - 4.0, cy - 9.0), Pos2::new(foot.right() - 2.0, cy + 9.0));
            let hovered = live && act.is_some() && {
                let r = click_area(ui, hit, "more-check");
                if r.clicked() {
                    pick = act;
                }
                r.hovered()
            };
            let fg = if act.is_none() {
                color::FG_MUTED
            } else if hovered {
                color::FG
            } else {
                color::ADD
            };
            text_mid(p, foot.right() - 6.0, cy, Align::Max, label, text::CAPTION, fg);
        }
    }
    pick
}
