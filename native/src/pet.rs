// The pet: the classic sprite for idle, sleeping, on fire and maxed out, and
// the desk sprite sheet while working. It only asks for a repaint when the
// next visible change is due, so a resting pet costs nothing between blinks.

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, TextureHandle, Vec2, epaint::QuadraticBezierShape};
use std::time::{Duration, Instant};

use crate::sprite;

const SPRITE: [&str; 9] = [
    ".########.",
    ".########.",
    "##########",
    "###o##o###",
    "##########",
    ".########.",
    ".########.",
    ".#.#..#.#.",
    ".#.#..#.#.",
];
const PIXEL: Color32 = Color32::from_rgb(0xd5, 0x76, 0x58);
const EYE: Color32 = Color32::from_rgb(0x22, 0x1b, 0x16);
const SWEAT: Color32 = Color32::from_rgb(0x8e, 0xc9, 0xf0);

// pixel art doesn't need 60 fps: the motion is a few device pixels at most
const ANIM_FPS: f32 = 12.0;
const BLINK_PERIOD: f32 = 5.5;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mood {
    Idle,
    Working,
    Fire,
    Tired,
    Sleeping,
}

impl Mood {
    /// CLAUDDY_STATE pins a mood (dev aid): idle, working, fire, tired, sleeping.
    pub fn from_env() -> Option<Self> {
        Some(match std::env::var("CLAUDDY_STATE").ok()?.as_str() {
            "working" => Mood::Working,
            "fire" => Mood::Fire,
            "tired" => Mood::Tired,
            "sleeping" => Mood::Sleeping,
            "idle" => Mood::Idle,
            _ => return None,
        })
    }
    // idle and tired stay still between blinks
    fn animates(self) -> bool {
        matches!(self, Mood::Fire | Mood::Sleeping)
    }
}

pub struct Pet {
    start: Instant,
    sheet: TextureHandle,
}

impl Pet {
    pub fn new(ctx: &egui::Context) -> Self {
        Pet { start: Instant::now(), sheet: sprite::load(ctx) }
    }

    fn t(&self) -> f32 {
        self.start.elapsed().as_secs_f32()
    }

    /// How long until the next visible change.
    pub fn next_frame(&self, mood: Mood) -> Duration {
        if mood == Mood::Working {
            return Duration::from_secs_f32(sprite::at(self.t()).1);
        }
        if mood.animates() {
            return Duration::from_secs_f32(1.0 / ANIM_FPS);
        }
        let phase = self.t() % BLINK_PERIOD / BLINK_PERIOD;
        if phase < 0.92 {
            Duration::from_secs_f32((0.92 - phase) * BLINK_PERIOD)
        } else {
            Duration::from_secs_f32(1.0 / ANIM_FPS)
        }
    }

    pub fn draw(&self, painter: &egui::Painter, area: Rect, mood: Mood) {
        let t = self.t();
        if mood == Mood::Working {
            sprite::draw(painter, area, &self.sheet, sprite::at(t).0);
            return;
        }
        let cell = (area.width().min(area.height() * 1.1) / 12.0).floor();
        let size = Vec2::new(cell * 10.0, cell * 9.0);
        let mut origin = Pos2::new((area.center().x - size.x / 2.0).round(), area.bottom() - size.y - cell);

        // mood offsets, snapped to whole pixels like the sprite itself
        let wave = |period: f32| (t * std::f32::consts::TAU / period).sin();
        match mood {
            Mood::Fire => origin.x += (wave(0.3) * cell * 0.1).round(),
            Mood::Sleeping => origin.y -= ((wave(5.6) + 1.0) * cell * 0.15).round(),
            Mood::Tired => origin.y += (cell * 0.3).round(), // slumped
            _ => {}
        }
        if mood == Mood::Fire {
            draw_flames(painter, area, t);
        }

        let phase = t % BLINK_PERIOD / BLINK_PERIOD;
        let eye_h = match mood {
            Mood::Sleeping => 0.15,
            _ if (0.92..0.98).contains(&phase) => 0.1,
            Mood::Tired => 0.4,
            _ => 1.0,
        };
        for (r, row) in SPRITE.iter().enumerate() {
            for (c, ch) in row.chars().enumerate() {
                if ch == '.' {
                    continue;
                }
                let min = origin + Vec2::new(c as f32 * cell, r as f32 * cell);
                painter.rect_filled(Rect::from_min_size(min, Vec2::splat(cell)), 0.0, PIXEL);
                if ch == 'o' {
                    let h = (cell * eye_h).max(1.0).round();
                    // tired eyes droop to the bottom of the cell
                    let y = if mood == Mood::Tired { cell - h } else { (cell - h) / 2.0 };
                    painter.rect_filled(Rect::from_min_size(min + Vec2::new(0.0, y), Vec2::new(cell, h)), 0.0, EYE);
                }
            }
        }
        if mood == Mood::Tired {
            let drop = origin + Vec2::new(cell * 9.6, cell * 0.6);
            painter.circle_filled(drop + Vec2::new(0.0, cell * 0.35), cell * 0.32, SWEAT);
            painter.add(Shape::convex_polygon(
                vec![drop, drop + Vec2::new(cell * 0.3, cell * 0.3), drop + Vec2::new(-cell * 0.3, cell * 0.3)],
                SWEAT,
                Stroke::NONE,
            ));
        }
    }
}

// The four flame tongues from the Electron sprite (viewBox 0 0 100 110), swaying.
fn draw_flames(painter: &egui::Painter, area: Rect, t: f32) {
    let tongues: [(&[[f32; 2]], Color32, f32); 4] = [
        (&[[12., 106.], [-5., 84.], [13., 35.], [9., 70.], [26., 80.], [40., 102.], [12., 106.]], Color32::from_rgb(0xed, 0x64, 0x30), 0.0),
        (&[[28., 108.], [7., 75.], [37., 12.], [29., 48.], [47., 62.], [65., 86.], [47., 108.]], Color32::from_rgb(0xf3, 0x91, 0x35), 0.2),
        (&[[48., 108.], [35., 76.], [68., 16.], [62., 51.], [79., 68.], [98., 91.], [77., 108.]], Color32::from_rgb(0xed, 0x68, 0x30), 0.4),
        (&[[77., 108.], [66., 86.], [91., 36.], [85., 74.], [96., 87.], [103., 101.], [89., 108.]], Color32::from_rgb(0xef, 0xa1, 0x40), 0.6),
    ];
    let side = area.width().min(area.height());
    let left = area.center().x - side / 2.0;
    let scale = side / 100.0;
    for (pts, color, delay) in tongues {
        let k = ((t + delay) * std::f32::consts::TAU / 1.7).sin();
        let sy = 0.95 + 0.08 * k;
        let base = area.bottom();
        let p = |i: usize| Pos2::new(left + pts[i][0] * scale, base - (110.0 - pts[i][1]) * scale * sy);
        for seg in 0..3 {
            let (a, c, b) = (p(seg * 2), p(seg * 2 + 1), p(seg * 2 + 2));
            painter.add(Shape::QuadraticBezier(QuadraticBezierShape::from_points_stroke([a, c, b], true, color, Stroke::NONE)));
        }
        painter.add(Shape::convex_polygon(vec![p(0), p(2), p(4), p(6)], color, Stroke::NONE));
    }
}
