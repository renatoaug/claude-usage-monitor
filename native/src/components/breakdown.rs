// "by model · 7 days" and "by project · 7 days": a foldable section of
// ranked rows, each with a dot, the name, a bar against the top row, and the
// tokens.

use eframe::egui::{Align2, FontId, Pos2, Rect, Ui, Vec2};

use crate::format::tokens;
use crate::ui::bar::bar;
use crate::ui::controls::click_area;
use crate::ui::skeleton::sk_line;
use crate::ui::icon;
use crate::ui::surface::hairline;
use crate::ui::theme::{color, text};
use crate::usage::Labeled;

const ROW_STEP: f32 = 20.0;

pub fn height(rows: usize, open: bool) -> f32 {
    1.0 + 9.0 + 12.0 + if open { rows.max(1) as f32 * ROW_STEP } else { 0.0 } + 9.0
}

/// Draw the section at `y`; true when its head was clicked (fold or unfold).
pub fn section(ui: &Ui, left: f32, right: f32, y: f32, id: &str, title: &str, rows: &[Labeled], open: bool, loading: bool, bar_color: eframe::egui::Color32) -> bool {
    let p = ui.painter();
    hairline(p, left, right, y);
    let head = Rect::from_min_size(Pos2::new(left, y), Vec2::new(right - left, 22.0));
    let resp = click_area(ui, head, id);
    let c = if resp.hovered() { color::FG } else { color::FG_SECTION };
    let hy = y + 16.0;
    p.text(Pos2::new(left, hy), Align2::LEFT_CENTER, title, FontId::proportional(text::SMALL), c);
    let t = ui.ctx().animate_bool_with_time(ui.scope_id().with((id, "open")), open, 0.16);
    icon::disclosure(p, Pos2::new(right - 5.0, hy), t, c);
    if !open {
        return resp.clicked();
    }
    let max = rows.iter().map(|r| r.tokens).max().unwrap_or(1).max(1);
    let mut cy = y + 22.0 + 13.0;
    if rows.is_empty() && loading {
        sk_line(p, left + 8.0, cy, 120.0, 8.0);
        sk_line(p, right - 30.0, cy, 30.0, 8.0);
    } else if rows.is_empty() {
        p.text(Pos2::new(left, cy), Align2::LEFT_CENTER, "no activity", FontId::proportional(text::SMALL), color::FG_MUTED.gamma_multiply(0.5));
    }
    // columns: name (takes the slack) · 40 px bar · 44 px value, 7 px apart
    let bar_x = right - 44.0 - 7.0 - 40.0;
    for (i, r) in rows.iter().enumerate() {
        p.circle_filled(Pos2::new(left + 1.5, cy), 1.5, color::DOTS[i.min(3)]);
        let clip = p.with_clip_rect(Rect::from_min_max(Pos2::new(left, cy - 8.0), Pos2::new(bar_x - 7.0, cy + 8.0)));
        clip.text(Pos2::new(left + 8.0, cy), Align2::LEFT_CENTER, &r.label, FontId::proportional(text::SMALL), color::FG_ROW);
        let track = Rect::from_min_size(Pos2::new(bar_x, cy - 1.5), Vec2::new(40.0, 3.0));
        bar(p, track, Some(r.tokens as f32 / max as f32), bar_color, color::TRACK_FAINT);
        p.text(Pos2::new(right, cy), Align2::RIGHT_CENTER, tokens(r.tokens), FontId::proportional(text::SMALL), color::FG_SECTION);
        cy += ROW_STEP;
    }
    resp.clicked()
}
