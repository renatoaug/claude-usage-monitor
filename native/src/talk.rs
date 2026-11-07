// When the pet speaks: port of the listener in renderer/pet.js. The bubble
// speaks on transitions only, at most once per REMARK_GAP. The headline
// moments (the day's first hello, a fresh window, catching fire) may jump
// that queue, and only they blip: a noise on every remark wears out far
// faster than the remark itself.
//
// Each poll's reading goes through `listen` once; every transition is
// relative to what the pet saw before, and a first reading is a baseline
// (launching at 95% is not "catching fire").

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::components::logos::Provider;
use crate::components::pet::Mood;
use crate::format::{clock, countdown, tokens};
use crate::poller::Snapshot;
use crate::voice::{self, Line, Recap, Tone};

const MIN: i64 = 60_000;
const REMARK_GAP: i64 = 10 * MIN;
const AWAY: i64 = 120 * MIN;
const STREAK: i64 = 90 * MIN;
const STREAK_GAP: i64 = 10 * MIN;
/// a longer gap between polls is the machine asleep, not work
const TALLY_GAP: i64 = 15_000;
/// a Codex or Cursor reading this old says nothing new
const STALE: i64 = 15 * MIN;

/// A remark to show: the line, whether it may jump the queue (and blip),
/// and its tone.
#[derive(Clone, Debug, PartialEq)]
pub struct Remark {
    pub line: Line,
    pub headline: bool,
    pub tone: Tone,
}

fn remark(line: Line, headline: bool, tone: Tone) -> Option<Remark> {
    Some(Remark { line, headline, tone })
}

fn tone_of(mood: Mood) -> Tone {
    match mood {
        Mood::Fire => Tone::Fire,
        Mood::Sleeping | Mood::Tired => Tone::Sleepy,
        _ => Tone::Normal,
    }
}

// what the current Claude window has been like, for the recap when it closes
#[derive(Default)]
struct Tally {
    peak: u64,
    active_ms: i64,
    acts: BTreeMap<String, i64>,
    at: i64,
}

/// Once-a-day remarks, kept by local date in the data dir so a restart
/// doesn't repeat them.
struct Told {
    path: Option<PathBuf>,
    days: BTreeMap<String, String>,
}

impl Told {
    fn load(path: Option<PathBuf>) -> Self {
        let days = path.as_ref().and_then(|p| std::fs::read(p).ok()).and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Told { path, days }
    }
    fn today(now: i64) -> String {
        crate::format::local(now).format("%Y-%m-%d").to_string()
    }
    fn already(&self, key: &str, now: i64) -> bool {
        self.days.get(key) == Some(&Self::today(now))
    }
    fn mark(&mut self, key: &str, now: i64) {
        self.days.insert(key.into(), Self::today(now));
        if let (Some(p), Ok(body)) = (&self.path, serde_json::to_vec(&self.days)) {
            let _ = std::fs::write(p, body);
        }
    }
}

pub struct Talk {
    told: Told,
    last_remark: Option<i64>,
    greeted: bool,
    last_live: Option<f64>,
    last_ext: [Option<f64>; 2],
    away_ms: i64,
    was_sleeping: bool,
    streak_from: Option<i64>,
    last_work: i64,
    streak_told: bool,
    tally: Tally,
}

/// What a poll showed, as the listener needs it.
pub struct Seen<'a> {
    pub snap: &'a Snapshot,
    pub mood: Mood,
    /// Claude's live session: (%, ms to reset)
    pub claude: Option<(f64, Option<i64>)>,
    pub fire: f64,
    pub hour: u32,
}

impl Talk {
    pub fn new(state: Option<PathBuf>) -> Self {
        Talk {
            told: Told::load(state),
            last_remark: None,
            greeted: false,
            last_live: None,
            last_ext: [None, None],
            away_ms: 0,
            was_sleeping: false,
            streak_from: None,
            last_work: 0,
            streak_told: false,
            tally: Tally::default(),
        }
    }

    /// May this remark be said now? Headlines skip the gap; a said remark
    /// starts it.
    pub fn allow(&mut self, r: &Remark, now: i64) -> bool {
        if !r.headline && self.last_remark.is_some_and(|t| now - t < REMARK_GAP) {
            return false;
        }
        self.last_remark = Some(now);
        true
    }

    // Codex and Cursor, whichever tab is on screen. A stale reading keeps the
    // baseline: the next fresh one is compared with the last number seen.
    fn listen_ext(&mut self, seen: &Seen, now: i64) -> Option<Remark> {
        let dur = |ms: i64| countdown(ms);
        for (i, provider) in [Provider::Codex, Provider::Cursor].into_iter().enumerate() {
            let (session, limits_at) = match provider {
                Provider::Codex => match &seen.snap.codex {
                    Some(c) => (c.session.clone(), c.limits_at),
                    None => (None, None),
                },
                _ => match &seen.snap.cursor {
                    Some(c) => (c.session.clone(), c.limits_at),
                    None => (None, None),
                },
            };
            let on = match provider {
                Provider::Codex => seen.snap.codex.is_some(),
                _ => seen.snap.cursor.is_some(),
            };
            if !on {
                self.last_ext[i] = None;
                continue;
            }
            let Some(w) = session else { continue };
            let Some(pct) = w.pct else { continue };
            if limits_at.is_none_or(|t| now - t > STALE) {
                continue;
            }
            let Some(was) = self.last_ext[i].replace(pct) else { continue };
            let name = provider.name();
            if was - pct > 25.0 {
                let span = if provider == Provider::Cursor { "month" } else { "window" };
                return remark(voice::ext_reset(was, name, span), true, Tone::Normal);
            } else if was < 100.0 && pct >= 100.0 {
                return remark(voice::ext_maxed(w.reset_ms.map(clock).as_deref(), name), false, Tone::Sleepy);
            } else if was < seen.fire && pct >= seen.fire && pct < 100.0 {
                return remark(voice::ext_fire(pct, w.reset_ms, &dur, name), true, Tone::Fire);
            }
        }
        None
    }

    fn note_session(&mut self, seen: &Seen, now: i64) {
        let Some(u) = &seen.snap.usage else { return };
        let dt = if self.tally.at > 0 { (now - self.tally.at).min(TALLY_GAP) } else { 0 };
        self.tally.at = now;
        if u.active {
            self.tally.active_ms += dt;
            if let Some(a) = &u.activity {
                *self.tally.acts.entry(a.clone()).or_default() += dt;
            }
        }
        self.tally.peak = self.tally.peak.max(u.session.tokens);
    }

    fn take_recap(&mut self, was: f64) -> Recap {
        let top = self.tally.acts.iter().max_by_key(|(_, ms)| **ms).map(|(a, _)| a.clone());
        let r = Recap { was, tokens: self.tally.peak, active_ms: self.tally.active_ms, top };
        self.tally = Tally { at: self.tally.at, ..Tally::default() };
        r
    }

    /// One poll's reading in; at most one remark out. The caller decides
    /// whether it's said (talk on, gap, screen).
    pub fn listen(&mut self, seen: &Seen, now: i64) -> Option<Remark> {
        let u = seen.snap.usage.as_ref()?;
        let fmt = |n: u64| tokens(n);
        let dur = |ms: i64| countdown(ms);
        let tone = tone_of(seen.mood);
        self.note_session(seen, now);
        let mut out = None;

        if !self.greeted {
            self.greeted = true;
            if !self.told.already("greeted", now) {
                self.told.mark("greeted", now);
                if let Some(l) = voice::greeting(&u.days30, seen.hour, &fmt) {
                    return remark(l, true, tone);
                }
            }
        }
        if let Some(r) = self.listen_ext(seen, now) {
            out = Some(r);
        }

        match seen.claude {
            Some((pct, reset)) => {
                let was = self.last_live.replace(pct);
                if let Some(was) = was {
                    if was - pct > 25.0 {
                        out = out.or(remark(voice::recap(&self.take_recap(was), &fmt, &dur), true, Tone::Normal));
                    } else if was < 100.0 && pct >= 100.0 {
                        out = out.or(remark(voice::maxed(reset.map(clock).as_deref()), false, Tone::Sleepy));
                    } else if was < seen.fire && pct >= seen.fire && pct < 100.0 {
                        out = out.or(remark(voice::fire(pct, None, reset, &dur), true, Tone::Fire));
                    }
                }
            }
            // logged out, or switching: the next reading starts over
            None => self.last_live = None,
        }

        let sleeping = seen.mood == Mood::Sleeping;
        if sleeping {
            self.away_ms = self.away_ms.max(u.last_activity_ms.unwrap_or(0));
        } else if self.was_sleeping {
            if self.away_ms >= AWAY {
                out = out.or(remark(voice::welcome(self.away_ms, seen.claude.map(|s| s.0), &dur), false, tone));
            }
            self.away_ms = 0;
        }
        self.was_sleeping = sleeping;

        if seen.mood == Mood::Working {
            if self.streak_from.is_none() || now - self.last_work > STREAK_GAP {
                self.streak_from = Some(now);
                self.streak_told = false;
            }
            self.last_work = now;
            let from = self.streak_from.unwrap_or(now);
            if !self.streak_told && now - from >= STREAK && out.is_none() {
                self.streak_told = true;
                out = remark(voice::streak(now - from, &dur), false, tone);
            }
        }

        if out.is_none() && !self.told.already("record", now) {
            if let Some(l) = voice::record(&u.days30, &fmt) {
                self.told.mark("record", now);
                out = remark(l, false, tone);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{Limits, Window};

    const T0: i64 = 1_790_000_000_000;

    fn snap(days30: Vec<u64>) -> Snapshot {
        let mut u = crate::usage::Usage::empty();
        u.days30 = days30;
        Snapshot {
            usage: Some(u),
            limits: Some(Limits { session: Window { pct: 0.0, reset_ms: None }, week: Window { pct: 0.0, reset_ms: None }, scoped: vec![] }),
            ..Default::default()
        }
    }

    fn seen(s: &Snapshot, mood: Mood, pct: Option<f64>) -> Seen<'_> {
        Seen { snap: s, mood, claude: pct.map(|p| (p, Some(3_600_000))), fire: 90.0, hour: 9 }
    }

    fn talk() -> Talk {
        Talk::new(None)
    }

    #[test]
    fn the_first_reading_greets_once_a_day() {
        let mut days = vec![0; 28];
        days.extend([5_000_000, 1_000_000]);
        let s = snap(days);
        let mut t = talk();
        let r = t.listen(&seen(&s, Mood::Idle, None), T0).unwrap();
        assert!(r.headline && r.line.text.starts_with("Good morning!"));
        assert!(t.listen(&seen(&s, Mood::Idle, None), T0 + 4000).is_none());
    }

    #[test]
    fn crossing_the_fire_line_speaks_but_launching_hot_does_not() {
        let s = snap(vec![0; 30]);
        let mut t = talk();
        assert!(t.listen(&seen(&s, Mood::Fire, Some(95.0)), T0).is_none());
        let mut t = talk();
        t.listen(&seen(&s, Mood::Idle, Some(80.0)), T0);
        let r = t.listen(&seen(&s, Mood::Fire, Some(91.0)), T0 + 4000).unwrap();
        assert_eq!(r.tone, Tone::Fire);
        assert!(r.line.text.starts_with("Getting warm, 91% already."));
    }

    #[test]
    fn a_rollover_recaps_the_window_that_closed() {
        let mut s = snap(vec![0; 30]);
        let mut t = talk();
        if let Some(u) = s.usage.as_mut() {
            u.active = true;
            u.activity = Some("editing".into());
            u.session.tokens = 34_000_000;
        }
        t.listen(&seen(&s, Mood::Working, Some(92.0)), T0);
        t.listen(&seen(&s, Mood::Working, Some(93.0)), T0 + 4000);
        let r = t.listen(&seen(&s, Mood::Idle, Some(1.0)), T0 + 8000).unwrap();
        assert!(r.headline);
        assert_eq!(r.line.text, "Fresh window! That was 34.0M tokens, mostly editing code.");
    }

    #[test]
    fn the_gap_holds_back_small_talk_but_not_headlines() {
        let mut t = talk();
        let small = Remark { line: voice::maxed(None), headline: false, tone: Tone::Normal };
        let big = Remark { line: voice::reset(90.0), headline: true, tone: Tone::Normal };
        assert!(t.allow(&small, T0));
        assert!(!t.allow(&small, T0 + MIN));
        assert!(t.allow(&big, T0 + MIN));
        assert!(t.allow(&small, T0 + 12 * MIN));
    }

    #[test]
    fn a_long_sleep_is_welcomed_back() {
        let mut s = snap(vec![0; 30]);
        let mut t = talk();
        t.greeted = true;
        if let Some(u) = s.usage.as_mut() {
            u.last_activity_ms = Some(3 * 3_600_000);
        }
        t.listen(&seen(&s, Mood::Sleeping, Some(12.0)), T0);
        let r = t.listen(&seen(&s, Mood::Working, Some(12.0)), T0 + 4000).unwrap();
        assert_eq!(r.line.text, "Welcome back! You were gone 3h 0m. The session's at 12%.");
    }
}
