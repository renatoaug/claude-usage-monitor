// The showcase video's clips: the real screens drawn frame by frame from
// the snapshot gallery's fixtures (no real account ever shows up), at 30 fps,
// on a transparent background. tools/showcase turns each folder of PNGs into
// a clip with alpha and lays them out in the video.
//
//   SHOWCASE_OUT=../tools/showcase/tmp cargo test --release showcase -- --ignored
//   SHOWCASE_ONLY=picker,talk ...   only these clips

use eframe::egui::{self, Rect, Vec2};
use egui_kittest::Harness;

use super::snapshots::{freeze_clock, limits, snapshot, usage, with_codex, with_cursor};
use super::{Frame, Size, View, compact, expanded, pet_only, settings};
use crate::auth::Limits;
use crate::components::logos::Logos;
use crate::components::pet::{Mood, Pet};
use crate::components::skins::SKINS;
use crate::poller::Snapshot;
use crate::prefs::Prefs;

const FPS: f32 = 30.0;

/// One frame of a clip: what's on screen.
struct Shot {
    snap: Snapshot,
    mood: Mood,
    view: View,
    pet: &'static str,
    /// pet only: the glance is up
    glance: bool,
    menu_bar: bool,
    /// the speech bubble and how far it has faded in
    bubble: Option<(String, f32)>,
    draft: Prefs,
}

fn shot(snap: Snapshot, mood: Mood, view: View) -> Shot {
    Shot { snap, mood, view, pet: "clauddy", glance: false, menu_bar: false, bubble: None, draft: Prefs::default() }
}

fn compact_view() -> View {
    View { size: Size::Compact, ..View::default() }
}

fn pet_view() -> View {
    View { size: Size::PetOnly, ..View::default() }
}

fn snap(active: bool, l: Limits) -> Snapshot {
    snapshot(usage(active, None), Some(l))
}

fn size_of(s: &Shot) -> Vec2 {
    let pet = Pet::frozen(&egui::Context::default(), 0.0);
    let logos = Logos::load(&egui::Context::default());
    let f = frame(s, &pet, &logos, Rect::NOTHING);
    match (s.view.settings, s.view.size) {
        (Some(tab), _) => settings::size(f.snap, tab, f.view),
        (None, Size::Compact) => compact::size(compact::rows(&f).len(), compact::reminder(&f).is_some()),
        (None, Size::Expanded) => expanded::size(f.snap, f.view, f.session, f.heads_up),
        (None, Size::PetOnly) => pet_only::size(),
    }
    .into()
}

fn frame<'a>(s: &'a Shot, pet: &'a Pet, logos: &'a Logos, area: Rect) -> Frame<'a> {
    Frame {
        area,
        card: area,
        snap: &s.snap,
        mood: s.mood,
        session: s.snap.limits.as_ref().and_then(|l| crate::components::meter::live(&l.session, None)),
        view: &s.view,
        pet,
        logos,
        menu_bar: s.menu_bar,
        heads_up: 80.0,
        bubble: s.bubble.as_ref().map(|(t, a)| (t.as_str(), *a)),
        update: Default::default(),
    }
}

struct State {
    shot: Shot,
    t: f32,
    assets: Option<(Pet, Logos)>,
}

fn draw(ui: &mut egui::Ui, st: &mut State) {
    let ctx = ui.ctx().clone();
    let (pet, logos) = st.assets.get_or_insert_with(|| (Pet::frozen(&ctx, 0.0), Logos::load(&ctx)));
    pet.set_time(st.t);
    let s = &st.shot;
    if let Some(skin) = SKINS.iter().find(|k| k.id == s.pet) {
        pet.set_skin(skin);
    }
    let area = ctx.content_rect().shrink(super::MARGIN);
    let f = frame(s, pet, logos, area);
    let _ = match (s.view.settings, s.view.size) {
        (Some(tab), _) => settings::draw(ui, &ctx, &f, tab, &s.draft, &Prefs::default(), &mut String::new()),
        (None, Size::Compact) => compact::draw(ui, &ctx, &f),
        (None, Size::Expanded) => expanded::draw(ui, &ctx, &f, &mut String::new()),
        (None, Size::PetOnly) => pet_only::draw(ui, &ctx, &f, Some(s.glance)).actions,
    };
}

fn write_png(path: &std::path::Path, w: u32, h: u32, rgba: &[u8]) {
    let file = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    let mut enc = png::Encoder::new(file, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(rgba).unwrap();
}

/// Render `frames` frames of a clip into `<out>/<name>/0000.png...`.
fn record(name: &str, frames: usize, ppp: f32, scene: impl Fn(usize) -> Shot) {
    let Some(out) = std::env::var_os("SHOWCASE_OUT") else { return };
    if let Ok(only) = std::env::var("SHOWCASE_ONLY")
        && !only.is_empty()
        && !only.split(',').any(|n| n == name)
    {
        return;
    }
    let dir = std::path::Path::new(&out).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let first = scene(0);
    let size = size_of(&first);
    let mut harness = Harness::builder().with_size(size).with_pixels_per_point(ppp).build_ui_state(draw, State { shot: first, t: 0.0, assets: None });
    for i in 0..frames {
        if i > 0 {
            harness.state_mut().shot = scene(i);
        }
        harness.state_mut().t = i as f32 / FPS;
        harness.run();
        let img = harness.render().unwrap();
        write_png(&dir.join(format!("{i:04}.png")), img.width(), img.height(), img.as_raw());
    }
    eprintln!("{name}: {frames} frames, {}x{}", (size.x * ppp) as u32, (size.y * ppp) as u32);
}

fn secs(s: f32) -> usize {
    (s * FPS) as usize
}

#[test]
#[ignore = "renders the showcase video's clips; run with SHOWCASE_OUT set"]
fn showcase() {
    freeze_clock();
    let working = || snap(true, limits(63.0, 41.0));

    // the hook: a pet that has hit the wall
    record("maxed", secs(5.5), 4.0, |_| shot(snap(false, limits(100.0, 72.0)), Mood::Tired, pet_view()));
    // the reveal and the outro
    record("hello", secs(10.0), 4.0, |_| shot(snap(false, limits(24.0, 30.0)), Mood::Idle, pet_view()));

    // every size, from its entrance to the end of the lineup
    record("expanded", secs(22.0), 3.0, |_| shot(working(), Mood::Working, View::default()));
    record("compact", secs(18.0), 3.0, |_| shot(working(), Mood::Working, compact_view()));
    record("menubar", secs(10.0), 3.0, |_| Shot { menu_bar: true, ..shot(working(), Mood::Working, compact_view()) });
    // pet only: the cursor comes over it in the spotlight, again in the lineup
    record("petonly", secs(14.0), 4.0, |i| Shot { glance: (25..100).contains(&i) || (255..360).contains(&i), ..shot(working(), Mood::Working, pet_view()) });

    // the picker: the pick walks through every pet, then settles on the wizard
    let order = ["clauddy", "wizard", "astronaut", "santa", "reindeer", "gingerbread", "pumpkin", "wizard"];
    record("picker", secs(10.0), 3.0, |i| {
        let k = (i.saturating_sub(30) / 30).min(order.len() - 1);
        let view = View { settings: Some(settings::Tab::Appearance), pet_picker: true, ..View::default() };
        Shot { draft: Prefs { pet: order[k].into(), ..Prefs::default() }, ..shot(working(), Mood::Idle, view) }
    });
    // all of them, side by side
    // the holidays' pets, on their own
    for (pet, mood) in [("santa", Mood::Working), ("reindeer", Mood::Idle), ("gingerbread", Mood::Working), ("pumpkin", Mood::Idle)] {
        record(pet, secs(8.0), 4.0, move |_| Shot { pet, ..shot(snap(mood == Mood::Working, limits(63.0, 41.0)), mood, pet_view()) });
    }

    // moods
    record("working", secs(8.0), 3.0, |_| shot(working(), Mood::Working, compact_view()));
    record("fire", secs(8.0), 3.0, |_| shot(snap(false, limits(94.0, 48.0)), Mood::Fire, compact_view()));
    record("sleeping", secs(8.0), 3.0, |_| {
        let mut u = usage(false, None);
        u.sleeping = true;
        shot(snapshot(u, Some(limits(18.0, 41.0))), Mood::Sleeping, compact_view())
    });

    // the real numbers: the session ticks up while it works
    record("numbers", secs(10.0), 2.5, |i| {
        let pct = if i < 60 {
            58.0
        } else if i < 120 {
            61.0
        } else {
            63.0
        };
        shot(snap(true, limits(pct, 41.0)), Mood::Working, View::default())
    });

    // the voice: four remarks, one after another
    let remarks = [
        (10, "Good morning! You spent 41.2M tokens yesterday.", Mood::Idle, 24.0),
        (105, "Getting warm, 91% already. It resets in 1h 12m.", Mood::Fire, 91.0),
        (200, "Fresh window! That was 34.0M tokens over 4h 10m.", Mood::Idle, 3.0),
        (295, "You've been at it for 2h 30m straight. Stretch break?", Mood::Working, 47.0),
    ];
    record("talk", secs(13.0), 3.0, |i| {
        let k = remarks.iter().rposition(|r| i >= r.0).unwrap_or(0);
        let (at, text, mood, pct) = remarks[k];
        let since = i.saturating_sub(at) as f32 / FPS;
        // fades in over 0.15 s, holds, and goes 0.15 s before the next one
        let until = remarks.get(k + 1).map_or(f32::MAX, |r| (r.0 - at) as f32 / FPS - 0.4);
        let shown = if i < at { 0.0 } else { (since / 0.15).min(1.0).min(((until - since) / 0.15).max(0.0)) };
        Shot { pet: "wizard", bubble: Some((text.into(), shown)), ..shot(snap(mood == Mood::Working, limits(pct, 41.0)), mood, View::default()) }
    });

    // Claude, Codex and Cursor
    record("trio", secs(6.5), 3.0, |_| Shot {
        pet: "wizard",
        ..shot(with_cursor(with_codex(snap(false, limits(63.0, 41.0)), false), true), Mood::Working, compact_view())
    });
    // where the week went
    record("details", secs(7.5), 2.5, |_| shot(working(), Mood::Working, View { details: true, ..View::default() }));
}
