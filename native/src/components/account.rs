// The account on the title band: (expanded) a signed-in dot, the name or
// email, and the plan badge.

use eframe::egui::{Align2, FontId, Painter, Pos2, Vec2};

use crate::auth::Profile;
use crate::ui::tag::{TagStyle, tag};
use crate::ui::text::ellipsis;
use crate::ui::theme::{color, text};

pub fn account(p: &Painter, left: f32, y: f32, profile: Option<&Profile>, expanded: bool) {
    let mut x = left;
    if expanded {
        p.circle_filled(Pos2::new(x + 2.0, y), 2.0, color::SIGNED_IN);
        x += 9.0;
    }
    let Some(pr) = profile else { return };
    // expanded has room for the email; compact shows the shorter name
    let who = if expanded { pr.email.as_deref().or(pr.name.as_deref()) } else { pr.name.as_deref().or(pr.email.as_deref()) };
    let who = who.map(|n| ellipsis(n, if expanded { 16 } else { 18 })).unwrap_or_default();
    let r = p.text(Pos2::new(x, y), Align2::LEFT_CENTER, who, FontId::proportional(text::SMALL), color::FG_MUTED);
    if let Some(plan) = pr.plan.as_deref() {
        let style = TagStyle { fg: color::PLAN, edge: color::PLAN_EDGE, wash: color::PLAN_WASH, pad: Vec2::new(3.0, 2.0) };
        tag(p, Pos2::new(r.right() + 8.0, y), Align2::LEFT_CENTER, &plan.to_lowercase(), &style);
    }
}
