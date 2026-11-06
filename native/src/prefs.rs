// The settings people change, kept in ~/.claude-usage-monitor/config.json,
// the file the Electron app writes too. Saving rewrites only these keys, so
// everything else in it (budgets, the pet, the voice) is left as it was.

use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prefs {
    /// gentle notifications on the way to the limit
    pub alerts: bool,
    pub heads_up: i32,
    pub near_limit: i32,
    /// the session % at which the pet catches fire
    pub fire: i32,
    /// floating on the desktop, or in the menu bar
    pub menu_bar: bool,
    /// Codex tracked alongside Claude
    pub codex: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs { alerts: true, heads_up: 80, near_limit: 95, fire: 90, menu_bar: false, codex: false }
    }
}

fn path() -> std::path::PathBuf {
    crate::auth::default_data_dir().join("config.json")
}

fn read() -> Value {
    std::fs::read(path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_else(|| json!({}))
}

impl Prefs {
    pub fn from_value(v: &Value) -> Self {
        let d = Prefs::default();
        let pct = |x: Option<i64>, fallback: i32| x.map(|n| n as i32).unwrap_or(fallback);
        let t = v["alertThresholds"].as_array();
        Prefs {
            alerts: v["alerts"].as_bool().unwrap_or(d.alerts),
            heads_up: pct(t.and_then(|a| a.first()).and_then(Value::as_i64), d.heads_up),
            near_limit: pct(t.and_then(|a| a.get(1)).and_then(Value::as_i64), d.near_limit),
            fire: pct(v["fireThreshold"].as_i64(), d.fire),
            menu_bar: v["mode"].as_str() == Some("menubar"),
            codex: v.pointer("/codex/enabled").and_then(Value::as_bool).unwrap_or(false),
        }
        .clamped()
    }

    pub fn load() -> Self {
        Prefs::from_value(&read())
    }

    /// Thresholds in range: 1-100, fire below 100 (100 is maxed out).
    pub fn clamped(mut self) -> Self {
        self.heads_up = self.heads_up.clamp(1, 100);
        self.near_limit = self.near_limit.clamp(1, 100);
        self.fire = self.fire.clamp(1, 99);
        self
    }

    /// Write these keys into `v`, leaving the rest alone.
    pub fn merge_into(&self, v: &mut Value) {
        if !v.is_object() {
            *v = json!({});
        }
        v["alerts"] = json!(self.alerts);
        v["alertThresholds"] = json!([self.heads_up, self.near_limit]);
        v["fireThreshold"] = json!(self.fire);
        v["mode"] = json!(if self.menu_bar { "menubar" } else { "floating" });
        if !v["codex"].is_object() {
            v["codex"] = json!({});
        }
        v["codex"]["enabled"] = json!(self.codex);
    }

    pub fn save(&self) {
        let mut v = read();
        self.merge_into(&mut v);
        if let Ok(body) = serde_json::to_string_pretty(&v) {
            let _ = std::fs::create_dir_all(crate::auth::default_data_dir());
            let _ = std::fs::write(path(), body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_electron_keys_and_falls_back_to_defaults() {
        let p = Prefs::from_value(&json!({"alerts": false, "alertThresholds": [70, 90], "fireThreshold": 85}));
        assert_eq!(p, Prefs { alerts: false, heads_up: 70, near_limit: 90, fire: 85, menu_bar: false, codex: false });
        assert!(Prefs::from_value(&json!({"codex": {"enabled": true}})).codex);
        assert!(Prefs::from_value(&json!({"mode": "menubar"})).menu_bar);
        assert_eq!(Prefs::from_value(&json!({})), Prefs::default());
    }

    #[test]
    fn saving_keeps_every_other_key() {
        let mut v = json!({"plan": "max5x", "pet": "pumpkin", "fireThreshold": 90});
        Prefs { fire: 75, ..Prefs::default() }.merge_into(&mut v);
        assert_eq!(v["fireThreshold"], 75);
        assert_eq!(v["pet"], "pumpkin");
        assert_eq!(v["plan"], "max5x");
    }

    #[test]
    fn thresholds_stay_in_range() {
        let p = Prefs { heads_up: 0, near_limit: 140, fire: 100, ..Prefs::default() }.clamped();
        assert_eq!((p.heads_up, p.near_limit, p.fire), (1, 100, 99));
    }
}
