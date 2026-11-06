// Text helpers egui leaves out.

use eframe::egui::{Align, Align2, Color32, Context, FontId, Painter, Pos2, Rect, Vec2};

/// egui has one weight per font: thicken by drawing twice, a hair apart.
pub fn bold(p: &Painter, pos: Pos2, align: Align2, text: &str, size: f32, color: Color32) -> Rect {
    let r = p.text(pos, align, text, FontId::proportional(size), color);
    p.text(pos + Vec2::new(0.4, 0.0), align, text, FontId::proportional(size), color);
    r
}

/// Draw one line with the middle of its lowercase letters on `mid`, so text
/// of any size lines up with icons, dots and other text on the same row (a
/// box's centre doesn't: it counts the space for ascenders and descenders).
/// `h` anchors `x` at the text's left, centre or right. Returns its box.
pub fn text_mid(p: &Painter, x: f32, mid: f32, h: Align, text: &str, size: f32, color: Color32) -> Rect {
    let galley = p.layout_no_wrap(text.into(), FontId::proportional(size), color);
    let (baseline, ascent) =
        galley.rows.first().and_then(|r| r.glyphs.first().map(|g| (r.pos.y + g.pos.y, g.font_ascent))).unwrap_or((galley.size().y * 0.78, size * 0.8));
    // the x-height is about 0.56 of the ascent in both SF and egui's font
    let x_height = ascent * 0.56;
    let w = galley.size().x;
    let left = match h {
        Align::Min => x,
        Align::Center => x - w / 2.0,
        Align::Max => x - w,
    };
    let pos = Pos2::new(left, mid + x_height / 2.0 - baseline);
    let rect = Rect::from_min_size(pos, galley.size());
    p.galley(pos, galley, color);
    rect
}

/// `text_mid`, thickened like `bold`.
pub fn bold_mid(p: &Painter, x: f32, mid: f32, h: Align, text: &str, size: f32, color: Color32) -> Rect {
    let r = text_mid(p, x, mid, h, text, size, color);
    text_mid(p, x + 0.4, mid, h, text, size, color);
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
