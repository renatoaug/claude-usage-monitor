// The account on the title band. Compact: the name and the plan. Expanded:
// the chip, a signed-in dot, the email, a caret that opens the account menu,
// and the plan.

use eframe::egui::{Align2, FontId, Painter, Pos2, Rect, Vec2};

use crate::auth::Profile;
use crate::ui::icon::{self, CHEVRON_DOWN};
use crate::ui::tag::{TagStyle, tag};
use crate::ui::text::ellipsis;
use crate::ui::theme::{color, text};

fn plan_tag(p: &Painter, x: f32, y: f32, plan: &str) -> Rect {
    let style = TagStyle { fg: color::PLAN, edge: color::PLAN_EDGE, wash: color::PLAN_WASH, pad: Vec2::new(3.0, 2.0) };
    tag(p, Pos2::new(x, y), Align2::LEFT_CENTER, &plan.to_lowercase(), &style)
}

/// Compact: the name (or email) and the plan.
pub fn account(p: &Painter, left: f32, y: f32, profile: Option<&Profile>) {
    let Some(pr) = profile else { return };
    let who = pr.name.as_deref().or(pr.email.as_deref()).map(|n| ellipsis(n, 18)).unwrap_or_default();
    let r = p.text(Pos2::new(left, y), Align2::LEFT_CENTER, who, FontId::proportional(text::SMALL), color::FG_MUTED);
    if let Some(plan) = pr.plan.as_deref() {
        plan_tag(p, r.right() + 8.0, y, plan);
    }
}

/// Expanded: the chip; returns the area that opens the account menu.
pub fn account_chip(p: &Painter, left: f32, y: f32, label: Option<&str>, plan: Option<&str>, hovered: bool) -> Rect {
    let fg = if hovered { color::FG } else { color::FG_MUTED };
    let who = label.map(|n| ellipsis(n, 16)).unwrap_or_else(|| "Claude".into());
    let r = p.text(Pos2::new(left + 9.0, y), Align2::LEFT_CENTER, who, FontId::proportional(text::SMALL), fg);
    // the dot sits on the text's visual middle, a hair below its box's centre
    p.circle_filled(Pos2::new(left + 2.0, r.center().y + 1.0), 2.0, color::SIGNED_IN);
    let caret = Pos2::new(r.right() + 6.0, r.center().y + 0.5);
    icon::draw(p, caret, CHEVRON_DOWN, 9.0, fg);
    let mut right = caret.x + 5.0;
    if let Some(plan) = plan {
        right = plan_tag(p, right + 5.0, y, plan).right();
    }
    Rect::from_min_max(Pos2::new(left - 2.0, y - 9.0), Pos2::new(right, y + 9.0))
}
