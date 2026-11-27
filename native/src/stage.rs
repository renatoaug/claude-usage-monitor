// The stage: when the pet plays (just the pet, with play on), and the
// see-through window over the screen where it does. Now and then, on its
// own, each pet does its number (the Getaway drives along the bottom of the
// screen, the Christmas pets snow, the others hop up onto a window); Clauddy
// also hops onto the window in front when a long turn ends, to tell you; and
// a pet picked up and flung flies off.
//
// The play itself (`play.rs`) is in screen points; this side asks the OS
// where things are and turns screen points into the windows' own.

use eframe::egui::{self, Context, Pos2, Rect, Vec2, pos2, vec2};
use std::time::{Duration, Instant};

use crate::components::pet::{Mood, Pet};
use crate::platform::desk;
use crate::play::{Done, Play, World};

// a turn at least this long gets a visit when it ends
const LONG_TURN: Duration = Duration::from_secs(60);
// the first number a while after launch, then one every 12 to 20 minutes
const FIRST_ACT: Duration = Duration::from_secs(90);
const ACT_EVERY: Duration = Duration::from_secs(12 * 60);
const ACT_JITTER: Duration = Duration::from_secs(8 * 60);
// how often the windows are read while something is on
const LOOK_EVERY: Duration = Duration::from_millis(400);
// a release faster than this (points per second) is a throw
const THROW_SPEED: f32 = 700.0;

/// What the stage needs from the app this frame.
pub struct Scene {
    /// just the pet, shown, with play on
    pub on: bool,
    pub mood: Mood,
    /// the pet on screen (its id), whose number it does
    pub skin: &'static str,
    /// the pet's feet, and how wide its frame is drawn, in screen points
    pub feet: Pos2,
    pub cell: f32,
    /// screen points per window point, and the window's pixels per screen point
    pub zoom: f32,
    pub ppp: f32,
    /// the monitor's size in window points, when the OS says (a fallback
    /// where the screen can't be asked for)
    pub monitor: Option<Vec2>,
}

struct Grab {
    /// from the cursor to the window's top-left
    offset: Vec2,
    /// from the window's top-left to the pet's feet
    feet: Vec2,
    trail: Vec<(Instant, Pos2)>,
}

pub struct Stage {
    play: Play,
    world: Option<World>,
    /// where the overlay goes: the screen below the menu bar (macOS keeps
    /// windows out of it, and would push the overlay down)
    canvas: Rect,
    /// where the OS last put the overlay (it may move or trim it), screen points
    shown_at: std::cell::Cell<Option<Rect>>,
    looked: Instant,
    last: Instant,
    started: Instant,
    working_since: Option<Instant>,
    next_act: Instant,
    /// CLAUDDY_PLAY=act|drive|sleigh|dare|peek|spell|float|haunt|gallop|snow|visit: start one soon after launch (dev aid)
    forced: Option<String>,
    grab: Option<Grab>,
    /// thrown: from the window's top-left to its feet, to bring the window after it
    thrown: Option<Vec2>,
}

impl Stage {
    pub fn new() -> Self {
        let now = Instant::now();
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(7, |d| d.as_nanos() as u64);
        Stage {
            play: Play::new(seed),
            world: None,
            canvas: Rect::NOTHING,
            shown_at: std::cell::Cell::new(None),
            looked: now,
            last: now,
            started: now,
            working_since: None,
            next_act: now + FIRST_ACT,
            forced: std::env::var("CLAUDDY_PLAY").ok(),
            grab: None,
            thrown: None,
        }
    }

    /// The pet is out on the screen (its window shows nothing then).
    pub fn away(&self) -> bool {
        self.play.pet_out()
    }

    /// Picked up at `cursor`; `window` and `feet` in screen points.
    pub fn grab(&mut self, cursor: Pos2, window: Rect, feet: Pos2) {
        self.grab = Some(Grab { offset: window.min - cursor, feet: feet - window.min, trail: vec![(Instant::now(), cursor)] });
    }

    /// Run a frame: triggers, the step, the overlay. Returns where the
    /// app's window should go (its top-left, screen points), if anywhere.
    pub fn frame(&mut self, ctx: &Context, s: &Scene, pet: &Pet) -> Option<Pos2> {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32();
        self.last = now;
        if !s.on {
            self.play.clear();
            self.grab = None;
            self.thrown = None;
            return None;
        }
        let mut move_to = None;

        if let Some(g) = &mut self.grab {
            let cursor = crate::platform::cursor::now()?;
            if ctx.input(|i| i.pointer.primary_down()) {
                g.trail.push((now, cursor));
                g.trail.retain(|(t, _)| now.duration_since(*t) < Duration::from_millis(120));
                ctx.request_repaint();
                return Some(cursor + g.offset);
            }
            // let go: fast enough, it flies
            let (t0, p0) = g.trail.first().copied().unwrap_or((now, cursor));
            let secs = now.duration_since(t0).as_secs_f32().max(1.0 / 120.0);
            let vel = (cursor - p0) / secs;
            let (offset, feet) = (g.offset, g.feet);
            self.grab = None;
            if vel.length() > THROW_SPEED {
                self.look(s);
                self.thrown = Some(feet);
                self.play.throw(cursor + offset + feet, vel * 0.9, s.stand_w());
            }
        }

        let away = self.play.pet_out();
        if !away && self.grab.is_none() {
            self.triggers(s, now);
        }
        self.play.focus(s.feet, s.body());
        if self.play.active() {
            if now.duration_since(self.looked) >= LOOK_EVERY || self.world.is_none() {
                self.look(s);
            }
            // the cursor, fresh every frame (the Gingerbread watches it)
            if let Some(w) = self.world.as_mut() {
                w.cursor = crate::platform::cursor::now();
            }
            let world = self.world.as_ref()?;
            match self.play.step(dt, world) {
                Some(Done::Settled(at)) => {
                    if let Some(feet) = self.thrown.take() {
                        move_to = Some(at - feet);
                    }
                }
                Some(Done::Home) | None => {}
            }
            self.show(ctx, s, pet);
            ctx.request_repaint();
        }
        move_to
    }

    // what starts something
    fn triggers(&mut self, s: &Scene, now: Instant) {
        if let Some(what) = self.forced.clone()
            && now.duration_since(self.started) > Duration::from_secs(3)
        {
            self.forced = None;
            match what.as_str() {
                "drive" => self.drive(s),
                "snow" => self.play.snow(),
                "sleigh" => self.with_world(s, |play, world, s| play.sleigh(s.feet, s.act_w(), world)),
                "dare" => self.with_world(s, |play, world, s| play.dare(s.feet, s.act_w(), world)),
                "peek" => self.peek(s),
                "spell" => self.spell(s),
                "float" => self.play.float(s.feet, s.act_w()),
                "haunt" => self.play.haunt(s.feet, s.act_w()),
                "gallop" => self.with_world(s, |play, world, s| play.gallop(s.feet, s.act_w(), world)),
                "visit" => self.visit(s, true),
                _ => self.act(s),
            }
        }
        let asleep = s.mood == Mood::Sleeping;
        // a long turn ended: Clauddy goes to tell you, on the window in front
        if s.mood == Mood::Working {
            self.working_since.get_or_insert(now);
        } else if let Some(since) = self.working_since.take()
            && now.duration_since(since) >= LONG_TURN
            && !asleep
            && s.skin == crate::components::skins::DEFAULT
        {
            self.visit(s, true);
        }
        // now and then, awake, its number
        if now >= self.next_act {
            self.next_act = now + ACT_EVERY + jitter(ACT_JITTER);
            if !asleep {
                self.act(s);
            }
        }
    }

    /// The pet's own number.
    fn act(&mut self, s: &Scene) {
        match s.skin {
            "getaway" => self.drive(s),
            "santa" => self.with_world(s, |play, world, s| play.sleigh(s.feet, s.act_w(), world)),
            "gingerbread" => self.with_world(s, |play, world, s| play.dare(s.feet, s.act_w(), world)),
            "reindeer" => self.with_world(s, |play, world, s| play.gallop(s.feet, s.act_w(), world)),
            "wizard" => self.spell(s),
            "astronaut" => self.play.float(s.feet, s.act_w()),
            "pumpkin" => self.play.haunt(s.feet, s.act_w()),
            "clauddy" => self.peek(s),
            _ => self.visit(s, false),
        }
    }

    // a fresh look at the screen, then `f` with it
    fn with_world(&mut self, s: &Scene, f: impl FnOnce(&mut Play, &World, &Scene)) {
        self.look(s);
        if let Some(world) = &self.world {
            f(&mut self.play, world, s);
        }
    }

    /// Clauddy plays peekaboo behind the window in front (wide enough, below
    /// the menu bar); with none (a window filling the screen), behind the
    /// bottom of the screen, over the Dock.
    fn peek(&mut self, s: &Scene) {
        self.look(s);
        let Some(world) = &self.world else { return };
        let floor = desk::Ledge { y: world.floor, x0: world.screen.left(), x1: world.screen.right() };
        let edge = world.ledges.iter().find(|l| l.y > world.screen.top() + s.body() * 1.5 && l.x1 - l.x0 > 5.0 * s.body()).copied().unwrap_or(floor);
        self.play.peek(s.feet, &edge, s.stand_w());
    }

    /// The Wizard poofs up onto a window in reach (or the floor) and back.
    fn spell(&mut self, s: &Scene) {
        self.look(s);
        let Some(world) = &self.world else { return };
        let half = s.body();
        let ledge = world
            .ledges
            .iter()
            .filter(|l| l.y > world.screen.top() + s.body() * 1.5 && l.x1 - l.x0 > 2.0 * half && (l.y - s.feet.y).abs() > 40.0)
            .min_by(|a, b| (a.y - s.feet.y).abs().total_cmp(&(b.y - s.feet.y).abs()));
        let spot = match ledge {
            Some(l) => pos2(((l.x0 + l.x1) / 2.0).clamp(l.x0 + half, l.x1 - half), l.y),
            None => pos2(world.screen.center().x, world.floor),
        };
        self.play.spell(s.feet, spot, s.act_w());
    }

    /// The Getaway takes the car out along the bottom of the screen.
    fn drive(&mut self, s: &Scene) {
        self.look(s);
        if let Some(world) = &self.world {
            self.play.drive(s.feet, s.act_w(), world);
        }
    }

    /// Up onto the front window (`front`) or any window in reach.
    fn visit(&mut self, s: &Scene, front: bool) {
        self.look(s);
        let Some(world) = &self.world else { return };
        let half = s.body() * 0.6;
        // a ledge on this screen, below the menu bar, wide enough, not the
        // pet's own level
        let fits = |l: &&desk::Ledge| {
            l.y > world.screen.top() + s.body() * 1.5
                && l.x1 - l.x0 > 2.0 * half
                && l.x0 >= world.screen.left() - 1.0
                && l.x1 <= world.screen.right() + 1.0
                && (l.y - s.feet.y).abs() > 24.0
        };
        let mut ledges = world.ledges.iter().filter(fits);
        let ledge = if front { ledges.next() } else { ledges.min_by(|a, b| (a.y - s.feet.y).abs().total_cmp(&(b.y - s.feet.y).abs())) };
        let Some(l) = ledge else { return };
        let spot = pos2(s.feet.x.clamp(l.x0 + half, l.x1 - half), l.y);
        self.play.visit(s.feet, spot, s.stand_w());
    }

    /// Read the screen and the windows on it.
    fn look(&mut self, s: &Scene) {
        self.looked = Instant::now();
        let (screen, usable) = desk::screen_at(s.feet).unwrap_or_else(|| {
            let size = s.monitor.map_or(vec2(1440.0, 900.0), |m| m * s.zoom);
            let screen = Rect::from_min_size(pos2((s.feet.x / size.x).floor() * size.x, (s.feet.y / size.y).floor() * size.y), size);
            (screen, Rect::from_min_max(screen.min, pos2(screen.max.x, screen.max.y - 48.0)))
        });
        // the floor: above the Dock or the taskbar, and never above the
        // pet's own feet (it lives there)
        let floor = usable.bottom().max(s.feet.y).min(screen.bottom());
        self.canvas = Rect::from_min_max(pos2(screen.left(), usable.top()), screen.max);
        // what can be seen: from the top of the overlay as the OS placed it
        let top = self.shown_at.get().map_or(usable.top(), |r| r.top().max(usable.top()));
        let screen = Rect::from_min_max(pos2(screen.left(), top), screen.max);
        let windows: Vec<Rect> = desk::windows(s.ppp).into_iter().filter(|w| w.intersects(screen)).collect();
        self.world = Some(World { screen, floor, ledges: desk::ledges(&windows), cursor: crate::platform::cursor::now() });
    }

    // the see-through window over the screen
    fn show(&self, ctx: &Context, s: &Scene, pet: &Pet) {
        let Some(world) = &self.world else { return };
        let z = s.zoom.max(0.01);
        let builder = egui::ViewportBuilder::default()
            .with_title("Clauddy")
            .with_decorations(false)
            .with_transparent(true)
            .with_mouse_passthrough(true)
            .with_window_level(egui::WindowLevel::AlwaysOnTop)
            .with_taskbar(false)
            .with_active(false)
            .with_resizable(false)
            .with_has_shadow(false)
            .with_position(self.canvas.min / z)
            .with_inner_size(self.canvas.size() / z);
        let id = egui::ViewportId::from_hash_of("clauddy-stage");
        ctx.show_viewport_immediate(id, builder, |ui, _| {
            // where the OS actually put it: macOS keeps windows inside the
            // usable area and may move or trim it
            let shown = ui.ctx().input(|i| i.viewport().inner_rect).map(|r| Rect::from_min_size((r.min.to_vec2() * z).to_pos2(), r.size() * z));
            self.shown_at.set(shown);
            let origin = shown.map_or(self.canvas.min, |r| r.min);
            self.play.draw(ui.painter(), origin, 1.0 / z, pet, s.mood, world);
        });
    }
}

impl Scene {
    /// The pet standing, as wide as its standing art (the hops, the throw).
    fn stand_w(&self) -> f32 {
        let b = crate::components::skins::find(self.skin).map_or([0.18, 0.27, 0.84, 0.79], |k| k.stand_box);
        self.cell * (b[2] - b[0])
    }

    /// Its own number's art, as wide as it is drawn.
    fn act_w(&self) -> f32 {
        crate::components::skins::find(self.skin).map_or(self.stand_w(), |k| self.cell * k.act_scale * (k.act_box[2] - k.act_box[0]))
    }

    /// The classic pet's body, for distances and room to stand.
    fn body(&self) -> f32 {
        self.cell * 0.66
    }
}

// up to `max`, different each time
fn jitter(max: Duration) -> Duration {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.subsec_nanos());
    max.mul_f64(nanos as f64 / 1e9)
}
