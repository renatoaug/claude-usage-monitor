// The two layouts. A screen draws from the data it's given and returns what
// the person asked for (`Action`); the app applies it. No screen reads the
// poller, the disk or the clock on its own, so a snapshot can render any
// state from fixtures.

pub mod compact;
pub mod expanded;
#[cfg(test)]
mod snapshots;

use crate::components::logos::Logos;
use crate::components::pet::{Mood, Pet};
use crate::poller::Snapshot;

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Size {
    Expanded,
    Compact,
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
}

impl Default for View {
    fn default() -> Self {
        View { size: Size::Expanded, details: false, models: true, projects: true }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Close,
    Resize(Size),
    OpenUsagePage,
    ToggleDetails,
    ToggleModels,
    ToggleProjects,
}

/// Everything a screen draws from.
pub struct Frame<'a> {
    pub snap: &'a Snapshot,
    pub mood: Mood,
    /// the session window still in force: (%, ms to reset)
    pub session: Option<(f64, Option<i64>)>,
    pub view: &'a View,
    pub pet: &'a Pet,
    pub logos: &'a Logos,
}

/// The card sits this far inside the window, all round.
pub const MARGIN: f32 = 6.0;
