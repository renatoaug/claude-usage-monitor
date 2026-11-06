// Text helpers egui leaves out.

use eframe::egui::{Align2, Color32, Context, FontId, Painter, Pos2, Rect, Vec2};

/// egui has one weight per font: thicken by drawing twice, a hair apart.
pub fn bold(p: &Painter, pos: Pos2, align: Align2, text: &str, size: f32, color: Color32) -> Rect {
    let r = p.text(pos, align, text, FontId::proportional(size), color);
    p.text(pos + Vec2::new(0.4, 0.0), align, text, FontId::proportional(size), color);
    r
}

/// Width of `text` at `size`, for centring a mixed line.
pub fn width(ctx: &Context, text: &str, size: f32) -> f32 {
    ctx.fonts_mut(|f| f.layout_no_wrap(text.into(), FontId::proportional(size), Color32::PLACEHOLDER).size().x)
}

/// Cut to `max` characters with an ellipsis.
pub fn ellipsis(s: &str, max: usize) -> String {
    if s.chars().count() <= max { s.into() } else { format!("{}…", s.chars().take(max.saturating_sub(1)).collect::<String>()) }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ellipsis_keeps_short_text_and_cuts_long_text() {
        assert_eq!(super::ellipsis("Re", 8), "Re");
        assert_eq!(super::ellipsis("orenato.ames@example.com", 8), "orenato…");
    }
}
