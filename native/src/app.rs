// The app shell: it owns the pet, the logos, the poll and the remembered
// view; each frame it works out the mood, hands a screen its data, and
// applies what the screen returns. It repaints only when the pet's next frame
// is due, a reading lands, or a clock ticks over.

use eframe::egui::{self, Context};
use std::time::{Duration, Instant};

use crate::components::logos::Logos;
use crate::components::meter::live;
use crate::components::pet::{Mood, Pet};
use crate::poller::{self, Command, Settings, Shared, Snapshot};
use std::sync::mpsc::Sender;
use crate::prefs::Prefs;
use crate::screens::{Action, Frame, Size, View, compact, expanded, pet_only, settings};
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
    /// the window's size as it eases toward `size`: (from, started)
    resize: Option<([f32; 2], Instant)>,
    current: [f32; 2],
    /// the panel fading away on its way to the menu bar
    leaving: Option<Instant>,
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

/// The window's size at launch, from the remembered view, before any reading.
pub fn initial_size() -> [f32; 2] {
    let view = load_view();
    let size = if Prefs::load().menu_bar { Size::Expanded } else { view.size };
    match size {
        Size::Compact => compact::size(1),
        Size::Expanded => expanded::size(&Snapshot::default(), &view),
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
            resize: None,
            current: [0.0, 0.0],
            leaving: None,
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
                Action::SubmitCode => {
                    let code = std::mem::take(&mut self.code);
                    self.send(Command::CompleteLogin(code));
                }
            }
            save_view(&self.view);
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
            let x = r.center().x / ppp - self.size[0] / 2.0;
            let y = r.bottom() / ppp + 2.0;
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x.round(), y.round())));
        }
        self.popover = true;
        self.popover_focused = false;
        self.entered = Instant::now();
        self.show(ctx, true);
        crate::platform::activate();
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn close_popover(&mut self, ctx: &Context) {
        self.popover = false;
        self.view.account_menu = false;
        self.show(ctx, false);
    }

    /// Floating or in the menu bar: put the icon up or take it down, and
    /// answer what was clicked there.
    fn menu_bar(&mut self, ctx: &Context, snap: &Snapshot, session: Option<(f64, Option<i64>)>) {
        use crate::platform::tray::{Tray, TrayEvent, line, title};
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
        tray.set_title(&title(session.map(|s| s.0), self.settings.fire_threshold));
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
                TrayEvent::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
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
        let mood = self.pinned.unwrap_or_else(|| mood_for(snap.usage.as_ref(), session.map(|s| s.0), self.settings.fire_threshold));
        let f = Frame { snap: &snap, mood, session, view: &self.view, pet: &self.pet, logos: &self.logos };

        let size = if self.prefs.menu_bar { Size::Expanded } else { self.view.size };

        // a new screen (Settings, a tab, a size) fades in; the panel fades
        // out on its way to the menu bar
        if (self.view.settings, size) != self.screen {
            self.screen = (self.view.settings, size);
            self.entered = Instant::now();
        }
        let mut fade = motion::progress(self.entered, motion::ENTER_S);
        if let Some(t) = self.leaving {
            fade = 1.0 - motion::progress(t, motion::LEAVE_S);
            if fade <= 0.0 {
                self.leaving = None;
            }
        }
        if fade < 1.0 {
            ui.multiply_opacity(fade);
            ctx.request_repaint();
        }

        let (want, mut actions) = match (self.view.settings, size) {
            (Some(tab), _) => (settings::size(&snap, tab), settings::draw(ui, &ctx, &f, tab, &self.draft, &self.prefs, &mut self.code)),
            (None, Size::Compact) => (compact::size(compact::rows(&f).len()), compact::draw(ui, &ctx, &f)),
            (None, Size::Expanded) => (expanded::size(&snap, &self.view), expanded::draw(ui, &ctx, &f, &mut self.code)),
            (None, Size::PetOnly) => {
                // the OS's answer where there is one; else what this window sees
                let seen = ctx.input(|i| i.pointer.hover_pos().is_some());
                let over = if cfg!(target_os = "macos") { Some(self.cursor.inside(seen)) } else { None };
                let out = pet_only::draw(ui, &ctx, &f, over);
                let origin = ctx.input(|i| i.viewport().inner_rect).map(|r| r.min.to_vec2()).unwrap_or_default();
                self.cursor.watch(Some(out.hot.iter().map(|r| r.translate(origin)).collect()));
                (pet_only::size(), out.actions)
            }
        };
        if self.prefs.menu_bar {
            actions.retain(|a| !matches!(a, Action::Resize(_)));
        }
        // just the pet: clicks off the pet fall through to what's behind
        let through = size == Size::PetOnly && self.view.settings.is_none() && !self.cursor.inside(false) && cfg!(target_os = "macos");
        if through != self.passthrough {
            self.passthrough = through;
            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(through));
        }
        if size != Size::PetOnly {
            self.cursor.watch(None);
        }
        // the window eases to its new size rather than jumping
        if want != self.size {
            let first = self.current == [0.0, 0.0];
            self.size = want;
            self.resize = (!first).then(|| (self.current, Instant::now()));
            if first {
                self.current = want;
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(want.into()));
            }
        }
        if let Some((from, start)) = self.resize {
            let t = motion::progress(start, motion::RESIZE_S);
            self.current = motion::lerp_size(from, self.size, t);
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(self.current.into()));
            if t >= 1.0 {
                self.resize = None;
            } else {
                ctx.request_repaint();
            }
        }
        self.apply(&ctx, actions);
        self.menu_bar(&ctx, &snap, session);
        self.shoot(&ctx);
        ctx.request_repaint_after(self.pet.next_frame(mood).min(Duration::from_secs(30)));
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
