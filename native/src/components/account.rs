// The account on the title band. Compact: the name and the plan. Expanded:
// the chip, a signed-in dot, the email, a caret that opens the account menu,
// and the plan.

use eframe::egui::{Align, Painter, Pos2, Rect, Vec2};

use crate::auth::Profile;
use crate::ui::icon;
use crate::ui::skeleton::sk_line;
use crate::ui::tag::{TagStyle, tag};
use crate::ui::text::{ellipsis, text_mid};
use crate::ui::theme::{color, text};

fn plan_tag(p: &Painter, x: f32, y: f32, plan: &str) -> Rect {
    let style = TagStyle { fg: color::PLAN, edge: color::PLAN_EDGE, wash: color::PLAN_WASH, pad: Vec2::new(3.0, 2.0) };
    tag(p, x, y, Align::Min, &plan.to_lowercase(), &style)
}

/// Compact: the name (or email) and the plan; a placeholder while loading.
pub fn account(p: &Painter, left: f32, y: f32, profile: Option<&Profile>, loading: bool) {
    let Some(pr) = profile else {
        if loading {
            sk_line(p, left, y, 54.0, 9.0);
        }
        return;
    };
    let who = pr.name.as_deref().or(pr.email.as_deref()).map(|n| ellipsis(n, 18)).unwrap_or_default();
    let r = text_mid(p, left, y, Align::Min, &who, text::SMALL, color::FG_MUTED);
    if let Some(plan) = pr.plan.as_deref() {
        plan_tag(p, r.right() + 8.0, y, plan);
    }
}

/// Expanded: the chip; returns the area that opens the account menu.
pub fn account_chip(p: &Painter, left: f32, y: f32, label: Option<&str>, plan: Option<&str>, hovered: bool, open: f32) -> Rect {
    let fg = if hovered { color::FG } else { color::FG_MUTED };
    // no one to name yet: hold the place
    if label.is_none() && plan.is_none() {
        p.circle_filled(Pos2::new(left + 2.0, y), 2.0, color::SKELETON);
        sk_line(p, left + 9.0, y, 110.0, 9.0);
        return Rect::from_min_max(Pos2::new(left - 2.0, y - 9.0), Pos2::new(left + 122.0, y + 9.0));
    }
    let who = label.map(|n| ellipsis(n, 14)).unwrap_or_else(|| "Claude".into());
    let r = text_mid(p, left + 9.0, y, Align::Min, &who, text::SMALL, fg);
    p.circle_filled(Pos2::new(left + 2.0, y), 2.0, color::SIGNED_IN);
    let caret = Pos2::new(r.right() + 6.0, y);
    icon::caret(p, caret, open, fg);
    let mut right = caret.x + 5.0;
    if let Some(plan) = plan {
        right = plan_tag(p, right + 5.0, y, plan).right();
    }
    Rect::from_min_max(Pos2::new(left - 2.0, y - 9.0), Pos2::new(right, y + 9.0))
}
