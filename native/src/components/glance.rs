// The pet-only glance: the usage as the pet's own speech bubble, just above
// its head, drawn like the pet's art: a dark pixel outline with stepped
// corners, two-tone paper, a stepped tail pointing down at the pet. One
// reading per service (logo, %, a block bar that fills in the pet's
// terracotta), then the button that opens the compact monitor.

use eframe::egui::{Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Ui, Vec2};

use super::logos::Logos;
use super::service_row::ServiceRow;
use crate::ui::controls::click_area;
use crate::ui::icon::{self, EXPAND};
use crate::ui::theme::color;

const H: f32 = 26.0;
const O: f32 = 3.0; // outline, as thick as the pet's
const TAIL: f32 = 9.0;
const BLOCKS: usize = 8;

/// How much room the glance needs above the point its tail touches.
pub const HEIGHT: f32 = H + O + TAIL;

fn mono() -> FontId {
    FontId::new(10.5, FontFamily::Monospace)
}

// a pixel-art box: two bands of outline, so the corners step in by one
// outline width instead of curving
fn pixel_box(p: &Painter, r: Rect, ink: Color32) {
    p.rect_filled(r.expand2(Vec2::new(O, 0.0)), 0.0, ink);
    p.rect_filled(r.expand2(Vec2::new(0.0, O)), 0.0, ink);
}

// the tail under the bubble, centred on `x`, stepping in 3 px a row
fn tail(p: &Painter, x: f32, top: f32) {
    for (row, w) in [18.0, 12.0, 6.0].into_iter().enumerate() {
        p.rect_filled(Rect::from_min_size(Pos2::new(x - w / 2.0, top + row as f32 * O), Vec2::new(w, O)), 0.0, color::INK);
    }
    for (row, w) in [12.0, 6.0].into_iter().enumerate() {
        p.rect_filled(Rect::from_min_size(Pos2::new(x - w / 2.0, top - O + row as f32 * O), Vec2::new(w, O)), 0.0, color::PAPER_SHADE);
    }
}

fn value_of(row: &ServiceRow) -> String {
    row.pct.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "—".into())
}

fn reading_w(p: &Painter, row: &ServiceRow) -> f32 {
    let text = p.layout_no_wrap(value_of(row), mono(), color::INK).size().x;
    5.0 + 12.0 + 4.0 + text.max(BLOCKS as f32 * 4.0 - 1.0 - 16.0) + 5.0
}

/// Draw the glance centred on `center_x`, its tail touching `bottom`, faded
/// by `shown` (0..1; it rises 4 px in three steps as it appears). Returns
/// true when its expand button is clicked.
pub fn glance(ui: &Ui, logos: &Logos, center_x: f32, bottom: f32, rows: &[ServiceRow], shown: f32) -> bool {
    let mut p = ui.painter().clone();
    p.multiply_opacity(shown);
    let widths: Vec<f32> = rows.iter().map(|r| reading_w(&p, r)).collect();
    let w = widths.iter().sum::<f32>() + 3.0 + 26.0;
    let slide = ((1.0 - shown) * 3.0).ceil() / 3.0 * 4.0;
    let top = bottom - TAIL - O - H + slide;
    let bubble = Rect::from_min_size(Pos2::new((center_x - w / 2.0).round(), top.round()), Vec2::new(w.round(), H));

    // a hard pixel shadow, the outline, then the paper in two tones
    let mut shade = p.clone();
    shade.set_opacity(shown);
    pixel_box(&shade, bubble.translate(Vec2::new(0.0, O)), color::BUBBLE_SHADOW);
    pixel_box(&p, bubble, color::INK);
    p.rect_filled(bubble, 0.0, color::PAPER);
    p.rect_filled(Rect::from_min_size(bubble.left_bottom() - Vec2::new(0.0, 4.0), Vec2::new(bubble.width(), 4.0)), 0.0, color::PAPER_SHADE);
    p.rect_filled(Rect::from_min_size(bubble.min, Vec2::new(bubble.width(), 2.0)), 0.0, color::PAPER_LIGHT);
    tail(&p, bubble.center().x, bubble.bottom() + O);

    let mut x = bubble.left();
    let mid = bubble.top() + 10.0;
    for (row, rw) in rows.iter().zip(&widths) {
        let urgent = row.pct.is_some_and(|v| v >= 80.0);
        let ink = if urgent { color::INK_URGENT } else { color::INK };
        logos.draw(&p, row.provider, Rect::from_center_size(Pos2::new(x + 5.0 + 6.0, mid), Vec2::splat(12.0)), color::INK);
        let vx = x + 5.0 + 12.0 + 4.0;
        for dx in [0.0, 0.5] {
            p.text(Pos2::new(vx + dx, mid), Align2::LEFT_CENTER, value_of(row), mono(), ink);
        }
        // the % as a row of pixel blocks under the reading
        let filled = row.pct.map(|v| ((v / 100.0) * BLOCKS as f64).ceil() as usize).unwrap_or(0).min(BLOCKS);
        let by = bubble.top() + 18.0;
        let bw = (rw - 10.0 - (BLOCKS - 1) as f32) / BLOCKS as f32;
        for i in 0..BLOCKS {
            let b = Rect::from_min_size(Pos2::new(x + 5.0 + i as f32 * (bw + 1.0), by), Vec2::new(bw, 3.0));
            let fill = if i < filled { if urgent { color::INK_URGENT } else { color::GLANCE_BAR } } else { color::INK_FAINT };
            p.rect_filled(b, 0.0, fill);
        }
        x += rw;
    }
    // a pixel rule, then the way back to the monitor
    p.rect_filled(Rect::from_min_size(Pos2::new(x, bubble.top() + 5.0), Vec2::new(2.0, H - 10.0)), 0.0, color::INK_FAINT);
    let restore = Rect::from_min_size(Pos2::new(x + 3.0, bubble.top()), Vec2::new(26.0, H));
    let hovered = shown > 0.5 && {
        let resp = click_area(ui, restore, "glance-restore");
        if resp.clicked() {
            return true;
        }
        resp.hovered()
    };
    icon::draw(&p, restore.center() - Vec2::new(0.0, 1.0), EXPAND, 12.0, if hovered { color::INK } else { color::INK.gamma_multiply(0.6) });
    false
}
