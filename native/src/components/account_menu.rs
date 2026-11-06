// The account menu under the chip: one row per Claude account (a click
// switches), × to remove one that isn't on screen (asks once), and a way to
// add another.

use eframe::egui::{Align2, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::accounts::Account;
use crate::ui::surface::hairline;
use crate::ui::text::ellipsis;
use crate::ui::theme::{color, radius, text};

const W: f32 = 176.0;
const ROW: f32 = 22.0;
const PAD: f32 = 7.0;

pub enum MenuPick {
    Switch(String),
    AskRemove(String),
    Remove(String),
    Add,
}

/// The menu's box when anchored at `top_left`.
pub fn menu_rect(top_left: Pos2, accounts: usize) -> Rect {
    Rect::from_min_size(top_left, Vec2::new(W, PAD * 2.0 + (accounts + 1) as f32 * ROW + 1.0))
}

/// How an account reads in the menu: its label, else "Account N".
pub fn account_label(a: &Account, index: usize) -> String {
    a.label.clone().unwrap_or_else(|| format!("Account {}", index + 1))
}

pub fn account_menu(ui: &Ui, top_left: Pos2, accounts: &[Account], active: &str, pending_remove: Option<&str>) -> Option<MenuPick> {
    let rect = menu_rect(top_left, accounts.len());
    let p = ui.painter();
    p.rect(rect, CornerRadius::same(radius::CONTROL), color::MENU, Stroke::new(1.0, color::MENU_EDGE), StrokeKind::Inside);
    // the menu swallows clicks that miss its rows
    let _ = ui.interact(rect, ui.scope_id().with("account-menu"), Sense::click());

    let mut pick = None;
    let inner_left = rect.left() + PAD;
    let inner_right = rect.right() - PAD;
    let mut y = rect.top() + PAD;
    let removable = accounts.len() > 1;
    for (i, a) in accounts.iter().enumerate() {
        let row = Rect::from_min_size(Pos2::new(inner_left, y), Vec2::new(inner_right - inner_left, ROW));
        let is_active = a.id == active;
        let x_rect = Rect::from_min_size(Pos2::new(row.right() - 46.0, row.top()), Vec2::new(46.0, ROW));
        let can_remove = removable && !is_active;
        let x_resp = can_remove.then(|| ui.interact(x_rect, ui.scope_id().with(("remove", &a.id)), Sense::click()));
        let resp = ui.interact(row, ui.scope_id().with(("account", &a.id)), Sense::click());
        if resp.hovered() && !is_active {
            p.rect_filled(row, CornerRadius::same(radius::TAG), color::MENU_HOVER);
        }
        let cy = row.center().y;
        p.circle_filled(Pos2::new(row.left() + 6.0, cy + 1.0), 2.0, if is_active { color::SIGNED_IN } else { color::REST });
        let fg = if is_active || resp.hovered() { color::FG } else { color::FG_MUTED };
        let label = ellipsis(&account_label(a, i), 20);
        p.text(Pos2::new(row.left() + 14.0, cy), Align2::LEFT_CENTER, label, FontId::proportional(text::SMALL), fg);
        if let Some(xr) = x_resp {
            let armed = pending_remove == Some(a.id.as_str());
            let (txt, c) = if armed { ("remove?", color::ERROR) } else { ("×", if xr.hovered() { color::FG } else { color::FG_MUTED }) };
            p.text(Pos2::new(row.right() - 4.0, cy), Align2::RIGHT_CENTER, txt, FontId::proportional(text::SMALL), c);
            if xr.clicked() {
                pick = Some(if armed { MenuPick::Remove(a.id.clone()) } else { MenuPick::AskRemove(a.id.clone()) });
            }
        }
        if pick.is_none() && resp.clicked() && !is_active {
            pick = Some(MenuPick::Switch(a.id.clone()));
        }
        y += ROW;
    }
    hairline(p, inner_left, inner_right, y + 0.5);
    let add = Rect::from_min_size(Pos2::new(inner_left, y + 1.0), Vec2::new(inner_right - inner_left, ROW));
    let resp = ui.interact(add, ui.scope_id().with("add-account"), Sense::click());
    let c = if resp.hovered() { color::FG } else { color::ADD };
    p.text(Pos2::new(add.left() + 4.0, add.center().y), Align2::LEFT_CENTER, "+ Add another account", FontId::proportional(text::SMALL), c);
    if resp.clicked() {
        pick = Some(MenuPick::Add);
    }
    pick
}
