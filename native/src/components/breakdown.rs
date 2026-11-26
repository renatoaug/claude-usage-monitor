// "by product · this week", "by model · 7 days", "by project · 7 days" and the
// like: a section of ranked rows, foldable or not, each with a dot, the name, a bar against
// the top row, and a value (the tokens, their API value, or a share of the
// week).

use eframe::egui::{Align2, FontId, Pos2, Rect, Ui, Vec2};

use crate::ui::bar::bar;
use crate::ui::controls::click_area;
use crate::ui::icon;
use crate::ui::skeleton::sk_line;
use crate::ui::surface::hairline;
use crate::ui::theme::{color, text};
use crate::usage::Labeled;

const ROW_STEP: f32 = 20.0;

/// A row's tokens, as the model and project sections show them.
pub fn tokens(v: u64) -> String {
    crate::format::tokens(v)
}

/// A row's share of the week, as the product section shows it.
pub fn percent(v: u64) -> String {
    format!("{v}%")
}

/// A row's API value, held in cents.
pub fn dollars(cents: u64) -> String {
    crate::format::dollars(cents as f64 / 100.0)
}

pub fn height(rows: usize, open: bool) -> f32 {
    1.0 + 9.0 + 12.0 + if open { rows.max(1) as f32 * ROW_STEP } else { 0.0 } + 9.0
}

/// A section to draw.
pub struct Section<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub rows: &'a [Labeled],
    pub open: bool,
    pub loading: bool,
    pub bar: eframe::egui::Color32,
    pub value: fn(u64) -> String,
    /// its head folds it (and shows the disclosure)
    pub foldable: bool,
    /// its rows can be picked (all but a folded `other` row)
    pub pick: bool,
}

/// What was clicked in a section.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hit {
    Head,
    Row(usize),
}

/// Draw the section at `y`; what was clicked in it, if anything.
pub fn section(ui: &Ui, left: f32, right: f32, y: f32, s: &Section) -> Option<Hit> {
    let p = ui.painter();
    hairline(p, left, right, y);
    let hy = y + 16.0;
    let mut hit = None;
    if s.foldable {
        let head = Rect::from_min_size(Pos2::new(left, y), Vec2::new(right - left, 22.0));
        let resp = click_area(ui, head, s.id);
        let c = if resp.hovered() { color::FG } else { color::FG_SECTION };
        p.text(Pos2::new(left, hy), Align2::LEFT_CENTER, s.title, FontId::proportional(text::SMALL), c);
        let t = ui.ctx().animate_bool_with_time(ui.scope_id().with((s.id, "open")), s.open, 0.16);
        icon::disclosure(p, Pos2::new(right - 5.0, hy), t, c);
        if resp.clicked() {
            hit = Some(Hit::Head);
        }
    } else {
        p.text(Pos2::new(left, hy), Align2::LEFT_CENTER, s.title, FontId::proportional(text::SMALL), color::FG_SECTION);
    }
    if !s.open {
        return hit;
    }
    let max = s.rows.iter().map(|r| r.tokens).max().unwrap_or(1).max(1);
    let mut cy = y + 22.0 + 13.0;
    if s.rows.is_empty() && s.loading {
        sk_line(p, left + 8.0, cy, 120.0, 8.0);
        sk_line(p, right - 30.0, cy, 30.0, 8.0);
    } else if s.rows.is_empty() {
        p.text(Pos2::new(left, cy), Align2::LEFT_CENTER, "no activity", FontId::proportional(text::SMALL), color::FG_MUTED.gamma_multiply(0.5));
    }
    // columns: name (takes the slack) · 40 px bar · 44 px value, 7 px apart
    let bar_x = right - 44.0 - 7.0 - 40.0;
    for (i, r) in s.rows.iter().enumerate() {
        let row = Rect::from_min_max(Pos2::new(left, cy - ROW_STEP / 2.0), Pos2::new(right, cy + ROW_STEP / 2.0));
        let picked = s.pick && !r.label.starts_with("other · ") && {
            let resp = click_area(ui, row, (s.id, "row", i));
            if resp.clicked() {
                hit = Some(Hit::Row(i));
            }
            resp.hovered()
        };
        p.circle_filled(Pos2::new(left + 1.5, cy), 1.5, color::DOTS[i.min(3)]);
        let clip = p.with_clip_rect(Rect::from_min_max(Pos2::new(left, cy - 8.0), Pos2::new(bar_x - 7.0, cy + 8.0)));
        clip.text(Pos2::new(left + 8.0, cy), Align2::LEFT_CENTER, &r.label, FontId::proportional(text::SMALL), if picked { color::FG } else { color::FG_ROW });
        let track = Rect::from_min_size(Pos2::new(bar_x, cy - 1.5), Vec2::new(40.0, 3.0));
        bar(p, track, Some(r.tokens as f32 / max as f32), s.bar, color::TRACK_FAINT);
        p.text(Pos2::new(right, cy), Align2::RIGHT_CENTER, (s.value)(r.tokens), FontId::proportional(text::SMALL), color::FG_SECTION);
        cy += ROW_STEP;
    }
    hit
}
