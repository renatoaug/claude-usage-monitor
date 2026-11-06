// The expanded panel's meters: label and % on top, a bar, a subtitle with
// the reset and the tokens behind it.

use eframe::egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Vec2};
use std::time::Instant;

use crate::auth::Window;
use crate::format::{countdown, reset_in, tokens};
use crate::poller::Snapshot;
use crate::ui::bar::bar;
use crate::ui::skeleton::sk_line;
use crate::ui::text::bold;
use crate::ui::theme::{color, text};

pub const METER_H: f32 = 54.0;
// the session's meter leads with a big number
pub const HERO_H: f32 = 70.0;

/// The meter's height: the first one (the session) is the hero.
pub fn height(first: bool) -> f32 {
    if first { HERO_H } else { METER_H }
}

pub struct Meter {
    pub label: String,
    pub pct: Option<f64>,
    pub sub: String,
}

/// A limits window still in force, its countdown rebased to now. A reading
/// whose reset already passed proves nothing about the new window: None,
/// never zero.
pub fn live(w: &Window, at: Option<Instant>) -> Option<(f64, Option<i64>)> {
    let elapsed = at.map(|a| a.elapsed().as_millis() as i64).unwrap_or(0);
    match w.reset_ms {
        Some(ms) if ms - elapsed <= 0 => None,
        Some(ms) => Some((w.pct, Some(ms - elapsed))),
        None => Some((w.pct, None)),
    }
}

/// One ramp for every meter: plain below 60, warm to 85, hot past it.
/// Returns (bar, number) colours.
pub fn level(pct: f64, accent: Color32) -> (Color32, Color32) {
    if pct >= 85.0 {
        (color::HOT, color::HOT)
    } else if pct >= 60.0 {
        (color::WARM, color::WARM)
    } else {
        (accent, color::FG)
    }
}

// a reading older than this says how old it is instead of its tokens
const STALE_MS: i64 = 15 * 60_000;

// a Codex window's subtitle: its reset and tokens; how old it is when the
// reading is stale; waiting when the window passed with no fresh reading
fn ext_sub(w: &Option<crate::codex::Window>, tokens: Option<u64>, stale: Option<i64>) -> String {
    let t = tokens.map(|n| format!("{} tokens", crate::format::tokens(n)));
    let join = |parts: Vec<Option<String>>| parts.into_iter().flatten().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
    let Some(w) = w else { return join(vec![Some("limits not recorded yet".into()), t]) };
    if w.pct.is_none() {
        return join(vec![Some("waiting for a fresh reading".into()), t]);
    }
    if let Some(age) = stale {
        let at = w.reset_ms.map(|ms| format!("resets {}", crate::format::clock(ms)));
        return join(vec![at, Some(format!("read {} ago", countdown(age)))]);
    }
    let s = join(vec![w.reset_ms.map(|ms| format!("resets in {}", reset_in(ms))), t]);
    if s.is_empty() { "—".into() } else { s }
}

/// Codex's meters: its session (or, on a weekly-only plan, its week in that
/// place) and its week.
pub fn codex_meters(c: &crate::codex::CodexUsage) -> Vec<Meter> {
    let now = crate::usage::now_ms();
    let stale = c.limits_at.map(|at| now - at).filter(|&age| age > STALE_MS);
    let pct = |w: &Option<crate::codex::Window>| w.as_ref().and_then(|w| w.pct);
    let mut v = vec![Meter {
        label: if c.weekly_only { "weekly · all models".into() } else { "current session".into() },
        pct: pct(&c.session),
        sub: ext_sub(&c.session, Some(if c.weekly_only { c.tokens_week } else { c.tokens_5h }), stale),
    }];
    if !c.weekly_only {
        v.push(Meter { label: "weekly · all models".into(), pct: pct(&c.weekly), sub: ext_sub(&c.weekly, Some(c.tokens_week), stale) });
    }
    v
}

/// The session, the week, and any per-model weekly limits. Connected but not
/// read yet (or throttled): the meters stay, empty, rather than ask to log in.
pub fn meters(snap: &Snapshot) -> Vec<Meter> {
    let (limits, at, usage) = (snap.limits.as_ref(), snap.limits_at, snap.usage.as_ref());
    let Some(l) = limits else {
        if !snap.connected {
            return vec![];
        }
        let sub = match snap.retry_at {
            Some(t) => format!("rate limited · retrying in {}", countdown(t.saturating_duration_since(Instant::now()).as_millis() as i64 + 59_999)),
            None => "waiting for a reading".into(),
        };
        let waiting = |label: &str| Meter { label: label.into(), pct: None, sub: sub.clone() };
        return vec![waiting("current session"), waiting("weekly · all models")];
    };
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
    // per-model limits pair with that model family's local tokens
    for sc in &l.scoped {
        let Some((pct, reset)) = live(&Window { pct: sc.pct, reset_ms: sc.reset_ms }, at) else { continue };
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

/// Draw `m` between `left` and `right`, starting at `y`. The hero (the
/// session) has a 27 px number on the label's baseline and a thicker bar.
pub fn meter(p: &Painter, left: f32, right: f32, y: f32, m: &Meter, hero: bool, accent: Color32) {
    let (fill, num) = level(m.pct.unwrap_or(0.0), accent);
    let value = m.pct.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "—".into());
    let (track_y, track_h) = if hero {
        let base = y + 8.0 + 30.0;
        p.text(Pos2::new(left, base), Align2::LEFT_BOTTOM, &m.label, FontId::proportional(text::BODY), color::FG_LABEL);
        p.text(Pos2::new(right, base + 4.0), Align2::RIGHT_BOTTOM, &value, FontId::proportional(text::HERO), num);
        (base + 9.0, 5.0)
    } else {
        let top = y + 19.0;
        p.text(Pos2::new(left, top), Align2::LEFT_CENTER, &m.label, FontId::proportional(text::BODY), color::FG_LABEL);
        bold(p, Pos2::new(right, top), Align2::RIGHT_CENTER, &value, text::VALUE, num);
        (top + 14.0, 4.0)
    };
    let track = Rect::from_min_size(Pos2::new(left, track_y), Vec2::new(right - left, track_h));
    bar(p, track, m.pct.map(|v| (v / 100.0) as f32), fill, color::TRACK);
    p.text(Pos2::new(left, track.bottom() + 5.0), Align2::LEFT_TOP, &m.sub, FontId::proportional(text::SMALL), color::FG_SUB);
}

/// A meter's placeholder while the first reading loads.
pub fn meter_skeleton(p: &Painter, left: f32, right: f32, y: f32, hero: bool) {
    let (label_y, value_w, value_h, track_y, track_h) =
        if hero { (y + 30.0, 46.0, 20.0, y + 47.0, 5.0) } else { (y + 19.0, 26.0, 10.0, y + 33.0, 4.0) };
    sk_line(p, left, label_y, 86.0, 9.0);
    sk_line(p, right - value_w, label_y - (value_h - 9.0) / 2.0, value_w, value_h);
    bar(p, Rect::from_min_size(Pos2::new(left, track_y), Vec2::new(right - left, track_h)), None, color::TRACK, color::TRACK);
    sk_line(p, left, track_y + track_h + 11.0, 170.0, 8.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_expired_window_is_no_reading_not_zero() {
        assert_eq!(live(&Window { pct: 80.0, reset_ms: Some(-5) }, None), None);
        assert_eq!(live(&Window { pct: 80.0, reset_ms: Some(60_000) }, None).map(|r| r.0), Some(80.0));
    }

    #[test]
    fn the_ramp_is_plain_then_warm_then_hot() {
        assert_eq!(level(10.0, color::ACCENT), (color::ACCENT, color::FG));
        assert_eq!(level(60.0, color::ACCENT).0, color::WARM);
        assert_eq!(level(85.0, color::CODEX_BAR).0, color::HOT);
    }

    #[test]
    fn no_account_means_no_meters() {
        assert!(meters(&Snapshot::default()).is_empty());
        let waiting = meters(&Snapshot { connected: true, ..Default::default() });
        assert_eq!(waiting.len(), 2);
        assert!(waiting.iter().all(|m| m.pct.is_none()));
    }
}

#[cfg(test)]
mod codex_tests {
    use super::*;
    use crate::codex::Window;

    #[test]
    fn a_codex_window_says_reset_tokens_or_how_old_it_is() {
        let w = Some(Window { pct: Some(20.0), reset_ms: Some(2 * 3_600_000), expired: false });
        assert!(ext_sub(&w, Some(5_000), None).starts_with("resets in 2h 0m ("));
        assert!(ext_sub(&w, Some(5_000), Some(20 * 60_000)).ends_with("read 20m ago"));
        let gone = Some(Window { pct: None, reset_ms: None, expired: true });
        assert_eq!(ext_sub(&gone, None, None), "waiting for a fresh reading");
        assert_eq!(ext_sub(&None, None, None), "limits not recorded yet");
    }
}
