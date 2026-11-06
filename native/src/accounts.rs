// Several Claude subscriptions in one widget. Port of accounts.js.
//
// An account is a token plus, optionally, the Claude config dir whose logs it
// reads. The first one ("default") keeps the paths the app always used, so an
// existing install carries over; every extra account gets its own folder
// under `accounts/` for auth.json and alerts.json. Only the active account is
// polled — and therefore only it can notify.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_ID: &str = "default";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub label: Option<String>,
    pub claude_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct State {
    pub active: String,
    pub accounts: Vec<Account>,
}

fn default_account() -> Account {
    Account { id: DEFAULT_ID.into(), label: None, claude_dir: None }
}

fn defaults() -> State {
    State { active: DEFAULT_ID.into(), accounts: vec![default_account()] }
}

// junk rows dropped; an unknown active id can't leave the widget on nothing
fn sane(j: &Value) -> State {
    let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string);
    let mut accounts: Vec<Account> = j
        .get("accounts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|a| Some(Account { id: text(a, "id")?, label: text(a, "label"), claude_dir: text(a, "claudeDir") }))
        .collect();
    // the default account can be removed like any other, so it's recreated
    // only when nothing is left
    if accounts.is_empty() {
        accounts.push(default_account());
    }
    let wanted = j.get("active").and_then(Value::as_str);
    let active = match wanted {
        Some(id) if accounts.iter().any(|a| a.id == id) => id.to_string(),
        _ => accounts[0].id.clone(),
    };
    State { active, accounts }
}

pub struct Accounts {
    base: PathBuf,
}

impl Accounts {
    pub fn new(base: Option<PathBuf>) -> Self {
        Accounts { base: base.unwrap_or_else(crate::auth::default_data_dir) }
    }

    fn file(&self) -> PathBuf {
        self.base.join("accounts.json")
    }

    // re-read on every call: a handful of rows, read only when the list is
    // shown or changed, and a stale cache would poll a token it thinks it has
    fn load(&self) -> State {
        fs::read(self.file())
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .map(|j| sane(&j))
            .unwrap_or_else(defaults)
    }

    fn save(&self, s: &State) {
        let _ = fs::create_dir_all(&self.base);
        if let Ok(body) = serde_json::to_string_pretty(s) {
            let _ = fs::write(self.file(), body);
        }
    }

    /// Where an account's own files (auth.json, alerts.json) live.
    pub fn data_dir_of(&self, id: &str) -> PathBuf {
        if id == DEFAULT_ID { self.base.clone() } else { self.base.join("accounts").join(id) }
    }

    pub fn list(&self) -> Vec<Account> {
        self.load().accounts
    }

    pub fn active_id(&self) -> String {
        self.load().active
    }

    pub fn active(&self) -> Account {
        let s = self.load();
        s.accounts.iter().find(|a| a.id == s.active).cloned().unwrap_or_else(|| s.accounts[0].clone())
    }

    pub fn set_active(&self, id: &str) -> Option<Account> {
        let mut s = self.load();
        let hit = s.accounts.iter().find(|a| a.id == id)?.clone();
        s.active = id.into();
        self.save(&s);
        Some(hit)
    }

    /// A fresh, logged-out account; the caller switches to it and logs in.
    pub fn add(&self, claude_dir: Option<&str>) -> String {
        let mut s = self.load();
        let base36 = |mut n: u128| {
            let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
            let mut out = Vec::new();
            loop {
                out.push(digits[(n % 36) as usize]);
                n /= 36;
                if n == 0 {
                    break;
                }
            }
            out.reverse();
            String::from_utf8(out).unwrap_or_default()
        };
        let now = crate::usage::now_ms() as u128;
        let mut id = format!("acct-{}", base36(now));
        // two clicks in the same millisecond must not mint the same id
        while s.accounts.iter().any(|a| a.id == id) {
            let mut r = [0u8; 4];
            let _ = getrandom::fill(&mut r);
            id = format!("acct-{}-{}", base36(now), base36(u32::from_le_bytes(r) as u128 % 1_679_616));
        }
        s.accounts.push(Account { id: id.clone(), label: None, claude_dir: claude_dir.filter(|d| !d.is_empty()).map(str::to_string) });
        self.save(&s);
        id
    }

    /// The login flow knows the email: a better name than "Account 2".
    pub fn label(&self, id: &str, text: Option<&str>) {
        let mut s = self.load();
        let text = text.filter(|t| !t.is_empty()).map(str::to_string);
        let Some(a) = s.accounts.iter_mut().find(|a| a.id == id) else { return };
        if a.label == text {
            return;
        }
        a.label = text;
        self.save(&s);
    }

    pub fn set_claude_dir(&self, id: &str, dir: Option<&str>) {
        let mut s = self.load();
        let Some(a) = s.accounts.iter_mut().find(|a| a.id == id) else { return };
        a.claude_dir = dir.filter(|d| !d.is_empty()).map(str::to_string);
        self.save(&s);
    }

    /// Removing an account takes its token with it: a stray auth.json would
    /// be a credential nobody can see or revoke from the UI.
    pub fn remove(&self, id: &str) -> bool {
        let mut s = self.load();
        if id == s.active || s.accounts.len() < 2 {
            return false; // the widget is showing it / never leave it with none
        }
        let Some(i) = s.accounts.iter().position(|a| a.id == id) else { return false };
        s.accounts.remove(i);
        self.save(&s);
        let dir = self.data_dir_of(id);
        if id == DEFAULT_ID {
            // its folder is the app's own data dir: drop the credentials, not
            // the settings or the account list beside them
            for f in ["auth.json", "alerts.json"] {
                let _ = fs::remove_file(dir.join(f));
            }
        } else {
            let _ = fs::remove_dir_all(dir);
        }
        true
    }

    /// Drops folders under `accounts/` that no listed account owns — never
    /// one holding an auth.json, so a bad read can't destroy a token.
    pub fn prune_orphans(&self) -> usize {
        let Ok(dirs) = fs::read_dir(self.base.join("accounts")) else { return 0 };
        let known: Vec<String> = self.list().into_iter().map(|a| a.id).collect();
        let mut n = 0;
        for d in dirs.flatten() {
            let name = d.file_name().to_string_lossy().into_owned();
            if !d.file_type().is_ok_and(|t| t.is_dir()) || known.contains(&name) {
                continue;
            }
            let dir = d.path();
            if dir.join("auth.json").exists() {
                continue;
            }
            if fs::remove_dir_all(&dir).is_ok() {
                n += 1;
            }
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn fx() -> (TempDir, Accounts) {
        let root = TempDir::new().unwrap();
        let a = Accounts::new(Some(root.path().join("base")));
        (root, a)
    }

    fn read(a: &Accounts) -> Value {
        serde_json::from_slice(&fs::read(a.file()).unwrap()).unwrap()
    }

    #[test]
    fn starts_with_a_single_default_account_and_writes_nothing() {
        let (_r, a) = fx();
        assert_eq!(a.list(), vec![default_account()]);
        assert_eq!(a.active_id(), "default");
        assert!(!a.file().exists());
    }

    #[test]
    fn the_default_account_keeps_the_original_data_dir() {
        let (_r, a) = fx();
        assert_eq!(a.data_dir_of("default"), a.base);
        assert_eq!(a.data_dir_of("acct-2"), a.base.join("accounts/acct-2"));
    }

    #[test]
    fn add_creates_a_logged_out_account_and_persists_it() {
        let (_r, a) = fx();
        let id = a.add(None);
        assert_ne!(id, "default");
        assert_eq!(a.list().len(), 2);
        assert_eq!(read(&a)["accounts"][1], json!({"id": id, "label": null, "claudeDir": null}));
    }

    #[test]
    fn adds_in_the_same_millisecond_still_get_distinct_ids() {
        let (_r, a) = fx();
        let ids: std::collections::HashSet<String> = (0..3).map(|_| a.add(None)).collect();
        assert_eq!(ids.len(), 3);
        assert_eq!(a.list().len(), 4);
    }

    #[test]
    fn set_active_only_accepts_an_account_that_exists() {
        let (_r, a) = fx();
        assert_eq!(a.set_active("nope"), None);
        assert_eq!(a.active_id(), "default");
        let id = a.add(None);
        assert_eq!(a.set_active(&id).unwrap().id, id);
        assert_eq!(read(&a)["active"], id);
    }

    #[test]
    fn label_names_an_account() {
        let (_r, a) = fx();
        let id = a.add(None);
        a.label(&id, Some("work@example.com"));
        assert_eq!(read(&a)["accounts"][1]["label"], "work@example.com");
        assert_eq!(a.list().iter().find(|x| x.id == id).unwrap().label.as_deref(), Some("work@example.com"));
    }

    #[test]
    fn remove_drops_the_account_and_its_token() {
        let (_r, a) = fx();
        let id = a.add(None);
        let dir = a.data_dir_of(&id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("auth.json"), "{}").unwrap();
        assert!(a.remove(&id));
        assert!(!dir.exists());
        assert!(!a.remove("gone"));
    }

    #[test]
    fn the_account_in_use_cannot_be_removed() {
        let (_r, a) = fx();
        let id = a.add(None);
        a.set_active(&id);
        assert!(!a.remove(&id));
        assert_eq!(a.list().len(), 2);
    }

    #[test]
    fn the_last_account_standing_cannot_be_removed() {
        let (_r, a) = fx();
        let id = a.add(None);
        a.set_active(&id);
        assert!(a.remove("default"));
        a.set_active(&id);
        assert!(!a.remove(&id));
        assert_eq!(a.list().len(), 1);
    }

    #[test]
    fn the_default_account_goes_keeping_files_that_are_not_its_own() {
        let (_r, a) = fx();
        let id = a.add(None);
        a.set_active(&id);
        fs::write(a.base.join("auth.json"), "{}").unwrap();
        fs::write(a.base.join("config.json"), "{}").unwrap();
        assert!(a.remove("default"));
        assert_eq!(a.list().iter().map(|x| x.id.clone()).collect::<Vec<_>>(), vec![id]);
        assert!(!a.base.join("auth.json").exists());
        assert!(a.base.join("config.json").exists());
    }

    #[test]
    fn a_corrupted_store_falls_back_to_a_usable_state() {
        let (_r, a) = fx();
        fs::create_dir_all(&a.base).unwrap();
        let junk = json!({"active": "ghost", "accounts": [{"id": "work"}, null, {"label": "no id"}]});
        fs::write(a.file(), junk.to_string()).unwrap();
        assert_eq!(a.list().iter().map(|x| x.id.clone()).collect::<Vec<_>>(), vec!["work"]);
        assert_eq!(a.active_id(), "work");
        fs::write(a.file(), "not json").unwrap();
        assert_eq!(a.list().len(), 1);
    }

    #[test]
    fn prune_orphans_drops_unowned_folders_never_one_with_a_token() {
        let (_r, a) = fx();
        assert_eq!(a.prune_orphans(), 0); // no extra account ever added
        let keep = a.add(None);
        let dir = |id: &str| a.base.join("accounts").join(id);
        fs::create_dir_all(dir(&keep)).unwrap();
        fs::create_dir_all(dir("acct-gone")).unwrap();
        fs::write(dir("acct-gone").join("alerts.json"), "{}").unwrap();
        fs::create_dir_all(dir("acct-with-token")).unwrap();
        fs::write(dir("acct-with-token").join("auth.json"), "{}").unwrap();
        assert_eq!(a.prune_orphans(), 1);
        assert!(dir(&keep).exists());
        assert!(!dir("acct-gone").exists());
        assert!(dir("acct-with-token").exists());
    }

    #[test]
    fn set_claude_dir_points_an_account_at_its_own_logs() {
        let (_r, a) = fx();
        let id = a.add(Some("/tmp/claude-work"));
        assert_eq!(a.list().iter().find(|x| x.id == id).unwrap().claude_dir.as_deref(), Some("/tmp/claude-work"));
        a.set_claude_dir(&id, None);
        assert_eq!(read(&a)["accounts"][1]["claudeDir"], Value::Null);
    }
}
