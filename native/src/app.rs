// The app shell: it owns the pet, the logos, the poll and the remembered
// view; each frame it works out the mood, hands a screen its data, and
// applies what the screen returns. It repaints only when the pet's next frame
// is due, a reading lands, or a clock ticks over.

use eframe::egui::{self, Context};
use std::time::{Duration, Instant};

use crate::components::logos::Logos;
use crate::components::meter::live;
use crate::components::pet::{Mood, Pet};
use crate::poller::{self, Settings, Shared, Snapshot};
use crate::screens::{Action, Frame, Size, View, compact, expanded};
use crate::ui::theme;
use crate::usage::Usage;

pub struct App {
    pet: Pet,
    logos: Logos,
    data: Shared,
    settings: Settings,
    pinned: Option<Mood>,
    view: View,
    size: [f32; 2],
    started: Instant,
    shot_sent: bool,
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
fn open_url(url: &str) {
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
        let settings = Settings::load();
        let data = poller::spawn(cc.egui_ctx.clone(), settings.clone());
        App {
            pet: Pet::new(&cc.egui_ctx),
            logos: Logos::load(&cc.egui_ctx),
            data,
            settings,
            pinned: Mood::from_env(),
            view: load_view(),
            size: [0.0, 0.0],
            started: Instant::now(),
            shot_sent: false,
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
            }
            save_view(&self.view);
        }
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

        let (want, actions) = match self.view.size {
            Size::Compact => (compact::size(compact::rows(&f).len()), compact::draw(ui, &ctx, &f)),
            Size::Expanded => (expanded::size(&f), expanded::draw(ui, &ctx, &f)),
        };
        if want != self.size {
            self.size = want;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(want.into()));
        }
        self.apply(&ctx, actions);
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
