// The pet at play on the screen, while it's just the pet. Each pet has a
// number of its own: Clauddy hops up onto the window in front, the Getaway
// drives along the bottom of the screen and waves on its way back, Santa
// flies his sleigh across the top dropping presents (one over your cursor),
// the Gingerbread dares you to catch it and runs from the cursor, the
// Reindeer snows. Any pet can be thrown, and lands wherever it falls.
//
// Everything here lives in screen points and is stepped by the app with the
// time that passed; it never reads the clock or the OS itself, so tests can
// drive it. The app draws it on a see-through window the size of the screen
// that clicks go through, up only while something is happening.

use eframe::egui::{Painter, Pos2, Rect, Vec2, pos2, vec2};

use crate::components::pet::{Mood, Pet, Pose};
use crate::platform::desk::Ledge;
use crate::ui::theme::color;

// points per second squared
const GRAVITY: f32 = 2200.0;
// the longest step taken in one hop, and how long a hop takes
const STRIDE: f32 = 150.0;
const CROUCH_S: f64 = 0.12;
const LAND_S: f64 = 0.10;
// how long it sits on a window before heading home
const SIT_S: f64 = 12.0;
// a landing this fast bounces
const BOUNCE_VY: f32 = 650.0;
const DIZZY_S: f64 = 1.8;
// snow: how long a fall lasts and how high it piles on an edge
const SNOW_S: f64 = 45.0;
const SNOW_RAMP_S: f64 = 6.0;
const PILE_MAX: f32 = 9.0;
const PILE_COL: f32 = 4.0;
const MELT_PER_S: f32 = 0.35;
// the drive: its top speed and how fast it gets there (points per second),
// how long it waves, and how often the exhaust puffs
const DRIVE_SPEED: f32 = 300.0;
const DRIVE_ACCEL: f32 = 450.0;
const WAVE_S: f64 = 1.6;
const PUFF_S: f64 = 0.09;
// the sleigh's speed, and how long a present rests where it lands
const SLEIGH_SPEED: f32 = 240.0;
const GIFT_REST: f32 = 7.0;
// the dare: how long it lasts, how fast it runs, how near the cursor scares it
const DARE_S: f64 = 18.0;
const RUN_SPEED: f32 = 430.0;
const SCARED_X: f32 = 170.0;
const TAUNT_S: f64 = 1.3;
// the astronaut's float, and how hard it heads home after
const FLOAT_S: f64 = 13.0;
const ROCKET_SPEED: f32 = 170.0;
// how fast it can turn (radians per second)
const ROCKET_TURN: f32 = 1.4;

/// What the pet plays on: the screen, its floor (above the Dock or the
/// taskbar), and the window edges it can land on.
#[derive(Clone, Debug)]
pub struct World {
    pub screen: Rect,
    pub floor: f32,
    pub ledges: Vec<Ledge>,
    /// where the cursor is, when the OS says
    pub cursor: Option<Pos2>,
}

impl World {
    /// The surface right under `p` (an edge or the floor), its height.
    fn ground_under(&self, p: Pos2) -> f32 {
        self.ledges.iter().filter(|l| l.holds(p.x) && l.y >= p.y - 1.0).map(|l| l.y).fold(self.floor, f32::min)
    }

    /// Whether something holds `p` up.
    fn holds(&self, p: Pos2) -> bool {
        (p.y - self.floor).abs() < 2.0 || self.ledges.iter().any(|l| l.holds(p.x) && (l.y - p.y).abs() < 2.0)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Snow,
    /// the car's exhaust
    Puff,
    /// a spark (the wizard's, the reindeer's trail)
    Spark,
    /// the pumpkin's bats and its ghost, drawn from its act strip
    Bat,
    Ghost,
    /// a present from the sleigh: falls, lands, rests, goes
    Gift,
}

#[derive(Clone, Debug)]
struct Bit {
    kind: Kind,
    pos: Pos2,
    vel: Vec2,
    size: f32,
    /// 0 far away .. 1 close: close flakes are bigger, faster, brighter, and
    /// only they land on the windows (the far ones fall behind them)
    depth: f32,
    spin: f32,
    /// which one, for the bits that come in kinds (a present's box)
    look: usize,
    age: f32,
    life: f32,
}

/// Snow resting on an edge, in columns.
#[derive(Clone, Debug)]
struct Pile {
    y: f32,
    x0: f32,
    cols: Vec<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    /// about to hop to `to`
    Crouch {
        start: f64,
        to: Pos2,
    },
    Hop {
        from: Pos2,
        to: Pos2,
        start: f64,
        dur: f64,
        rise: f32,
    },
    Land {
        start: f64,
    },
    Sit {
        until: f64,
    },
    /// thrown, fallen or bounced: gravity has it
    Fly,
    Dizzy {
        until: f64,
    },
    /// driving along the floor at `speed`; the last puff of exhaust at `puff`
    Drive {
        speed: f32,
        puff: f64,
    },
    /// stopped, waving at you
    Wave {
        until: f64,
    },
    /// flying from `from` to `to` (the sleigh)
    Glide {
        from: Pos2,
        to: Pos2,
        start: f64,
        dur: f64,
    },
    /// running along the floor (the Gingerbread)
    Run {
        speed: f32,
        dir: f32,
    },
    /// facing you, teasing
    Taunt {
        until: f64,
    },
    /// beat `i` of a scripted number, from where the last one left it
    Keys {
        i: usize,
        start: f64,
        from: Pos2,
        zoom0: f32,
    },
    /// flying its rocket about, heading this way (radians)
    Float {
        heading: f32,
    },
}

// a scripted number carries its beats inline; there is only ever one roam,
// so the size costs nothing and the goal stays Copy
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum Goal {
    /// up onto `spot`, a while there, then back `home`
    Visit { spot: Pos2, home: Pos2, back: bool },
    /// thrown: stays where it ends up
    Stay,
    /// down to the floor, off the screen's right side, back in from the
    /// left, a wave at `wave_x`, and back up `home`
    Drive { home: Pos2, lap: u8, wave_x: f32, waved: bool },
    /// across the top of the screen dropping presents where `drops` say
    /// (NaN: none), then back down `home`
    Sleigh { home: Pos2, drops: [f32; 4], dropped: u8, back: bool },
    /// down to the floor, running from the cursor until `until`, then home
    Dare { home: Pos2, until: f64, teased: f64 },
    /// a scripted number: `keys[..n]` in turn, then home
    Show { home: Pos2, keys: [Key; KEYS], n: usize },
    /// floating free until `until`, then back home (the astronaut)
    Float { home: Pos2, until: f64, waved: bool, aim: Pos2 },
    /// out over the screen in an arc and back from the other side (the reindeer)
    Gallop { home: Pos2, back: bool, trail: f64 },
}

impl Goal {
    fn home(&self) -> Option<Pos2> {
        match *self {
            Goal::Visit { home, .. }
            | Goal::Drive { home, .. }
            | Goal::Sleigh { home, .. }
            | Goal::Dare { home, .. }
            | Goal::Show { home, .. }
            | Goal::Float { home, .. }
            | Goal::Gallop { home, .. } => Some(home),
            Goal::Stay => None,
        }
    }
}

/// The pet out of its window.
#[derive(Clone, Debug)]
pub struct Roam {
    /// its feet, in screen points
    at: Pos2,
    vel: Vec2,
    phase: Phase,
    goal: Goal,
    facing: f32,
    /// its body's width on screen (its car's, driving)
    width: f32,
    /// in its own vehicle: drawn from the pet's act strip
    ride: bool,
    /// turned this far (radians; the astronaut tumbles), and drawn this
    /// many times its size (Clauddy coming up to the glass)
    spin: f32,
    zoom: f32,
}

/// One beat of a scripted number: where the pet goes (its feet), how long
/// it takes, which act frame shows, how big, and what it sets off on arrival.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Key {
    at: Pos2,
    dur: f64,
    frame: usize,
    zoom: f32,
    emit: Emit,
    /// a hop's height on the way there (0: straight)
    arc: f32,
    /// hidden below this line (screen y): behind a window's top edge; NaN: none
    clip: f32,
}

impl Key {
    fn arc(self, arc: f32) -> Self {
        Key { arc, ..self }
    }
    fn behind(self, edge: f32) -> Self {
        Key { clip: edge, ..self }
    }
}

/// A key frame that shows the pet standing in its mood (not its act strip).
const STAND: usize = usize::MAX;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Emit {
    None,
    /// the wizard's sparks, at the cursor
    Sparks,
    /// bats out of the pumpkin's mouth
    Bats,
    /// a ghost that follows the cursor
    Ghost,
}

const KEYS: usize = 12;

fn key(at: Pos2, dur: f64, frame: usize, zoom: f32, emit: Emit) -> Key {
    Key { at, dur, frame, zoom, emit, arc: 0.0, clip: f32::NAN }
}

/// What a step ended.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Done {
    /// back where it left from
    Home,
    /// thrown, it came to rest here (its feet): home moves there
    Settled(Pos2),
}

pub struct Play {
    t: f64,
    rng: u64,
    bits: Vec<Bit>,
    piles: Vec<Pile>,
    /// snow falls between these (seconds of play time)
    snow: Option<(f64, f64)>,
    snow_owed: f32,
    /// where the pet is (its feet) and how wide it is: the snow falls
    /// around it, not over the whole screen
    focus: (Pos2, f32),
    roam: Option<Roam>,
}

impl Play {
    pub fn new(seed: u64) -> Self {
        Play { t: 0.0, rng: seed | 1, bits: vec![], piles: vec![], snow: None, snow_owed: 0.0, focus: (Pos2::ZERO, 60.0), roam: None }
    }

    /// Something is on screen, or about to be.
    pub fn active(&self) -> bool {
        !self.bits.is_empty() || !self.piles.is_empty() || self.snowing() || self.roam.is_some()
    }

    /// The pet is out of its window.
    pub fn pet_out(&self) -> bool {
        self.roam.is_some()
    }

    fn snowing(&self) -> bool {
        self.snow.is_some_and(|(_, end)| self.t < end)
    }

    /// Stop everything at once (the size changed, play was turned off).
    pub fn clear(&mut self) {
        self.bits.clear();
        self.piles.clear();
        self.snow = None;
        self.roam = None;
    }

    fn rand(&mut self) -> f32 {
        // xorshift: plenty for snowflakes
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / (1u64 << 24) as f32
    }

    fn between(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.rand()
    }

    // ---- what starts something ------------------------------------------

    /// Where the pet is: its feet, and its body's width. While it's out, its
    /// own place counts instead.
    pub fn focus(&mut self, feet: Pos2, width: f32) {
        self.focus = (self.roam.as_ref().map_or(feet, |r| r.at), width);
    }

    /// Snow around the pet for a while.
    pub fn snow(&mut self) {
        if !self.snowing() {
            self.snow = Some((self.t, self.t + SNOW_S));
        }
    }

    /// Hop from `home` (its feet) up onto `spot` on a window, sit a while,
    /// and come back. `width`: its body on screen.
    pub fn visit(&mut self, home: Pos2, spot: Pos2, width: f32) {
        if self.roam.is_some() {
            return;
        }
        let goal = Goal::Visit { spot, home, back: false };
        self.roam = Some(Roam {
            at: home,
            vel: Vec2::ZERO,
            phase: Phase::Land { start: self.t - LAND_S },
            goal,
            facing: (spot.x - home.x).signum(),
            width,
            ride: false,
            spin: 0.0,
            zoom: 1.0,
        });
    }

    /// Drive: down from `home` (its feet) to the floor, along it off the
    /// right side of the screen, back in from the left with a wave on the
    /// way, and back up home. `width`: the car's on screen.
    pub fn drive(&mut self, home: Pos2, width: f32, world: &World) {
        if self.roam.is_some() {
            return;
        }
        // the wave halfway back, or not at all with no room before home
        let wave_x = if home.x - world.screen.left() > 3.0 * width { (world.screen.left() + home.x) / 2.0 } else { f32::INFINITY };
        let goal = Goal::Drive { home, lap: 0, wave_x, waved: false };
        let phase = if (home.y - world.floor).abs() < 2.0 {
            Phase::Drive { speed: 0.0, puff: self.t }
        } else {
            Phase::Crouch { start: self.t, to: pos2(home.x, world.floor) }
        };
        self.roam = Some(Roam { at: home, vel: Vec2::ZERO, phase, goal, facing: 1.0, width, ride: true, spin: 0.0, zoom: 1.0 });
    }

    /// Santa's sleigh: in from the left along the top of the screen, three
    /// presents and one over the cursor dropped on the way, off the right,
    /// and back down `home`. `width`: the sleigh and its reindeer.
    pub fn sleigh(&mut self, home: Pos2, width: f32, world: &World) {
        if self.roam.is_some() {
            return;
        }
        // low enough that the antlers clear the top, even before the OS has
        // said where it put the overlay (it may sit lower than asked)
        let band = world.screen.top() + width * 0.45 + 48.0;
        let (from, to) = (pos2(world.screen.left() - width, band), pos2(world.screen.right() + width, band));
        let (l, r) = (world.screen.left() + width * 0.5, world.screen.right() - width * 0.5);
        let mut drops = [f32::NAN; 4];
        for (i, d) in drops.iter_mut().enumerate().take(3) {
            *d = l + (r - l) * (i as f32 + 0.2 + 0.6 * self.rand()) / 3.0;
        }
        drops[3] = world.cursor.map_or(f32::NAN, |c| c.x);
        let dur = ((to.x - from.x) / SLEIGH_SPEED) as f64;
        let goal = Goal::Sleigh { home, drops, dropped: 0, back: false };
        // and the snow comes with it, and stays round Santa a while after
        self.snow();
        self.roam = Some(Roam {
            at: from,
            vel: Vec2::ZERO,
            phase: Phase::Glide { from, to, start: self.t, dur },
            goal,
            facing: 1.0,
            width,
            ride: true,
            spin: 0.0,
            zoom: 1.0,
        });
    }

    /// The Gingerbread's dare: down to the floor, a tease, and a run from
    /// the cursor whenever it comes near, for a while; then home.
    pub fn dare(&mut self, home: Pos2, width: f32, world: &World) {
        if self.roam.is_some() {
            return;
        }
        let goal = Goal::Dare { home, until: self.t + DARE_S, teased: self.t };
        let phase = if (home.y - world.floor).abs() < 2.0 {
            Phase::Taunt { until: self.t + TAUNT_S }
        } else {
            Phase::Crouch { start: self.t, to: pos2(home.x, world.floor) }
        };
        self.roam = Some(Roam { at: home, vel: Vec2::ZERO, phase, goal, facing: 1.0, width, ride: true, spin: 0.0, zoom: 1.0 });
    }

    fn show(&mut self, home: Pos2, width: f32, keys: &[Key]) {
        let mut all = [key(home, 0.0, 0, 1.0, Emit::None); KEYS];
        all[..keys.len()].copy_from_slice(keys);
        let goal = Goal::Show { home, keys: all, n: keys.len() };
        let phase = Phase::Keys { i: 0, start: self.t, from: home, zoom0: 1.0 };
        self.roam = Some(Roam { at: home, vel: Vec2::ZERO, phase, goal, facing: 1.0, width, ride: true, spin: 0.0, zoom: 1.0 });
    }

    /// The Wizard: a poof, a poof up on `spot`, a wand raised and sparks
    /// at the cursor, a wave, and poof home.
    pub fn spell(&mut self, home: Pos2, spot: Pos2, width: f32) {
        if self.roam.is_some() {
            return;
        }
        let k = |at, dur, frame, emit| key(at, dur, frame, 1.0, emit);
        self.show(
            home,
            width,
            &[
                k(home, 0.45, 2, Emit::None),
                k(spot, 0.0, 2, Emit::None),
                k(spot, 0.45, 2, Emit::None),
                k(spot, 0.9, 0, Emit::None),
                k(spot, 1.3, 1, Emit::Sparks),
                k(spot, 1.4, 3, Emit::None),
                k(spot, 0.45, 2, Emit::None),
                k(home, 0.0, 2, Emit::None),
                k(home, 0.45, 2, Emit::None),
            ],
        );
    }

    /// Clauddy plays peekaboo behind a window: up onto its top edge, down
    /// behind it, a peek over the edge, gone, a peek further along with a
    /// wave, and out and home.
    pub fn peek(&mut self, home: Pos2, edge: &Ledge, width: f32) {
        if self.roam.is_some() {
            return;
        }
        let tall = width * 0.85;
        let (lo, hi) = (edge.x0 + width, edge.x1 - width);
        let (a, b) = (lo + (hi - lo) * 0.25, lo + (hi - lo) * 0.75);
        let y = edge.y;
        // hidden: all of it under the edge; peeking: the top of its head and its eyes
        let (hidden, peeking) = (y + tall * 1.15, y + tall * 0.45);
        let k = |x: f32, y: f32, dur: f64, frame: usize| key(pos2(x, y), dur, frame, 1.0, Emit::None);
        self.show(
            home,
            width,
            &[
                k(home.x, home.y, 0.15, STAND),
                k(a, y, 0.6, STAND).arc(70.0),
                k(a, hidden, 0.35, STAND).behind(y),
                k(a, peeking, 0.55, STAND).behind(y),
                k(a, peeking, 1.5, STAND).behind(y),
                k(a, hidden, 0.3, STAND).behind(y),
                k(b, hidden, 0.8, STAND).behind(y),
                k(b, peeking, 0.45, 3).behind(y),
                k(b, peeking, 1.6, 3).behind(y),
                k(b, y, 0.4, STAND).arc(40.0),
                k(home.x, home.y, 0.7, STAND).arc(80.0),
            ],
        );
    }

    /// The Pumpkin cackles, bats fly out of its mouth, and a ghost follows
    /// the cursor a while.
    pub fn haunt(&mut self, home: Pos2, width: f32) {
        if self.roam.is_some() {
            return;
        }
        // a beat sets off its emit as it begins: the first is only a start
        self.show(home, width, &[key(home, 0.05, 0, 1.0, Emit::None), key(home, 2.6, 0, 1.0, Emit::Bats), key(home, 0.3, 0, 1.0, Emit::Ghost)]);
    }

    /// The Astronaut lifts off and floats free, turning slowly and bouncing
    /// off the screen's sides, then flies back home.
    pub fn float(&mut self, home: Pos2, width: f32) {
        if self.roam.is_some() {
            return;
        }
        // off up and to one side, at a gentle climb
        let side = if self.rand() < 0.5 { std::f32::consts::PI } else { 0.0 };
        let heading = side + if side == 0.0 { -0.35 } else { 0.35 };
        let goal = Goal::Float { home, until: self.t + FLOAT_S, waved: false, aim: home };
        self.roam = Some(Roam {
            at: home,
            vel: Vec2::angled(heading) * 120.0,
            phase: Phase::Float { heading },
            goal,
            facing: 1.0,
            width,
            ride: true,
            spin: 0.0,
            zoom: 1.0,
        });
    }

    /// The Reindeer takes off in an arc off the right side of the screen,
    /// comes back from the left high up, and lands home, leaving sparkles.
    pub fn gallop(&mut self, home: Pos2, width: f32, world: &World) {
        if self.roam.is_some() {
            return;
        }
        let band = world.screen.top() + width * 0.6 + 48.0;
        let to = pos2(world.screen.right() + width, band + (home.y - band) * 0.3);
        let dur = (((to - home).length() / SLEIGH_SPEED) * 0.9) as f64;
        let goal = Goal::Gallop { home, back: false, trail: self.t };
        self.roam = Some(Roam {
            at: home,
            vel: Vec2::ZERO,
            phase: Phase::Glide { from: home, to, start: self.t, dur },
            goal,
            facing: 1.0,
            width,
            ride: true,
            spin: 0.0,
            zoom: 1.0,
        });
    }

    /// Thrown from `at` (its feet) at `vel`.
    pub fn throw(&mut self, at: Pos2, vel: Vec2, width: f32) {
        self.roam = Some(Roam { at, vel, phase: Phase::Fly, goal: Goal::Stay, facing: vel.x.signum(), width, ride: false, spin: 0.0, zoom: 1.0 });
    }

    // ---- the step -------------------------------------------------------

    /// Move everything on by `dt` seconds. Returns what ended, if anything.
    pub fn step(&mut self, dt: f32, world: &World) -> Option<Done> {
        let dt = dt.clamp(0.0, 0.05);
        self.t += dt as f64;
        self.fall_snow(dt, world);
        self.move_bits(dt, world);
        self.settle_piles(dt, world);
        self.move_pet(dt, world)
    }

    fn fall_snow(&mut self, dt: f32, world: &World) {
        let Some((start, end)) = self.snow else { return };
        if self.t >= end {
            self.snow = None;
            return;
        }
        // eases in and out: a few flakes, the fall, a few again
        let ramp = ((self.t - start).min(end - self.t) / SNOW_RAMP_S).clamp(0.0, 1.0) as f32;
        self.snow_owed += dt * 24.0 * ramp;
        let (feet, w) = self.focus;
        // a patch of sky over the pet: a few bodies wide, a few tall
        let spread = (w * 4.5).clamp(200.0, 380.0);
        let top = (feet.y - (w * 5.0).clamp(240.0, 360.0)).max(world.screen.top() - 8.0);
        while self.snow_owed >= 1.0 {
            self.snow_owed -= 1.0;
            let depth = self.rand();
            // thicker in the middle, thinning out to the sides
            let x = feet.x + (self.rand() + self.rand() - 1.0) * spread / 2.0;
            let (y, spin) = (top + self.between(-20.0, 20.0), self.between(0.0, 6.3));
            let size = (2.0 + depth * 4.0).round();
            let vy = 35.0 + depth * 70.0 + self.between(0.0, 15.0);
            // the far flakes fade out on the ground behind the pet
            let life = ((feet.y + 4.0 - y) / vy).max(1.0);
            self.bits.push(Bit { kind: Kind::Snow, look: 0, pos: pos2(x, y), vel: vec2(0.0, vy), size, depth, spin, age: 0.0, life });
        }
    }

    fn move_bits(&mut self, dt: f32, world: &World) {
        let t = self.t as f32;
        let mut landed: Vec<(f32, f32, f32, f32)> = vec![];
        self.bits.retain_mut(|b| {
            b.age += dt;
            let was = b.pos;
            match b.kind {
                Kind::Snow => {
                    // a sway, wider for the close flakes
                    b.pos.x += ((t * 1.1 + b.spin).sin() * 14.0 * (0.4 + b.depth) + 6.0) * dt;
                    b.pos.y += b.vel.y * dt;
                }
                Kind::Puff => {
                    b.vel *= 1.0 - 2.5 * dt;
                    b.pos += b.vel * dt;
                    b.size += 10.0 * dt;
                }
                Kind::Spark => {
                    if b.age >= 0.0 {
                        b.vel *= 1.0 - 2.2 * dt;
                        b.vel.y += 120.0 * dt;
                        b.pos += b.vel * dt;
                    }
                }
                Kind::Bat => {
                    if b.age >= 0.0 {
                        // a fluttering flight that climbs away
                        b.pos += b.vel * dt;
                        b.pos.y += (b.age * 9.0 + b.spin).sin() * 70.0 * dt;
                        b.vel.y -= 20.0 * dt;
                    }
                }
                Kind::Ghost => {
                    // drifts after the cursor, beside it and a little above
                    let target = world.cursor.map_or(b.pos, |c| c + vec2(46.0, -40.0));
                    b.pos += (target - b.pos) * (1.6 * dt).min(1.0);
                    b.pos.y += (b.age * 2.6).sin() * 14.0 * dt;
                }
                Kind::Gift => {
                    if b.vel != Vec2::ZERO {
                        b.vel.y += GRAVITY * 0.7 * dt;
                        b.vel.x *= 1.0 - 0.8 * dt;
                        b.pos += b.vel * dt;
                        // lands on the first window edge (or the floor) below it
                        let ground = world.ledges.iter().filter(|l| l.holds(b.pos.x) && l.y >= was.y - 1.0).map(|l| l.y).fold(world.floor, f32::min);
                        if b.vel.y > 0.0 && b.pos.y >= ground {
                            b.pos.y = ground;
                            if b.vel.y > 420.0 {
                                b.vel = vec2(b.vel.x * 0.5, -b.vel.y * 0.3);
                            } else {
                                b.vel = Vec2::ZERO;
                                b.life = b.age + GIFT_REST;
                            }
                        }
                    }
                }
            }
            if b.kind == Kind::Snow && b.depth > 0.55 {
                // a close flake lands on the first edge it crosses
                let hit = world
                    .ledges
                    .iter()
                    .map(|l| (l.y, l.x0, l.x1))
                    .chain([(world.floor, world.screen.left(), world.screen.right())])
                    .find(|&(y, x0, x1)| (x0..=x1).contains(&b.pos.x) && was.y <= y && b.pos.y >= y);
                if let Some((y, x0, x1)) = hit {
                    landed.push((y, x0, x1, b.pos.x));
                    return false;
                }
            }
            b.age < b.life && b.pos.y < world.screen.bottom() + 20.0
        });
        for (y, x0, x1, x) in landed {
            let pile = match self.piles.iter().position(|p| (p.y - y).abs() < 1.0 && (p.x0 - x0).abs() < 1.0) {
                Some(i) => &mut self.piles[i],
                None => {
                    let n = ((x1 - x0) / PILE_COL).ceil().max(1.0) as usize;
                    self.piles.push(Pile { y, x0, cols: vec![0.0; n] });
                    self.piles.last_mut().expect("just pushed")
                }
            };
            let i = (((x - x0) / PILE_COL) as usize).min(pile.cols.len() - 1);
            add_snow(&mut pile.cols, i, 0.9);
        }
    }

    fn settle_piles(&mut self, dt: f32, world: &World) {
        let melting = !self.snowing();
        let floor = (world.floor, world.screen.left());
        let mut fallen: Vec<(Pos2, f32)> = vec![];
        self.piles.retain_mut(|p| {
            // a window that moved or closed drops its snow
            let here = (p.y - floor.0).abs() < 1.0 || world.ledges.iter().any(|l| (l.y - p.y).abs() < 1.0 && (l.x0 - p.x0).abs() < 1.0);
            if !here {
                for (i, h) in p.cols.iter().enumerate() {
                    if *h > 1.5 && i % 2 == 0 {
                        fallen.push((pos2(p.x0 + i as f32 * PILE_COL, p.y - h), *h));
                    }
                }
                return false;
            }
            if melting {
                p.cols.iter_mut().for_each(|h| *h = (*h - MELT_PER_S * dt).max(0.0));
            }
            p.cols.iter().any(|h| *h > 0.05)
        });
        for (pos, h) in fallen {
            let spin = self.between(0.0, 6.3);
            self.bits.push(Bit { kind: Kind::Snow, look: 0, pos, vel: vec2(0.0, 120.0 + h * 10.0), size: 4.0, depth: 0.5, spin, age: 0.0, life: 10.0 });
        }
    }

    fn move_pet(&mut self, dt: f32, world: &World) -> Option<Done> {
        let t = self.t;
        let roam = self.roam.as_mut()?;
        match roam.phase {
            Phase::Crouch { start, to } => {
                if t - start >= CROUCH_S {
                    let dist = (to - roam.at).length();
                    // clears the higher end by a little, whichever way it goes
                    let rise = 36.0 + (roam.at.y - to.y).abs() / 2.0;
                    let dur = (0.32 + dist as f64 * 0.0007).min(0.9);
                    roam.facing = if (to.x - roam.at.x).abs() > 1.0 { (to.x - roam.at.x).signum() } else { roam.facing };
                    roam.phase = Phase::Hop { from: roam.at, to, start: t, dur, rise };
                }
            }
            Phase::Hop { from, to, start, dur, rise } => {
                let k = ((t - start) / dur).min(1.0) as f32;
                // a parabola through `from` and `to` that tops `rise` above `from`
                let y = from.y + (to.y - from.y) * k - 4.0 * rise * k * (1.0 - k);
                roam.at = pos2(from.x + (to.x - from.x) * k, y);
                if k >= 1.0 {
                    roam.at = to;
                    // home holds it wherever it is: it lives in mid-air
                    let home = roam.goal.home();
                    roam.phase = if world.holds(to) || home == Some(to) { Phase::Land { start: t } } else { Phase::Fly };
                    roam.vel = Vec2::ZERO;
                }
            }
            Phase::Land { start } => {
                if t - start >= LAND_S {
                    return self.next(world);
                }
            }
            Phase::Sit { until } => {
                if !world.holds(roam.at) {
                    // the window under it went away: it falls
                    roam.phase = Phase::Fly;
                } else if t >= until {
                    if let Goal::Visit { back, .. } = &mut roam.goal {
                        *back = true;
                    }
                    return self.next(world);
                }
            }
            Phase::Fly => {
                let was = roam.at;
                roam.vel.y += GRAVITY * dt;
                roam.at += roam.vel * dt;
                let half = roam.width / 2.0;
                // the screen's sides and top bounce it back
                if roam.at.x - half < world.screen.left() || roam.at.x + half > world.screen.right() {
                    roam.at.x = roam.at.x.clamp(world.screen.left() + half, world.screen.right() - half);
                    roam.vel.x = -roam.vel.x * 0.6;
                }
                if roam.at.y - roam.width < world.screen.top() && roam.vel.y < 0.0 {
                    roam.vel.y = -roam.vel.y * 0.4;
                }
                if roam.vel.y > 0.0 {
                    let ground = world.ground_under(pos2(roam.at.x, was.y));
                    if roam.at.y >= ground {
                        roam.at.y = ground;
                        if roam.vel.y > BOUNCE_VY {
                            roam.vel = vec2(roam.vel.x * 0.55, -roam.vel.y * 0.32);
                        } else {
                            roam.vel = Vec2::ZERO;
                            roam.phase = match roam.goal {
                                Goal::Stay => Phase::Dizzy { until: t + DIZZY_S },
                                _ => Phase::Land { start: t },
                            };
                        }
                    }
                }
            }
            Phase::Dizzy { until } => {
                if t >= until {
                    let at = roam.at;
                    self.roam = None;
                    return Some(Done::Settled(at));
                }
            }
            Phase::Drive { speed, puff } => {
                let Goal::Drive { home, lap, wave_x, waved } = &mut roam.goal else { return None };
                let speed = (speed + DRIVE_ACCEL * dt).min(DRIVE_SPEED);
                roam.facing = 1.0;
                roam.at.x += speed * dt;
                let half = roam.width / 2.0;
                // off the right side, back in from the left
                if roam.at.x - half > world.screen.right() {
                    roam.at.x = world.screen.left() - half;
                    *lap += 1;
                }
                let mut puff = puff;
                if t - puff >= PUFF_S && roam.at.x + half > world.screen.left() {
                    puff = t;
                    let jitter = ((t * 37.0).sin() * 8.0) as f32;
                    let at = pos2(roam.at.x - half * 0.95, roam.at.y - roam.width * 0.1 + jitter * 0.3);
                    self.bits.push(Bit {
                        kind: Kind::Puff,
                        look: 0,
                        pos: at,
                        vel: vec2(-60.0 - speed * 0.1, -25.0 + jitter),
                        size: 3.0,
                        depth: 1.0,
                        spin: 0.0,
                        age: 0.0,
                        life: 0.7,
                    });
                }
                roam.phase = Phase::Drive { speed, puff };
                if *lap >= 1 && !*waved && roam.at.x >= *wave_x {
                    *waved = true;
                    roam.phase = Phase::Wave { until: t + WAVE_S };
                } else if *lap >= 1 && roam.at.x >= home.x {
                    roam.at.x = home.x;
                    if (home.y - roam.at.y).abs() < 2.0 {
                        self.roam = None;
                        return Some(Done::Home);
                    }
                    let home = *home;
                    roam.phase = Phase::Crouch { start: t, to: home };
                }
            }
            Phase::Wave { until } => {
                if t >= until {
                    roam.phase = Phase::Drive { speed: 0.0, puff: t };
                }
            }
            Phase::Glide { from, to, start, dur } if matches!(roam.goal, Goal::Gallop { .. }) => {
                let Goal::Gallop { home, back, trail } = &mut roam.goal else { return None };
                let k = ((t - start) / dur).min(1.0) as f32;
                // an arc: up and away going out, down and home coming back
                let lift = 4.0 * k * (1.0 - k) * roam.width * if *back { 0.3 } else { 1.2 };
                let ease = if *back { k * k * (3.0 - 2.0 * k) } else { k };
                roam.at = pos2(from.x + (to.x - from.x) * k, from.y + (to.y - from.y) * ease - lift);
                roam.facing = 1.0;
                if t - *trail > 0.05 {
                    *trail = t;
                    let pos = roam.at - vec2(roam.width * 0.45, roam.width * 0.35);
                    let j = (t * 53.0).sin() as f32;
                    self.bits.push(Bit {
                        kind: Kind::Spark,
                        look: 2,
                        pos,
                        vel: vec2(-30.0, 25.0 + j * 15.0),
                        size: 3.0,
                        depth: 1.0,
                        spin: j,
                        age: 0.0,
                        life: 1.2,
                    });
                }
                if k >= 1.0 {
                    if *back {
                        self.roam = None;
                        return Some(Done::Home);
                    }
                    *back = true;
                    let band = world.screen.top() + roam.width * 0.6 + 48.0;
                    let from = pos2(world.screen.left() - roam.width, band);
                    let home = *home;
                    let dur = (((from - home).length() / SLEIGH_SPEED) * 1.1) as f64;
                    roam.at = from;
                    roam.phase = Phase::Glide { from, to: home, start: t, dur };
                }
            }
            Phase::Glide { from, to, start, dur } => {
                let Goal::Sleigh { home, drops, dropped, back } = &mut roam.goal else { return None };
                let k = ((t - start) / dur).min(1.0) as f32;
                // across the top with a bob; coming home, it eases down
                let ease = if *back { k * k * (3.0 - 2.0 * k) } else { k };
                let bob = (t as f32 * 3.2).sin() * 6.0 * (1.0 - if *back { k } else { 0.0 });
                roam.at = pos2(from.x + (to.x - from.x) * k, from.y + (to.y - from.y) * ease + bob);
                roam.facing = (to.x - from.x).signum();
                if !*back {
                    for (i, x) in drops.iter().enumerate() {
                        if *dropped & (1 << i) == 0 && roam.at.x >= *x {
                            *dropped |= 1 << i;
                            let at = pos2(roam.at.x - roam.width * 0.3, roam.at.y - roam.width * 0.1);
                            let spin = (t * 7.0 + i as f64).sin() as f32;
                            self.bits.push(Bit {
                                kind: Kind::Gift,
                                look: i % 4,
                                pos: at,
                                vel: vec2(SLEIGH_SPEED * 0.35, -60.0),
                                size: 1.0,
                                depth: 1.0,
                                spin,
                                age: 0.0,
                                life: 60.0,
                            });
                        }
                    }
                }
                if k >= 1.0 {
                    if *back {
                        self.roam = None;
                        return Some(Done::Home);
                    }
                    *back = true;
                    let from = pos2(world.screen.right() + roam.width, roam.at.y);
                    let home = *home;
                    let dur = (((from - home).length() / SLEIGH_SPEED) * 1.15) as f64;
                    roam.at = from;
                    roam.phase = Phase::Glide { from, to: home, start: t, dur };
                }
            }
            Phase::Keys { i, start, from, zoom0 } => {
                let Goal::Show { keys, n, .. } = roam.goal else { return None };
                let k = keys[i];
                let p = if k.dur <= 0.0 { 1.0 } else { ((t - start) / k.dur).min(1.0) as f32 };
                let ease = p * p * (3.0 - 2.0 * p);
                roam.at = from + (k.at - from) * ease - vec2(0.0, 4.0 * k.arc * p * (1.0 - p));
                roam.zoom = zoom0 + (k.zoom - zoom0) * ease;
                if p >= 1.0 {
                    if i + 1 >= n {
                        self.roam = None;
                        return Some(Done::Home);
                    }
                    let next = keys[i + 1];
                    let (at, zoom, width) = (roam.at, roam.zoom, roam.width);
                    roam.phase = Phase::Keys { i: i + 1, start: t, from: at, zoom0: zoom };
                    if next.emit == Emit::Sparks
                        && let Some(c) = world.cursor
                    {
                        roam.facing = (c.x - at.x).signum();
                    }
                    let facing = roam.facing;
                    self.emit(next.emit, at, width * zoom, facing, world);
                }
            }
            Phase::Float { heading } => {
                let Goal::Float { home, until, waved, aim } = &mut roam.goal else { return None };
                if t < *until {
                    // the rocket cruises from one spot of the screen to the
                    // next, turning gently towards it
                    let area = Rect::from_min_max(
                        pos2(world.screen.left() + roam.width, world.screen.top() + roam.width * 1.2),
                        pos2(world.screen.right() - roam.width, world.floor - roam.width * 0.6),
                    );
                    if (*aim - roam.at).length() < roam.width * 1.5 || !area.contains(*aim) {
                        // a spot far enough away, picked from the clock
                        let r = |k: f64| ((t * k).sin() * 43758.5453).fract().abs() as f32;
                        *aim = pos2(area.left() + area.width() * r(12.9898), area.top() + area.height() * r(78.233));
                    }
                    let want = (*aim - roam.at).angle();
                    let mut turn = want - heading;
                    while turn > std::f32::consts::PI {
                        turn -= std::f32::consts::TAU;
                    }
                    while turn < -std::f32::consts::PI {
                        turn += std::f32::consts::TAU;
                    }
                    let heading = heading + turn.clamp(-ROCKET_TURN * dt, ROCKET_TURN * dt);
                    roam.vel += (Vec2::angled(heading) * ROCKET_SPEED - roam.vel) * (1.5 * dt).min(1.0);
                    roam.at += roam.vel * dt;
                    roam.phase = Phase::Float { heading };
                    if !*waved && (roam.at.x - world.screen.center().x).abs() < roam.width {
                        *waved = true;
                    }
                } else {
                    // home: a spring on it
                    let to = *home - roam.at;
                    roam.vel += to * 3.0 * dt - roam.vel * 2.2 * dt;
                    roam.at += roam.vel * dt;
                    if to.length() < 3.0 && roam.vel.length() < 25.0 {
                        self.roam = None;
                        return Some(Done::Home);
                    }
                }
                // nose along the way it flies; facing left, the art is mirrored
                let v = roam.vel;
                if v.length() > 5.0 {
                    roam.facing = if v.x < 0.0 { -1.0 } else { 1.0 };
                    let tilt = v.y.atan2(v.x.abs()).clamp(-0.4, 0.4);
                    roam.spin = if roam.facing < 0.0 { -tilt } else { tilt };
                }
            }
            Phase::Run { speed, dir } => {
                let Goal::Dare { home, until, teased } = &mut roam.goal else { return None };
                let half = roam.width / 2.0;
                // the cursor near it, at its level or above: it's coming for it
                let chaser = chaser(world, roam.at);
                let (want, dir) = match chaser {
                    Some(c) => (RUN_SPEED, if (roam.at.x - c.x).abs() < 1.0 { dir } else { (roam.at.x - c.x).signum() }),
                    None if t >= *until => (RUN_SPEED * 0.7, (home.x - roam.at.x).signum()),
                    // nobody after it: it jogs about on its own
                    None => (RUN_SPEED * 0.5, dir),
                };
                let speed = if want > speed { (speed + 1400.0 * dt).min(want) } else { (speed - 900.0 * dt).max(want) };
                roam.at.x += dir * speed * dt;
                roam.facing = dir;
                roam.phase = Phase::Run { speed, dir };
                let (l, r) = (world.screen.left() + half, world.screen.right() - half);
                if roam.at.x <= l || roam.at.x >= r {
                    roam.at.x = roam.at.x.clamp(l, r);
                    match chaser {
                        // cornered: a leap over the cursor to the open side
                        Some(c) => {
                            let land = (c.x - dir * (SCARED_X + 60.0)).clamp(l, r);
                            roam.phase = Phase::Crouch { start: t, to: pos2(land, roam.at.y) };
                        }
                        // jogging: turn round at the edge
                        None => roam.phase = Phase::Run { speed: speed * 0.5, dir: -dir },
                    }
                }
                if chaser.is_none() && t >= *until && (home.x - roam.at.x).abs() <= STRIDE {
                    let home = *home;
                    roam.phase = Phase::Crouch { start: t, to: home };
                } else if chaser.is_none() && t < *until && t - *teased > 3.5 {
                    *teased = t;
                    roam.phase = Phase::Taunt { until: t + TAUNT_S };
                }
            }
            Phase::Taunt { until } => {
                // a tease is cut short by the cursor coming for it
                if t >= until || chaser(world, roam.at).is_some() {
                    // off again, now this way, now that
                    let dir = if (t * 7.3).fract() > 0.5 { -roam.facing } else { roam.facing };
                    roam.phase = Phase::Run { speed: 0.0, dir };
                }
            }
        }
        None
    }

    // what a beat sets off
    fn emit(&mut self, emit: Emit, at: Pos2, size: f32, facing: f32, world: &World) {
        let bit = |kind, look, pos, vel: Vec2, size, life| Bit { kind, look, pos, vel, size, depth: 1.0, spin: 0.0, age: 0.0, life };
        match emit {
            Emit::None => {}
            Emit::Sparks => {
                let tip = at + vec2(facing * size * 0.45, -size * 0.55);
                let aim = world.cursor.map_or(vec2(facing, -0.3).normalized(), |c| (c - tip).normalized());
                for i in 0..36 {
                    let a = aim.angle() + self.between(-0.35, 0.35);
                    let speed = self.between(500.0, 900.0);
                    let s = self.between(3.0, 6.0);
                    let life = self.between(0.6, 1.1);
                    let mut b = bit(Kind::Spark, i % 2, tip, Vec2::angled(a) * speed, s, life);
                    b.age = -(i as f32) * 0.02;
                    self.bits.push(b);
                }
            }
            Emit::Bats => {
                let mouth = at - vec2(0.0, size * 0.35);
                for i in 0..8 {
                    let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
                    let vel = vec2(dir * self.between(160.0, 300.0), self.between(-220.0, -90.0));
                    let mut b = bit(Kind::Bat, 0, mouth, vel, size * 1.1, 9.0);
                    b.spin = self.between(0.0, 6.3);
                    b.age = -(i as f32) * 0.28;
                    self.bits.push(b);
                }
            }
            Emit::Ghost => {
                let pos = at - vec2(0.0, size * 0.6);
                self.bits.push(bit(Kind::Ghost, 0, pos, Vec2::ZERO, size * 1.1, 7.0));
            }
        }
    }

    // the next move; home ends the roam
    fn next(&mut self, world: &World) -> Option<Done> {
        let done = next_move(self.roam.as_mut()?, world, self.t);
        if done.is_some() {
            self.roam = None;
        }
        done
    }

    // ---- drawing --------------------------------------------------------

    /// Draw it all. `origin`: the screen point at the overlay's top-left;
    /// `scale`: overlay points per screen point.
    pub fn draw(&self, p: &Painter, origin: Pos2, scale: f32, pet: &Pet, mood: Mood, world: &World) {
        let at = |s: Pos2| pos2((s.x - origin.x) * scale, (s.y - origin.y) * scale);
        // far flakes first, so the close ones (and the pet) pass in front
        for b in self.bits.iter().filter(|b| b.depth < 0.55) {
            draw_bit(p, b, at(b.pos), scale);
        }
        for pile in &self.piles {
            draw_pile(p, pile, &at, scale);
        }
        for b in self.bits.iter().filter(|b| b.kind == Kind::Gift) {
            // fades out at the end of its rest
            let fade = ((b.life - b.age) / 0.6).clamp(0.0, 1.0);
            let w = self.roam.as_ref().map_or(40.0, |r| r.width * 0.18).max(28.0);
            pet.draw_gift(p, at(b.pos), w * scale, b.look, fade);
        }
        // the pumpkin's bats and ghost come from its act strip
        for b in self.bits.iter().filter(|b| matches!(b.kind, Kind::Bat | Kind::Ghost) && b.age >= 0.0) {
            let fade = (b.age / 0.4).min(1.0) * ((b.life - b.age) / 0.6).clamp(0.0, 1.0);
            let frame = if b.kind == Kind::Ghost { 3 } else { 1 + (b.age * 10.0) as usize % 2 };
            pet.draw_act_frame(p, at(b.pos), b.size * scale, frame, b.vel.x < 0.0, fade);
        }
        if let Some(r) = &self.roam {
            self.draw_pet(p, r, &at, scale, pet, mood, world);
        }
        for b in self.bits.iter().filter(|b| b.depth >= 0.55) {
            draw_bit(p, b, at(b.pos), scale);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_pet(&self, p: &Painter, r: &Roam, at: &dyn Fn(Pos2) -> Pos2, scale: f32, pet: &Pet, mood: Mood, world: &World) {
        let t = self.t;
        // its shadow on the surface below, smaller and fainter the higher it
        // is; none while it hides behind a window
        let hiding = matches!((r.phase, r.goal), (Phase::Keys { i, .. }, Goal::Show { keys, .. }) if !keys[i].clip.is_nan());
        if !hiding {
            let ground = world.ground_under(r.at);
            let lift = ((ground - r.at.y) / 300.0).clamp(0.0, 1.0);
            let shadow = at(pos2(r.at.x, ground));
            let (sw, sh) = (r.width * 0.42 * (1.0 - lift * 0.6) * scale, r.width * 0.07 * (1.0 - lift * 0.6) * scale);
            p.add(eframe::egui::Shape::ellipse_filled(shadow, vec2(sw, sh), color::PET_SHADOW.gamma_multiply(1.0 - lift * 0.7)));
        }
        if r.ride {
            // its car: wheels turning while it drives, a wave, a squash on the hops
            let (frame, squash) = match r.phase {
                Phase::Drive { speed, .. } => (if speed > DRIVE_SPEED * 0.9 { 2 } else { (t * 12.0) as usize % 2 }, Vec2::splat(1.0)),
                Phase::Wave { .. } | Phase::Taunt { .. } => (3, Vec2::splat(1.0)),
                // the sleigh gallops, and Santa waves as he passes the middle
                Phase::Glide { .. } if r.facing > 0.0 && (r.at.x - world.screen.center().x).abs() < r.width * 0.6 => (3, Vec2::splat(1.0)),
                Phase::Glide { .. } => ((t * 8.0) as usize % 3, Vec2::splat(1.0)),
                Phase::Run { speed, .. } if speed > 30.0 => ((t * 14.0) as usize % 3, Vec2::splat(1.0)),
                Phase::Run { .. } => (0, Vec2::splat(1.0)),
                Phase::Crouch { .. } | Phase::Land { .. } => (0, vec2(1.06, 0.9)),
                _ => (0, vec2(0.97, 1.04)),
            };
            let frame = match (r.phase, r.goal) {
                (Phase::Keys { i, start, .. }, Goal::Show { keys, .. }) => {
                    let k = keys[i];
                    // behind a window's edge: only what's above it shows
                    let clipped =
                        if k.clip.is_nan() { p.clone() } else { p.with_clip_rect(Rect::from_min_max(pos2(-1e5, -1e5), pos2(1e5, at(pos2(0.0, k.clip)).y))) };
                    if k.frame == STAND {
                        let hopping = k.arc > 0.0 && t - start < k.dur;
                        let pose = if hopping { Pose::Jump } else { Pose::Stand(mood) };
                        let _ = pet.draw_pose(&clipped, at(r.at), r.width * r.zoom * scale, pose, Vec2::splat(1.0), r.facing < 0.0);
                    } else {
                        pet.draw_act(&clipped, at(r.at), r.width * r.zoom * scale, k.frame, squash, r.facing < 0.0, r.spin);
                    }
                    return;
                }
                (Phase::Float { .. }, Goal::Float { until, waved, .. }) => {
                    if waved && t < until && (r.at.x - world.screen.center().x).abs() < r.width * 1.5 {
                        3
                    } else if t >= until || r.vel.length() > 120.0 {
                        (t * 10.0) as usize % 2
                    } else {
                        2
                    }
                }
                (Phase::Glide { .. }, Goal::Gallop { back, .. }) if back && (r.at.x - world.screen.center().x).abs() < r.width * 0.8 => 3,
                (Phase::Glide { .. }, Goal::Gallop { .. }) => (t * 9.0) as usize % 3,
                _ => frame,
            };
            pet.draw_act(p, at(r.at), r.width * r.zoom * scale, frame, squash, r.facing < 0.0, r.spin);
            return;
        }
        let (pose, squash) = match r.phase {
            Phase::Crouch { .. } | Phase::Land { .. } => (Pose::Crouch, vec2(1.12, 0.86)),
            Phase::Hop { start, dur, .. } => {
                let k = ((t - start) / dur) as f32;
                (if k < 0.5 { Pose::Jump } else { Pose::Fall }, vec2(0.94, 1.08))
            }
            Phase::Fly => (if r.vel.y < 0.0 { Pose::Jump } else { Pose::Fall }, vec2(0.95, 1.06)),
            Phase::Dizzy { .. } => (Pose::Dizzy, Vec2::splat(1.0)),
            Phase::Keys { .. }
            | Phase::Float { .. }
            | Phase::Sit { .. }
            | Phase::Drive { .. }
            | Phase::Wave { .. }
            | Phase::Glide { .. }
            | Phase::Run { .. }
            | Phase::Taunt { .. } => (Pose::Stand(mood), Vec2::splat(1.0)),
        };
        let feet = at(r.at);
        let head = pet.draw_pose(p, feet, r.width * scale, pose, squash, r.facing < 0.0);
        if let Phase::Dizzy { .. } = r.phase {
            // stars round its head
            for i in 0..3 {
                let a = t as f32 * 5.0 + i as f32 * 2.1;
                let c = pos2(head.center().x + a.cos() * head.width() * 0.35, head.top() - 6.0 * scale + a.sin() * head.width() * 0.08);
                star(p, c, 3.0 * scale);
            }
        }
    }
}

/// The cursor near the pet, at its level or above: coming for it.
fn chaser(world: &World, at: Pos2) -> Option<Pos2> {
    world.cursor.filter(|c| (c.x - at.x).abs() < SCARED_X && c.y > at.y - 280.0 && c.y < at.y + 60.0)
}

/// Snow added at column `i`; what doesn't fit rolls to a lower neighbour.
fn add_snow(cols: &mut [f32], i: usize, amount: f32) {
    let mut at = i;
    for _ in 0..4 {
        let left = at.checked_sub(1).map(|j| (j, cols[j]));
        let right = (at + 1 < cols.len()).then(|| (at + 1, cols[at + 1]));
        let low = [left, right].into_iter().flatten().filter(|&(_, h)| h + 1.5 < cols[at]).min_by(|a, b| a.1.total_cmp(&b.1));
        match low {
            Some((j, _)) => at = j,
            None => break,
        }
    }
    cols[at] = (cols[at] + amount).min(PILE_MAX);
}

/// What comes after a landing or a rest: the next hop, a sit, or home.
fn next_move(roam: &mut Roam, world: &World, t: f64) -> Option<Done> {
    if let Goal::Dare { home, until, .. } = roam.goal {
        if t >= until && (roam.at - home).length() < 2.0 {
            return Some(Done::Home);
        }
        roam.phase = Phase::Taunt { until: t + TAUNT_S };
        return None;
    }
    if let Goal::Drive { home, lap, .. } = roam.goal {
        // landed: on the floor it drives off; back home, it's done
        if lap >= 1 && (roam.at - home).length() < 2.0 {
            return Some(Done::Home);
        }
        roam.phase = Phase::Drive { speed: 0.0, puff: t };
        return None;
    }
    let Goal::Visit { spot, home, back } = roam.goal else {
        roam.phase = Phase::Dizzy { until: t };
        return None;
    };
    let target = if back { home } else { spot };
    if (roam.at - target).length() < 2.0 {
        if back {
            return Some(Done::Home);
        }
        roam.phase = Phase::Sit { until: t + SIT_S };
        return None;
    }
    // close enough: one jump there; else a step along the surface it's on
    let dx = target.x - roam.at.x;
    let to = if dx.abs() <= STRIDE { target } else { pos2(roam.at.x + STRIDE * dx.signum(), roam.at.y) };
    // stepping past the end of a window, it lands wherever is below
    let to = if to == target || world.holds(to) { to } else { pos2(to.x, world.ground_under(to)) };
    roam.phase = Phase::Crouch { start: t, to };
    None
}

fn draw_bit(p: &Painter, b: &Bit, c: Pos2, scale: f32) {
    let s = b.size * scale;
    match b.kind {
        Kind::Snow => {
            // a pixel flake: close ones bright with a soft edge, far ones dim;
            // each fades in out of the air, and the far ones fade out at the end
            let fade = (0.35 + b.depth * 0.65).min(1.0) * (b.age / 0.8).min(1.0) * ((b.life - b.age) / 0.5).clamp(0.0, 1.0);
            let r = Rect::from_center_size(c, Vec2::splat(s.round().max(1.0)));
            if b.depth > 0.55 {
                p.rect_filled(r.expand(scale), 0.0, color::SNOW_OUTLINE);
            }
            p.rect_filled(r, 0.0, color::SNOW.gamma_multiply(fade));
        }
        Kind::Gift | Kind::Bat | Kind::Ghost => {}
        Kind::Spark => {
            if b.age < 0.0 {
                return;
            }
            let fade = (1.0 - b.age / b.life).clamp(0.0, 1.0);
            let col = color::SPARK[b.look % 3];
            p.rect_filled(Rect::from_center_size(c, Vec2::splat(s.round().max(scale))), 0.0, col.gamma_multiply(fade));
        }

        Kind::Puff => {
            // a grey pixel puff, growing as it thins out
            let fade = (1.0 - b.age / b.life).clamp(0.0, 1.0) * 0.8;
            p.rect_filled(Rect::from_center_size(c, Vec2::splat(s.round().max(scale))), 0.0, color::PUFF.gamma_multiply(fade));
        }
    }
}

fn draw_pile(p: &Painter, pile: &Pile, at: &dyn Fn(Pos2) -> Pos2, scale: f32) {
    for (i, h) in pile.cols.iter().enumerate() {
        if *h < 0.5 {
            continue;
        }
        let x = pile.x0 + i as f32 * PILE_COL;
        let h = h.round();
        let top = at(pos2(x, pile.y - h));
        let base = at(pos2(x + PILE_COL, pile.y));
        let r = Rect::from_min_max(top, base);
        p.rect_filled(r, 0.0, color::SNOW_SHADE);
        // the lit top: two thirds of the depth
        p.rect_filled(Rect::from_min_max(r.min, pos2(r.max.x, r.min.y + (r.height() * 0.6).max(scale))), 0.0, color::SNOW);
    }
}

fn star(p: &Painter, c: Pos2, s: f32) {
    // a four-point pixel star
    p.rect_filled(Rect::from_center_size(c, vec2(s, s * 3.0)), 0.0, color::DIZZY_STAR);
    p.rect_filled(Rect::from_center_size(c, vec2(s * 3.0, s)), 0.0, color::DIZZY_STAR);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        World {
            screen: Rect::from_min_size(Pos2::ZERO, vec2(1440.0, 900.0)),
            floor: 820.0,
            ledges: vec![Ledge { y: 300.0, x0: 200.0, x1: 900.0 }],
            cursor: None,
        }
    }

    fn run(play: &mut Play, s: &World, secs: f32) -> Option<Done> {
        let mut done = None;
        for _ in 0..(secs * 60.0) as usize {
            done = done.or(play.step(1.0 / 60.0, s));
        }
        done
    }

    #[test]
    fn a_visit_goes_up_onto_the_window_sits_and_comes_home() {
        let s = world();
        let mut play = Play::new(7);
        let home = pos2(1200.0, 820.0);
        play.visit(home, pos2(850.0, 300.0), 60.0);
        run(&mut play, &s, 4.0);
        let r = play.roam.as_ref().expect("still out");
        assert_eq!(r.at, pos2(850.0, 300.0), "on the window");
        assert!(matches!(r.phase, Phase::Sit { .. }));
        assert_eq!(run(&mut play, &s, SIT_S as f32 + 6.0), Some(Done::Home));
        assert!(!play.pet_out());
    }

    #[test]
    fn a_window_that_goes_away_drops_the_pet() {
        let mut s = world();
        let mut play = Play::new(7);
        play.visit(pos2(900.0, 820.0), pos2(850.0, 300.0), 60.0);
        run(&mut play, &s, 3.0);
        s.ledges.clear();
        run(&mut play, &s, 0.7);
        let r = play.roam.as_ref().expect("still out");
        assert_eq!(r.at.y, 820.0, "on the floor");
    }

    #[test]
    fn the_drive_goes_off_the_right_comes_back_waving_and_gets_home() {
        let s = world();
        let mut play = Play::new(7);
        let home = pos2(1000.0, 500.0);
        play.drive(home, 90.0, &s);
        // down on the floor, then off the right
        run(&mut play, &s, 1.5);
        assert_eq!(play.roam.as_ref().expect("out").at.y, 820.0, "on the floor");
        let mut waved = false;
        let mut done = None;
        for _ in 0..60 * 20 {
            done = play.step(1.0 / 60.0, &s);
            waved |= play.roam.as_ref().is_some_and(|r| matches!(r.phase, Phase::Wave { .. }));
            if done.is_some() {
                break;
            }
        }
        assert!(waved, "it waves on the way back");
        assert_eq!(done, Some(Done::Home));
        assert!(play.bits.iter().any(|b| b.kind == Kind::Puff) || !play.active(), "exhaust behind it");
    }

    #[test]
    fn the_sleigh_drops_presents_one_over_the_cursor_and_lands_home() {
        let mut s = world();
        s.cursor = Some(pos2(640.0, 500.0));
        let mut play = Play::new(5);
        let home = pos2(1100.0, 600.0);
        play.sleigh(home, 180.0, &s);
        // across the top: four presents, one right under the cursor
        run(&mut play, &s, 8.0);
        let gifts: Vec<&Bit> = play.bits.iter().filter(|b| b.kind == Kind::Gift).collect();
        assert_eq!(gifts.len(), 4);
        assert!(gifts.iter().any(|g| g.look == 3 && (g.pos.x - 640.0).abs() < 200.0), "one over the cursor");
        // they land on the window or the floor and rest
        assert!(gifts.iter().all(|g| g.pos.y == 300.0 || g.pos.y == 820.0), "landed: {:?}", gifts.iter().map(|g| g.pos).collect::<Vec<_>>());
        let mut done = None;
        for _ in 0..60 * 20 {
            done = done.or(play.step(1.0 / 60.0, &s));
        }
        assert_eq!(done, Some(Done::Home));
    }

    #[test]
    fn the_gingerbread_runs_from_the_cursor_then_goes_home() {
        let mut s = world();
        let mut play = Play::new(5);
        let home = pos2(700.0, 600.0);
        play.dare(home, 60.0, &s);
        run(&mut play, &s, 2.5);
        let at = play.roam.as_ref().expect("out").at;
        assert_eq!(at.y, 820.0, "on the floor");
        // the cursor comes at it from the left: it runs right
        s.cursor = Some(pos2(at.x - 80.0, 800.0));
        run(&mut play, &s, 0.6);
        assert!(play.roam.as_ref().expect("out").at.x > at.x + 60.0, "ran away");
        // left alone, it heads home when the dare is over
        s.cursor = None;
        let mut done = None;
        for _ in 0..60 * 30 {
            done = done.or(play.step(1.0 / 60.0, &s));
        }
        assert_eq!(done, Some(Done::Home));
    }

    // runs until the pet is home, or panics
    fn until_home(play: &mut Play, s: &World, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            if play.step(1.0 / 60.0, s) == Some(Done::Home) {
                return;
            }
        }
        panic!("never got home");
    }

    #[test]
    fn clauddy_peeks_from_behind_the_window_and_comes_home() {
        let s = world();
        let mut play = Play::new(1);
        play.peek(pos2(1200.0, 600.0), &Ledge { y: 300.0, x0: 200.0, x1: 900.0 }, 60.0);
        run(&mut play, &s, 1.5);
        let r = play.roam.as_ref().expect("out");
        let Goal::Show { keys, .. } = r.goal else { panic!("a show") };
        let Phase::Keys { i, .. } = r.phase else { panic!("keyed") };
        assert!(keys[i].clip == 300.0 && r.at.y > 300.0, "behind the window's edge: {:?}", r.at);
        until_home(&mut play, &s, 12.0);
    }

    #[test]
    fn the_wizard_poofs_away_sparks_at_the_cursor_and_poofs_home() {
        let mut s = world();
        s.cursor = Some(pos2(100.0, 100.0));
        let mut play = Play::new(1);
        play.spell(pos2(1200.0, 600.0), pos2(500.0, 300.0), 60.0);
        run(&mut play, &s, 2.4);
        assert_eq!(play.roam.as_ref().expect("out").at, pos2(500.0, 300.0), "up on the window");
        run(&mut play, &s, 0.3);
        let sparks: Vec<&Bit> = play.bits.iter().filter(|b| b.kind == Kind::Spark && b.age > 0.0).collect();
        assert!(!sparks.is_empty() && sparks.iter().all(|b| b.vel.x < 0.0 && b.pos.y < 300.0), "towards the cursor, up and left");
        until_home(&mut play, &s, 10.0);
    }

    #[test]
    fn the_astronaut_floats_inside_the_screen_then_comes_home() {
        let s = world();
        let mut play = Play::new(9);
        play.float(pos2(700.0, 600.0), 60.0);
        for _ in 0..60 * 12 {
            play.step(1.0 / 60.0, &s);
            let at = play.roam.as_ref().expect("out").at;
            assert!(s.screen.expand(1.0).contains(at), "inside the screen: {at:?}");
        }
        until_home(&mut play, &s, 15.0);
    }

    #[test]
    fn the_pumpkin_lets_out_bats_and_a_ghost_that_follows_the_cursor() {
        let mut s = world();
        s.cursor = Some(pos2(300.0, 300.0));
        let mut play = Play::new(1);
        play.haunt(pos2(700.0, 600.0), 60.0);
        until_home(&mut play, &s, 5.0);
        assert_eq!(play.bits.iter().filter(|b| b.kind == Kind::Bat).count(), 8);
        run(&mut play, &s, 4.0);
        let ghost = play.bits.iter().find(|b| b.kind == Kind::Ghost).expect("a ghost");
        assert!((ghost.pos - pos2(346.0, 260.0)).length() < 40.0, "by the cursor: {:?}", ghost.pos);
    }

    #[test]
    fn the_reindeer_gallops_off_and_back_home() {
        let s = world();
        let mut play = Play::new(1);
        play.gallop(pos2(700.0, 600.0), 60.0, &s);
        until_home(&mut play, &s, 20.0);
        assert!(play.bits.iter().any(|b| b.kind == Kind::Spark), "a trail");
    }

    #[test]
    fn a_throw_bounces_and_settles_where_it_lands() {
        let s = world();
        let mut play = Play::new(7);
        play.throw(pos2(1000.0, 500.0), vec2(-900.0, -600.0), 60.0);
        let Some(Done::Settled(at)) = run(&mut play, &s, 6.0) else { panic!("never settled") };
        // on the window's edge or the floor, never in mid-air
        assert!(at.y == 820.0 || (at.y == 300.0 && (200.0..=900.0).contains(&at.x)), "landed at {at:?}");
    }

    #[test]
    fn snow_falls_round_the_pet_piles_where_it_stands_and_melts_after() {
        let s = world();
        let mut play = Play::new(3);
        // the pet on the window's edge
        play.focus(pos2(500.0, 300.0), 60.0);
        play.snow();
        run(&mut play, &s, 10.0);
        assert!(play.bits.iter().all(|b| (b.pos.x - 500.0).abs() < 260.0), "only round the pet");
        run(&mut play, &s, 20.0);
        let on_window = play.piles.iter().find(|p| p.y == 300.0).map_or(0.0, |p| p.cols.iter().sum::<f32>());
        assert!(on_window > 10.0, "snow on the window: {on_window}");
        assert!(play.piles.iter().all(|p| p.cols.iter().all(|h| *h <= PILE_MAX)));
        run(&mut play, &s, 60.0);
        assert!(!play.active(), "melted and gone");
    }

    // PLAY_PREVIEW=dir cargo test --release play_preview -- --ignored:
    // frames of each kind of play over a made-up desk, to look at
    #[test]
    #[ignore = "writes preview images"]
    fn play_preview() {
        use std::cell::RefCell;
        use std::rc::Rc;
        let dir = std::env::var("PLAY_PREVIEW").expect("PLAY_PREVIEW=dir");
        let desk = World {
            screen: Rect::from_min_size(Pos2::ZERO, vec2(900.0, 600.0)),
            floor: 560.0,
            ledges: vec![Ledge { y: 250.0, x0: 200.0, x1: 700.0 }],
            cursor: None,
        };
        let play = Rc::new(RefCell::new(Play::new(11)));
        let (shared, w) = (play.clone(), desk.clone());
        let pet: RefCell<Option<Pet>> = RefCell::new(None);
        let mut harness = egui_kittest::Harness::builder().with_size(vec2(900.0, 600.0)).with_pixels_per_point(1.0).build_ui(move |ui| {
            let p = ui.painter();
            p.rect_filled(ui.max_rect(), 0.0, color::PANE);
            // a window, its title bar, and the floor
            p.rect_filled(Rect::from_min_max(pos2(200.0, 250.0), pos2(700.0, 600.0)), 8.0, color::SETTINGS);
            p.rect_filled(Rect::from_min_max(pos2(200.0, 250.0), pos2(700.0, 278.0)), 8.0, color::MENU);
            p.rect_filled(Rect::from_min_max(pos2(0.0, 560.0), pos2(900.0, 600.0)), 0.0, color::TABS);
            // kept across frames, or its textures go before they're drawn
            let mut slot = pet.borrow_mut();
            let pet = slot.get_or_insert_with(|| Pet::frozen(ui.ctx(), 0.3));
            // the car is the Getaway's; everything else is the classic pet
            let driving = shared.borrow().roam.as_ref().is_some_and(|r| r.ride);
            pet.set_skin(crate::components::skins::find(if driving { "getaway" } else { "clauddy" }).expect("a pet"));
            pet.set_hat("santa");
            shared.borrow().draw(p, Pos2::ZERO, 1.0, pet, Mood::Idle, &w);
        });
        let mut shot = |name: &str, secs: f32| {
            for _ in 0..(secs * 60.0) as usize {
                play.borrow_mut().step(1.0 / 60.0, &desk);
            }
            harness.run_steps(1);
            harness.render().expect("render").save(format!("{dir}/{name}.png")).expect("save");
        };
        play.borrow_mut().focus(pos2(450.0, 560.0), 60.0);
        play.borrow_mut().visit(pos2(450.0, 560.0), pos2(450.0, 250.0), 60.0);
        shot("visit-crouch", 0.15);
        shot("visit-air", 0.35);
        shot("visit-sit", 2.0);
        play.borrow_mut().focus(pos2(450.0, 250.0), 60.0);
        play.borrow_mut().snow();
        shot("snow", 25.0);
        play.borrow_mut().clear();
        // the Getaway's drive
        play.borrow_mut().drive(pos2(450.0, 560.0), 110.0, &desk);
        shot("drive", 0.8);
        shot("drive-back", 1.6);
        shot("drive-wave", 0.6);
        play.borrow_mut().clear();
        play.borrow_mut().throw(pos2(150.0, 500.0), vec2(900.0, -1100.0), 60.0);
        shot("throw-air", 0.3);
        shot("throw-dizzy", 2.2);
    }

    // every hat in every mood and pose, to check they sit on the head
    #[test]
    #[ignore = "writes a preview image"]
    fn hat_preview() {
        let dir = std::env::var("PLAY_PREVIEW").expect("PLAY_PREVIEW=dir");
        let pet: std::cell::RefCell<Option<Pet>> = std::cell::RefCell::new(None);
        let mut harness = egui_kittest::Harness::builder().with_size(vec2(1000.0, 960.0)).with_pixels_per_point(1.0).build_ui(move |ui| {
            let p = ui.painter();
            p.rect_filled(ui.max_rect(), 0.0, color::PANE);
            let mut slot = pet.borrow_mut();
            let pet = slot.get_or_insert_with(|| Pet::frozen(ui.ctx(), 0.3));
            let moods = [Mood::Idle, Mood::Sleeping, Mood::Fire, Mood::Tired, Mood::Working];
            for (r, (id, _)) in crate::components::skins::HATS.iter().enumerate() {
                pet.set_hat(id);
                for (c, mood) in moods.iter().enumerate() {
                    let space = Rect::from_min_size(pos2(10.0 + c as f32 * 100.0, 10.0 + r as f32 * 118.0), vec2(90.0, 100.0));
                    pet.draw_fit(p, space, *mood);
                }
                for (c, pose) in [Pose::Crouch, Pose::Jump, Pose::Fall, Pose::Dizzy].iter().enumerate() {
                    pet.draw_pose(p, pos2(560.0 + c as f32 * 110.0, 100.0 + r as f32 * 118.0), 70.0, *pose, Vec2::splat(1.0), c == 1);
                }
            }
        });
        harness.run_steps(2);
        harness.render().expect("render").save(format!("{dir}/hats.png")).expect("save");
    }

    #[test]
    fn a_pile_rolls_to_the_lower_side() {
        let mut cols = vec![0.0, 5.0, 2.0];
        add_snow(&mut cols, 1, 1.0);
        assert_eq!(cols, vec![1.0, 5.0, 2.0]);
    }
}
