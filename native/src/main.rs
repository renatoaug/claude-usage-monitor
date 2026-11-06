// Spike: the classic pet in a native, transparent, always-on-top window.
// Goal: prove the widget can idle near 0% CPU. Mood via CLAUDDY_STATE
// (idle | working | fire | sleeping) or the keys 1-4.

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Vec2, epaint::QuadraticBezierShape};
use std::time::{Duration, Instant};

mod usage;

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

// pixel art doesn't need 60 fps: the motion is a few device pixels at most
const ANIM_FPS: f32 = 12.0;
const BLINK_PERIOD: f32 = 5.5;

#[derive(Clone, Copy, PartialEq)]
enum Mood {
    Idle,
    Working,
    Fire,
    Sleeping,
}

impl Mood {
    fn from_env() -> Self {
        match std::env::var("CLAUDDY_STATE").as_deref() {
            Ok("working") => Mood::Working,
            Ok("fire") => Mood::Fire,
            Ok("sleeping") => Mood::Sleeping,
            _ => Mood::Idle,
        }
    }
    // Idle stays still between blinks: no frame is drawn unless something moves.
    fn animates(self) -> bool {
        matches!(self, Mood::Working | Mood::Fire | Mood::Sleeping)
    }
}

struct Pet {
    mood: Mood,
    start: Instant,
    shot_sent: bool,
}

impl Pet {
    fn t(&self) -> f32 {
        self.start.elapsed().as_secs_f32()
    }

    // When the next visible change happens, so idle sleeps until the blink.
    fn next_frame(&self) -> Duration {
        if self.mood.animates() {
            return Duration::from_secs_f32(1.0 / ANIM_FPS);
        }
        let phase = self.t() % BLINK_PERIOD / BLINK_PERIOD;
        let (blink_from, blink_to) = (0.92, 1.0);
        if phase < blink_from {
            Duration::from_secs_f32((blink_from - phase) * BLINK_PERIOD)
        } else {
            let _ = blink_to;
            Duration::from_secs_f32(1.0 / ANIM_FPS)
        }
    }

    fn draw(&self, painter: &egui::Painter, area: Rect) {
        let t = self.t();
        let cell = (area.width() / 12.0).floor();
        let size = Vec2::new(cell * 10.0, cell * 9.0);
        let mut origin = Pos2::new(area.center().x - size.x / 2.0, area.bottom() - size.y - cell);

        // mood offsets, snapped to whole pixels like the sprite itself
        let wave = |period: f32| (t * std::f32::consts::TAU / period).sin();
        match self.mood {
            Mood::Working => origin.y -= (wave(0.6).abs() * cell * 0.8).round(),
            Mood::Fire => origin.x += (wave(0.3) * cell * 0.1).round(),
            Mood::Sleeping => origin.y -= ((wave(5.6) + 1.0) * cell * 0.15).round(),
            Mood::Idle => {}
        }

        if self.mood == Mood::Fire {
            draw_flames(painter, area, t);
        }

        let phase = t % BLINK_PERIOD / BLINK_PERIOD;
        let eye_h = if self.mood == Mood::Sleeping {
            0.15
        } else if (0.92..0.98).contains(&phase) {
            0.1
        } else {
            1.0
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
                    let eye = Rect::from_min_size(min + Vec2::new(0.0, (cell - h) / 2.0), Vec2::new(cell, h));
                    painter.rect_filled(eye, 0.0, EYE);
                }
            }
        }
    }
}

// The four flame tongues from index.html (viewBox 0 0 100 110), swaying.
fn draw_flames(painter: &egui::Painter, area: Rect, t: f32) {
    let tongues: [(&[[f32; 2]], Color32, f32); 4] = [
        (&[[12., 106.], [-5., 84.], [13., 35.], [9., 70.], [26., 80.], [40., 102.], [12., 106.]], Color32::from_rgb(0xed, 0x64, 0x30), 0.0),
        (&[[28., 108.], [7., 75.], [37., 12.], [29., 48.], [47., 62.], [65., 86.], [47., 108.]], Color32::from_rgb(0xf3, 0x91, 0x35), 0.2),
        (&[[48., 108.], [35., 76.], [68., 16.], [62., 51.], [79., 68.], [98., 91.], [77., 108.]], Color32::from_rgb(0xed, 0x68, 0x30), 0.4),
        (&[[77., 108.], [66., 86.], [91., 36.], [85., 74.], [96., 87.], [103., 101.], [89., 108.]], Color32::from_rgb(0xef, 0xa1, 0x40), 0.6),
    ];
    let scale = area.width() / 100.0;
    for (pts, color, delay) in tongues {
        let k = ((t + delay) * std::f32::consts::TAU / 1.7).sin();
        let sy = 0.95 + 0.08 * k;
        let base = area.bottom();
        let p = |i: usize| Pos2::new(area.left() + pts[i][0] * scale, base - (110.0 - pts[i][1]) * scale * sy);
        // three quadratic segments: start → (c1) → p2 → (c2) → p4 → (c3) → p6
        for seg in 0..3 {
            let a = p(seg * 2);
            let c = p(seg * 2 + 1);
            let b = p(seg * 2 + 2);
            painter.add(Shape::QuadraticBezier(QuadraticBezierShape::from_points_stroke(
                [a, c, b],
                true,
                color,
                Stroke::NONE,
            )));
        }
        // fill the tongue's core between the three anchor points
        painter.add(Shape::convex_polygon(vec![p(0), p(2), p(4), p(6)], color, Stroke::NONE));
    }
}

impl eframe::App for Pet {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ctx.input(|i| {
            for (key, mood) in [
                (egui::Key::Num1, Mood::Idle),
                (egui::Key::Num2, Mood::Working),
                (egui::Key::Num3, Mood::Fire),
                (egui::Key::Num4, Mood::Sleeping),
            ] {
                if i.key_pressed(key) {
                    self.mood = mood;
                }
            }
        });

        // CLAUDDY_SHOT=path.png: save one frame after a second, then quit (dev aid)
        if let Ok(path) = std::env::var("CLAUDDY_SHOT") {
            if self.t() > 1.0 && !self.shot_sent {
                self.shot_sent = true;
                let cb = egui::ScreenshotCallback::new(move |img| {
                    let file = std::fs::File::create(&path).expect("shot file");
                    let mut enc = png::Encoder::new(file, img.size[0] as u32, img.size[1] as u32);
                    enc.set_color(png::ColorType::Rgba);
                    enc.set_depth(png::BitDepth::Eight);
                    let bytes: Vec<u8> = img.pixels.iter().flat_map(|p| p.to_array()).collect();
                    enc.write_header().unwrap().write_image_data(&bytes).unwrap();
                    std::process::exit(0);
                });
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(cb));
            }
            ctx.request_repaint_after(Duration::from_millis(200));
        }

        let area = ui.max_rect();
        let resp = ui.interact(area, ui.scope_id().with("pet"), egui::Sense::click_and_drag());
        if resp.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        self.draw(ui.painter(), area);

        ctx.request_repaint_after(self.next_frame());
    }
}

fn main() -> eframe::Result {
    // `clauddy --usage`: print one reading of the local logs as JSON (dev aid)
    if std::env::args().any(|a| a == "--usage") {
        let path = std::env::home_dir().unwrap_or_default().join(".claude-usage-monitor/config.json");
        let cfg: usage::Config =
            std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let mut reader = usage::Reader::new(None);
        let t = Instant::now();
        let u = reader.get_usage(&cfg);
        let cold = t.elapsed();
        let t = Instant::now();
        reader.get_usage(&cfg);
        eprintln!("cold {cold:?}, warm {:?}", t.elapsed());
        println!("{}", serde_json::to_string_pretty(&u).unwrap_or_default());
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Clauddy")
            .with_inner_size([140.0, 140.0])
            .with_transparent(true)
            .with_decorations(false)
            .with_has_shadow(false)
            .with_always_on_top()
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "Clauddy",
        options,
        Box::new(|_cc| Ok(Box::new(Pet { mood: Mood::from_env(), start: Instant::now(), shot_sent: false }))),
    )
}
