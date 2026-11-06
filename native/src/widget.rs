// The compact widget, laid out like the Electron one: the pet on the left
// with what it's up to beneath it, one row per connected service on the
// right (logo, name, %, bar) and the selected service's reset under them.
// With one service, its row sits alone, centred on the pet. It repaints only
// when the pet's next frame is due, a reading arrives, or a clock ticks over.

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::time::{Duration, Instant};

use crate::auth::{Limits, Window};
use crate::logos::{Logos, Provider};
use crate::pet::{Mood, Pet};
use crate::poller::{self, Settings, Shared, Snapshot};
use crate::usage::Usage;

const CARD: Color32 = Color32::from_rgb(0x22, 0x19, 0x13);
const CARD_GLOW: Color32 = Color32::from_rgba_premultiplied(0x0c, 0x07, 0x04, 0x10);
const BORDER: Color32 = Color32::from_rgba_premultiplied(0x1d, 0x17, 0x11, 0x20);
const TEXT: Color32 = Color32::from_rgb(0xee, 0xea, 0xe5);
const MID: Color32 = Color32::from_rgb(0xe5, 0xb5, 0x5d);
const HAIR: Color32 = Color32::from_rgba_premultiplied(19, 19, 19, 19);
const GREEN: Color32 = Color32::from_rgb(0x7e, 0xc7, 0x7d);
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

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Size {
    Expanded,
    Compact,
}

/// What the widget remembers between runs: its size and which parts of the
/// expanded panel are open (Electron kept this in localStorage).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct View {
    pub size: Size,
    pub details: bool,
    pub models: bool,
    pub projects: bool,
}

impl Default for View {
    fn default() -> Self {
        View { size: Size::Expanded, details: false, models: true, projects: true }
    }
}

impl View {
    fn path() -> std::path::PathBuf {
        crate::auth::default_data_dir().join("native-ui.json")
    }
    fn load() -> Self {
        std::fs::read(Self::path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }
    fn save(&self) {
        if let Ok(body) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::path(), body);
        }
    }
}

// the card itself: the panel colour, its border and warm top-left glow; the
// whole card drags the window (buttons drawn later sit on top)
fn card_frame(ui: &egui::Ui, ctx: &egui::Context, radius: u8) -> Rect {
    let card = ui.max_rect().shrink(MARGIN);
    let p = ui.painter();
    p.rect(card, CornerRadius::same(radius), CARD, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);
    let glow = p.with_clip_rect(card.shrink(1.0));
    for i in 0..16 {
        let r = 140.0 - i as f32 * 8.0;
        glow.circle_filled(card.left_top() + Vec2::new(card.width() * 0.2, 0.0), r, CARD_GLOW.gamma_multiply(0.18));
    }
    let drag = ui.interact(card, ui.scope_id().with("drag"), Sense::click_and_drag());
    if drag.drag_started() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    card
}

/// Open a URL in the default browser.
fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let cmd = ("open", vec![url]);
    #[cfg(target_os = "windows")]
    let cmd = ("cmd", vec!["/C", "start", "", url]);
    #[cfg(all(unix, not(target_os = "macos")))]
    let cmd = ("xdg-open", vec![url]);
    let _ = std::process::Command::new(cmd.0).args(cmd.1).spawn();
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
    view: View,
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
            size: [0.0, 0.0],
            view: View::load(),
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
    let m = ms.max(0) / 60_000;
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


// the account on the title band: a dot (expanded), the name, the plan badge
fn account(p: &egui::Painter, card: Rect, y: f32, snap: &Snapshot, dot: bool) {
    let mut x = card.left() + 10.0;
    if dot {
        p.circle_filled(Pos2::new(x + 2.0, y), 2.0, Color32::from_rgb(0xa1, 0xb7, 0x86));
        x += 9.0;
    }
    let name = snap
        .profile
        .as_ref()
        .and_then(|pr| if dot { pr.email.as_deref().or(pr.name.as_deref()) } else { pr.name.as_deref().or(pr.email.as_deref()) })
        .map(|n| short(n, if dot { 16 } else { 18 }))
        .unwrap_or_default();
    let name_rect = p.text(Pos2::new(x, y), Align2::LEFT_CENTER, name, FontId::proportional(10.0), MUTED);
    if let Some(plan) = snap.profile.as_ref().and_then(|pr| pr.plan.as_deref()) {
        let tr = p.text(Pos2::new(name_rect.right() + 8.0, y), Align2::LEFT_CENTER, plan.to_lowercase(), FontId::proportional(8.0), PLAN);
        p.rect(
            tr.expand2(Vec2::new(3.0, 2.0)),
            CornerRadius::same(2),
            Color32::from_rgba_premultiplied(1, 1, 0, 10),
            Stroke::new(1.0, Color32::from_rgba_premultiplied(27, 22, 16, 38)),
            egui::StrokeKind::Inside,
        );
    }
}

// one ramp for every meter: plain below 60, warm to 85, hot past it
fn level(pct: f64) -> (Color32, Color32) {
    if pct >= 85.0 {
        (HOT, HOT)
    } else if pct >= 60.0 {
        (MID, MID)
    } else {
        (Provider::Claude.bar(), TEXT)
    }
}

/// "2h 13m (12:50 PM)": the countdown and the clock time it lands on.
pub fn reset_in(ms: i64) -> String {
    let at = chrono::Local::now() + chrono::Duration::milliseconds(ms.max(0));
    let clock = if ms >= 6 * 86_400_000 {
        at.format("%b %-d").to_string()
    } else if ms >= 86_400_000 {
        at.format("%a %-I:%M %p").to_string()
    } else {
        at.format("%-I:%M %p").to_string()
    };
    format!("{} ({clock})", countdown(ms))
}

struct Meter {
    label: String,
    pct: Option<f64>,
    sub: String,
}

fn meters(limits: Option<&Limits>, at: Option<Instant>, usage: Option<&Usage>) -> Vec<Meter> {
    let Some(l) = limits else { return vec![] };
    let session_tokens = usage.map(|u| u.session.tokens).unwrap_or(0);
    let week_tokens = usage.map(|u| u.week.tokens).unwrap_or(0);
    let session = live(&l.session, at);
    let week = live(&l.week, at);
    let mut out = vec![
        Meter {
            label: "current session".into(),
            pct: session.map(|s| s.0),
            sub: match session {
                Some((_, Some(ms))) => format!("resets in {} · {} tokens", reset_in(ms), tokens(session_tokens)),
                _ => "no active session".into(),
            },
        },
        Meter {
            label: "weekly · all models".into(),
            pct: week.map(|w| w.0),
            sub: match week {
                Some((_, Some(ms))) => format!("resets in {} · {} tokens", reset_in(ms), tokens(week_tokens)),
                _ => format!("{} tokens · last 7 days", tokens(week_tokens)),
            },
        },
    ];
    // per-model weekly limits, paired with that model family's local tokens
    for sc in &l.scoped {
        let w = Window { pct: sc.pct, reset_ms: sc.reset_ms };
        let Some((pct, reset)) = live(&w, at) else { continue };
        let key = sc.label.to_lowercase();
        let family: u64 = usage
            .map(|u| u.by_model.iter().filter(|m| m.label.to_lowercase().starts_with(&key)).map(|m| m.tokens).sum())
            .unwrap_or(0);
        out.push(Meter {
            label: format!("weekly · {key}"),
            pct: Some(pct),
            sub: match reset {
                Some(ms) => format!("resets in {} · {} tokens", reset_in(ms), tokens(family)),
                None => format!("{} tokens", tokens(family)),
            },
        });
    }
    out
}

// the expanded panel's layout, top to bottom (Electron's paddings)
const EX_W: f32 = 280.0;
const EX_SIDE: f32 = 18.0;
const EX_STAGE_TOP: f32 = 38.0;
const EX_STAGE_H: f32 = 132.0;
const STATUS_H: f32 = 34.0;
const METER_H: f32 = 54.0;
const CONNECT_H: f32 = 56.0;
const TOGGLE_H: f32 = 32.0;
const MODEL_ROWS: usize = 4;
const PROJECT_ROWS: usize = 6;
const ROW_STEP: f32 = 20.0;
const HEAT_H: f32 = 35.0;

fn section_h(rows: usize, open: bool) -> f32 {
    1.0 + 9.0 + 12.0 + if open { rows.max(1) as f32 * ROW_STEP } else { 0.0 } + 9.0
}

fn expanded_height(meter_count: usize, view: &View, models: usize, projects: usize) -> f32 {
    let mut h = EX_STAGE_TOP + EX_STAGE_H + STATUS_H;
    h += if meter_count > 0 { meter_count as f32 * METER_H + 12.0 } else { CONNECT_H };
    h += TOGGLE_H;
    if view.details {
        h += section_h(models.min(MODEL_ROWS), view.models) + section_h(projects.min(PROJECT_ROWS), view.projects) + HEAT_H;
    }
    h + 14.0
}

fn hair(p: &egui::Painter, left: f32, right: f32, y: f32) {
    p.hline(left..=right, y, Stroke::new(1.0, HAIR));
}

const HEAT: [Color32; 5] = [
    Color32::from_rgba_premultiplied(13, 13, 13, 13),
    Color32::from_rgb(0x6b, 0x44, 0x30),
    Color32::from_rgb(0x9c, 0x63, 0x40),
    Color32::from_rgb(0xcf, 0x8b, 0x52),
    Color32::from_rgb(0xf0, 0xb4, 0x64),
];

/// The 30-day map's level for a day: 0 when idle, then 1-4 by share of the busiest.
pub fn heat_level(v: u64, max: u64) -> usize {
    if v == 0 {
        return 0;
    }
    let r = v as f64 / max.max(1) as f64;
    if r < 0.3 {
        1
    } else if r < 0.6 {
        2
    } else if r < 0.85 {
        3
    } else {
        4
    }
}

const DOTS: [Color32; 4] = [
    Color32::from_rgb(0xd9, 0x77, 0x57),
    Color32::from_rgb(0xc8, 0xa4, 0x75),
    Color32::from_rgb(0x80, 0x77, 0x69),
    Color32::from_rgb(0xb1, 0x96, 0x7c),
];

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
const MIN_ICON: &[&[(f32, f32)]] = &[&[(5., 12.), (19., 12.)]];
const USAGE_ICON: &[&[(f32, f32)]] = &[&[(7., 7.), (17., 7.), (17., 17.)], &[(7., 17.), (17., 7.)]];
const GEAR_ICON: &[&[(f32, f32)]] = &[
    &[(12., 2.), (12., 5.)],
    &[(12., 19.), (12., 22.)],
    &[(2., 12.), (5., 12.)],
    &[(19., 12.), (22., 12.)],
    &[(4.9, 4.9), (7., 7.)],
    &[(17., 17.), (19.1, 19.1)],
    &[(4.9, 19.1), (7., 17.)],
    &[(17., 7.), (19.1, 4.9)],
    &[(12., 6.), (16.2, 7.8), (18., 12.), (16.2, 16.2), (12., 18.), (7.8, 16.2), (6., 12.), (7.8, 7.8), (12., 6.)],
    &[(12., 10.), (14., 12.), (12., 14.), (10., 12.), (12., 10.)],
];
const CHEVRON: &[&[(f32, f32)]] = &[&[(7., 10.), (12., 15.), (17., 10.)]];
const CHEVRON_RIGHT: &[&[(f32, f32)]] = &[&[(10., 7.), (15., 12.), (10., 17.)]];
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

impl Widget {
    fn set_size(&mut self, size: Size) {
        self.view.size = size;
        self.view.save();
    }

    fn resize(&mut self, ctx: &egui::Context, want: [f32; 2]) {
        if want != self.size {
            self.size = want;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(want.into()));
        }
    }

    fn expanded(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, snap: &Snapshot, mood: Mood) {
        let usage = snap.usage.as_ref();
        let meters = meters(snap.limits.as_ref(), snap.limits_at, usage);
        let models: Vec<_> = usage.map(|u| u.by_model.iter().take(MODEL_ROWS).cloned().collect()).unwrap_or_default();
        let projects: Vec<_> = usage.map(|u| u.by_project.iter().take(PROJECT_ROWS).cloned().collect()).unwrap_or_default();
        let h = expanded_height(meters.len(), &self.view, models.len(), projects.len());
        self.resize(ctx, [EX_W + 2.0 * MARGIN, h + 2.0 * MARGIN]);

        let card = card_frame(ui, ctx, 10);
        let (left, right) = (card.left() + EX_SIDE, card.right() - EX_SIDE);
        let p = ui.painter().clone();

        // title band: account chip, then gear, usage page, pet-only, minimize, close
        let band_y = card.top() + 19.0;
        account(&p, card, card.top() + 16.5, snap, true);
        let bx = card.right() - 8.0 - 9.5;
        if button(ui, Pos2::new(bx, band_y), "close", CLOSE_ICON) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if button(ui, Pos2::new(bx - 23.0, band_y), "min", MIN_ICON) {
            self.set_size(Size::Compact);
        }
        let _pet_only = button(ui, Pos2::new(bx - 46.0, band_y), "pet-only", PET_ICON); // pet-only: next
        if button(ui, Pos2::new(bx - 69.0, band_y), "usage", USAGE_ICON) {
            open_url("https://claude.ai/settings/usage");
        }
        let _gear = button(ui, Pos2::new(bx - 92.0, band_y), "gear", GEAR_ICON); // settings: next

        // the stage: a dark pane with the pet near its left edge
        let stage = Rect::from_min_size(Pos2::new(left, card.top() + EX_STAGE_TOP), Vec2::new(right - left, EX_STAGE_H));
        p.rect(stage, CornerRadius::same(6), Color32::from_rgb(0x0f, 0x12, 0x10), Stroke::new(1.0, Color32::from_white_alpha(6)), egui::StrokeKind::Inside);
        let pane = p.with_clip_rect(stage);
        let pet_area = if mood == Mood::Working {
            Rect::from_min_max(Pos2::new(stage.left() + 14.0, stage.top() + 8.0), Pos2::new(stage.left() + 144.0, stage.bottom() - 2.0))
        } else {
            let cell = 7.0;
            Rect::from_min_max(
                Pos2::new(stage.left() + 26.0 - cell, stage.bottom() - 3.0 + cell - 10.0 * cell),
                Pos2::new(stage.left() + 26.0 + 11.0 * cell, stage.bottom() - 3.0 + cell),
            )
        };
        self.pet.draw(&pane, pet_area, mood);

        // status line: dot, the mood (or what Claude is doing), today's tokens, live
        let mut y = stage.bottom() + 10.0 + 7.0;
        let (dot, word_color) = match mood {
            Mood::Working => (GREEN, GREEN),
            Mood::Fire => (Color32::from_rgb(0xe0, 0x55, 0x3f), Color32::from_rgb(0xe0, 0x83, 0x68)),
            Mood::Tired => (Color32::from_rgb(0xb6, 0xa9, 0x9c), Color32::from_rgb(0xc2, 0xb4, 0xa6)),
            _ => (Color32::from_rgb(0x6b, 0x63, 0x5a), TEXT),
        };
        p.circle_filled(Pos2::new(left + 3.5, y), 3.5, dot);
        let word = status_word(mood, usage.and_then(|u| u.activity.as_deref()));
        let mut wr = p.text(Pos2::new(left + 13.0, y), Align2::LEFT_CENTER, &word, FontId::proportional(11.0), word_color);
        p.text(Pos2::new(left + 13.45, y), Align2::LEFT_CENTER, &word, FontId::proportional(11.0), word_color);
        if let Some(u) = usage {
            let sep = p.text(Pos2::new(wr.right() + 6.0, y), Align2::LEFT_CENTER, "·", FontId::proportional(11.0), MUTED.gamma_multiply(0.5));
            wr = p.text(
                Pos2::new(sep.right() + 6.0, y),
                Align2::LEFT_CENTER,
                format!("{} tokens today", tokens(u.today_tokens)),
                FontId::proportional(10.0),
                MUTED,
            );
        }
        let _ = wr;
        if snap.limits.is_some() {
            let t = p.text(Pos2::new(right - 5.0, y), Align2::RIGHT_CENTER, "live", FontId::proportional(8.0), Color32::from_rgb(0xab, 0xbd, 0x96));
            p.rect(
                t.expand2(Vec2::new(5.0, 3.0)),
                CornerRadius::same(2),
                Color32::from_rgba_premultiplied(0, 1, 0, 8),
                Stroke::new(1.0, Color32::from_rgba_premultiplied(21, 24, 17, 33)),
                egui::StrokeKind::Inside,
            );
        }
        y = stage.bottom() + STATUS_H;
        hair(&p, left, right, y);

        // the meters, or the way in when no account is connected
        if meters.is_empty() {
            let r = Rect::from_min_size(Pos2::new(left, y + 12.0), Vec2::new(right - left, CONNECT_H - 24.0));
            p.rect(r, CornerRadius::same(6), Color32::from_white_alpha(3), Stroke::new(1.0, Color32::from_white_alpha(10)), egui::StrokeKind::Inside);
            p.text(r.center() - Vec2::new(0.0, 6.0), Align2::CENTER_CENTER, "Connect your Claude account", FontId::proportional(11.0), TEXT);
            p.text(r.center() + Vec2::new(0.0, 8.0), Align2::CENTER_CENTER, "to see the real session and week %", FontId::proportional(9.5), MUTED);
            y += CONNECT_H;
        } else {
            for m in &meters {
                let top = y + 12.0 + 7.0;
                p.text(Pos2::new(left, top), Align2::LEFT_CENTER, &m.label, FontId::proportional(11.0), Color32::from_rgb(0xbd, 0xb7, 0xaf));
                let (fill, num) = level(m.pct.unwrap_or(0.0));
                let value = m.pct.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "—".into());
                p.text(Pos2::new(right, top), Align2::RIGHT_CENTER, &value, FontId::proportional(12.0), num);
                p.text(Pos2::new(right + 0.35, top), Align2::RIGHT_CENTER, &value, FontId::proportional(12.0), num);
                let track = Rect::from_min_size(Pos2::new(left, top + 7.0 + 7.0), Vec2::new(right - left, 4.0));
                p.rect_filled(track, CornerRadius::same(3), Color32::from_white_alpha(10));
                if let Some(v) = m.pct {
                    let w = track.width() * (v / 100.0).clamp(0.0, 1.0) as f32;
                    p.rect_filled(Rect::from_min_size(track.min, Vec2::new(w, 4.0)), CornerRadius::same(3), fill);
                }
                p.text(Pos2::new(left, track.bottom() + 5.0), Align2::LEFT_TOP, &m.sub, FontId::proportional(10.0), Color32::from_rgb(0xa8, 0xa2, 0x9a));
                y += METER_H;
            }
            y += 12.0;
        }

        // Details: by model, by project, the 30-day map
        hair(&p, left, right, y);
        let toggle = Rect::from_min_size(Pos2::new(left, y), Vec2::new(right - left, TOGGLE_H));
        let tr = ui.interact(toggle, ui.scope_id().with("details"), Sense::click());
        let c = if tr.hovered() { TEXT } else { Color32::from_rgb(0xc4, 0xb9, 0xaa) };
        let ty = y + 9.0 + 11.0;
        p.text(Pos2::new(left, ty), Align2::LEFT_CENTER, "Details", FontId::proportional(11.0), c);
        icon(&p, Pos2::new(right - 5.0, ty), if self.view.details { CHEVRON } else { CHEVRON_RIGHT }, c);
        if tr.clicked() {
            self.view.details = !self.view.details;
            self.view.save();
        }
        y += TOGGLE_H;
        if !self.view.details {
            return;
        }

        for (key, title, rows, open) in [
            ("models", "by model · 7 days", &models, self.view.models),
            ("projects", "by project · 7 days", &projects, self.view.projects),
        ] {
            hair(&p, left, right, y);
            let head = Rect::from_min_size(Pos2::new(left, y), Vec2::new(right - left, 22.0));
            let hr = ui.interact(head, ui.scope_id().with(key), Sense::click());
            let c = if hr.hovered() { TEXT } else { Color32::from_rgb(0xaa, 0xa5, 0x9d) };
            let hy = y + 1.0 + 9.0 + 6.0;
            p.text(Pos2::new(left, hy), Align2::LEFT_CENTER, title, FontId::proportional(10.0), c);
            icon(&p, Pos2::new(right - 5.0, hy), if open { CHEVRON } else { CHEVRON_RIGHT }, c);
            if hr.clicked() {
                if key == "models" {
                    self.view.models = !self.view.models;
                } else {
                    self.view.projects = !self.view.projects;
                }
                self.view.save();
            }
            if open {
                let max = rows.iter().map(|r| r.tokens).max().unwrap_or(1).max(1);
                let mut ry = y + 1.0 + 9.0 + 12.0;
                if rows.is_empty() {
                    p.text(Pos2::new(left, ry + 7.0 + 6.0), Align2::LEFT_CENTER, "no activity", FontId::proportional(10.0), MUTED.gamma_multiply(0.5));
                }
                for (i, r) in rows.iter().enumerate() {
                    let cy = ry + 7.0 + 6.0;
                    p.circle_filled(Pos2::new(left + 1.5, cy), 1.5, DOTS[i.min(3)]);
                    let bar_x = right - 44.0 - 7.0 - 40.0;
                    let clip = p.with_clip_rect(Rect::from_min_max(Pos2::new(left, cy - 8.0), Pos2::new(bar_x - 7.0, cy + 8.0)));
                    clip.text(Pos2::new(left + 8.0, cy), Align2::LEFT_CENTER, &r.label, FontId::proportional(10.0), Color32::from_rgb(0xd0, 0xca, 0xc2));
                    let track = Rect::from_min_size(Pos2::new(bar_x, cy - 1.5), Vec2::new(40.0, 3.0));
                    p.rect_filled(track, CornerRadius::same(2), Color32::from_white_alpha(6));
                    let w = 40.0 * r.tokens as f32 / max as f32;
                    p.rect_filled(Rect::from_min_size(track.min, Vec2::new(w, 3.0)), CornerRadius::same(2), Color32::from_rgb(0xc2, 0x86, 0x5e));
                    p.text(Pos2::new(right, cy), Align2::RIGHT_CENTER, tokens(r.tokens), FontId::proportional(10.0), Color32::from_rgb(0xaa, 0xa3, 0x9b));
                    ry += ROW_STEP;
                }
            }
            y += section_h(rows.len(), open);
        }

        // the 30-day map
        hair(&p, left, right, y);
        let my = y + 1.0 + 9.0 + 5.0;
        let heat_text = Color32::from_rgb(0xaa, 0xa3, 0x9b);
        p.text(Pos2::new(left, my), Align2::LEFT_CENTER, "30 days", FontId::proportional(9.0), heat_text);
        if let Some(u) = usage {
            p.text(Pos2::new(right, my), Align2::RIGHT_CENTER, format!("{} tokens", tokens(u.month_tokens)), FontId::proportional(9.0), heat_text);
            let max = u.days30.iter().copied().max().unwrap_or(1);
            let n = u.days30.len().max(1) as f32;
            let w = (right - left - 2.0 * (n - 1.0)) / n;
            for (i, &v) in u.days30.iter().enumerate() {
                let x = left + i as f32 * (w + 2.0);
                let sq = Rect::from_min_size(Pos2::new(x, my + 6.0 + 7.0), Vec2::new(w, 7.0));
                p.rect_filled(sq, CornerRadius::same(1), HEAT[heat_level(v, max)]);
            }
        }
    }

    fn compact(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, snap: &Snapshot, session: Option<(f64, Option<i64>)>, mood: Mood) {
        // the connected services, one row each (Codex and Cursor come later)
        let rows = vec![Row { provider: Provider::Claude, pct: session.map(|s| s.0), reset_ms: session.and_then(|s| s.1) }];
        self.resize(ctx, size_for(rows.len()));

        let card = card_frame(ui, ctx, 9);
        let p = ui.painter();

        // title band: the account and plan on the left, the controls on the right
        let band_y = card.top() + 15.5;
        account(p, card, band_y, snap, false);
        let right = card.right() - 8.0 - 9.5;
        if button(ui, Pos2::new(right, band_y), "close", CLOSE_ICON) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if button(ui, Pos2::new(right - 23.0, band_y), "expand", EXPAND_ICON) {
            self.set_size(Size::Expanded);
        }
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

        match self.view.size {
            Size::Compact => self.compact(ui, &ctx, &snap, session, mood),
            Size::Expanded => self.expanded(ui, &ctx, &snap, mood),
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
    fn the_heat_map_climbs_by_share_of_the_busiest_day() {
        assert_eq!(heat_level(0, 100), 0);
        assert_eq!(heat_level(10, 100), 1);
        assert_eq!(heat_level(50, 100), 2);
        assert_eq!(heat_level(70, 100), 3);
        assert_eq!(heat_level(100, 100), 4);
    }

    #[test]
    fn the_meter_ramp_is_plain_then_warm_then_hot() {
        assert_eq!(level(10.0).1, TEXT);
        assert_eq!(level(60.0).0, MID);
        assert_eq!(level(85.0).0, HOT);
    }

    #[test]
    fn no_account_means_no_meters_and_details_grow_the_panel() {
        assert!(meters(None, None, None).is_empty());
        let closed = View { details: false, ..View::default() };
        let open = View { details: true, ..View::default() };
        assert!(expanded_height(2, &open, 3, 5) > expanded_height(2, &closed, 3, 5));
        let folded = View { details: true, models: false, projects: false, ..View::default() };
        assert!(expanded_height(2, &folded, 3, 5) < expanded_height(2, &open, 3, 5));
    }

    #[test]
    fn countdowns_and_token_counts_read_short() {
        assert_eq!(countdown(13 * 60_000), "13m");
        assert_eq!(countdown((2 * 60 + 13) * 60_000), "2h 13m");
        assert_eq!(countdown((25 * 60) * 60_000), "1d 1h");
        assert_eq!(countdown(13 * 60_000 + 59_000), "13m");
        assert!(reset_in(2 * 3_600_000).starts_with("2h 0m ("));
        assert_eq!(tokens(37_800_000), "37.8M");
        assert_eq!(tokens(1_120_000_000), "1.12B");
        assert_eq!(tokens(950), "950");
        assert!(reset_clock(60_000).starts_with("resets "));
    }
}
