// The pet: Clauddy in its moods, from the sprite sheets. While working it
// sits at its desk; otherwise it stands alone, idle, asleep, on fire or
// maxed out. It only asks for a repaint when its next frame is due, so a
// resting pet costs nothing between frames.

use eframe::egui::{self, Painter, Pos2, Rect, Vec2};
use std::time::{Duration, Instant};

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use super::skins::{self, Box4, Skin};
use super::sprite::{self, Seq, Sheet};

fn rect_of(b: Box4) -> Rect {
    Rect::from_min_max(Pos2::new(b[0], b[1]), Pos2::new(b[2], b[3]))
}

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

/// The pet out on the screen, at play.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Pose {
    /// standing (or sitting on a window) in a mood; never at its desk
    Stand(Mood),
    Crouch,
    Jump,
    Fall,
    Dizzy,
}

// a pet's sheets, decoded the first time it shows
struct Sheets {
    working: Sheet,
    moods: Sheet,
    play: Option<Sheet>,
}

pub struct Pet {
    ctx: egui::Context,
    start: Instant,
    sheets: RefCell<HashMap<&'static str, std::rc::Rc<Sheets>>>,
    /// the pet on screen
    skin: Cell<&'static Skin>,
    /// a fixed moment, for snapshots and the showcase's frames
    frozen: Cell<Option<f32>>,
    /// the hat it wears (an index into `skins::HATS`), and their sheet
    hat: Cell<Option<usize>>,
    hats: RefCell<Option<std::rc::Rc<Sheet>>>,
    acts: RefCell<HashMap<&'static str, std::rc::Rc<Sheet>>>,
    gifts: RefCell<Option<std::rc::Rc<Sheet>>>,
}

// Where the head is in each frame, as fractions of the frame: its centre,
// its top, and its size against the standing pet's (measured from the
// classic sheets, so a hat sits on it in every mood and pose)
#[derive(Clone, Copy)]
enum Art {
    Working,
    Moods,
    Play,
}

const MOODS_TOP: [f32; 16] = [78.0, 84.0, 84.0, 78.0, 80.0, 85.0, 78.0, 85.0, 84.0, 84.0, 84.0, 84.0, 81.0, 82.0, 83.0, 82.0];
const PLAY_HEAD: [(f32, f32); 4] = [(128.5, 97.0), (127.0, 85.0), (120.5, 79.0), (128.5, 85.0)];

fn head(art: Art, frame: usize) -> (f32, f32, f32) {
    let (x, y, k) = match art {
        Art::Moods => (130.0, MOODS_TOP[frame % 16], 1.0),
        Art::Working => (112.0, 77.0, 0.84),
        Art::Play => (PLAY_HEAD[frame % 4].0, PLAY_HEAD[frame % 4].1, 0.9),
    };
    (x / 256.0, y / 256.0, k)
}

// in the hats sheet, each brim's middle sits at this height of its frame,
// and a hat sinks this far onto the head (fractions of a frame)
const BRIM: f32 = 199.0 / 256.0;
const SINK: f32 = 9.0 / 256.0;
// each hat's top in its frame (of 256), in the sheet's order
const HAT_TOP: [f32; 8] = [48.0, 6.0, 91.0, 66.0, 100.0, 74.0, 48.0, 69.0];

/// How high `hat` reaches on `frame` of `art`, as a fraction of the frame.
fn hat_reach(art: Art, frame: usize, hat: usize) -> f32 {
    let (_, y, k) = head(art, frame);
    y + SINK - (BRIM - HAT_TOP[hat % 8] / 256.0) * k
}

impl Pet {
    pub fn new(ctx: &egui::Context) -> Self {
        Pet {
            ctx: ctx.clone(),
            start: Instant::now(),
            sheets: RefCell::default(),
            skin: Cell::new(&skins::SKINS[0]),
            frozen: Cell::new(None),
            hat: Cell::new(None),
            hats: RefCell::default(),
            acts: RefCell::default(),
            gifts: RefCell::default(),
        }
    }

    /// The pet on screen, by id.
    pub fn skin_id(&self) -> &'static str {
        self.skin.get().id
    }

    /// Draw the pet's own act (its strip's `frame`): its feet (or wheels) at
    /// `feet`, its art `width` wide, stretched by `squash`, facing left when
    /// `flip`. A pet without an act strip stands in its idle pose.
    pub fn draw_act(&self, painter: &Painter, feet: Pos2, width: f32, frame: usize, squash: Vec2, flip: bool, angle: f32) {
        let skin = self.skin.get();
        let Some(sheet) = self.act_sheet(skin) else {
            let _ = self.draw_pose(painter, feet, width, Pose::Stand(Mood::Idle), squash, flip);
            return;
        };
        let b = rect_of(skin.act_box);
        // a frame may be wider than tall (the sleigh and its reindeer)
        let side = width / b.width();
        let (sx, sy) = (side * squash.x, side / sheet.aspect() * squash.y);
        let cx = if flip { 1.0 - b.center().x } else { b.center().x };
        let min = Pos2::new(feet.x - cx * sx, feet.y - b.max.y * sy);
        let rect = Rect::from_min_size(min, Vec2::new(sx, sy));
        if angle == 0.0 {
            sheet.draw_facing(painter, rect, frame, flip);
        } else {
            // turned round the middle of the art
            let pivot = Pos2::new(feet.x, min.y + b.center().y * sy);
            sheet.draw_turned(painter, rect, frame, flip, angle, pivot);
        }
    }

    /// One frame of the act strip on its own (the pumpkin's bats and ghost):
    /// centred on `center`, a frame `width` wide, faded by `fade`.
    pub fn draw_act_frame(&self, painter: &Painter, center: Pos2, width: f32, frame: usize, flip: bool, fade: f32) {
        let Some(sheet) = self.act_sheet(self.skin.get()) else { return };
        let rect = Rect::from_center_size(center, Vec2::new(width, width / sheet.aspect()));
        sheet.draw_turned_faded(painter, rect, frame, flip, fade);
    }

    fn act_sheet(&self, skin: &'static Skin) -> Option<std::rc::Rc<Sheet>> {
        let png = skin.act?;
        Some(self.acts.borrow_mut().entry(skin.id).or_insert_with(|| std::rc::Rc::new(Sheet::load(&self.ctx, &format!("{}-act", skin.id), png, 4, 1))).clone())
    }

    /// A present from Santa's sack (`look`: which box), its bottom at
    /// `feet`, `width` wide, faded by `fade`.
    pub fn draw_gift(&self, painter: &Painter, feet: Pos2, width: f32, look: usize, fade: f32) {
        let sheet = self.gifts.borrow_mut().get_or_insert_with(|| std::rc::Rc::new(Sheet::load(&self.ctx, "gifts", skins::GIFTS_PNG, 4, 1))).clone();
        let b = rect_of(skins::GIFT_BOX);
        let side = width / b.width();
        let min = Pos2::new(feet.x - b.center().x * side, feet.y - b.max.y * side);
        sheet.draw_faded(painter, Rect::from_min_size(min, Vec2::splat(side)), look % 4, fade);
    }

    /// Wear this hat from now on ("" or an unknown id: none).
    pub fn set_hat(&self, id: &str) {
        self.hat.set(skins::HATS.iter().position(|h| h.0 == id));
    }

    /// A hat on its own, filling `rect` (the hat picker's tiles).
    pub fn draw_hat_icon(&self, p: &Painter, rect: Rect, hat: usize) {
        let sheet = self.hats.borrow_mut().get_or_insert_with(|| std::rc::Rc::new(Sheet::load(&self.ctx, "hats", skins::HATS_PNG, 4, 2))).clone();
        // the hats fill the lower part of their frames: lift the frame so
        // the hat, not the empty top, sits in the middle
        let side = rect.width().min(rect.height()) * 1.3;
        let cell = Rect::from_center_size(rect.center() - Vec2::new(0.0, side * 0.12), Vec2::splat(side));
        sheet.draw(p, cell, hat);
    }

    /// Draw `hat` on the head of `frame` of `art`, drawn filling `cell`.
    fn draw_hat(&self, p: &Painter, skin: &Skin, cell: Rect, art: Art, frame: usize, flip: bool, hat: Option<usize>) {
        let Some(hat) = hat.filter(|_| skin.id == skins::HAT_WEARER) else { return };
        let sheet = self.hats.borrow_mut().get_or_insert_with(|| std::rc::Rc::new(Sheet::load(&self.ctx, "hats", skins::HATS_PNG, 4, 2))).clone();
        let (x, y, k) = head(art, frame);
        let x = if flip { 1.0 - x } else { x };
        let (w, h) = (cell.width() * k, cell.height() * k);
        let brim = Pos2::new(cell.left() + x * cell.width(), cell.top() + (y + SINK) * cell.height());
        let rect = Rect::from_min_size(Pos2::new(brim.x - w / 2.0, brim.y - BRIM * h), Vec2::new(w, h));
        sheet.draw_facing(p, rect, hat, flip);
    }

    /// Show this pet from now on (the saved choice, or the season's).
    pub fn set_skin(&self, skin: &'static Skin) {
        self.skin.set(skin);
    }

    fn sheets(&self, skin: &'static Skin) -> std::rc::Rc<Sheets> {
        self.sheets
            .borrow_mut()
            .entry(skin.id)
            .or_insert_with(|| {
                std::rc::Rc::new(Sheets {
                    working: Sheet::load(&self.ctx, &format!("{}-working", skin.id), skin.working, 4, 2),
                    moods: Sheet::load(&self.ctx, &format!("{}-moods", skin.id), skin.moods, 4, 4),
                    play: skin.play.map(|png| Sheet::load(&self.ctx, &format!("{}-play", skin.id), png, 4, 1)),
                })
            })
            .clone()
    }

    /// A pet held at `t` seconds into its loops, so snapshots are stable.
    #[cfg(test)]
    pub fn frozen(ctx: &egui::Context, t: f32) -> Self {
        Pet { frozen: Cell::new(Some(t)), ..Pet::new(ctx) }
    }

    /// Move a frozen pet to `t` seconds (the showcase, frame by frame).
    #[cfg(test)]
    pub fn set_time(&self, t: f32) {
        self.frozen.set(Some(t));
    }

    fn t(&self) -> f32 {
        self.frozen.get().unwrap_or_else(|| self.start.elapsed().as_secs_f32())
    }

    /// How long until the next visible change.
    pub fn next_frame(&self, mood: Mood) -> Duration {
        Duration::from_secs_f32(sprite::at(mood.seq(), self.t()).1.max(1.0 / 60.0))
    }

    /// Draw the pet as large as fits in `space`, centred on its body: the
    /// desk scene by its whole art, a standing mood by its body alone, so
    /// switching moods doesn't move it (Zz may spill past `space`; the flames
    /// are fitted whole, so they never cover the title band).
    /// Returns where the body landed, for things that sit on or above it.
    pub fn draw_fit(&self, painter: &Painter, space: Rect, mood: Mood) -> Rect {
        self.draw_skin_fit(painter, space, mood, self.skin.get())
    }

    /// Draw the pet at one size whoever it is (just the pet): a frame `cell`
    /// points wide, its feet (or its desk's) at `feet`. Every pet's art is
    /// drawn at the classic sprite's scale, so the same `cell` gives every
    /// pet the same body, its hat or sack spilling past it as it may (at its
    /// desk, in the same frame, the pet sits a little smaller).
    /// Returns where the art landed (with a hat, up to its top).
    pub fn draw_at(&self, painter: &Painter, feet: Pos2, cell: f32, mood: Mood) -> Rect {
        let (frame_rect, body, kind, frame) = self.place(feet, cell, mood);
        let skin = self.skin.get();
        let sheets = self.sheets(skin);
        let sheet = if mood == Mood::Working { &sheets.working } else { &sheets.moods };
        sheet.draw(painter, frame_rect, frame);
        self.draw_hat(painter, skin, frame_rect, kind, frame, false, self.hat.get().filter(|_| skin.id == skins::HAT_WEARER));
        body
    }

    /// Where `draw_at` would put the art, drawing nothing.
    pub fn body_at(&self, feet: Pos2, cell: f32, mood: Mood) -> Rect {
        self.place(feet, cell, mood).1
    }

    // `draw_at`'s frame, the art in it, which sheet and which frame
    fn place(&self, feet: Pos2, cell: f32, mood: Mood) -> (Rect, Rect, Art, usize) {
        let skin = self.skin.get();
        let frame = sprite::at(mood.seq(), self.t()).0;
        let (kind, side, bottom, art) = match mood {
            Mood::Working => (Art::Working, cell, skin.work_box[3], skin.work_box),
            Mood::Fire => (Art::Moods, cell, skin.stand_box[3], skin.fire_box),
            _ => (Art::Moods, cell, skin.stand_box[3], skin.stand_box),
        };
        let mut art = rect_of(art);
        if let Some(h) = self.hat.get().filter(|_| skin.id == skins::HAT_WEARER) {
            art.min.y = art.min.y.min(hat_reach(kind, 0, h));
        }
        let min = Pos2::new(feet.x - side / 2.0, feet.y - bottom * side);
        let frame_rect = Rect::from_min_size(min, Vec2::splat(side));
        (frame_rect, Rect::from_min_max(min + art.min.to_vec2() * side, min + art.max.to_vec2() * side), kind, frame)
    }

    /// `draw_fit` for a given pet (the Settings picker's thumbnails).
    pub fn draw_skin_fit(&self, painter: &Painter, space: Rect, mood: Mood, skin: &'static Skin) -> Rect {
        let frame = sprite::at(mood.seq(), self.t()).0;
        let sheets = self.sheets(skin);
        // where the art sits inside a frame, as fractions (measured per pet)
        let (sheet, art) = match mood {
            Mood::Working => (&sheets.working, rect_of(skin.work_box)),
            // the flames rise well above the body: fit them in, not just the body
            Mood::Fire => (&sheets.moods, rect_of(skin.fire_box)),
            _ => (&sheets.moods, rect_of(skin.stand_box)),
        };
        // a hat is part of it: fitted in, and what sits above clears it
        let kind = if mood == Mood::Working { Art::Working } else { Art::Moods };
        let hat = self.hat.get().filter(|_| skin.id == skins::HAT_WEARER);
        let mut art = art;
        if let Some(h) = hat {
            art.min.y = art.min.y.min(hat_reach(kind, 0, h));
        }
        let side = (space.width() / art.width()).min(space.height() / art.height());
        let min = space.center() - art.center().to_vec2() * side;
        let cell = Rect::from_min_size(min, Vec2::splat(side));
        sheet.draw(painter, cell, frame);
        self.draw_hat(painter, skin, cell, kind, frame, false, hat);
        Rect::from_min_max(min + art.min.to_vec2() * side, min + art.max.to_vec2() * side)
    }

    /// Draw the pet at play: its feet at `feet`, its body `width` wide,
    /// stretched by `squash` (a crouch is wide and low), facing left when
    /// `flip`. Returns where the body landed.
    pub fn draw_pose(&self, painter: &Painter, feet: Pos2, width: f32, pose: Pose, squash: Vec2, flip: bool) -> Rect {
        let skin = self.skin.get();
        let sheets = self.sheets(skin);
        let (sheet, frame, art) = match (pose, &sheets.play) {
            (Pose::Stand(mood), _) => {
                let mood = if mood == Mood::Working { Mood::Idle } else { mood };
                (&sheets.moods, sprite::at(mood.seq(), self.t()).0, Art::Moods)
            }
            (Pose::Crouch, Some(play)) => (play, 0, Art::Play),
            (Pose::Jump, Some(play)) => (play, 1, Art::Play),
            (Pose::Fall, Some(play)) => (play, 2, Art::Play),
            (Pose::Dizzy, Some(play)) => (play, 3, Art::Play),
            // no strip: its idle face up, its tired one coming down
            (Pose::Crouch | Pose::Jump, None) => (&sheets.moods, 0, Art::Moods),
            (Pose::Fall | Pose::Dizzy, None) => (&sheets.moods, 12, Art::Moods),
        };
        let b = rect_of(skin.stand_box);
        let side = width / b.width();
        let (sx, sy) = (side * squash.x, side * squash.y);
        let cx = if flip { 1.0 - b.center().x } else { b.center().x };
        let min = Pos2::new(feet.x - cx * sx, feet.y - b.max.y * sy);
        let cell = Rect::from_min_size(min, Vec2::new(sx, sy));
        sheet.draw_facing(painter, cell, frame, flip);
        self.draw_hat(painter, skin, cell, art, frame, flip, self.hat.get());
        let left = if flip { 1.0 - b.max.x } else { b.min.x };
        Rect::from_min_size(Pos2::new(min.x + left * sx, min.y + b.min.y * sy), Vec2::new(b.width() * sx, b.height() * sy))
    }

    /// Draw the pet in `area`. The desk scene fills the area's width and sits
    /// on its bottom; the standing pet is sized so its body spans about 3/4
    /// of the area's width, feet on the bottom edge less a tenth of its height.
    pub fn draw(&self, painter: &Painter, area: Rect, mood: Mood) {
        let frame = sprite::at(mood.seq(), self.t()).0;
        let sheets = self.sheets(self.skin.get());
        if mood == Mood::Working {
            // the art fills the lower ~80% of each frame: let the frame
            // overflow the top so the desk, not empty sky, sets the size
            let side = area.width().min(area.height() * 1.2);
            let rect = Rect::from_min_size(Pos2::new(area.center().x - side / 2.0, area.bottom() - side * 0.92), Vec2::splat(side));
            sheets.working.draw(painter, rect, frame);
            self.draw_hat(painter, self.skin.get(), rect, Art::Working, frame, false, self.hat.get());
        } else {
            let side = area.width() * 1.12;
            let feet = area.bottom() - area.height() * 0.1;
            let rect = Rect::from_min_size(Pos2::new(area.center().x - side / 2.0, feet - side * self.skin.get().stand_box[3]), Vec2::splat(side));
            sheets.moods.draw(painter, rect, frame);
            self.draw_hat(painter, self.skin.get(), rect, Art::Moods, frame, false, self.hat.get());
        }
    }
}
