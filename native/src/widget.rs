// The compact widget: a small card with the pet, what it's up to, and the
// session and week bars from the account's real %. It repaints only when the
// pet's next frame is due, a reading arrives, or a countdown ticks over.

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::time::{Duration, Instant};

use crate::auth::Window;
use crate::pet::{Mood, Pet};
use crate::poller::{self, Settings, Shared, Snapshot};
use crate::usage::Usage;

const CARD: Color32 = Color32::from_rgb(0x24, 0x1b, 0x15);
const BORDER: Color32 = Color32::from_rgba_premultiplied(0x2e, 0x25, 0x1c, 0x40);
const TEXT: Color32 = Color32::from_rgb(0xee, 0xea, 0xe5);
const MUTED: Color32 = Color32::from_rgb(0xa3, 0x9f, 0x98);
const TRACK: Color32 = Color32::from_rgba_premultiplied(0x1a, 0x1a, 0x1a, 0x1a);
const CORAL: Color32 = Color32::from_rgb(0xd9, 0x77, 0x57);
const AMBER: Color32 = Color32::from_rgb(0xe3, 0xb5, 0x5c);
const HOT: Color32 = Color32::from_rgb(0xef, 0x5f, 0x3c);
const GREEN: Color32 = Color32::from_rgb(0x7e, 0xc7, 0x7d);

pub const SIZE: [f32; 2] = [236.0, 312.0];

pub struct Widget {
    pet: Pet,
    data: Shared,
    settings: Settings,
    pinned: Option<Mood>,
    started: Instant,
    shot_sent: bool,
}

impl Widget {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings = Settings::load();
        let data = poller::spawn(cc.egui_ctx.clone(), settings.clone());
        Widget {
            pet: Pet::new(&cc.egui_ctx),
            data,
            settings,
            pinned: Mood::from_env(),
            started: Instant::now(),
            shot_sent: false,
        }
    }
}

/// A limits window still in force, its countdown rebased to now. A reading
/// whose reset already passed proves nothing about the new window: None,
/// never zero.
fn live(w: &Window, at: Option<Instant>) -> Option<(f64, Option<i64>)> {
    let elapsed = at.map(|a| a.elapsed().as_millis() as i64).unwrap_or(0);
    match w.reset_ms {
        Some(ms) if ms - elapsed <= 0 => None,
        Some(ms) => Some((w.pct, Some(ms - elapsed))),
        None => Some((w.pct, None)),
    }
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

pub fn countdown(ms: i64) -> String {
    let m = (ms.max(0) + 59_999) / 60_000;
    let (d, h, m) = (m / 1440, m % 1440 / 60, m % 60);
    match (d, h) {
        (0, 0) => format!("{m}m"),
        (0, _) => format!("{h}h {m}m"),
        _ => format!("{d}d {h}h"),
    }
}

pub fn tokens(n: u64) -> String {
    match n {
        n if n >= 1_000_000_000 => format!("{:.2}B", n as f64 / 1e9),
        n if n >= 1_000_000 => format!("{:.1}M", n as f64 / 1e6),
        n if n >= 1_000 => format!("{:.1}k", n as f64 / 1e3),
        n => n.to_string(),
    }
}

fn status_word(m: Mood) -> &'static str {
    match m {
        Mood::Working => "working",
        Mood::Fire => "on fire",
        Mood::Tired => "maxed out",
        Mood::Sleeping => "sleeping",
        Mood::Idle => "idle",
    }
}

fn bar(p: &egui::Painter, rect: Rect, label: &str, reading: Option<(f64, Option<i64>)>, color: Color32, fire: f64) {
    let (pct, reset) = match reading {
        Some((pct, reset)) => (Some(pct), reset),
        None => (None, None),
    };
    p.text(rect.left_top(), Align2::LEFT_TOP, label, FontId::proportional(12.5), TEXT);
    let value = pct.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "—".into());
    let hot = pct.is_some_and(|v| v >= fire);
    p.text(rect.right_top(), Align2::RIGHT_TOP, value, FontId::proportional(14.0), if hot { HOT } else { TEXT });
    let track = Rect::from_min_size(rect.left_top() + Vec2::new(0.0, 21.0), Vec2::new(rect.width(), 5.0));
    p.rect_filled(track, CornerRadius::same(3), TRACK);
    if let Some(v) = pct {
        let w = (track.width() * (v / 100.0).clamp(0.0, 1.0) as f32).max(if v > 0.0 { 5.0 } else { 0.0 });
        let fill = Rect::from_min_size(track.min, Vec2::new(w, track.height()));
        p.rect_filled(fill, CornerRadius::same(3), if hot { HOT } else { color });
    }
    let sub = match reset {
        Some(ms) => format!("resets in {}", countdown(ms)),
        None if pct.is_none() => "waiting for a reading".into(),
        None => String::new(),
    };
    p.text(track.left_bottom() + Vec2::new(0.0, 5.0), Align2::LEFT_TOP, sub, FontId::proportional(11.0), MUTED);
}

impl eframe::App for Widget {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let snap: Snapshot = self.data.lock().map(|s| s.clone()).unwrap_or_default();
        let session = snap.limits.as_ref().and_then(|l| live(&l.session, snap.limits_at));
        let week = snap.limits.as_ref().and_then(|l| live(&l.week, snap.limits_at));
        let mood =
            self.pinned.unwrap_or_else(|| mood_for(snap.usage.as_ref(), session.map(|s| s.0), self.settings.fire_threshold));

        let card = ui.max_rect().shrink(6.0);
        let p = ui.painter();
        p.rect(card, CornerRadius::same(18), CARD, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);

        // the whole card drags the window; the close button sits on top
        let drag = ui.interact(card, ui.scope_id().with("drag"), Sense::click_and_drag());
        if drag.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        // header: status dot, the account, close
        let head = Rect::from_min_size(card.min + Vec2::new(14.0, 12.0), Vec2::new(card.width() - 28.0, 18.0));
        let dot = match mood {
            Mood::Working => GREEN,
            Mood::Fire | Mood::Tired => HOT,
            _ => MUTED,
        };
        p.circle_filled(head.left_center() + Vec2::new(3.0, 0.0), 3.5, dot);
        let who = snap
            .profile
            .as_ref()
            .and_then(|pr| pr.plan.clone().map(|plan| format!("Claude · {plan}")))
            .unwrap_or_else(|| "Claude".into());
        p.text(head.left_center() + Vec2::new(13.0, 0.0), Align2::LEFT_CENTER, who, FontId::proportional(12.5), MUTED);
        let close = Rect::from_center_size(head.right_center() - Vec2::new(6.0, 0.0), Vec2::splat(18.0));
        let close_resp = ui.interact(close, ui.scope_id().with("close"), Sense::click());
        let x_color = if close_resp.hovered() { TEXT } else { MUTED };
        p.text(close.center(), Align2::CENTER_CENTER, "×", FontId::proportional(16.0), x_color);
        if close_resp.clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // the pet
        let pet_area = Rect::from_center_size(Pos2::new(card.center().x, head.bottom() + 72.0), Vec2::new(150.0, 124.0));
        self.pet.draw(p, pet_area, mood);

        // what it's up to
        let line_y = pet_area.bottom() + 8.0;
        let today = snap.usage.as_ref().map(|u| format!(" · {} today", tokens(u.today_tokens))).unwrap_or_default();
        p.text(
            Pos2::new(card.center().x, line_y),
            Align2::CENTER_TOP,
            format!("{}{today}", status_word(mood)),
            FontId::proportional(12.0),
            MUTED,
        );

        // the bars, or a nudge to connect
        let rows = Rect::from_min_size(Pos2::new(card.left() + 16.0, line_y + 26.0), Vec2::new(card.width() - 32.0, 40.0));
        if snap.connected || snap.limits.is_some() {
            let fire = self.settings.fire_threshold;
            bar(p, rows, "session", session, CORAL, fire);
            bar(p, rows.translate(Vec2::new(0.0, 52.0)), "week", week, AMBER, fire);
        } else {
            p.text(rows.center_top() + Vec2::new(0.0, 14.0), Align2::CENTER_TOP, "not connected", FontId::proportional(12.5), TEXT);
            p.text(
                rows.center_top() + Vec2::new(0.0, 34.0),
                Align2::CENTER_TOP,
                "log in from the Electron app for now",
                FontId::proportional(11.0),
                MUTED,
            );
        }

        // CLAUDDY_SHOT=path.png: save one frame after 3 s, then quit (dev aid)
        if let Ok(path) = std::env::var("CLAUDDY_SHOT") {
            if self.started.elapsed().as_secs_f32() > 3.0 && !self.shot_sent {
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

        // next repaint: the pet's next frame, or a countdown ticking over
        ctx.request_repaint_after(self.pet.next_frame(mood).min(Duration::from_secs(30)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(active: bool, sleeping: bool) -> Usage {
        let mut u = blank();
        u.active = active;
        u.sleeping = sleeping;
        u
    }

    fn blank() -> Usage {
        crate::usage::Reader::new(Some(std::env::temp_dir().join("clauddy-no-such-dir"))).get_usage(&Default::default())
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

    #[test]
    fn an_expired_window_is_no_reading_not_zero() {
        let w = Window { pct: 80.0, reset_ms: Some(-5) };
        assert_eq!(live(&w, None), None);
        let w = Window { pct: 80.0, reset_ms: Some(60_000) };
        assert_eq!(live(&w, None).map(|r| r.0), Some(80.0));
    }

    #[test]
    fn countdowns_and_token_counts_read_short() {
        assert_eq!(countdown(13 * 60_000), "13m");
        assert_eq!(countdown((2 * 60 + 13) * 60_000), "2h 13m");
        assert_eq!(countdown((25 * 60) * 60_000), "1d 1h");
        assert_eq!(tokens(37_800_000), "37.8M");
        assert_eq!(tokens(1_120_000_000), "1.12B");
        assert_eq!(tokens(950), "950");
    }
}
