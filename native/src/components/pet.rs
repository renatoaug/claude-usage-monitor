// The pet: Clauddy in its moods, from the sprite sheets. While working it
// sits at its desk; otherwise it stands alone, idle, asleep, on fire or
// maxed out. It only asks for a repaint when its next frame is due, so a
// resting pet costs nothing between frames.

use eframe::egui::{self, Painter, Pos2, Rect, Vec2};
use std::time::{Duration, Instant};

use super::sprite::{self, Seq, Sheet};

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

    fn seq(self) -> Seq {
        match self {
            Mood::Working => sprite::WORKING,
            Mood::Idle => sprite::IDLE,
            Mood::Sleeping => sprite::SLEEPING,
            Mood::Fire => sprite::FIRE,
            Mood::Tired => sprite::TIRED,
        }
    }
}

pub struct Pet {
    start: Instant,
    working: Sheet,
    moods: Sheet,
    /// a fixed moment, for snapshots
    frozen: Option<f32>,
}

impl Pet {
    pub fn new(ctx: &egui::Context) -> Self {
        Pet {
            start: Instant::now(),
            working: Sheet::load(ctx, "working-sheet", sprite::WORKING_SHEET, 4, 2),
            moods: Sheet::load(ctx, "moods-sheet", sprite::MOODS_SHEET, 4, 4),
            frozen: None,
        }
    }

    /// A pet held at `t` seconds into its loops, so snapshots are stable.
    #[cfg(test)]
    pub fn frozen(ctx: &egui::Context, t: f32) -> Self {
        Pet { frozen: Some(t), ..Pet::new(ctx) }
    }

    fn t(&self) -> f32 {
        self.frozen.unwrap_or_else(|| self.start.elapsed().as_secs_f32())
    }

    /// How long until the next visible change.
    pub fn next_frame(&self, mood: Mood) -> Duration {
        Duration::from_secs_f32(sprite::at(mood.seq(), self.t()).1.max(1.0 / 60.0))
    }

    /// Draw the pet in `area`. The desk scene fills the area's width and sits
    /// on its bottom; the standing pet is sized so its body spans about 3/4
    /// of the area's width, feet on the bottom edge less a tenth of its height.
    pub fn draw(&self, painter: &Painter, area: Rect, mood: Mood) {
        let frame = sprite::at(mood.seq(), self.t()).0;
        if mood == Mood::Working {
            // the art fills the lower ~80% of each frame: let the frame
            // overflow the top so the desk, not empty sky, sets the size
            let side = area.width().min(area.height() * 1.2);
            let rect = Rect::from_min_size(Pos2::new(area.center().x - side / 2.0, area.bottom() - side * 0.92), Vec2::splat(side));
            self.working.draw(painter, rect, frame);
        } else {
            let side = area.width() * 1.12;
            let feet = area.bottom() - area.height() * 0.1;
            let rect = Rect::from_min_size(Pos2::new(area.center().x - side / 2.0, feet - side * 0.79), Vec2::splat(side));
            self.moods.draw(painter, rect, frame);
        }
    }
}
