// The pet-only glance: the usage as the pet's own speech bubble, above its
// head. Paper with a pixel ink outline and a stepped tail pointing down; one
// reading per service (logo, %, a green bar that fills with it), then the
// button that opens the compact monitor.

use eframe::egui::{Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Ui, Vec2};

use super::logos::Logos;
use super::service_row::ServiceRow;
use crate::ui::controls::click_area;
use crate::ui::icon::{self, EXPAND};
use crate::ui::theme::color;

const H: f32 = 24.0;
const SIZE: f32 = 10.0;

fn mono() -> FontId {
    FontId::new(SIZE, FontFamily::Monospace)
}

// a block with the pixel outline the bubble uses: ink on the four sides,
// corners left open, so it reads as stepped rather than rounded
fn pixel_box(p: &Painter, r: Rect, fill: Color32, ink: Color32) {
    p.rect_filled(r.expand2(Vec2::new(2.0, 0.0)), 0.0, ink);
    p.rect_filled(r.expand2(Vec2::new(0.0, 2.0)), 0.0, ink);
    p.rect_filled(r, 0.0, fill);
}

// the tail under the bubble, centred on `x`, stepping down 2 px a row
fn tail(p: &Painter, x: f32, top: f32) {
    for (row, w) in [14.0, 10.0, 6.0, 2.0].into_iter().enumerate() {
        p.rect_filled(Rect::from_min_size(Pos2::new(x - w / 2.0, top + row as f32 * 2.0), Vec2::new(w, 2.0)), 0.0, color::INK);
    }
    for (row, w) in [10.0, 6.0, 2.0].into_iter().enumerate() {
        p.rect_filled(Rect::from_min_size(Pos2::new(x - w / 2.0, top + row as f32 * 2.0), Vec2::new(w, 2.0)), 0.0, color::PAPER);
    }
}

fn reading_w(p: &Painter, row: &ServiceRow) -> f32 {
    let value = row.pct.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "—".into());
    4.0 + 11.0 + 3.0 + p.layout_no_wrap(value, mono(), color::INK).size().x + 4.0
}

/// Draw the glance centred on `center_x` with its top at `top`, faded by
/// `shown` (0..1, it slides down 4 px as it appears). Returns true when its
/// expand button is clicked.
pub fn glance(ui: &Ui, logos: &Logos, center_x: f32, top: f32, rows: &[ServiceRow], shown: f32) -> bool {
    let mut p = ui.painter().clone();
    p.multiply_opacity(shown);
    let widths: Vec<f32> = rows.iter().map(|r| reading_w(&p, r)).collect();
    let inner_w: f32 = widths.iter().sum::<f32>() + 2.0 + 4.0 + 26.0;
    let w = inner_w + 5.0;
    // it slides into place in three steps, as the CSS did
    let slide = ((1.0 - shown) * 3.0).ceil() / 3.0 * 4.0;
    let bubble = Rect::from_min_size(Pos2::new(center_x - w / 2.0, top - slide), Vec2::new(w, H));

    // shadow, outline, paper, tail
    let shadow = bubble.translate(Vec2::new(2.0, 4.0));
    p.rect_filled(shadow.expand2(Vec2::new(2.0, 0.0)), 0.0, color::BUBBLE_SHADOW);
    p.rect_filled(shadow.expand2(Vec2::new(0.0, 2.0)), 0.0, color::BUBBLE_SHADOW);
    pixel_box(&p, bubble, color::PAPER, color::INK);
    tail(&p, bubble.center().x, bubble.bottom());

    let mut x = bubble.left() + 3.0;
    let mid = bubble.center().y - 1.0;
    for (row, rw) in rows.iter().zip(&widths) {
        let urgent = row.pct.is_some_and(|v| v >= 80.0);
        logos.draw(&p, row.provider, Rect::from_center_size(Pos2::new(x + 4.0 + 5.5, mid), Vec2::splat(11.0)), color::INK);
        let value = row.pct.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "—".into());
        let vx = x + 4.0 + 11.0 + 3.0;
        p.text(Pos2::new(vx, mid), Align2::LEFT_CENTER, &value, mono(), if urgent { color::INK_URGENT } else { color::INK });
        p.text(Pos2::new(vx + 0.4, mid), Align2::LEFT_CENTER, &value, mono(), if urgent { color::INK_URGENT } else { color::INK });
        // the % as a pixel bar under the reading
        let bar = Rect::from_min_size(Pos2::new(x + 4.0, bubble.bottom() - 4.0), Vec2::new(rw - 8.0, 2.0));
        p.rect_filled(bar, 0.0, color::INK_FAINT);
        if let Some(v) = row.pct {
            p.rect_filled(Rect::from_min_size(bar.min, Vec2::new(bar.width() * (v / 100.0).clamp(0.0, 1.0) as f32, 2.0)), 0.0, color::GLANCE_BAR);
        }
        x += rw;
    }
    // a separator, then the way back to the monitor
    p.rect_filled(Rect::from_min_size(Pos2::new(x + 2.0, mid - 6.0), Vec2::new(2.0, 12.0)), 0.0, color::INK_SEP);
    let restore = Rect::from_min_size(Pos2::new(x + 6.0, bubble.top()), Vec2::new(26.0, H));
    let clicked = shown > 0.5 && {
        let resp = click_area(ui, restore, "glance-restore");
        let ink = if resp.hovered() { color::INK } else { color::INK.gamma_multiply(0.55) };
        icon::draw(&p, restore.center(), EXPAND, 11.0, ink);
        resp.clicked()
    };
    if shown <= 0.5 {
        icon::draw(&p, restore.center(), EXPAND, 11.0, color::INK.gamma_multiply(0.55));
    }
    clicked
}
