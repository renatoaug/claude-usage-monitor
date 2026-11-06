// Sprite sheets baked into the binary: a grid of frames with a transparent
// background, and the sequences that play them. Only the frame changes, so
// playing one costs next to nothing.

use eframe::egui::{self, ColorImage, Painter, Pos2, Rect, TextureHandle, TextureOptions, Vec2};

use crate::ui::theme::color;

pub struct Sheet {
    tex: TextureHandle,
    cols: usize,
    rows: usize,
}

impl Sheet {
    pub fn load(ctx: &egui::Context, name: &str, png_bytes: &'static [u8], cols: usize, rows: usize) -> Self {
        let mut dec = png::Decoder::new(std::io::Cursor::new(png_bytes));
        dec.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
        let mut reader = dec.read_info().expect("embedded sheet");
        let mut buf = vec![0; reader.output_buffer_size().expect("sheet size")];
        let info = reader.next_frame(&mut buf).expect("embedded sheet");
        let img = ColorImage::from_rgba_unmultiplied([info.width as usize, info.height as usize], &buf[..info.buffer_size()]);
        Sheet { tex: ctx.load_texture(name, img, TextureOptions::LINEAR), cols, rows }
    }

    /// Draw `frame` (row-major) filling `rect`.
    pub fn draw(&self, p: &Painter, rect: Rect, frame: usize) {
        let (c, r) = ((frame % self.cols) as f32, (frame / self.cols) as f32);
        let (w, h) = (1.0 / self.cols as f32, 1.0 / self.rows as f32);
        let uv = Rect::from_min_size(Pos2::new(c * w, r * h), Vec2::new(w, h));
        p.image(self.tex.id(), rect, uv, color::NO_TINT);
    }
}

/// A loop of (frame, seconds).
pub type Seq = &'static [(usize, f32)];

/// The frame to show at `t` seconds into `seq`, and how long until it changes.
pub fn at(seq: Seq, t: f32) -> (usize, f32) {
    let total: f32 = seq.iter().map(|s| s.1).sum();
    let mut x = t.rem_euclid(total);
    for &(frame, secs) in seq {
        if x < secs {
            return (frame, secs - x);
        }
        x -= secs;
    }
    (seq[0].0, seq[0].1)
}

pub static WORKING_SHEET: &[u8] = include_bytes!("../../assets/working-sheet.png");
pub static MOODS_SHEET: &[u8] = include_bytes!("../../assets/moods-sheet.png");

// working (4 x 2): typing in turns, a look at the screen, a blink, stuck,
// happy, a cheer, back to work
pub const WORKING: Seq = &[
    (0, 0.2), (1, 0.2), (0, 0.2), (1, 0.2), (3, 0.2), (0, 0.2), (1, 0.2), (2, 0.15),
    (0, 0.2), (1, 0.2), (4, 0.7), (0, 0.2), (1, 0.2), (5, 0.5), (6, 0.6), (5, 0.3),
    (3, 0.2), (7, 0.2), (0, 0.2), (1, 0.2), (3, 0.2), (0, 0.2), (1, 0.2), (3, 0.2),
];

// moods (4 x 4), one row each. Idle mostly holds still: a breath, a blink.
pub const IDLE: Seq = &[(0, 2.4), (1, 0.6), (0, 2.2), (2, 0.15), (3, 0.6)];
pub const SLEEPING: Seq = &[(4, 0.8), (5, 0.8), (6, 0.8), (7, 0.8)];
pub const FIRE: Seq = &[(8, 0.12), (9, 0.12), (10, 0.12), (11, 0.12)];
pub const TIRED: Seq = &[(12, 0.45), (13, 0.45), (14, 0.45), (15, 0.45)];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loop_wraps_and_counts_down_to_the_next_frame() {
        assert_eq!(at(WORKING, 0.0).0, 0);
        assert_eq!(at(WORKING, 0.25).0, 1);
        assert!((at(WORKING, 0.25).1 - 0.15).abs() < 1e-4);
        let total: f32 = WORKING.iter().map(|s| s.1).sum();
        assert_eq!(at(WORKING, total + 0.05).0, 0);
    }

    #[test]
    fn every_frame_exists_in_its_sheet() {
        assert!(WORKING.iter().all(|s| s.0 < 8));
        assert!([IDLE, SLEEPING, FIRE, TIRED].iter().all(|q| q.iter().all(|s| s.0 < 16)));
    }

    #[test]
    fn the_embedded_sheets_decode_to_their_grids() {
        for (bytes, size) in [(WORKING_SHEET, (1024, 512)), (MOODS_SHEET, (1024, 1024))] {
            let r = png::Decoder::new(std::io::Cursor::new(bytes)).read_info().unwrap();
            assert_eq!((r.info().width, r.info().height), size);
        }
    }
}
