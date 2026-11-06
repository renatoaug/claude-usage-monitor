// The working animation: Clauddy at its desk under the lamp, from a sprite
// sheet (4 x 2 frames, 256 px each, transparent background) baked into the
// binary. Only the frame changes, so playing it costs next to nothing.

use eframe::egui::{self, ColorImage, Pos2, Rect, TextureHandle, TextureOptions, Vec2};

static SHEET: &[u8] = include_bytes!("../assets/working-sheet.png");
const COLS: usize = 4;
const ROWS: usize = 2;

// (frame, seconds): typing in turns, a look at the screen, a blink, stuck,
// happy, a cheer, back to work
const SEQ: [(usize, f32); 24] = [
    (0, 0.2),
    (1, 0.2),
    (0, 0.2),
    (1, 0.2),
    (3, 0.2),
    (0, 0.2),
    (1, 0.2),
    (2, 0.15),
    (0, 0.2),
    (1, 0.2),
    (4, 0.7),
    (0, 0.2),
    (1, 0.2),
    (5, 0.5),
    (6, 0.6),
    (5, 0.3),
    (3, 0.2),
    (7, 0.2),
    (0, 0.2),
    (1, 0.2),
    (3, 0.2),
    (0, 0.2),
    (1, 0.2),
    (3, 0.2),
];

fn total() -> f32 {
    SEQ.iter().map(|s| s.1).sum()
}

/// The frame to show at `t` seconds, and how long until it changes.
pub fn at(t: f32) -> (usize, f32) {
    let mut x = t.rem_euclid(total());
    for &(frame, secs) in &SEQ {
        if x < secs {
            return (frame, secs - x);
        }
        x -= secs;
    }
    (SEQ[0].0, SEQ[0].1)
}

pub fn load(ctx: &egui::Context) -> TextureHandle {
    let mut dec = png::Decoder::new(std::io::Cursor::new(SHEET));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = dec.read_info().expect("embedded sheet");
    let mut buf = vec![0; reader.output_buffer_size().expect("sheet size")];
    let info = reader.next_frame(&mut buf).expect("embedded sheet");
    let img = ColorImage::from_rgba_unmultiplied([info.width as usize, info.height as usize], &buf[..info.buffer_size()]);
    ctx.load_texture("working-sheet", img, TextureOptions::LINEAR)
}

/// Draw `frame` as large as fits in `area`, sitting on its bottom edge.
pub fn draw(painter: &egui::Painter, area: Rect, tex: &TextureHandle, frame: usize) {
    let side = area.width().min(area.height());
    let rect = Rect::from_min_size(Pos2::new(area.center().x - side / 2.0, area.bottom() - side), Vec2::splat(side));
    let (c, r) = ((frame % COLS) as f32, (frame / COLS) as f32);
    let uv = Rect::from_min_size(
        Pos2::new(c / COLS as f32, r / ROWS as f32),
        Vec2::new(1.0 / COLS as f32, 1.0 / ROWS as f32),
    );
    painter.image(tex.id(), rect, uv, egui::Color32::WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_loop_wraps_and_counts_down_to_the_next_frame() {
        assert_eq!(at(0.0).0, 0);
        assert_eq!(at(0.25).0, 1);
        assert!((at(0.25).1 - 0.15).abs() < 1e-4);
        assert_eq!(at(total() + 0.05).0, 0);
        assert!(SEQ.iter().all(|s| s.0 < COLS * ROWS));
    }

    #[test]
    fn the_embedded_sheet_decodes_to_the_grid() {
        let dec = png::Decoder::new(std::io::Cursor::new(SHEET));
        let r = dec.read_info().unwrap();
        assert_eq!((r.info().width, r.info().height), (1024, 512));
    }
}
