// Compact: the pet on the left with what it's up to beneath it, one row per
// connected service on the right and the selected one's reset under them.
// With a single service its row sits alone, centred on the pet.

use eframe::egui::{Align, Context, Pos2, Rect, Ui, Vec2};

use crate::ui::text::text_mid;

use super::{Action, Frame, MARGIN, Size, sessions, shown};
use crate::ui::controls::click_area;
use crate::components::account::account;
use crate::components::logos::Provider;
use crate::components::service_row::{ROW_GAP, ROW_H, ServiceRow, service_row, service_row_skeleton};
use crate::components::status::caption;
use crate::format::clock;
use crate::ui::controls::button_row;
use crate::ui::icon;
use crate::ui::surface::{card, halo};
use crate::ui::theme::{color, text};

const CARD_W: f32 = 216.0;
const PAD_TOP: f32 = 32.0;
const PAD: f32 = 12.0;
const STAGE: Vec2 = Vec2::new(70.0, 70.0);
// the bottom line: the pet's word on the left, the reset on the right
const LINE_H: f32 = 14.0;

fn rows_h(rows: usize) -> f32 {
    rows as f32 * ROW_H + rows.saturating_sub(1) as f32 * ROW_GAP
}

/// Window size for `rows` service rows.
pub fn size(rows: usize) -> [f32; 2] {
    let card_h = PAD_TOP + (rows_h(rows) + LINE_H).max(STAGE.y) + PAD;
    [CARD_W + 2.0 * MARGIN, card_h + 2.0 * MARGIN]
}

/// One row per connected service.
pub fn rows(f: &Frame) -> Vec<ServiceRow> {
    sessions(f.snap, f.session).into_iter().map(|(provider, pct, reset_ms)| ServiceRow { provider, pct, reset_ms }).collect()
}

/// The services working right now, for the logos under the pet.
pub fn workers(f: &Frame) -> Vec<Provider> {
    super::workers(f.snap)
}

pub fn draw(ui: &Ui, ctx: &Context, f: &Frame) -> Vec<Action> {
    let mut actions = vec![];
    let rows = rows(f);
    let rect = f.area;
    card(ui, ctx, f.card);
    let p = ui.painter();

    // title band: the account, then pet-only, expand and close
    let band_y = rect.top() + 15.5;
    account(p, rect.left() + 10.0, band_y, f.snap.profile.as_ref(), !f.snap.loaded);
    match button_row(ui, rect.right() - 8.0, band_y, &[("close", icon::CLOSE), ("expand", icon::EXPAND), ("pet-only", icon::PET)]) {
        Some(0) => actions.push(Action::Close),
        Some(1) => actions.push(Action::Resize(Size::Expanded)),
        Some(2) => actions.push(Action::Resize(Size::PetOnly)),
        _ => {}
    }

    // the content: the pet's column on the left, the rows on the right, and
    // a bottom line with the pet's word under it and the reset at the right
    let content = Rect::from_min_max(Pos2::new(rect.left() + PAD, rect.top() + PAD_TOP), Pos2::new(rect.right() - PAD, rect.bottom() - PAD));
    let line_y = content.bottom() - 6.0;
    let above = Rect::from_min_max(content.left_top(), Pos2::new(content.right(), line_y - LINE_H / 2.0 - 2.0));

    let column = Rect::from_min_max(above.left_top(), Pos2::new(above.left() + STAGE.x, above.bottom()));
    halo(p, column, color::CLAUDE_BAR.gamma_multiply(0.06));
    let _ = f.pet.draw_fit(p, column.shrink2(Vec2::new(2.0, 2.0)), f.mood);
    let activity = f.snap.usage.as_ref().and_then(|u| u.activity.as_deref());
    caption(p, ctx, f.logos, column.center().x, line_y, f.mood, activity, &workers(f));

    // the rows, centred on the space above the bottom line
    let col_left = column.right() + 8.0;
    let selected = shown(f);
    let mut y = above.center().y - rows_h(rows.len()) / 2.0;
    for row in &rows {
        let at = Pos2::new(col_left, y);
        // with several, a row picks the service the panel shows
        if rows.len() > 1 && click_area(ui, Rect::from_min_size(at, Vec2::new(content.right() - col_left, ROW_H)), ("row", row.provider.name())).clicked() {
            actions.push(Action::SelectProvider(row.provider));
        }
        if f.snap.loaded {
            service_row(p, f.logos, at, content.right() - col_left, row, row.provider == selected);
        } else {
            service_row_skeleton(p, f.logos, at, content.right() - col_left, row.provider);
        }
        y += ROW_H + ROW_GAP;
    }
    if !f.snap.loaded {
        crate::ui::skeleton::sk_line(p, content.right() - 62.0, line_y, 62.0, 7.0);
    } else if let Some(ms) = rows.iter().find(|r| r.provider == selected).and_then(|r| r.reset_ms) {
        text_mid(p, content.right(), line_y, Align::Max, &format!("resets {}", clock(ms)), text::TINY, color::FG_RESET);
    }
    actions
}

#[cfg(test)]
mod tests {
    #[test]
    fn one_service_keeps_the_card_as_tall_as_the_pet() {
        assert_eq!(super::size(1)[1], super::size(0)[1]);
        assert!(super::size(3)[1] > super::size(1)[1]);
    }
}
