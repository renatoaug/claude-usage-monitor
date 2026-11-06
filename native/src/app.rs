// The app shell: it owns the pet, the logos, the poll and the remembered
// view; each frame it works out the mood, hands a screen its data, and
// applies what the screen returns. It repaints only when the pet's next frame
// is due, a reading lands, or a clock ticks over.

use eframe::egui::{self, Context, Vec2};
use std::time::{Duration, Instant};

use crate::components::logos::Logos;
use crate::components::meter::live;
use crate::components::pet::{Mood, Pet};
use crate::poller::{self, Command, Settings, Shared, Snapshot};
use std::sync::mpsc::Sender;
use crate::prefs::Prefs;
use crate::screens::{Action, Frame, MARGIN, Size, View, compact, expanded, pet_only, settings};
use crate::ui::{motion, theme};
use crate::usage::Usage;

pub struct App {
    pet: Pet,
    logos: Logos,
    data: Shared,
    commands: Sender<Command>,
    /// the login code being pasted
    code: String,
    /// what config.json holds, and the copy Settings edits
    prefs: Prefs,
    draft: Prefs,
    settings: Settings,
    pinned: Option<Mood>,
    view: View,
    size: [f32; 2],
    started: Instant,
    shot_sent: bool,
    /// the cursor, watched while just the pet is up
    cursor: crate::platform::cursor::Watch,
    /// whether clicks fall through the window (just the pet, off the pet)
    passthrough: bool,
    /// the menu-bar icon, while Clauddy lives there
    tray: Option<crate::platform::tray::Tray>,
    /// the panel is open under the menu-bar icon; `focused` once it took focus
    popover: bool,
    popover_focused: bool,
    /// the window is shown (it hides while Clauddy lives in the menu bar)
    shown: bool,
    /// the screen on show (Settings tab, size) and when it came in, to fade it in
    screen: (Option<settings::Tab>, Size),
    entered: Instant,
    /// the panel fading away on its way to the menu bar
    leaving: Option<Instant>,
    /// the window as asked of the OS (larger than `size` while the card morphs)
    window: [f32; 2],
    /// the pixels per point the window was last sized at
    sized_at: f32,
    /// the pet's voice: what it noticed, and what it's saying now
    talk: crate::talk::Talk,
    heard: u64,
    said: Option<Said>,
    /// the card's size as it shows now, and the morph under way (from, started)
    card_now: Vec2,
    morph: Option<(Vec2, Instant)>,
}

/// Working wins; otherwise the session's heat; otherwise rest.
pub fn mood_for(usage: Option<&Usage>, session_pct: Option<f64>, fire: f64) -> Mood {
    if usage.is_some_and(|u| u.active) {
        return Mood::Working;
    }
    match session_pct {
        Some(p) if p >= 100.0 => Mood::Tired,
        Some(p) if p >= fire => Mood::Fire,
        _ if usage.is_some_and(|u| u.sleeping) => Mood::Sleeping,
        _ => Mood::Idle,
    }
}

/// The mood across every connected service: any of them working wins; with
/// none working, the hottest session sets it (an expired Codex or Cursor
/// reading doesn't count; Cursor's session is its billing month). Claude's
/// logs alone say when it's asleep.
pub fn mood_across(snap: &Snapshot, claude: Option<(f64, Option<i64>)>, fire: f64) -> Mood {
    if !crate::screens::workers(snap).is_empty() {
        return Mood::Working;
    }
    let hottest = crate::screens::sessions(snap, claude).into_iter().filter_map(|s| s.1).fold(None, |m: Option<f64>, p| Some(m.map_or(p, |m| m.max(p))));
    mood_for(snap.usage.as_ref(), hottest, fire)
}

/// A remark on screen: the words for each size, and when it goes.
struct Said {
    text: String,
    short: String,
    since: Instant,
    until: Instant,
}

impl Said {
    /// 0..1 as it appears and goes (0.15 s each way)
    fn shown(&self) -> f32 {
        let now = Instant::now();
        if now >= self.until {
            return (1.0 - now.duration_since(self.until).as_secs_f32() / 0.15).max(0.0);
        }
        (now.duration_since(self.since).as_secs_f32() / 0.15).min(1.0)
    }
}

/// The window's size at launch, from the remembered view, before any reading.
/// On macOS the window is a fixed, transparent canvas the card lives in: it
/// never resizes (a resize shows one empty frame, which reads as a blink),
/// and clicks off the card fall through to what's behind. Elsewhere the
/// window fits the card.
const FIXED_CANVAS: bool = cfg!(target_os = "macos");
/// Wide enough for the expanded panel, tall enough for its Details open
/// or any Settings tab; it grows if a screen ever needs more.
const CANVAS: [f32; 2] = [292.0, 780.0];

pub fn initial_size() -> [f32; 2] {
    // the menu bar's popover is an ordinary window the card's size
    if FIXED_CANVAS && !Prefs::load().menu_bar {
        return CANVAS;
    }
    let view = load_view();
    let size = if Prefs::load().menu_bar { Size::Expanded } else { view.size };
    match size {
        Size::Compact => compact::size(1, false),
        Size::Expanded => expanded::size(&Snapshot::default(), &view, None, 80.0),
        Size::PetOnly => pet_only::size(),
    }
}

fn view_path() -> std::path::PathBuf {
    crate::auth::default_data_dir().join("native-ui.json")
}

fn load_view() -> View {
    std::fs::read(view_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn save_view(v: &View) {
    if let Ok(body) = serde_json::to_string_pretty(v) {
        let _ = std::fs::write(view_path(), body);
    }
}

/// Open a URL in the default browser.
pub fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let (cmd, args) = ("open", vec![url]);
    #[cfg(target_os = "windows")]
    let (cmd, args) = ("cmd", vec!["/C", "start", "", url]);
    #[cfg(all(unix, not(target_os = "macos")))]
    let (cmd, args) = ("xdg-open", vec![url]);
    let _ = std::process::Command::new(cmd).args(args).spawn();
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install_font(&cc.egui_ctx);
        crate::alerts::ask_permission();
        let settings = Settings::load();
        let (data, commands) = poller::spawn(cc.egui_ctx.clone(), settings.clone());
        App {
            pet: Pet::new(&cc.egui_ctx),
            logos: Logos::load(&cc.egui_ctx),
            data,
            commands,
            code: String::new(),
            prefs: Prefs::load(),
            draft: Prefs::load(),
            settings,
            pinned: Mood::from_env(),
            view: load_view(),
            size: [0.0, 0.0],
            started: Instant::now(),
            shot_sent: false,
            cursor: crate::platform::cursor::Watch::new(&cc.egui_ctx),
            passthrough: false,
            tray: None,
            popover: false,
            popover_focused: false,
            shown: !Prefs::load().menu_bar,
            screen: (None, Size::Expanded),
            entered: Instant::now(),
            leaving: None,
            window: initial_size(),
            sized_at: 0.0,
            talk: crate::talk::Talk::new(Some(crate::auth::default_data_dir().join("native-voice.json"))),
            heard: 0,
            said: None,
            card_now: Vec2::ZERO,
            morph: None,
        }
    }

    fn apply(&mut self, ctx: &Context, actions: Vec<Action>) {
        for a in actions {
            match a {
                Action::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Action::Resize(s) => self.view.size = s,
                Action::OpenUsagePage => open_url("https://claude.ai/settings/usage"),
                Action::ToggleDetails => self.view.details = !self.view.details,
                Action::ToggleModels => self.view.models = !self.view.models,
                Action::ToggleProjects => self.view.projects = !self.view.projects,
                Action::ToggleAccountMenu => {
                    self.view.account_menu = !self.view.account_menu;
                    self.view.pending_remove = None;
                }
                Action::CloseAccountMenu => {
                    self.view.account_menu = false;
                    self.view.pending_remove = None;
                }
                Action::SwitchAccount(id) => {
                    self.view.account_menu = false;
                    self.send(Command::Switch(id));
                }
                Action::AskRemoveAccount(id) => self.view.pending_remove = Some(id),
                Action::RemoveAccount(id) => {
                    self.view.pending_remove = None;
                    self.send(Command::Remove(id));
                }
                Action::AddAccount => {
                    self.view.account_menu = false;
                    self.code.clear();
                    self.send(Command::Add);
                }
                Action::BeginLogin => {
                    self.code.clear();
                    self.send(Command::BeginLogin);
                }
                Action::ToggleSettings => {
                    // opening starts a fresh draft; closing drops unsaved edits
                    self.draft = self.prefs.clone();
                    self.view.settings = if self.view.settings.is_some() { None } else { Some(settings::Tab::Accounts) };
                    self.view.account_menu = false;
                }
                Action::SettingsTab(t) => self.view.settings = Some(t),
                Action::EditPrefs(p) => self.draft = p,
                Action::SaveSettings => {
                    // moving to the menu bar: let the panel fade out first
                    if self.draft.menu_bar && !self.prefs.menu_bar {
                        self.leaving = Some(Instant::now());
                    }
                    self.draft.save();
                    self.prefs = self.draft.clone();
                    self.settings.fire_threshold = self.prefs.fire as f64;
                    self.send(Command::Prefs(self.prefs.clone()));
                    self.view.settings = None;
                }
                Action::Logout => self.send(Command::Logout),
                Action::SelectProvider(p) => self.view.provider = p,
                Action::SetCodex(on) => {
                    // connecting a service takes effect at once, like logging in
                    self.prefs.codex = on;
                    self.draft.codex = on;
                    self.prefs.save();
                    self.send(Command::Prefs(self.prefs.clone()));
                    if on {
                        self.view.provider = crate::components::logos::Provider::Codex;
                    }
                }
                Action::Remind(p, on) => self.send(Command::Remind(p, on)),
                Action::HidePopover => self.close_popover(ctx),
                Action::ToggleMute => {
                    let now = crate::usage::now_ms();
                    let until = if self.prefs.muted_until > now { 0 } else { now + 3_600_000 };
                    self.prefs.muted_until = until;
                    self.draft.muted_until = until;
                    self.prefs.save();
                }
                Action::DismissBubble => {
                    if let Some(s) = &mut self.said {
                        s.until = s.until.min(Instant::now());
                    }
                }
                Action::SetCursor(on) => {
                    self.prefs.cursor = on;
                    self.draft.cursor = on;
                    self.prefs.save();
                    self.send(Command::Prefs(self.prefs.clone()));
                    if on {
                        self.view.provider = crate::components::logos::Provider::Cursor;
                    }
                }
                Action::SubmitCode => {
                    let code = std::mem::take(&mut self.code);
                    self.send(Command::CompleteLogin(code));
                }
            }
            save_view(&self.view);
        }
    }

    /// Hear one poll. A remark is said when the voice is on, nothing covers
    /// the pet (Settings, pet-only), and the gap allows; only headlines blip,
    /// and never in the menu bar or the compact face, nor while muted.
    fn listen(&mut self, snap: &Snapshot, mood: crate::components::pet::Mood, session: Option<(f64, Option<i64>)>, size: Size) {
        let now = crate::usage::now_ms();
        let hour = chrono::Timelike::hour(&chrono::Local::now());
        let seen = crate::talk::Seen { snap, mood, claude: session, fire: self.settings.fire_threshold, hour };
        let Some(r) = self.talk.listen(&seen, now) else { return };
        let quiet = !self.prefs.talk || self.view.settings.is_some() || size == Size::PetOnly;
        if quiet || !self.talk.allow(&r, now) {
            return;
        }
        let words = if size == Size::Compact { &r.line.short } else { &r.line.text };
        let start = Instant::now();
        let stay = Duration::from_millis((words.len() as u64 * 70 + 2500).max(4500));
        self.said = Some(Said { text: r.line.text.clone(), short: r.line.short.clone(), since: start, until: start + stay });
        if r.headline && self.prefs.sound && !self.prefs.menu_bar && size == Size::Expanded && self.prefs.muted_until <= now {
            crate::voice::play(&r.line.text, r.tone);
        }
    }

    fn show(&mut self, ctx: &Context, on: bool) {
        if on != self.shown {
            self.shown = on;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(on));
        }
    }

    /// Open the panel under the menu-bar icon (`icon`: its box, physical px).
    fn open_popover(&mut self, ctx: &Context, icon: Option<egui::Rect>) {
        let ppp = ctx.pixels_per_point();
        if let Some(r) = icon {
            let x = r.center().x / ppp - self.size[0] / 2.0; // the card's width, centred under the icon
            let y = r.bottom() / ppp + 2.0;
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x.round(), y.round())));
        }
        self.popover = true;
        self.popover_focused = false;
        if self.passthrough {
            self.passthrough = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(false));
        }
        self.entered = Instant::now();
        self.show(ctx, true);
        crate::platform::activate();
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn close_popover(&mut self, ctx: &Context) {
        self.popover = false;
        self.view.account_menu = false;
        // always say it: eframe shows a window after its first frame even
        // when it was built hidden, so `shown` can't be trusted here
        self.shown = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    /// Floating or in the menu bar: put the icon up or take it down, and
    /// answer what was clicked there.
    fn menu_bar(&mut self, ctx: &Context, snap: &Snapshot, session: Option<(f64, Option<i64>)>) {
        use crate::platform::tray::{Tray, TrayEvent, line};
        let wanted = self.prefs.menu_bar && self.leaving.is_none();
        if wanted && self.tray.is_none() {
            self.tray = Tray::new(ctx);
            if self.tray.is_some() {
                self.close_popover(ctx);
            }
        } else if !wanted && self.tray.is_some() {
            self.tray = None;
            self.popover = false;
            self.show(ctx, true);
        }
        let Some(tray) = &mut self.tray else { return };
        // the session closest to its limit; 🔥 past the fire threshold
        let sessions: Vec<_> = crate::screens::sessions(snap, session).into_iter().map(|s| s.1).collect();
        let text = crate::platform::tray::title_hottest(&sessions, self.settings.fire_threshold);
        tray.set_title(&text);
        // the menu leads with the limits, as the panel does
        let mut lines = vec![line("Session", session.map(|s| s.0), session.and_then(|s| s.1))];
        if let Some(l) = &snap.limits {
            let week = live(&l.week, snap.limits_at);
            lines.push(line("Weekly", week.map(|w| w.0), week.and_then(|w| w.1)));
            for sc in &l.scoped {
                let w = live(&crate::auth::Window { pct: sc.pct, reset_ms: sc.reset_ms }, snap.limits_at);
                lines.push(line(&format!("{} weekly", sc.label), w.map(|w| w.0), w.and_then(|w| w.1)));
            }
        }
        // the other services, each with its own windows
        let ext = |w: &Option<crate::codex::Window>| (w.as_ref().and_then(|w| w.pct), w.as_ref().and_then(|w| w.reset_ms));
        if let Some(c) = &snap.codex {
            let (p, r) = ext(&c.session);
            lines.push(line(if c.weekly_only { "Codex weekly" } else { "Codex session" }, p, r));
            if !c.weekly_only {
                let (p, r) = ext(&c.weekly);
                lines.push(line("Codex weekly", p, r));
            }
        }
        if let Some(c) = &snap.cursor {
            let (p, r) = ext(&c.session);
            lines.push(line("Cursor usage", p, r));
            let (p, r) = ext(&c.api);
            lines.push(line("Cursor API", p, r));
        }
        tray.set_lines(lines);
        let events = tray.take_events();
        let icon = tray.rect();
        for e in events {
            match e {
                TrayEvent::Open => self.open_popover(ctx, icon),
                TrayEvent::Floating => {
                    self.prefs.menu_bar = false;
                    self.draft.menu_bar = false;
                    self.prefs.save();
                }
            }
        }
        // a click anywhere else takes focus away: the popover goes
        let focused = ctx.input(|i| i.viewport().focused);
        if self.popover && focused == Some(true) {
            self.popover_focused = true;
        }
        if self.popover && self.popover_focused && focused == Some(false) {
            self.close_popover(ctx);
        }
    }

    fn send(&self, cmd: Command) {
        let _ = self.commands.send(cmd);
    }

    // CLAUDDY_SHOT=path.png: save one frame after 8 s, then quit (dev aid)
    fn shoot(&mut self, ctx: &Context) {
        let Ok(path) = std::env::var("CLAUDDY_SHOT") else { return };
        if self.started.elapsed().as_secs_f32() > 8.0 && !self.shot_sent {
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
}

impl eframe::App for App {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let snap: Snapshot = self.data.lock().map(|s| s.clone()).unwrap_or_default();
        let session = snap.limits.as_ref().and_then(|l| live(&l.session, snap.limits_at));
        let mood = self
            .pinned
            .unwrap_or_else(|| mood_across(&snap, session, self.settings.fire_threshold));
        let size = if self.prefs.menu_bar { Size::Expanded } else { self.view.size };
        let screen = (self.view.settings, size);
        // the zoom: the draft's while Settings is open, so a step shows at once
        let zoom = if self.view.settings.is_some() { self.draft.zoom } else { self.prefs.zoom } as f32 / 100.0;
        if ctx.zoom_factor() != zoom {
            ctx.set_zoom_factor(zoom);
        }
        // it takes hold a frame later; then the window, sized in points, is
        // asked for again in the new pixels
        let ppp = ctx.pixels_per_point();
        if self.sized_at != ppp {
            self.sized_at = ppp;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(self.window.into()));
        }

        // the settled window for this screen
        let want = match screen {
            (Some(tab), _) => settings::size(&snap, tab, &self.view),
            (None, Size::Compact) => {
                let services = crate::screens::connected(&snap);
                let shown = if services.contains(&self.view.provider) { self.view.provider } else { crate::components::logos::Provider::Claude };
                let heads_up = self.prefs.heads_up.min(self.prefs.near_limit) as f64;
                let remind = snap.loaded && crate::components::reminder::state(&snap, shown, session, heads_up).is_some();
                compact::size(services.len(), remind)
            }
            (None, Size::Expanded) => expanded::size(&snap, &self.view, session, self.prefs.heads_up.min(self.prefs.near_limit) as f64),
            (None, Size::PetOnly) => pet_only::size(),
        };
        let card_of = |s: [f32; 2]| Vec2::new(s[0] - 2.0 * MARGIN, s[1] - 2.0 * MARGIN);

        // A new size morphs the card itself: the window (transparent) takes
        // the larger of the two sizes at once, the card eases from the old
        // size to the new one inside it, revealing the new content as it
        // opens, and the window settles to the exact size after. Nothing
        // waits on the OS resizing a window frame by frame.
        if want != self.size {
            let morph = self.size != [0.0, 0.0] && size != Size::PetOnly && self.screen.1 != Size::PetOnly && self.leaving.is_none();
            let fixed = FIXED_CANVAS && !self.prefs.menu_bar;
            let window = if fixed {
                [CANVAS[0].max(want[0]).max(self.window[0]), CANVAS[1].max(want[1]).max(self.window[1])]
            } else if morph {
                [self.window[0].max(want[0]), self.window[1].max(want[1])]
            } else {
                want
            };
            self.morph = morph.then(|| (self.card_now, Instant::now()));
            self.size = want;
            if window != self.window {
                self.window = window;
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(window.into()));
            }
        }
        let settled = card_of(want);
        match self.morph {
            Some((from, start)) => {
                let t = motion::progress(start, motion::MORPH_S);
                self.card_now = from + (settled - from) * t;
                if t >= 1.0 {
                    self.morph = None;
                    if !(FIXED_CANVAS && !self.prefs.menu_bar) && self.window != want {
                        self.window = want;
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(want.into()));
                    }
                } else {
                    ctx.request_repaint();
                }
            }
            None => self.card_now = settled,
        }

        // the new content comes up from part-way, never from nothing (no
        // blink); the panel fades all the way out only on its way to the menu bar
        if screen != self.screen {
            self.screen = screen;
            self.entered = Instant::now();
        }
        let mut fade = 0.55 + 0.45 * motion::progress(self.entered, motion::ENTER_S);
        if let Some(t) = self.leaving {
            fade = 1.0 - motion::progress(t, motion::LEAVE_S);
            if fade <= 0.0 {
                self.leaving = None;
            }
        }
        let origin = ctx.content_rect().min + Vec2::splat(MARGIN);
        let area = egui::Rect::from_min_size(origin, settled);
        let card = egui::Rect::from_min_size(origin, self.card_now);
        // the card stays solid while its content fades: never a see-through moment
        if size != Size::PetOnly && self.leaving.is_none() {
            // the shown service's shade in the expanded panel
            let shown = self.view.provider;
            let tint = if size == Size::Expanded && self.view.settings.is_none() && crate::screens::connected(&snap).contains(&shown) {
                shown.surface()
            } else {
                crate::ui::surface::Tint::Claude
            };
            crate::ui::surface::card_backdrop(ui.painter(), card, tint);
        }
        if fade < 1.0 {
            ui.multiply_opacity(fade);
            ctx.request_repaint();
        }
        if self.morph.is_some() {
            // the content shows only inside the card as it opens or closes
            ui.set_clip_rect(card.expand(1.0));
        }
        // a new poll: what the pet makes of it
        if snap.seq != self.heard {
            self.heard = snap.seq;
            self.listen(&snap, mood, session, size);
        }
        let shown = self.said.as_ref().map_or(0.0, Said::shown);
        if shown <= 0.0 && self.said.as_ref().is_some_and(|s| Instant::now() > s.until) {
            self.said = None;
        } else if let Some(s) = &self.said {
            // through the fades every frame; while it rests, wake when it goes
            let rest = s.until.saturating_duration_since(Instant::now());
            ctx.request_repaint_after(if shown < 1.0 || rest.is_zero() { Duration::ZERO } else { rest });
        }
        let bubble = self.said.as_ref().map(|s| (if size == Size::Compact { s.short.as_str() } else { s.text.as_str() }, shown));
        let f = Frame {
            area,
            card,
            snap: &snap,
            mood,
            session,
            view: &self.view,
            pet: &self.pet,
            logos: &self.logos,
            menu_bar: self.prefs.menu_bar,
            heads_up: self.prefs.heads_up.min(self.prefs.near_limit) as f64,
            bubble,
            mute: (self.prefs.talk && self.prefs.sound && !self.prefs.menu_bar).then(|| self.prefs.muted_until > crate::usage::now_ms()),
        };

        // the areas that take the pointer; clicks anywhere else fall through
        let mut hot = vec![card];
        let mut actions = match screen {
            (Some(tab), _) => settings::draw(ui, &ctx, &f, tab, &self.draft, &self.prefs, &mut self.code),
            (None, Size::Compact) => compact::draw(ui, &ctx, &f),
            (None, Size::Expanded) => expanded::draw(ui, &ctx, &f, &mut self.code),
            (None, Size::PetOnly) => {
                // the OS's answer where there is one; else what this window sees
                let seen = ctx.input(|i| i.pointer.hover_pos().is_some());
                let over = if cfg!(target_os = "macos") { Some(self.cursor.inside(seen)) } else { None };
                let out = pet_only::draw(ui, &ctx, &f, over);
                hot = out.hot;
                out.actions
            }
        };
        if self.prefs.menu_bar {
            actions.retain(|a| !matches!(a, Action::Resize(_)));
        }
        // just the pet: clicks off the pet fall through to what's behind
        let watching = ((FIXED_CANVAS && !self.prefs.menu_bar) || size == Size::PetOnly) && self.shown && cfg!(target_os = "macos");
        if watching {
            let at = ctx.input(|i| i.viewport().inner_rect).map(|r| r.min.to_vec2()).unwrap_or_default();
            // window points are zoomed; the screen's are not
            let z = ctx.zoom_factor();
            self.cursor.watch(Some(hot.iter().map(|r| r.translate(at) * z).collect()));
        } else {
            self.cursor.watch(None);
        }
        let through = watching && !self.cursor.inside(false);
        if through != self.passthrough {
            self.passthrough = through;
            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(through));
        }
        self.apply(&ctx, actions);
        self.menu_bar(&ctx, &snap, session);
        self.shoot(&ctx);
        let mut next = self.pet.next_frame(mood).min(Duration::from_secs(30));
        // a refused reminder's note goes away on time
        if let Some(left) = crate::components::reminder::error_left(&snap) {
            next = next.min(left);
        }
        ctx.request_repaint_after(next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(active: bool, sleeping: bool) -> Usage {
        let mut u = crate::usage::Reader::new(Some(std::env::temp_dir().join("clauddy-no-such-dir"))).get_usage(&Default::default());
        u.active = active;
        u.sleeping = sleeping;
        u
    }

    #[test]
    fn working_wins_then_heat_then_rest() {
        assert_eq!(mood_for(Some(&reading(true, false)), Some(100.0), 90.0), Mood::Working);
        assert_eq!(mood_for(Some(&reading(false, false)), Some(100.0), 90.0), Mood::Tired);
        assert_eq!(mood_for(Some(&reading(false, true)), Some(95.0), 90.0), Mood::Fire);
        assert_eq!(mood_for(Some(&reading(false, true)), Some(40.0), 90.0), Mood::Sleeping);
        assert_eq!(mood_for(Some(&reading(false, false)), None, 90.0), Mood::Idle);
        assert_eq!(mood_for(None, None, 90.0), Mood::Idle);
    }
}
