// The 30-day map: one square per day, warmer the busier.

use eframe::egui::{Align2, CornerRadius, FontId, Painter, Pos2, Rect, Vec2};

use crate::format::tokens;
use crate::ui::surface::hairline;
use crate::ui::theme::{color, radius, text};

pub const HEAT_H: f32 = 35.0;

/// 0 when idle, then 1-4 by share of the busiest day.
pub fn heat_level(v: u64, max: u64) -> usize {
    if v == 0 {
        return 0;
    }
    match v as f64 / max.max(1) as f64 {
        r if r < 0.3 => 1,
        r if r < 0.6 => 2,
        r if r < 0.85 => 3,
        _ => 4,
    }
}

pub fn heat_map(p: &Painter, left: f32, right: f32, y: f32, days: &[u64], total: u64) {
    hairline(p, left, right, y);
    let my = y + 15.0;
    p.text(Pos2::new(left, my), Align2::LEFT_CENTER, "30 days", FontId::proportional(text::CAPTION), color::FG_SECTION);
    p.text(Pos2::new(right, my), Align2::RIGHT_CENTER, format!("{} tokens", tokens(total)), FontId::proportional(text::CAPTION), color::FG_SECTION);
    let max = days.iter().copied().max().unwrap_or(1);
    let n = days.len().max(1) as f32;
    let w = (right - left - 2.0 * (n - 1.0)) / n;
    for (i, &v) in days.iter().enumerate() {
        let sq = Rect::from_min_size(Pos2::new(left + i as f32 * (w + 2.0), my + 13.0), Vec2::new(w, 7.0));
        p.rect_filled(sq, CornerRadius::same(radius::TAG), color::HEAT[heat_level(v, max)]);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_map_climbs_by_share_of_the_busiest_day() {
        use super::heat_level;
        assert_eq!(heat_level(0, 100), 0);
        assert_eq!(heat_level(10, 100), 1);
        assert_eq!(heat_level(50, 100), 2);
        assert_eq!(heat_level(70, 100), 3);
        assert_eq!(heat_level(100, 100), 4);
    }
}
