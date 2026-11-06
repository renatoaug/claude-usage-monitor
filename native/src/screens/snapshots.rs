// Every screen in its states, rendered headless and compared with the PNGs in
// `tests/snapshots/`. This is the gallery: open those files to review the UI.
// `UPDATE_SNAPSHOTS=1 cargo test` rewrites them after an intended change.

use eframe::egui;
use egui_kittest::Harness;

use super::{Frame, Size, View, compact, expanded};
use crate::auth::{Limits, Profile, Scoped, Window};
use crate::components::logos::Logos;
use crate::components::pet::{Mood, Pet};
use crate::poller::Snapshot;
use crate::usage::{Labeled, Reader, Usage};

const H: i64 = 3_600_000;

fn freeze_clock() {
    let t = chrono::DateTime::parse_from_rfc3339("2026-11-06T14:27:00+00:00").unwrap();
    let _ = crate::format::FROZEN.set(t);
}

fn usage(active: bool, activity: Option<&str>) -> Usage {
    let mut u = Reader::new(Some(std::env::temp_dir().join("clauddy-no-such-dir"))).get_usage(&Default::default());
    u.active = active;
    u.activity = activity.map(str::to_string);
    u.today_tokens = 37_800_000;
    u.session.tokens = 5_200_000;
    u.week.tokens = 1_120_000_000;
    u.month_tokens = 8_070_000_000;
    let l = |label: &str, tokens: u64| Labeled { label: label.into(), tokens };
    u.by_model = vec![l("Opus 5", 915_500_000), l("Fable 5.1", 200_400_000)];
    u.by_project = vec![l("clauddy", 577_800_000), l("personal-site", 366_200_000), l("little-experiments", 70_100_000)];
    u.days30 = (0..30u64).map(|i| (i * 37 % 11) * 40_000_000).collect();
    u
}

fn limits(session: f64, week: f64) -> Limits {
    Limits {
        session: Window { pct: session, reset_ms: Some(2 * H + 13 * 60_000) },
        week: Window { pct: week, reset_ms: Some(25 * H) },
        scoped: vec![Scoped { label: "Fable".into(), pct: 12.0, reset_ms: Some(25 * H) }],
    }
}

fn snapshot(u: Usage, l: Option<Limits>) -> Snapshot {
    Snapshot {
        usage: Some(u),
        connected: l.is_some(),
        limits: l,
        limits_at: None,
        profile: Some(Profile { email: Some("you@example.com".into()), name: Some("Re".into()), plan: Some("Max".into()) }),
        error: None,
        loaded: true,
        retry_at: None,
        accounts: vec![],
        active: String::new(),
        login: Default::default(),
    }
}

struct Case {
    name: &'static str,
    snap: Snapshot,
    mood: Mood,
    view: View,
}

fn render(case: Case) {
    freeze_clock();
    let session = case.snap.limits.as_ref().and_then(|l| crate::components::meter::live(&l.session, None));
    let size = |f: &Frame| match case.view.size {
        Size::Compact => compact::size(compact::rows(f).len()),
        Size::Expanded => expanded::size(f.snap, f.view),
    };
    let mut assets: Option<(Pet, Logos)> = None;
    // first pass measures the window, as the app does
    let window = {
        let ctx = egui::Context::default();
        let pet = Pet::frozen(&ctx, 0.3);
        let logos = Logos::load(&ctx);
        let f = Frame { snap: &case.snap, mood: case.mood, session, view: &case.view, pet: &pet, logos: &logos };
        size(&f)
    };
    let mut harness = Harness::builder().with_size(window).with_pixels_per_point(2.0).build_ui(|ui| {
        let ctx = ui.ctx().clone();
        let (pet, logos) = assets.get_or_insert_with(|| (Pet::frozen(&ctx, 0.3), Logos::load(&ctx)));
        let f = Frame { snap: &case.snap, mood: case.mood, session, view: &case.view, pet, logos };
        let _ = match case.view.size {
            Size::Compact => compact::draw(ui, &ctx, &f),
            Size::Expanded => expanded::draw(ui, &ctx, &f, &mut String::new()),
        };
    });
    harness.run();
    harness.snapshot(case.name);
}

fn compact_view() -> View {
    View { size: Size::Compact, ..View::default() }
}

#[test]
fn compact_idle() {
    render(Case { name: "compact-idle", snap: snapshot(usage(false, None), Some(limits(42.0, 68.0))), mood: Mood::Idle, view: compact_view() });
}

#[test]
fn compact_working() {
    render(Case {
        name: "compact-working",
        snap: snapshot(usage(true, Some("running")), Some(limits(42.0, 68.0))),
        mood: Mood::Working,
        view: compact_view(),
    });
}

#[test]
fn compact_urgent() {
    render(Case { name: "compact-urgent", snap: snapshot(usage(false, None), Some(limits(92.0, 68.0))), mood: Mood::Fire, view: compact_view() });
}

#[test]
fn compact_signed_out() {
    render(Case { name: "compact-signed-out", snap: snapshot(usage(false, None), None), mood: Mood::Sleeping, view: compact_view() });
}

#[test]
fn expanded_idle() {
    render(Case { name: "expanded-idle", snap: snapshot(usage(false, None), Some(limits(42.0, 68.0))), mood: Mood::Idle, view: View::default() });
}

#[test]
fn expanded_working_with_details() {
    render(Case {
        name: "expanded-working-details",
        snap: snapshot(usage(true, Some("editing")), Some(limits(42.0, 88.0))),
        mood: Mood::Working,
        view: View { details: true, ..View::default() },
    });
}

#[test]
fn expanded_signed_out() {
    render(Case { name: "expanded-signed-out", snap: snapshot(usage(false, None), None), mood: Mood::Idle, view: View::default() });
}

#[test]
fn expanded_maxed_out_folded() {
    render(Case {
        name: "expanded-maxed-out-folded",
        snap: snapshot(usage(false, None), Some(limits(100.0, 70.0))),
        mood: Mood::Tired,
        view: View { details: true, models: false, projects: false, ..View::default() },
    });
}

#[test]
fn expanded_connected_without_a_reading() {
    let mut snap = snapshot(usage(false, None), None);
    snap.connected = true; // throttled, or the first read hasn't landed
    render(Case { name: "expanded-waiting", snap, mood: Mood::Idle, view: View::default() });
}

#[test]
fn compact_loading() {
    let snap = Snapshot::default(); // the first poll hasn't landed
    render(Case { name: "compact-loading", snap, mood: Mood::Idle, view: compact_view() });
}

#[test]
fn expanded_loading() {
    render(Case { name: "expanded-loading", snap: Snapshot::default(), mood: Mood::Idle, view: View::default() });
}

fn two_accounts(mut snap: Snapshot) -> Snapshot {
    use crate::accounts::Account;
    snap.accounts = vec![
        Account { id: "default".into(), label: Some("you@example.com".into()), claude_dir: None },
        Account { id: "acct-2".into(), label: Some("work@example.com".into()), claude_dir: None },
    ];
    snap.active = "default".into();
    snap
}

#[test]
fn expanded_account_menu() {
    let view = View { account_menu: true, ..View::default() };
    render(Case { name: "expanded-account-menu", snap: two_accounts(snapshot(usage(false, None), Some(limits(42.0, 68.0)))), mood: Mood::Idle, view });
}

#[test]
fn expanded_account_menu_confirming_a_removal() {
    let view = View { account_menu: true, pending_remove: Some("acct-2".into()), ..View::default() };
    render(Case {
        name: "expanded-account-menu-remove",
        snap: two_accounts(snapshot(usage(false, None), Some(limits(42.0, 68.0)))),
        mood: Mood::Idle,
        view,
    });
}

#[test]
fn expanded_logging_in() {
    let mut snap = two_accounts(snapshot(usage(false, None), None));
    snap.login = crate::poller::Login::AwaitingCode;
    render(Case { name: "expanded-login", snap, mood: Mood::Idle, view: View::default() });
}

#[test]
fn expanded_login_failed() {
    let mut snap = two_accounts(snapshot(usage(false, None), None));
    snap.login = crate::poller::Login::Failed("exchange 400: invalid code".into());
    render(Case { name: "expanded-login-failed", snap, mood: Mood::Idle, view: View::default() });
}
