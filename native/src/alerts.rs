// The notifications Clauddy sends on its own:
//
// - a limit crossing a threshold (80% and 95% by default) notifies once;
//   only the highest threshold makes a sound;
// - an alert re-arms when usage drops back below it, and what is armed is
//   kept in alerts.json (the same file older versions wrote, per account), so a relaunch
//   at 85% doesn't repeat the 80% alert;
// - changing the thresholds re-arms everything;
// - a session pushed past 80% that rolls over to ~0 says the budget is back;
// - a dead token says so, once.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::format::{clock, countdown};

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub title: String,
    pub body: String,
    /// only the last threshold earns a sound
    pub sound: bool,
}

/// One limit to watch: "Session", "Weekly usage", "Fable weekly".
#[derive(Debug, Clone)]
pub struct Scope {
    pub label: String,
    pub pct: Option<f64>,
    pub reset_ms: Option<i64>,
    /// the session line also says how long is left
    pub session: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct State {
    pub armed: BTreeSet<String>,
    pub last_session_pct: Option<f64>,
}

// "you can work again" is only worth saying near the ceiling
const RESET_FROM: f64 = 80.0;
const RESET_TO: f64 = 5.0;

impl State {
    pub fn path(dir: &Path) -> PathBuf {
        dir.join("alerts.json")
    }

    pub fn load(dir: &Path) -> Self {
        std::fs::read(Self::path(dir)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn save(&self, dir: &Path) {
        if let Ok(body) = serde_json::to_vec(self) {
            let _ = std::fs::create_dir_all(dir);
            let _ = std::fs::write(Self::path(dir), body);
        }
    }

    /// The limits that crossed a threshold since last time. Returns the notes
    /// to send and whether the state changed (to save it).
    pub fn check(&mut self, enabled: bool, thresholds: &[i32], scopes: &[Scope]) -> (Vec<Note>, bool) {
        let mut notes = vec![];
        let mut dirty = false;
        if !enabled || thresholds.is_empty() {
            return (notes, dirty);
        }
        let top = *thresholds.iter().max().unwrap_or(&100);
        for s in scopes {
            let Some(pct) = s.pct else { continue };
            for &t in thresholds {
                let key = format!("{}:{t}", s.label);
                if pct >= t as f64 {
                    if self.armed.insert(key) {
                        dirty = true;
                        let urgent = t == top;
                        notes.push(Note {
                            title: format!("{} at {}%{}", s.label, pct.round(), if urgent { " — almost out" } else { "" }),
                            body: reset_line(s.reset_ms, s.session),
                            sound: urgent,
                        });
                    }
                } else if self.armed.remove(&key) {
                    dirty = true; // re-arm once it drops below
                }
            }
        }
        (notes, dirty)
    }

    /// A session that was near the ceiling and rolled over. Returns the note,
    /// and whether the state changed.
    pub fn window_reset(&mut self, enabled: bool, pct: Option<f64>) -> (Option<Note>, bool) {
        let Some(pct) = pct else { return (None, false) };
        let was = self.last_session_pct.replace(pct);
        let note = (enabled && was.is_some_and(|w| w >= RESET_FROM) && pct <= RESET_TO).then(|| Note {
            title: "Session window reset".into(),
            body: "full budget again".into(),
            sound: false,
        });
        let dirty = was.is_none_or(|w| w.round() != pct.round());
        (note, dirty)
    }

    /// New thresholds: everything may fire again.
    pub fn rearm(&mut self) {
        self.armed.clear();
    }
}

/// The token went dead: otherwise the widget would quietly fall back.
pub fn auth_lost() -> Note {
    Note { title: "Clauddy lost access to your usage".into(), body: "open Settings to reconnect".into(), sound: false }
}

// the alert's second line: the reset, and for the session how long is left
fn reset_line(reset_ms: Option<i64>, session: bool) -> String {
    let Some(ms) = reset_ms else { return String::new() };
    let at = format!("resets {}", clock(ms));
    if session && ms > 0 { format!("{} left · {at}", countdown(ms)) } else { at }
}

/// Ask macOS, once, for permission to notify: through UNUserNotificationCenter, with the app in front so macOS
/// shows its Allow / Don't Allow dialog in the middle of the screen. After
/// an answer macOS remembers it and this returns at once. Call on the main
/// thread at launch.
pub fn ask_permission() {
    #[cfg(target_os = "macos")]
    mac::ask_permission();
}

/// Show a note. macOS only delivers from a signed bundle (see
/// native/scripts/bundle.sh); run loose, the note is dropped rather than
/// crash. Windows and Linux go through notify-rust.
pub fn send(note: &Note) {
    #[cfg(target_os = "macos")]
    mac::send(note);
    #[cfg(not(target_os = "macos"))]
    {
        let mut n = notify_rust::Notification::new();
        n.summary(&note.title).body(&note.body).appname("Clauddy");
        let _ = n.show();
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use block2::RcBlock;
    use objc2::MainThreadMarker;
    use objc2::runtime::Bool;
    use objc2_app_kit::NSApplication;
    use objc2_foundation::{NSBundle, NSError, NSString};
    use objc2_user_notifications::{
        UNAuthorizationOptions, UNMutableNotificationContent, UNNotificationRequest, UNNotificationSound, UNUserNotificationCenter,
    };
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::Note;

    /// The notification centre exists only for a bundled app; asking for it
    /// from a loose binary throws.
    fn bundled() -> bool {
        NSBundle::mainBundle().bundleIdentifier().is_some()
    }

    pub fn ask_permission() {
        if !bundled() {
            return;
        }
        // a widget lives outside the Dock; come to the front for the moment
        // of asking, so the question is the centred dialog, not a passing banner
        if let Some(mtm) = MainThreadMarker::new() {
            NSApplication::sharedApplication(mtm).activate();
        }
        let done = RcBlock::new(|_granted: Bool, _err: *mut NSError| {});
        UNUserNotificationCenter::currentNotificationCenter()
            .requestAuthorizationWithOptions_completionHandler(UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound, &done);
    }

    pub fn send(note: &Note) {
        if !bundled() {
            return;
        }
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(&note.title));
        content.setBody(&NSString::from_str(&note.body));
        if note.sound {
            content.setSound(Some(&UNNotificationSound::defaultSound()));
        }
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let id = NSString::from_str(&format!("clauddy-{}-{}", std::process::id(), SEQ.fetch_add(1, Ordering::Relaxed)));
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(&id, &content, None);
        UNUserNotificationCenter::currentNotificationCenter().addNotificationRequest_withCompletionHandler(&request, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(pct: f64) -> Vec<Scope> {
        vec![Scope { label: "Session".into(), pct: Some(pct), reset_ms: Some(2 * 3_600_000 + 39 * 60_000), session: true }]
    }

    #[test]
    fn each_threshold_fires_once_and_only_the_top_one_sounds() {
        let mut s = State::default();
        let (notes, dirty) = s.check(true, &[80, 95], &session(82.0));
        assert!(dirty);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Session at 82%");
        assert!(notes[0].body.starts_with("2h 39m left · resets "));
        assert!(!notes[0].sound);
        assert!(s.check(true, &[80, 95], &session(85.0)).0.is_empty());
        let (notes, _) = s.check(true, &[80, 95], &session(96.0));
        assert_eq!(notes[0].title, "Session at 96% — almost out");
        assert!(notes[0].sound);
    }

    #[test]
    fn dropping_below_re_arms() {
        let mut s = State::default();
        s.check(true, &[80, 95], &session(82.0));
        let (notes, dirty) = s.check(true, &[80, 95], &session(10.0));
        assert!(notes.is_empty() && dirty);
        assert_eq!(s.check(true, &[80, 95], &session(81.0)).0.len(), 1);
    }

    #[test]
    fn a_relaunch_remembers_what_already_fired() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut s = State::default();
        s.check(true, &[80, 95], &session(85.0));
        s.save(dir.path());
        let mut again = State::load(dir.path());
        assert!(again.check(true, &[80, 95], &session(85.0)).0.is_empty());
    }

    #[test]
    fn reads_the_electron_file() {
        let s: State = serde_json::from_str(r#"{"armed":["Session:80","Weekly usage:80"],"lastSessionPct":83}"#).unwrap();
        assert!(s.armed.contains("Weekly usage:80"));
        assert_eq!(s.last_session_pct, Some(83.0));
    }

    #[test]
    fn off_means_silent_and_new_thresholds_re_arm() {
        let mut s = State::default();
        assert!(s.check(false, &[80, 95], &session(99.0)).0.is_empty());
        s.check(true, &[80, 95], &session(85.0));
        s.rearm();
        assert_eq!(s.check(true, &[70, 95], &session(85.0)).0.len(), 1);
    }

    #[test]
    fn a_rollover_from_near_the_ceiling_says_the_budget_is_back() {
        let mut s = State::default();
        s.window_reset(true, Some(86.0));
        let (note, _) = s.window_reset(true, Some(2.0));
        assert_eq!(note.unwrap().title, "Session window reset");
        // from low usage it stays quiet
        let mut quiet = State::default();
        quiet.window_reset(true, Some(40.0));
        assert!(quiet.window_reset(true, Some(1.0)).0.is_none());
    }
}
