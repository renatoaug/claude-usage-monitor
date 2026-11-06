// The compact widget, laid out like the Electron one: the pet on the left
// with what it's up to beneath it, one row per connected service on the
// right (logo, name, %, bar) and the selected service's reset under them.
// With one service, its row sits alone, centred on the pet. It repaints only
// when the pet's next frame is due, a reading arrives, or a clock ticks over.

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::time::{Duration, Instant};

use crate::auth::Window;
use crate::logos::{Logos, Provider};
use crate::pet::{Mood, Pet};
use crate::poller::{self, Settings, Shared, Snapshot};
use crate::usage::Usage;

const CARD: Color32 = Color32::from_rgb(0x22, 0x19, 0x13);
const CARD_GLOW: Color32 = Color32::from_rgba_premultiplied(0x0c, 0x07, 0x04, 0x10);
const BORDER: Color32 = Color32::from_rgba_premultiplied(0x1d, 0x17, 0x11, 0x20);
const MUTED: Color32 = Color32::from_rgb(0xa3, 0x9f, 0x98);
const ICON: Color32 = Color32::from_rgb(0xaa, 0xa5, 0x9d);
const PLAN: Color32 = Color32::from_rgb(0xc6, 0xa3, 0x85);
const ROW_ON: Color32 = Color32::from_rgb(0xee, 0xe4, 0xd9);
const ROW_OFF: Color32 = Color32::from_rgb(0xaa, 0xa2, 0x98);
const URGENT: Color32 = Color32::from_rgb(0xf3, 0xa3, 0x8b);
const HOT: Color32 = Color32::from_rgb(0xee, 0x70, 0x5e);
const TRACK: Color32 = Color32::from_rgba_premultiplied(0x0d, 0x0d, 0x0d, 0x0d);
const MOOD_TEXT: Color32 = Color32::from_rgb(0xb9, 0xaf, 0xa3);
const RESET: Color32 = Color32::from_rgb(0xb4, 0xa1, 0x8a);
const STAGE_GLOW: Color32 = Color32::from_rgba_premultiplied(0x0e, 0x07, 0x05, 0x10);

// the Electron `collapsed.dual` card: 216 px wide, a 32 px title band, a
// 66 px pet column beside the rows, 12 px padding
const CARD_W: f32 = 216.0;
const PAD_TOP: f32 = 32.0;
const PAD: f32 = 12.0;
const STAGE: Vec2 = Vec2::new(66.0, 70.0);
const ROW_H: f32 = 29.0;
const ROW_GAP: f32 = 5.0;
const RESET_H: f32 = 7.0 + 10.0;
const MARGIN: f32 = 6.0;

/// Window size for `rows` service rows.
pub fn size_for(rows: usize) -> [f32; 2] {
    let rows_h = rows as f32 * ROW_H + rows.saturating_sub(1) as f32 * ROW_GAP + RESET_H;
    let card_h = PAD_TOP + rows_h.max(STAGE.y) + PAD;
    [CARD_W + 2.0 * MARGIN, card_h + 2.0 * MARGIN]
}

struct Row {
    provider: Provider,
    pct: Option<f64>,
    reset_ms: Option<i64>,
}

pub struct Widget {
    pet: Pet,
    logos: Logos,
    data: Shared,
    settings: Settings,
    pinned: Option<Mood>,
    started: Instant,
    shot_sent: bool,
    size: [f32; 2],
}

impl Widget {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        use_system_font(&cc.egui_ctx);
        let settings = Settings::load();
        let data = poller::spawn(cc.egui_ctx.clone(), settings.clone());
        Widget {
            pet: Pet::new(&cc.egui_ctx),
            logos: Logos::load(&cc.egui_ctx),
            data,
            settings,
            pinned: Mood::from_env(),
            started: Instant::now(),
            shot_sent: false,
            size: size_for(1),
        }
    }
}

// The system UI font, as the Electron panel uses (-apple-system).
fn use_system_font(ctx: &egui::Context) {
    let Ok(bytes) = std::fs::read("/System/Library/Fonts/SFNS.ttf") else { return };
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("system".into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
    fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "system".into());
    ctx.set_fonts(fonts);
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

/// "resets 4:40 PM", or with the weekday when it's not today.
pub fn reset_clock(ms: i64) -> String {
    let at = chrono::Local::now() + chrono::Duration::milliseconds(ms.max(0));
    if at.date_naive() == chrono::Local::now().date_naive() {
        format!("resets {}", at.format("%-I:%M %p"))
    } else {
        format!("resets {}", at.format("%a %-I:%M %p"))
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

// the word under the pet: what Claude is doing while it works, else the mood
fn status_word(m: Mood, activity: Option<&str>) -> String {
    match m {
        Mood::Working => activity.unwrap_or("working").to_string(),
        Mood::Fire => "on fire".into(),
        Mood::Tired => "maxed out".into(),
        Mood::Sleeping => "sleeping".into(),
        Mood::Idle => "idle".into(),
    }
}

// lucide-style glyphs from the Electron controls, in a 24-unit box
fn icon(p: &egui::Painter, center: Pos2, paths: &[&[(f32, f32)]], color: Color32) {
    let k = 9.0 / 24.0;
    for path in paths {
        let pts: Vec<Pos2> = path.iter().map(|&(x, y)| center + Vec2::new((x - 12.0) * k, (y - 12.0) * k)).collect();
        p.add(egui::Shape::line(pts, Stroke::new(1.1, color)));
    }
}

const PET_ICON: &[&[(f32, f32)]] = &[
    &[(5., 4.), (19., 4.), (19., 8.), (22., 8.), (22., 18.), (19., 18.), (19., 21.), (16., 21.), (16., 18.), (8., 18.), (8., 21.), (5., 21.), (5., 18.), (2., 18.), (2., 8.), (5., 8.), (5., 4.)],
    &[(8., 11.), (8., 13.)],
    &[(16., 11.), (16., 13.)],
];
const EXPAND_ICON: &[&[(f32, f32)]] =
    &[&[(15., 3.), (21., 3.), (21., 9.)], &[(9., 21.), (3., 21.), (3., 15.)], &[(21., 3.), (14., 10.)], &[(3., 21.), (10., 14.)]];
const CLOSE_ICON: &[&[(f32, f32)]] = &[&[(18., 6.), (6., 18.)], &[(6., 6.), (18., 18.)]];

// a 19 px control button; true when clicked
fn button(ui: &egui::Ui, center: Pos2, id: &str, glyph: &[&[(f32, f32)]]) -> bool {
    let rect = Rect::from_center_size(center, Vec2::splat(19.0));
    let resp = ui.interact(rect, ui.scope_id().with(id), Sense::click());
    let p = ui.painter();
    let (bg, fg) = if resp.hovered() {
        (Color32::from_white_alpha(60), Color32::WHITE)
    } else {
        (Color32::from_white_alpha(4), ICON)
    };
    p.rect(rect, CornerRadius::same(4), bg, Stroke::new(1.0, Color32::from_white_alpha(9)), egui::StrokeKind::Inside);
    icon(p, center, glyph, fg);
    resp.clicked()
}

fn short(s: &str, max: usize) -> String {
    if s.chars().count() <= max { s.into() } else { format!("{}…", s.chars().take(max - 1).collect::<String>()) }
}

impl Widget {
    // one service row: logo, name and % on top, the bar beneath
    fn row(&self, p: &egui::Painter, top_left: Pos2, width: f32, row: &Row, selected: bool, urgent_at: f64) {
        let urgent = row.pct.is_some_and(|v| v >= urgent_at);
        let text = if selected { ROW_ON } else { ROW_OFF };
        let x = top_left.x + 5.0;
        let line_y = top_left.y + 3.0 + 8.0;
        let logo = Rect::from_center_size(Pos2::new(x + 6.0, line_y), Vec2::splat(12.0));
        self.logos.draw(p, row.provider, logo, if selected { row.provider.tint() } else { text });
        let name = if urgent { format!("{} !", row.provider.name()) } else { row.provider.name().into() };
        p.text(Pos2::new(x + 17.0, line_y), Align2::LEFT_CENTER, name, FontId::proportional(10.0), if urgent { URGENT } else { text });
        let value = row.pct.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "—".into());
        let right = top_left.x + width;
        p.text(Pos2::new(right, line_y), Align2::RIGHT_CENTER, value, FontId::proportional(15.0), if urgent { URGENT } else { text });
        let track = Rect::from_min_size(Pos2::new(x + 17.0, top_left.y + 3.0 + 16.5 + 4.0), Vec2::new(right - (x + 17.0), 2.0));
        p.rect_filled(track, CornerRadius::same(1), TRACK);
        if let Some(v) = row.pct {
            let w = track.width() * (v / 100.0).clamp(0.0, 1.0) as f32;
            let mut fill = if urgent { HOT } else { row.provider.bar() };
            if !selected && !urgent {
                fill = fill.gamma_multiply(0.55);
            }
            p.rect_filled(Rect::from_min_size(track.min, Vec2::new(w, 2.0)), CornerRadius::same(1), fill);
        }
    }
}

impl eframe::App for Widget {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let snap: Snapshot = self.data.lock().map(|s| s.clone()).unwrap_or_default();
        let session = snap.limits.as_ref().and_then(|l| live(&l.session, snap.limits_at));
        let mood =
            self.pinned.unwrap_or_else(|| mood_for(snap.usage.as_ref(), session.map(|s| s.0), self.settings.fire_threshold));

        // the connected services, one row each (Codex and Cursor come later)
        let rows = vec![Row { provider: Provider::Claude, pct: session.map(|s| s.0), reset_ms: session.and_then(|s| s.1) }];
        let want = size_for(rows.len());
        if want != self.size {
            self.size = want;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(want.into()));
        }

        let card = ui.max_rect().shrink(MARGIN);
        let p = ui.painter();
        p.rect(card, CornerRadius::same(9), CARD, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);
        // the panel's warm glow from the top-left, kept inside the card
        let glow = p.with_clip_rect(card.shrink(1.0));
        for i in 0..16 {
            let r = 140.0 - i as f32 * 8.0;
            glow.circle_filled(card.left_top() + Vec2::new(card.width() * 0.2, 0.0), r, CARD_GLOW.gamma_multiply(0.18));
        }

        // the whole card drags the window; the buttons sit on top
        let drag = ui.interact(card, ui.scope_id().with("drag"), Sense::click_and_drag());
        if drag.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        // title band: the account and plan on the left, the controls on the right
        let band_y = card.top() + 15.5;
        let name = snap
            .profile
            .as_ref()
            .and_then(|pr| pr.name.as_deref().or(pr.email.as_deref()))
            .map(|n| short(n, 18))
            .unwrap_or_default();
        let name_rect = p.text(Pos2::new(card.left() + 10.0, band_y), Align2::LEFT_CENTER, name, FontId::proportional(10.0), MUTED);
        if let Some(plan) = snap.profile.as_ref().and_then(|pr| pr.plan.as_deref()) {
            let tr = p.text(
                Pos2::new(name_rect.right() + 8.0, band_y),
                Align2::LEFT_CENTER,
                plan.to_lowercase(),
                FontId::proportional(8.0),
                PLAN,
            );
            p.rect(
                tr.expand2(Vec2::new(3.0, 2.0)),
                CornerRadius::same(2),
                Color32::from_rgba_premultiplied(1, 1, 0, 10),
                Stroke::new(1.0, Color32::from_rgba_premultiplied(27, 22, 16, 38)),
                egui::StrokeKind::Inside,
            );
        }
        let right = card.right() - 8.0 - 9.5;
        if button(ui, Pos2::new(right, band_y), "close", CLOSE_ICON) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let _expand = button(ui, Pos2::new(right - 23.0, band_y), "expand", EXPAND_ICON); // expanded view: next
        let _pet_only = button(ui, Pos2::new(right - 46.0, band_y), "pet-only", PET_ICON); // pet-only: next

        // content: the pet column beside the rows, both centred vertically
        let p = ui.painter();
        let content = Rect::from_min_max(
            Pos2::new(card.left() + PAD, card.top() + PAD_TOP),
            Pos2::new(card.right() - PAD, card.bottom() - PAD),
        );
        let stage = Rect::from_center_size(Pos2::new(content.left() + STAGE.x / 2.0, content.center().y), STAGE);
        for (r, a) in [(30.0, 0.5), (22.0, 0.8), (14.0, 1.0)] {
            p.circle_filled(stage.center(), r, STAGE_GLOW.gamma_multiply(a));
        }
        // the pet sits high in its column, clear of the word beneath it
        let pet_area = if mood == Mood::Working {
            Rect::from_min_size(Pos2::new(stage.center().x - 31.0, stage.top() + 8.0), Vec2::new(62.0, 52.0))
        } else {
            Rect::from_min_size(Pos2::new(stage.center().x - 36.0, stage.top() + 4.0), Vec2::new(72.0, 60.0))
        };
        self.pet.draw(p, pet_area, mood);

        // under the pet: the worker's logo (or a dot) and the word
        let activity = snap.usage.as_ref().and_then(|u| u.activity.as_deref());
        let word = status_word(mood, activity);
        let word_w = ctx.fonts_mut(|f| f.layout_no_wrap(word.clone(), FontId::proportional(9.0), MOOD_TEXT).size().x);
        let line_y = card.bottom() - PAD - 6.0;
        let mark_w = if mood == Mood::Working { 10.0 } else { 4.0 };
        let left = stage.center().x - (mark_w + 5.0 + word_w) / 2.0;
        if mood == Mood::Working {
            let logo = Rect::from_min_size(Pos2::new(left, line_y - 5.0), Vec2::splat(10.0));
            self.logos.draw(p, Provider::Claude, logo, Color32::from_rgb(0xdf, 0x97, 0x77));
        } else {
            let dot = match mood {
                Mood::Fire => Color32::from_rgb(0xe0, 0x55, 0x3f),
                Mood::Tired => Color32::from_rgb(0xb6, 0xa9, 0x9c),
                _ => Color32::from_rgb(0x6b, 0x63, 0x5a),
            };
            p.circle_filled(Pos2::new(left + 2.0, line_y), 2.0, dot);
        }
        p.text(Pos2::new(left + mark_w + 5.0, line_y), Align2::LEFT_CENTER, word, FontId::proportional(9.0), MOOD_TEXT);

        // the rows, then the selected service's reset
        let col_left = content.left() + STAGE.x + 8.0;
        let col_w = content.right() - col_left;
        let rows_h = rows.len() as f32 * ROW_H + rows.len().saturating_sub(1) as f32 * ROW_GAP + RESET_H;
        let mut y = content.center().y - rows_h / 2.0;
        for (i, row) in rows.iter().enumerate() {
            self.row(p, Pos2::new(col_left, y), col_w, row, i == 0, 80.0);
            y += ROW_H + ROW_GAP;
        }
        y -= ROW_GAP;
        if let Some(ms) = rows[0].reset_ms {
            p.text(Pos2::new(content.right(), y + 7.0), Align2::RIGHT_TOP, reset_clock(ms), FontId::proportional(8.0), RESET);
        }

        // CLAUDDY_SHOT=path.png: save one frame after 3 s, then quit (dev aid)
        if let Ok(path) = std::env::var("CLAUDDY_SHOT") {
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

        // next repaint: the pet's next frame, or the reset clock ticking over
        ctx.request_repaint_after(self.pet.next_frame(mood).min(Duration::from_secs(30)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank() -> Usage {
        crate::usage::Reader::new(Some(std::env::temp_dir().join("clauddy-no-such-dir"))).get_usage(&Default::default())
    }

    fn reading(active: bool, sleeping: bool) -> Usage {
        let mut u = blank();
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

    #[test]
    fn an_expired_window_is_no_reading_not_zero() {
        let w = Window { pct: 80.0, reset_ms: Some(-5) };
        assert_eq!(live(&w, None), None);
        let w = Window { pct: 80.0, reset_ms: Some(60_000) };
        assert_eq!(live(&w, None).map(|r| r.0), Some(80.0));
    }

    #[test]
    fn the_word_under_the_pet_is_the_activity_while_working() {
        assert_eq!(status_word(Mood::Working, Some("running")), "running");
        assert_eq!(status_word(Mood::Working, None), "working");
        assert_eq!(status_word(Mood::Idle, Some("running")), "idle");
    }

    #[test]
    fn one_service_keeps_the_card_as_tall_as_the_pet() {
        assert_eq!(size_for(1)[1], size_for(0)[1]);
        assert!(size_for(3)[1] > size_for(1)[1]);
    }

    #[test]
    fn countdowns_and_token_counts_read_short() {
        assert_eq!(countdown(13 * 60_000), "13m");
        assert_eq!(countdown((2 * 60 + 13) * 60_000), "2h 13m");
        assert_eq!(countdown((25 * 60) * 60_000), "1d 1h");
        assert_eq!(tokens(37_800_000), "37.8M");
        assert_eq!(tokens(1_120_000_000), "1.12B");
        assert_eq!(tokens(950), "950");
        assert!(reset_clock(60_000).starts_with("resets "));
    }
}
