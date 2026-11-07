// A progress bar: a track and its fill.

use eframe::egui::{Color32, CornerRadius, Painter, Rect, Vec2};

use super::theme::radius;

/// `share` is 0..1; None draws the empty track only.
pub fn bar(p: &Painter, track: Rect, share: Option<f32>, fill: Color32, track_color: Color32) {
    let r = CornerRadius::same(radius::CONTROL.min((track.height() / 2.0).ceil() as u8));
    p.rect_filled(track, r, track_color);
    if let Some(s) = share {
        let w = track.width() * s.clamp(0.0, 1.0);
        p.rect_filled(Rect::from_min_size(track.min, Vec2::new(w, track.height())), r, fill);
    }
}
