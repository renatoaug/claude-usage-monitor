// Line glyphs (lucide's, as the Electron controls used) drawn as strokes in a
// 24-unit box.

use eframe::egui::{Color32, Painter, Pos2, Shape, Stroke, Vec2};

pub type Glyph = &'static [&'static [(f32, f32)]];

pub const CLOSE: Glyph = &[&[(18., 6.), (6., 18.)], &[(6., 6.), (18., 18.)]];
pub const MINIMIZE: Glyph = &[&[(5., 12.), (19., 12.)]];
pub const EXPAND: Glyph = &[&[(15., 3.), (21., 3.), (21., 9.)], &[(9., 21.), (3., 21.), (3., 15.)], &[(21., 3.), (14., 10.)], &[(3., 21.), (10., 14.)]];
pub const EXTERNAL: Glyph = &[&[(7., 7.), (17., 7.), (17., 17.)], &[(7., 17.), (17., 7.)]];
pub const PET: Glyph = &[
    &[
        (5., 4.),
        (19., 4.),
        (19., 8.),
        (22., 8.),
        (22., 18.),
        (19., 18.),
        (19., 21.),
        (16., 21.),
        (16., 18.),
        (8., 18.),
        (8., 21.),
        (5., 21.),
        (5., 18.),
        (2., 18.),
        (2., 8.),
        (5., 8.),
        (5., 4.),
    ],
    &[(8., 11.), (8., 13.)],
    &[(16., 11.), (16., 13.)],
];
pub const GEAR: Glyph = &[
    &[(12., 2.), (12., 5.)],
    &[(12., 19.), (12., 22.)],
    &[(2., 12.), (5., 12.)],
    &[(19., 12.), (22., 12.)],
    &[(4.9, 4.9), (7., 7.)],
    &[(17., 17.), (19.1, 19.1)],
    &[(4.9, 19.1), (7., 17.)],
    &[(17., 7.), (19.1, 4.9)],
    &[(12., 6.), (16.2, 7.8), (18., 12.), (16.2, 16.2), (12., 18.), (7.8, 16.2), (6., 12.), (7.8, 7.8), (12., 6.)],
    &[(12., 10.), (14., 12.), (12., 14.), (10., 12.), (12., 10.)],
];
/// lucide log-out: a door frame, an arrow leaving it
pub const LOG_OUT: Glyph = &[
    &[(9., 21.), (5., 21.), (3.6, 20.4), (3., 19.), (3., 5.), (3.6, 3.6), (5., 3.), (9., 3.)],
    &[(16., 17.), (21., 12.), (16., 7.)],
    &[(21., 12.), (9., 12.)],
];
pub const CHEVRON_DOWN: Glyph = &[&[(7., 10.), (12., 15.), (17., 10.)]];

/// Draw `glyph` centred on `center`, `size` points across.
pub fn draw(p: &Painter, center: Pos2, glyph: Glyph, size: f32, color: Color32) {
    draw_rotated(p, center, glyph, size, color, 0.0);
}

/// Draw `glyph` turned `angle` radians (clockwise) about its centre.
pub fn draw_rotated(p: &Painter, center: Pos2, glyph: Glyph, size: f32, color: Color32, angle: f32) {
    let k = size / 24.0;
    let (sin, cos) = angle.sin_cos();
    for path in glyph {
        let pts: Vec<Pos2> = path
            .iter()
            .map(|&(x, y)| {
                let (dx, dy) = ((x - 12.0) * k, (y - 12.0) * k);
                center + Vec2::new(dx * cos - dy * sin, dx * sin + dy * cos)
            })
            .collect();
        p.add(Shape::line(pts, Stroke::new(1.1, color)));
    }
}

/// A disclosure chevron: pointing right when closed, down when open, turning
/// between the two as `open` (0..1, animated) goes.
pub fn disclosure(p: &Painter, center: Pos2, open: f32, color: Color32) {
    draw_rotated(p, center, CHEVRON_DOWN, 9.0, color, -std::f32::consts::FRAC_PI_2 * (1.0 - open));
}

/// A menu caret: down when closed, up when open, flipping as `open` goes.
pub fn caret(p: &Painter, center: Pos2, open: f32, color: Color32) {
    draw_rotated(p, center, CHEVRON_DOWN, 9.0, color, std::f32::consts::PI * open);
}
