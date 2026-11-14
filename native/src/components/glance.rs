// The pet-only glance: the usage as the pet's own speech bubble, just above
// its head, drawn like the pet's art: a dark pixel outline with stepped
// corners, two-tone paper, a stepped tail pointing down at the pet. One
// reading per service (logo, %, a block bar that fills in the pet's
// terracotta, the week's % and a thinner bar beside it); hovering slides out
// the button that opens the compact monitor.

use eframe::egui::{Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Ui, Vec2};

use super::logos::Logos;
use super::service_row::ServiceRow;
use crate::ui::controls::click_area;
use crate::ui::icon::{self, EXPAND};
use crate::ui::theme::color;

const H: f32 = 28.0;
const WEEK_GAP: f32 = 4.0;
// the expand button's strip, the rule included
const RESTORE_W: f32 = 29.0;
pub(crate) const O: f32 = 3.0; // outline, as thick as the pet's
pub(crate) const TAIL: f32 = 9.0;
const BLOCKS: usize = 8;

/// How much room the glance needs above the point its tail touches.
pub const HEIGHT: f32 = H + O + TAIL;

fn mono() -> FontId {
    FontId::new(10.5, FontFamily::Monospace)
}

// a pixel-art box: two bands of outline, so the corners step in by one
// outline width instead of curving
pub(crate) fn pixel_box(p: &Painter, r: Rect, ink: Color32) {
    p.rect_filled(r.expand2(Vec2::new(O, 0.0)), 0.0, ink);
    p.rect_filled(r.expand2(Vec2::new(0.0, O)), 0.0, ink);
}

// the tail under the bubble, centred on `x`, stepping in 3 px a row
pub(crate) fn tail(p: &Painter, x: f32, top: f32) {
    for (row, w) in [18.0, 12.0, 6.0].into_iter().enumerate() {
        p.rect_filled(Rect::from_min_size(Pos2::new(x - w / 2.0, top + row as f32 * O), Vec2::new(w, O)), 0.0, color::INK);
    }
    for (row, w) in [12.0, 6.0].into_iter().enumerate() {
        p.rect_filled(Rect::from_min_size(Pos2::new(x - w / 2.0, top - O + row as f32 * O), Vec2::new(w, O)), 0.0, color::PAPER_SHADE);
    }
}

fn value_of(pct: Option<f64>) -> String {
    pct.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "—".into())
}

// the session's value, then the week's after a gap when there is one
fn text_w(p: &Painter, row: &ServiceRow, week: Option<f64>) -> f32 {
    let session = p.layout_no_wrap(value_of(row.pct), mono(), color::INK).size().x;
    session + week.map(|w| WEEK_GAP + p.layout_no_wrap(value_of(Some(w)), mono(), color::INK).size().x).unwrap_or(0.0)
}

fn reading_w(p: &Painter, row: &ServiceRow, week: Option<f64>) -> f32 {
    5.0 + 12.0 + 4.0 + text_w(p, row, week).max(BLOCKS as f32 * 4.0 - 1.0 - 16.0) + 5.0
}

// a row of pixel blocks, `filled` of them lit
fn blocks(p: &Painter, x: f32, y: f32, w: f32, h: f32, pct: Option<f64>, urgent: bool) {
    let filled = pct.map(|v| ((v / 100.0) * BLOCKS as f64).ceil() as usize).unwrap_or(0).min(BLOCKS);
    let bw = (w - (BLOCKS - 1) as f32) / BLOCKS as f32;
    for i in 0..BLOCKS {
        let b = Rect::from_min_size(Pos2::new(x + i as f32 * (bw + 1.0), y), Vec2::new(bw, h));
        let fill = if i < filled { if urgent { color::INK_URGENT } else { color::GLANCE_BAR } } else { color::INK_FAINT };
        p.rect_filled(b, 0.0, fill);
    }
}

/// Draw the glance with its tail at `center_x`, touching `bottom`, faded by
/// `shown` (0..1; it rises 4 px in three steps as it appears). Each reading
/// is the session % in bold, the week's % beside it, and a bar for each.
/// `hover` (0..1) slides the expand button out on the right. Returns whether
/// that button was clicked, and the glance's box.
#[allow(clippy::too_many_arguments)]
pub fn glance(ui: &Ui, logos: &Logos, center_x: f32, bottom: f32, rows: &[ServiceRow], weeks: &[Option<f64>], shown: f32, hover: f32) -> (bool, Rect) {
    let mut p = ui.painter().clone();
    p.multiply_opacity(shown);
    let week_of = |i: usize| weeks.get(i).copied().flatten();
    let widths: Vec<f32> = rows.iter().enumerate().map(|(i, r)| reading_w(&p, r, week_of(i))).collect();
    let base = widths.iter().sum::<f32>();
    let w = base + RESTORE_W * hover;
    let slide = ((1.0 - shown) * 3.0).ceil() / 3.0 * 4.0;
    let top = bottom - TAIL - O - H + slide;
    // the readings stay put; the button grows the bubble to the right
    let bubble = Rect::from_min_size(Pos2::new((center_x - base / 2.0).round(), top.round()), Vec2::new(w.round(), H));

    // a hard pixel shadow, the outline, then the paper in two tones
    let mut shade = p.clone();
    shade.set_opacity(shown);
    pixel_box(&shade, bubble.translate(Vec2::new(0.0, O)), color::BUBBLE_SHADOW);
    pixel_box(&p, bubble, color::INK);
    p.rect_filled(bubble, 0.0, color::PAPER);
    p.rect_filled(Rect::from_min_size(bubble.left_bottom() - Vec2::new(0.0, 4.0), Vec2::new(bubble.width(), 4.0)), 0.0, color::PAPER_SHADE);
    p.rect_filled(Rect::from_min_size(bubble.min, Vec2::new(bubble.width(), 2.0)), 0.0, color::PAPER_LIGHT);
    tail(&p, center_x.round(), bubble.bottom() + O);

    let mut x = bubble.left();
    let mid = bubble.top() + 10.0;
    for (i, (row, rw)) in rows.iter().zip(&widths).enumerate() {
        let week = week_of(i);
        let urgent = row.pct.is_some_and(|v| v >= 80.0);
        let ink = if urgent { color::INK_URGENT } else { color::INK };
        logos.draw(&p, row.provider, Rect::from_center_size(Pos2::new(x + 5.0 + 6.0, mid), Vec2::splat(12.0)), color::INK);
        let vx = x + 5.0 + 12.0 + 4.0;
        let session = value_of(row.pct);
        for dx in [0.0, 0.5] {
            p.text(Pos2::new(vx + dx, mid), Align2::LEFT_CENTER, session.clone(), mono(), ink);
        }
        let bx = x + 5.0;
        let bw = rw - 10.0;
        blocks(&p, bx, bubble.top() + 17.0, bw, 3.0, row.pct, urgent);
        if let Some(wk) = week {
            let week_urgent = wk >= 80.0;
            let wx = vx + p.layout_no_wrap(session, mono(), color::INK).size().x + WEEK_GAP;
            let week_ink = if week_urgent { color::INK_URGENT } else { color::INK.gamma_multiply(0.6) };
            p.text(Pos2::new(wx, mid), Align2::LEFT_CENTER, value_of(Some(wk)), mono(), week_ink);
            blocks(&p, bx, bubble.top() + 21.0, bw, 2.0, Some(wk), week_urgent);
        }
        x += rw;
    }
    // on hover: a pixel rule, then the way back to the monitor
    if hover > 0.0 {
        let mut b = p.clone();
        b.multiply_opacity(hover);
        b.rect_filled(Rect::from_min_size(Pos2::new(x, bubble.top() + 5.0), Vec2::new(2.0, H - 10.0)), 0.0, color::INK_FAINT);
        let restore = Rect::from_min_size(Pos2::new(x + 3.0, bubble.top()), Vec2::new(RESTORE_W - 3.0, H));
        let hovered = shown > 0.5 && hover > 0.5 && {
            let resp = click_area(ui, restore, "glance-restore");
            if resp.clicked() {
                return (true, bubble.expand(O));
            }
            resp.hovered()
        };
        icon::draw(&b, restore.center() - Vec2::new(0.0, 1.0), EXPAND, 12.0, if hovered { color::INK } else { color::INK.gamma_multiply(0.6) });
    }
    (false, bubble.expand(O))
}
