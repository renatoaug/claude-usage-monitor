// The title band's ⋯: its button, whose dots ripple in coral while a newer
// release is out, and the menu that drops from it: Change pet, Settings, the
// usage page, and at the foot the version with the update.

use eframe::egui::{Align, Context, CornerRadius, FontId, Painter, Pos2, Rect, Response, Stroke, StrokeKind, Ui, Vec2};

use crate::ui::controls::{BUTTON, click_area};
use crate::ui::icon::{self, Glyph};
use crate::ui::surface::hairline;
use crate::ui::tag::{TagStyle, tag};
use crate::ui::text::text_mid;
use crate::ui::theme::{color, radius, text};
use crate::updates::Update;

pub const W: f32 = 196.0;
const ROW: f32 = 28.0;
const PAD: f32 = 6.0;
const ROWS: usize = 3;
// the foot: the version and the update, on a panel of its own
const FOOT: f32 = 26.0;
const FOOT_UPDATE: f32 = 46.0;
// a row's icon sits on a small square
const BADGE: f32 = 20.0;
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
    Rect::from_min_size(Pos2::new(top_right.x - W, top_right.y), Vec2::new(W, PAD * 2.0 + ROWS as f32 * ROW + 5.0 + foot))
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
        // the icon on its square: coral on hover
        let badge = Rect::from_center_size(Pos2::new(row.left() + 6.0 + BADGE / 2.0, cy), Vec2::splat(BADGE));
        let (wash, ink) = if hovered { (color::ACCENT.gamma_multiply(0.18), color::ACCENT) } else { (color::WASH, color::ICON) };
        p.rect(badge, CornerRadius::same(radius::CONTROL), wash, Stroke::new(1.0, color::WASH_EDGE), StrokeKind::Inside);
        // a touch up: the text's capitals, not its descenders, set where its middle reads
        icon::draw(&p, badge.center() - Vec2::new(0.0, 0.5), glyph, 11.0, ink);
        text_mid(&p, badge.right() + 9.0, cy, Align::Min, label, text::SMALL, if hovered { color::FG } else { color::FG_ROW });
        y += ROW;
    }
    y += 2.0;
    hairline(&p, left + 4.0, right - 4.0, y + 0.5);
    y += 2.0;

    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    // the plain foot sits as far from the menu's bottom as from the line
    // above it; the update card keeps the padding all round
    let card = matches!(update, Update::Available(_) | Update::Updating);
    let foot = Rect::from_min_max(Pos2::new(left, y), Pos2::new(right, rect.bottom() - if card { PAD } else { 2.0 }));
    if let Some(p2) = footer(ui, &p, foot, &version, update, live) {
        pick = Some(p2);
    }
    if open && raw < 1.0 || !open {
        ctx.request_repaint();
    }
    pick
}

/// The version, and the update, at the foot: the version on a tag and a
/// link to check, which turns in place into the answer (a
/// spinner, "up to date", a retry) or, with a release out, a coral card
/// with the jump and an Update button. Checking never closes the menu.
fn footer(ui: &Ui, p: &Painter, foot: Rect, version: &str, update: &Update, live: bool) -> Option<MorePick> {
    let mut pick = None;
    let panel = foot.shrink2(Vec2::new(0.0, 2.0));
    let t = ui.ctx().input(|i| i.time) as f32;
    match update {
        Update::Available(_) | Update::Updating => {
            p.rect(
                panel,
                CornerRadius::same(radius::CONTROL),
                color::ACCENT.gamma_multiply(0.14),
                Stroke::new(1.0, color::ACCENT.gamma_multiply(0.35)),
                StrokeKind::Inside,
            );
            let (title, sub) = match update {
                Update::Available(tag) => (format!("{tag} is out"), format!("you have {version}")),
                _ => ("Updating…".to_string(), version.to_string()),
            };
            text_mid(p, panel.left() + 10.0, panel.center().y - 6.0, Align::Min, &title, text::SMALL, color::FG);
            text_mid(p, panel.left() + 10.0, panel.center().y + 6.5, Align::Min, &sub, text::CAPTION, color::FG_MUTED);
            let btn = Rect::from_min_size(Pos2::new(panel.right() - 8.0 - 58.0, panel.center().y - 11.0), Vec2::new(58.0, 22.0));
            if matches!(update, Update::Available(_)) {
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
            } else {
                spinner(ui, p, Pos2::new(btn.right() - 12.0, btn.center().y), color::ACCENT, t);
            }
        }
        _ => {
            let cy = panel.center().y;
            text_mid(p, panel.left() + 10.0, cy, Align::Min, "Clauddy", text::CAPTION, color::FG_ROW);
            let name_w = p.layout_no_wrap("Clauddy".to_string(), FontId::proportional(text::CAPTION), color::FG_ROW).size().x;
            let _ = tag(
                p,
                panel.left() + 10.0 + name_w + 6.0,
                cy,
                Align::Min,
                version,
                &TagStyle { fg: color::PLAN, edge: color::PLAN_EDGE, wash: color::PLAN_WASH, pad: Vec2::new(4.0, 2.0) },
            );
            // the check, and what it answered, on the right
            let right = panel.right() - 8.0;
            match update {
                Update::Checking => {
                    text_mid(p, right - 18.0, cy, Align::Max, "Checking…", text::CAPTION, color::FG_MUTED);
                    spinner(ui, p, Pos2::new(right - 6.0, cy), color::FG_MUTED, t);
                }
                Update::UpToDate => {
                    icon::draw(p, Pos2::new(right - 6.0, cy), icon::CHECK, 11.0, color::LIVE);
                    text_mid(p, right - 16.0, cy, Align::Max, "Up to date", text::CAPTION, color::LIVE);
                }
                _ => {
                    let (label, fg) = if matches!(update, Update::Failed(_)) { ("Retry", color::ERROR) } else { ("Check", color::ADD) };
                    let w = p.layout_no_wrap(label.to_string(), FontId::proportional(text::CAPTION), fg).size().x;
                    let btn = Rect::from_min_max(Pos2::new(right - w - 24.0, cy - 10.0), Pos2::new(right + 2.0, cy + 10.0));
                    let hovered = live && {
                        let r = click_area(ui, btn, "more-check");
                        if r.clicked() {
                            pick = Some(MorePick::CheckUpdates);
                        }
                        r.hovered()
                    };
                    let ink = if hovered { color::FG } else { fg };
                    icon::draw(p, Pos2::new(btn.left() + 10.0, cy - 0.5), icon::REFRESH, 11.0, ink);
                    text_mid(p, btn.left() + 18.0, cy, Align::Min, label, text::CAPTION, ink);
                }
            }
        }
    }
    pick
}

/// A refresh arrow going round, while something is under way.
fn spinner(ui: &Ui, p: &Painter, center: Pos2, ink: eframe::egui::Color32, t: f32) {
    icon::draw_rotated(p, center, icon::REFRESH, 11.0, ink, t * 5.0);
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(30));
}
