// The two layouts. A screen draws from the data it's given and returns what
// the person asked for (`Action`); the app applies it. No screen reads the
// poller, the disk or the clock on its own, so a snapshot can render any
// state from fixtures.

pub mod compact;
pub mod expanded;
pub mod pet_only;
pub mod settings;
#[cfg(test)]
mod snapshots;

use crate::components::logos::{Logos, Provider};
use crate::components::pet::{Mood, Pet};
use crate::poller::Snapshot;

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Size {
    Expanded,
    Compact,
    /// just the pet, floating; hovering shows a usage glance
    #[serde(rename = "pet-only")]
    PetOnly,
}

/// What the widget remembers between runs: its size and which parts of the
/// expanded panel are open.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct View {
    pub size: Size,
    pub details: bool,
    pub models: bool,
    pub projects: bool,
    /// the account menu is open (not remembered)
    #[serde(skip)]
    pub account_menu: bool,
    /// an account whose × was clicked once: the next click removes it
    #[serde(skip)]
    pub pending_remove: Option<String>,
    /// Settings is open, on this tab (not remembered)
    #[serde(skip)]
    pub settings: Option<settings::Tab>,
    /// the service the panel shows when more than one is connected
    pub provider: Provider,
}

impl Default for View {
    fn default() -> Self {
        View { size: Size::Expanded, details: false, models: true, projects: true, account_menu: false, pending_remove: None, settings: None, provider: Provider::Claude }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Action {
    Close,
    Resize(Size),
    OpenUsagePage,
    ToggleDetails,
    ToggleModels,
    ToggleProjects,
    ToggleAccountMenu,
    CloseAccountMenu,
    SwitchAccount(String),
    AskRemoveAccount(String),
    RemoveAccount(String),
    AddAccount,
    BeginLogin,
    SubmitCode,
    ToggleSettings,
    SelectProvider(Provider),
    SetCodex(bool),
    SetCursor(bool),
    Remind(Provider, bool),
    /// the speech bubble was clicked away
    DismissBubble,
    /// the menu bar's popover goes back into the bar
    HidePopover,
    SettingsTab(settings::Tab),
    EditPrefs(crate::prefs::Prefs),
    SaveSettings,
    Logout,
}

/// Everything a screen draws from.
pub struct Frame<'a> {
    /// where the screen lays out: the card at its full, settled size
    pub area: eframe::egui::Rect,
    /// the card as it shows right now, which eases toward `area` when the
    /// screen changes (the content is revealed within it)
    pub card: eframe::egui::Rect,
    pub snap: &'a Snapshot,
    pub mood: Mood,
    /// the session window still in force: (%, ms to reset)
    pub session: Option<(f64, Option<i64>)>,
    pub view: &'a View,
    pub pet: &'a Pet,
    pub logos: &'a Logos,
    /// the panel is the menu bar's popover: no minimize, no pet-only
    pub menu_bar: bool,
    /// the lowest alert threshold: from here a reset reminder is offered
    pub heads_up: f64,
    /// what the pet is saying, and how far it has appeared (0..1)
    pub bubble: Option<(&'a str, f32)>,
}

/// The card sits this far inside the window, all round.
pub const MARGIN: f32 = 6.0;

/// The services connected now, Claude first.
pub fn connected(snap: &Snapshot) -> Vec<Provider> {
    let mut v = vec![Provider::Claude];
    if snap.codex.is_some() {
        v.push(Provider::Codex);
    }
    if snap.cursor.is_some() {
        v.push(Provider::Cursor);
    }
    v
}

/// Each connected service's session (Cursor's is its billing month): the %
/// still in force and the ms to its reset. `claude` is Claude's live window.
pub fn sessions(snap: &Snapshot, claude: Option<(f64, Option<i64>)>) -> Vec<(Provider, Option<f64>, Option<i64>)> {
    let ext = |w: &Option<crate::codex::Window>| (codex_pct(w), w.as_ref().and_then(|w| w.reset_ms));
    connected(snap)
        .into_iter()
        .map(|p| {
            let (pct, reset) = match p {
                Provider::Claude => (claude.map(|s| s.0), claude.and_then(|s| s.1)),
                Provider::Codex => ext(&snap.codex.as_ref().and_then(|c| c.session.clone())),
                Provider::Cursor => ext(&snap.cursor.as_ref().and_then(|c| c.session.clone())),
            };
            (p, pct, reset)
        })
        .collect()
}

/// The services working right now.
pub fn workers(snap: &Snapshot) -> Vec<Provider> {
    let mut v = vec![];
    if snap.usage.as_ref().is_some_and(|u| u.active) {
        v.push(Provider::Claude);
    }
    if snap.codex.as_ref().is_some_and(|c| c.active) {
        v.push(Provider::Codex);
    }
    if snap.cursor.as_ref().is_some_and(|c| c.active) {
        v.push(Provider::Cursor);
    }
    v
}

/// The service the panel shows: the chosen one while it's connected.
pub fn shown(f: &Frame) -> Provider {
    if connected(f.snap).contains(&f.view.provider) { f.view.provider } else { Provider::Claude }
}

/// A Codex window's % while it's in force (an expired reading doesn't count).
pub fn codex_pct(w: &Option<crate::codex::Window>) -> Option<f64> {
    w.as_ref().and_then(|w| w.pct)
}
