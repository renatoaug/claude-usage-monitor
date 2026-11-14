// Compact: the pet on the left with what it's up to beneath it, one row per
// connected service on the right and the selected one's reset under them.
// With a single service its row sits alone, centred on the pet.

use eframe::egui::{Align, Context, FontId, LayerId, Order, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::ui::text::text_mid;

use super::{Action, Frame, MARGIN, Size, sessions, shown, weeks};
use crate::components::account::account;
use crate::components::logos::Provider;
use crate::components::service_row::{ROW_GAP, ROW_H, ServiceRow, service_row, service_row_skeleton};
use crate::components::status::caption;
use crate::format::clock;
use crate::ui::controls::button_row;
use crate::ui::controls::click_area;
use crate::ui::icon;
use crate::ui::surface::{card, halo};
use crate::ui::theme::{color, radius, text};

const CARD_W: f32 = 216.0;
const PAD_TOP: f32 = 32.0;
const PAD: f32 = 12.0;
const STAGE: Vec2 = Vec2::new(70.0, 70.0);
// the bottom line: the pet's word on the left, the reset on the right
const LINE_H: f32 = 14.0;

fn rows_h(rows: usize) -> f32 {
    rows as f32 * ROW_H + rows.saturating_sub(1) as f32 * ROW_GAP
}

// the reset reminder's line under everything, when it's offered
const REMIND_H: f32 = 28.0;

/// Window size for `rows` service rows, and the reminder's line.
pub fn size(rows: usize, reminder: bool) -> [f32; 2] {
    let card_h = PAD_TOP + (rows_h(rows) + LINE_H).max(STAGE.y) + PAD + if reminder { REMIND_H } else { 0.0 };
    [CARD_W + 2.0 * MARGIN, card_h + 2.0 * MARGIN]
}

/// Whether the shown service's reset reminder is offered (and on).
pub fn reminder(f: &Frame) -> Option<bool> {
    if !f.snap.loaded {
        return None;
    }
    crate::components::reminder::state(f.snap, shown(f), f.session, f.heads_up)
}

/// One row per connected service.
pub fn rows(f: &Frame) -> Vec<ServiceRow> {
    sessions(f.snap, f.session)
        .into_iter()
        .zip(weeks(f.snap))
        .map(|((provider, pct, reset_ms), week)| ServiceRow { provider, pct, reset_ms, week: week.map(|w| w.0), week_reset_ms: week.and_then(|w| w.1) })
        .collect()
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
    let remind = reminder(f);
    let below = if remind.is_some() { REMIND_H } else { 0.0 };
    let content = Rect::from_min_max(Pos2::new(rect.left() + PAD, rect.top() + PAD_TOP), Pos2::new(rect.right() - PAD, rect.bottom() - PAD - below));
    if let Some(on) = remind
        && crate::components::reminder::button_right(ui, content.right(), content.bottom() + 6.0, on)
    {
        actions.push(Action::Remind(shown(f), !on));
    }
    let line_y = content.bottom() - 6.0;
    let above = Rect::from_min_max(content.left_top(), Pos2::new(content.right(), line_y - LINE_H / 2.0 - 2.0));

    let column = Rect::from_min_max(above.left_top(), Pos2::new(above.left() + STAGE.x, above.bottom()));
    halo(p, column, color::CLAUDE_BAR.gamma_multiply(0.06));
    let body = f.pet.draw_fit(p, column.shrink2(Vec2::new(2.0, 2.0)), f.mood);
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
            // the week's % is unlabelled: hovering it says what it is
            if let Some(week) = service_row(p, f.logos, at, content.right() - col_left, row, row.provider == selected)
                && ui.interact(week, ui.scope_id().with(("week", row.provider.name())), Sense::hover()).hovered()
            {
                week_tag(ui, week, f.card, row.week_reset_ms);
            }
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
    // a remark over everything, its tail on the pet's head
    if let Some((said, shown)) = f.bubble {
        let tip = Pos2::new(body.center().x + 8.0, body.top() + 4.0);
        if crate::components::bubble::bubble(ui, said, tip, 18.0, rect.width() - 24.0, rect.shrink(6.0), shown) {
            actions.push(Action::DismissBubble);
        }
    }
    actions
}

/// The week's small tag over its %, while hovered: "week · resets Sat
/// 3:27 PM", in the menus' colours, kept inside the card.
fn week_tag(ui: &Ui, week: Rect, card: Rect, reset_ms: Option<i64>) {
    let p = ui.ctx().layer_painter(LayerId::new(Order::Tooltip, ui.scope_id().with("week-tag")));
    let label = match reset_ms {
        Some(ms) => format!("week · resets {}", clock(ms)),
        None => "this week".into(),
    };
    let galley = p.layout_no_wrap(label, FontId::proportional(text::TINY), color::FG_LABEL);
    let size = galley.size() + Vec2::new(10.0, 5.0);
    let x = (week.center().x - size.x / 2.0).clamp(card.left() + 4.0, card.right() - 4.0 - size.x);
    let tag = Rect::from_min_size(Pos2::new(x, week.top() - size.y - 1.0), size);
    p.rect(tag, radius::CONTROL, color::MENU, Stroke::new(1.0, color::MENU_EDGE), StrokeKind::Inside);
    p.galley(tag.center() - galley.size() / 2.0, galley, color::FG_LABEL);
}

#[cfg(test)]
mod tests {
    #[test]
    fn one_service_keeps_the_card_as_tall_as_the_pet() {
        assert_eq!(super::size(1, false)[1], super::size(0, false)[1]);
        assert!(super::size(3, false)[1] > super::size(1, false)[1]);
    }
}
