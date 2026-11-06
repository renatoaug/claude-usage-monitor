// One-shot reset reminders, asked for by hand and separate from the
// threshold alerts. Port of reminders.js; the file is the Electron app's
// `reset-reminders.json`, so a reminder armed in one fires in the other.
//
// A reminder is saved before it can fire and consumed before it's delivered,
// so a restart neither loses nor repeats it. Claude's are per account
// (`claude:<id>`); Codex and Cursor have one connection each.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::components::logos::Provider;

const HOUR: i64 = 3_600_000;
/// Cursor's budget is a billing month, so a reset can be weeks out
const MAX_AHEAD: i64 = 32 * 24 * HOUR;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reminder {
    pub key: String,
    pub provider: Provider,
    pub label: String,
    /// when the reset is due (epoch ms)
    pub at: i64,
    /// the session % when it was armed: a lower one after `at` proves the reset
    pub pct: f64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    #[serde(default)]
    pending: Vec<serde_json::Value>,
    #[serde(default)]
    delivered: BTreeMap<String, serde_json::Value>,
}

pub struct Reminders {
    path: PathBuf,
    pending: Vec<Reminder>,
    delivered: BTreeMap<String, i64>,
}

impl Reminders {
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("reset-reminders.json");
        let file: File = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        // a row that doesn't parse is dropped, as the Electron app does
        let pending =
            file.pending.into_iter().filter_map(|v| serde_json::from_value::<Reminder>(v).ok()).filter(|r| r.at.is_positive() && r.pct.is_finite()).collect();
        let delivered = file.delivered.into_iter().filter_map(|(k, v)| Some((k, v.as_f64()? as i64))).collect();
        Reminders { path, pending, delivered }
    }

    // write first, then keep: a failed write leaves things as they were
    fn persist(&mut self, pending: Vec<Reminder>, delivered: BTreeMap<String, i64>, now: i64) -> bool {
        let recent: BTreeMap<String, i64> = delivered.into_iter().filter(|(_, at)| now - at < HOUR).collect();
        let body = serde_json::json!({ "pending": pending, "delivered": recent });
        let tmp = self.path.with_extension("json.tmp");
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let ok = std::fs::write(&tmp, body.to_string()).is_ok() && std::fs::rename(&tmp, &self.path).is_ok();
        if ok {
            self.pending = pending;
            self.delivered = recent;
        }
        ok
    }

    pub fn list(&self) -> &[Reminder] {
        &self.pending
    }

    /// Arm (or re-arm) one. Refused when the reset has passed or is absurdly
    /// far, or when it can't be saved.
    pub fn arm(&mut self, r: Reminder, now: i64) -> bool {
        if r.at <= now || r.at > now + MAX_AHEAD || !r.pct.is_finite() {
            return false;
        }
        let mut next: Vec<Reminder> = self.pending.iter().filter(|p| p.key != r.key).cloned().collect();
        next.push(r);
        self.persist(next, self.delivered.clone(), now)
    }

    pub fn cancel(&mut self, key: &str, now: i64) -> bool {
        if !self.pending.iter().any(|r| r.key == key) {
            return true;
        }
        let next = self.pending.iter().filter(|r| r.key != key).cloned().collect();
        self.persist(next, self.delivered.clone(), now)
    }

    /// The reminders now due, consumed before they're delivered.
    pub fn take_due(&mut self, now: i64) -> Vec<Reminder> {
        let (due, rest): (Vec<Reminder>, Vec<Reminder>) = self.pending.iter().cloned().partition(|r| r.at <= now);
        if due.is_empty() {
            return due;
        }
        let mut delivered = self.delivered.clone();
        for r in &due {
            delivered.insert(r.key.clone(), now);
        }
        if self.persist(rest, delivered, now) { due } else { vec![] }
    }
}

/// The reminder's key: Claude's carry the account.
pub fn key(provider: Provider, account: &str) -> String {
    match provider {
        Provider::Claude => format!("claude:{account}"),
        Provider::Codex => "codex".into(),
        Provider::Cursor => "cursor".into(),
    }
}

/// What a due reminder says. Only a fresh, lower reading after the reset
/// proves the budget is back; otherwise it says the time arrived.
pub fn note(r: &Reminder, confirmed: bool) -> crate::alerts::Note {
    let (title, body) = if confirmed {
        ("Your session budget is back", format!("{}: session budget is available again.", r.label))
    } else {
        ("Time to check your session budget", format!("{}: the scheduled session reset time has arrived. Open usage to confirm the new budget.", r.label))
    };
    crate::alerts::Note { title: title.into(), body, sound: false }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const NOW: i64 = 1_790_000_000_000;

    fn claude(at: i64) -> Reminder {
        Reminder { key: "claude:default".into(), provider: Provider::Claude, label: "Claude · you".into(), at, pct: 92.0 }
    }

    #[test]
    fn armed_survives_a_restart_and_fires_once() {
        let dir = TempDir::new().unwrap();
        let mut r = Reminders::load(dir.path());
        assert!(r.arm(claude(NOW + HOUR), NOW));
        let mut again = Reminders::load(dir.path());
        assert_eq!(again.list().len(), 1);
        assert!(again.take_due(NOW).is_empty());
        assert_eq!(again.take_due(NOW + HOUR).len(), 1);
        assert!(Reminders::load(dir.path()).take_due(NOW + 2 * HOUR).is_empty());
    }

    #[test]
    fn a_passed_or_far_reset_is_refused_and_re_arming_replaces() {
        let dir = TempDir::new().unwrap();
        let mut r = Reminders::load(dir.path());
        assert!(!r.arm(claude(NOW - 1), NOW));
        assert!(!r.arm(claude(NOW + 40 * 24 * HOUR), NOW));
        assert!(r.arm(claude(NOW + HOUR), NOW));
        assert!(r.arm(claude(NOW + 2 * HOUR), NOW));
        assert_eq!(r.list().len(), 1);
        assert_eq!(r.list()[0].at, NOW + 2 * HOUR);
        assert!(r.cancel("claude:default", NOW));
        assert!(r.list().is_empty());
    }

    #[test]
    fn reads_the_electron_file() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("reset-reminders.json"),
            r#"{"pending":[{"key":"cursor","provider":"cursor","label":"Cursor","at":1790000100000,"pct":81},{"key":"x","provider":"nope"}],"delivered":{"codex":1790000000000}}"#,
        )
        .unwrap();
        let r = Reminders::load(dir.path());
        assert_eq!(r.list().len(), 1);
        assert_eq!(r.list()[0].provider, Provider::Cursor);
        assert_eq!(key(Provider::Claude, "a1"), "claude:a1");
    }

    #[test]
    fn the_note_claims_a_reset_only_when_a_reading_proves_it() {
        assert_eq!(note(&claude(NOW), true).title, "Your session budget is back");
        assert!(note(&claude(NOW), false).body.contains("reset time has arrived"));
    }
}
