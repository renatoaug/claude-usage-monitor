// Every screen in its states, rendered headless and compared with the PNGs in
// `tests/snapshots/`. This is the gallery: open those files to review the UI.
// `UPDATE_SNAPSHOTS=1 cargo test` rewrites them after an intended change.

use eframe::egui::{self, Pos2, Rect, Vec2};
use egui_kittest::Harness;

use super::{Frame, Size, View, compact, expanded, pet_only, settings};
use crate::auth::{Limits, Profile, Scoped, Window};
use crate::components::logos::Logos;
use crate::components::pet::{Mood, Pet};
use crate::poller::Snapshot;
use crate::prefs::Prefs;
use crate::usage::{Labeled, Reader, Usage};

const H: i64 = 3_600_000;

pub(super) fn freeze_clock() {
    let t = chrono::DateTime::parse_from_rfc3339("2026-11-06T14:27:00+00:00").unwrap();
    let _ = crate::format::FROZEN.set(t);
}

pub(super) fn usage(active: bool, activity: Option<&str>) -> Usage {
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

pub(super) fn limits(session: f64, week: f64) -> Limits {
    Limits {
        session: Window { pct: session, reset_ms: Some(2 * H + 13 * 60_000) },
        week: Window { pct: week, reset_ms: Some(25 * H) },
        scoped: vec![Scoped { label: "Fable".into(), pct: 12.0, reset_ms: Some(25 * H) }],
        weekly_only: false,
    }
}

/// A plan with no 5-hour window: its week in the session's place.
pub(super) fn weekly_only(week: f64) -> Limits {
    let w = Window { pct: week, reset_ms: Some(25 * H) };
    Limits { session: w.clone(), week: w, weekly_only: true, ..limits(0.0, week) }
}

pub(super) fn snapshot(u: Usage, l: Option<Limits>) -> Snapshot {
    Snapshot {
        usage: Some(u),
        connected: l.is_some(),
        limits: l,
        limits_at: None,
        profile: Some(Profile { email: Some("you@example.com".into()), name: Some("Re".into()), plan: Some("Max".into()) }),
        loaded: true,
        retry_at: None,
        accounts: vec![],
        active: String::new(),
        login: Default::default(),
        codex: None,
        codex_found: None,
        cursor: None,
        cursor_found: None,
        reminders: vec![],
        reminder_error: None,
        seq: 0,
    }
}

struct Case {
    name: &'static str,
    snap: Snapshot,
    mood: Mood,
    view: View,
}

fn render_with(case: Case, draft: Prefs) {
    render_inner(case, draft)
}

fn render(case: Case) {
    render_inner(case, Prefs::default())
}

fn render_inner(case: Case, draft: Prefs) {
    freeze_clock();
    let session = case.snap.limits.as_ref().and_then(|l| crate::components::meter::live(&l.session, None));
    let size = |f: &Frame| match (case.view.settings, case.view.size) {
        (Some(tab), _) => settings::size(f.snap, tab, f.view),
        (None, Size::Compact) => compact::size(compact::rows(f).len(), compact::reminder(f).is_some()),
        (None, Size::Expanded) => expanded::size(f.snap, f.view, f.session, f.heads_up),
        (None, Size::PetOnly) => pet_only::size(),
    };
    let mut assets: Option<(Pet, Logos)> = None;
    // first pass measures the window, as the app does
    let window = {
        let ctx = egui::Context::default();
        let pet = Pet::frozen(&ctx, 0.3);
        if let Some(skin) = crate::components::skins::SKINS.iter().find(|s| case.name.starts_with(s.id)) {
            pet.set_skin(skin);
        }
        let logos = Logos::load(&ctx);
        let full = eframe::egui::Rect::NOTHING;
        let f = Frame {
            area: full,
            card: full,
            snap: &case.snap,
            mood: case.mood,
            session,
            view: &case.view,
            pet: &pet,
            logos: &logos,
            menu_bar: case.name.ends_with("menu-bar"),
            heads_up: 80.0,
            bubble: case
                .name
                .ends_with("-talking")
                .then_some((if case.name.starts_with("compact") { "91%, getting warm!" } else { "Getting warm, 91% already. It resets in 1h 12m." }, 1.0)),
            update: if case.name.ends_with("-update") { crate::updates::Update::Available("v1.4.0".into()) } else { Default::default() },
        };
        size(&f)
    };
    let mut harness = Harness::builder().with_size(window).with_pixels_per_point(2.0).build_ui(|ui| {
        let ctx = ui.ctx().clone();
        let (pet, logos) = assets.get_or_insert_with(|| (Pet::frozen(&ctx, 0.3), Logos::load(&ctx)));
        if let Some(skin) = crate::components::skins::SKINS.iter().find(|s| case.name.starts_with(s.id)) {
            pet.set_skin(skin);
        }
        let area = ctx.content_rect().shrink(super::MARGIN);
        let f = Frame {
            area,
            card: area,
            snap: &case.snap,
            mood: case.mood,
            session,
            view: &case.view,
            pet,
            logos,
            menu_bar: case.name.ends_with("menu-bar"),
            heads_up: 80.0,
            bubble: case
                .name
                .ends_with("-talking")
                .then_some((if case.name.starts_with("compact") { "91%, getting warm!" } else { "Getting warm, 91% already. It resets in 1h 12m." }, 1.0)),
            update: if case.name.ends_with("-update") { crate::updates::Update::Available("v1.4.0".into()) } else { Default::default() },
        };
        let _ = match (case.view.settings, case.view.size) {
            (Some(tab), _) => settings::draw(ui, &ctx, &f, tab, &draft, &Prefs::default(), &mut String::new()),
            (None, Size::Compact) => compact::draw(ui, &ctx, &f),
            (None, Size::Expanded) => expanded::draw(ui, &ctx, &f, &mut String::new()),
            (None, Size::PetOnly) => pet_only::draw(ui, &ctx, &f, Some(case.name.ends_with("glance"))).actions,
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
    render(Case { name: "compact-working", snap: snapshot(usage(true, Some("running")), Some(limits(42.0, 68.0))), mood: Mood::Working, view: compact_view() });
}

#[test]
fn compact_urgent() {
    render(Case { name: "compact-urgent", snap: snapshot(usage(false, None), Some(limits(92.0, 68.0))), mood: Mood::Fire, view: compact_view() });
}

#[test]
fn compact_weekly_only() {
    render(Case { name: "compact-weekly-only", snap: snapshot(usage(false, None), Some(weekly_only(64.0))), mood: Mood::Idle, view: compact_view() });
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
fn expanded_weekly_only() {
    render(Case { name: "expanded-weekly-only", snap: snapshot(usage(false, None), Some(weekly_only(64.0))), mood: Mood::Idle, view: View::default() });
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
    render(Case { name: "expanded-account-menu-remove", snap: two_accounts(snapshot(usage(false, None), Some(limits(42.0, 68.0)))), mood: Mood::Idle, view });
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

fn settings_view(tab: settings::Tab) -> View {
    View { settings: Some(tab), ..View::default() }
}

#[test]
fn settings_accounts() {
    let snap = two_accounts(snapshot(usage(false, None), Some(limits(42.0, 68.0))));
    render(Case { name: "settings-accounts", snap, mood: Mood::Idle, view: settings_view(settings::Tab::Accounts) });
}

#[test]
fn settings_accounts_logging_in() {
    let mut snap = two_accounts(snapshot(usage(false, None), None));
    snap.login = crate::poller::Login::AwaitingCode;
    render(Case { name: "settings-accounts-login", snap, mood: Mood::Idle, view: settings_view(settings::Tab::Accounts) });
}

#[test]
fn settings_alerts_with_an_unsaved_edit() {
    let snap = snapshot(usage(false, None), Some(limits(42.0, 68.0)));
    let draft = Prefs { fire: 85, ..Prefs::default() };
    render_with(Case { name: "settings-alerts", snap, mood: Mood::Idle, view: settings_view(settings::Tab::Alerts) }, draft);
}

#[test]
fn settings_appearance() {
    let snap = snapshot(usage(false, None), Some(limits(42.0, 68.0)));
    render(Case { name: "settings-appearance", snap, mood: Mood::Idle, view: settings_view(settings::Tab::Appearance) });
}

fn pet_only_view() -> View {
    View { size: Size::PetOnly, ..View::default() }
}

#[test]
fn pet_only() {
    render(Case { name: "pet-only", snap: snapshot(usage(false, None), Some(limits(42.0, 68.0))), mood: Mood::Idle, view: pet_only_view() });
}

#[test]
fn pet_only_with_its_glance() {
    render(Case { name: "pet-only-glance", snap: snapshot(usage(false, None), Some(limits(42.0, 68.0))), mood: Mood::Idle, view: pet_only_view() });
}

#[test]
fn pet_only_weekly_only_glance() {
    render(Case { name: "pet-only-weekly-only-glance", snap: snapshot(usage(false, None), Some(weekly_only(64.0))), mood: Mood::Idle, view: pet_only_view() });
}

#[test]
fn pet_only_working_with_its_glance() {
    render(Case {
        name: "pet-only-working-glance",
        snap: snapshot(usage(true, Some("running")), Some(limits(86.0, 68.0))),
        mood: Mood::Working,
        view: pet_only_view(),
    });
}

pub(super) fn with_codex(mut snap: Snapshot, working: bool) -> Snapshot {
    use crate::codex::{CodexUsage, Window};
    let l = |label: &str, tokens: u64| Labeled { label: label.into(), tokens };
    snap.codex = Some(CodexUsage {
        active: working,
        last_seen: Some(0),
        plan: Some("plus".into()),
        session: Some(Window { pct: Some(28.0), reset_ms: Some(3 * H), expired: false }),
        weekly: Some(Window { pct: Some(64.0), reset_ms: Some(4 * 24 * H), expired: false }),
        weekly_only: false,
        limits_at: None,
        tokens_today: 12_400_000,
        tokens_5h: 3_100_000,
        tokens_week: 88_000_000,
        by_model: vec![l("gpt-6", 70_000_000), l("gpt-6-mini", 18_000_000)],
        by_project: vec![l("clauddy", 52_000_000), l("api", 36_000_000)],
        days30: (0..30u64).map(|i| (i * 13 % 7) * 4_000_000).collect(),
        month_tokens: 380_000_000,
    });
    snap.codex_found = Some((true, Some("plus".into())));
    snap
}

#[test]
fn compact_with_codex() {
    render(Case {
        name: "compact-codex",
        snap: with_codex(snapshot(usage(true, Some("running")), Some(limits(42.0, 68.0))), true),
        mood: Mood::Working,
        view: compact_view(),
    });
}

#[test]
fn expanded_with_codex_on_claude() {
    render(Case {
        name: "expanded-codex-claude-tab",
        snap: with_codex(snapshot(usage(false, None), Some(limits(42.0, 68.0))), false),
        mood: Mood::Idle,
        view: View::default(),
    });
}

#[test]
fn expanded_showing_codex() {
    use crate::components::logos::Provider;
    render(Case {
        name: "expanded-codex-tab",
        snap: with_codex(snapshot(usage(false, None), Some(limits(42.0, 68.0))), true),
        mood: Mood::Working,
        view: View { provider: Provider::Codex, details: true, ..View::default() },
    });
}

#[test]
fn settings_codex_card() {
    use crate::components::logos::Provider;
    render(Case {
        name: "settings-codex",
        snap: with_codex(snapshot(usage(false, None), Some(limits(42.0, 68.0))), false),
        mood: Mood::Idle,
        view: View { provider: Provider::Codex, settings: Some(settings::Tab::Accounts), ..View::default() },
    });
}

#[test]
fn pet_only_glance_with_codex() {
    render(Case {
        name: "pet-only-codex-glance",
        snap: with_codex(snapshot(usage(false, None), Some(limits(42.0, 68.0))), false),
        mood: Mood::Idle,
        view: pet_only_view(),
    });
}

pub(super) fn with_cursor(mut snap: Snapshot, working: bool) -> Snapshot {
    use crate::codex::Window;
    snap.cursor = Some(crate::cursor::CursorUsage {
        active: working,
        activity: working.then_some("editing"),
        last_seen: Some(0),
        plan: Some("pro".into()),
        session: Some(Window { pct: Some(57.0), reset_ms: Some(12 * 24 * H), expired: false }),
        api: Some(Window { pct: Some(23.0), reset_ms: Some(12 * 24 * H), expired: false }),
        limits_at: None,
        signed_out: false,
    });
    snap.cursor_found = Some((true, Some("pro".into())));
    snap
}

#[test]
fn compact_with_all_three() {
    render(Case {
        name: "compact-trio",
        snap: with_cursor(with_codex(snapshot(usage(false, None), Some(limits(42.0, 68.0))), false), true),
        mood: Mood::Working,
        view: compact_view(),
    });
}

#[test]
fn expanded_showing_cursor() {
    use crate::components::logos::Provider;
    render(Case {
        name: "expanded-cursor-tab",
        snap: with_cursor(with_codex(snapshot(usage(false, None), Some(limits(42.0, 68.0))), false), true),
        mood: Mood::Working,
        view: View { provider: Provider::Cursor, details: true, ..View::default() },
    });
}

#[test]
fn settings_cursor_card() {
    use crate::components::logos::Provider;
    render(Case {
        name: "settings-cursor",
        snap: with_cursor(snapshot(usage(false, None), Some(limits(42.0, 68.0))), false),
        mood: Mood::Idle,
        view: View { provider: Provider::Cursor, settings: Some(settings::Tab::Accounts), ..View::default() },
    });
}

#[test]
fn expanded_in_the_menu_bar() {
    render(Case { name: "expanded-menu-bar", snap: snapshot(usage(false, None), Some(limits(42.0, 68.0))), mood: Mood::Idle, view: View::default() });
}

#[test]
fn expanded_offering_a_reset_reminder() {
    let mut snap = snapshot(usage(false, None), Some(limits(86.0, 68.0)));
    snap.limits_at = Some(std::time::Instant::now());
    render(Case { name: "expanded-reminder", snap, mood: Mood::Idle, view: View::default() });
}

#[test]
fn expanded_with_a_reminder_on() {
    let mut snap = snapshot(usage(false, None), Some(limits(42.0, 68.0)));
    snap.reminders = vec![crate::reminders::Reminder {
        key: "claude:".into(),
        provider: crate::components::logos::Provider::Claude,
        label: "Claude · you".into(),
        at: 0,
        pct: 86.0,
    }];
    render(Case { name: "expanded-reminder-on", snap, mood: Mood::Idle, view: View::default() });
}

#[test]
fn expanded_talking() {
    render(Case { name: "expanded-talking", snap: snapshot(usage(false, None), Some(limits(91.0, 68.0))), mood: Mood::Fire, view: View::default() });
}

#[test]
fn compact_talking() {
    render(Case { name: "compact-talking", snap: snapshot(usage(false, None), Some(limits(91.0, 68.0))), mood: Mood::Fire, view: compact_view() });
}

#[test]
fn compact_offering_a_reset_reminder() {
    let mut snap = snapshot(usage(false, None), Some(limits(86.0, 68.0)));
    snap.limits_at = Some(std::time::Instant::now());
    render(Case { name: "compact-reminder", snap, mood: Mood::Idle, view: compact_view() });
}

#[test]
fn settings_offering_an_update() {
    render(Case {
        name: "settings-update",
        snap: snapshot(usage(false, None), Some(limits(42.0, 68.0))),
        mood: Mood::Idle,
        view: View { settings: Some(settings::Tab::Accounts), ..View::default() },
    });
}

fn pet_snap(session: f64, working: bool) -> Snapshot {
    snapshot(usage(working, working.then_some("editing")), Some(limits(session, 68.0)))
}

#[test]
fn wizard_expanded() {
    render(Case { name: "wizard-expanded", snap: pet_snap(42.0, false), mood: Mood::Idle, view: View::default() });
}

#[test]
fn wizard_expanded_working() {
    render(Case { name: "wizard-expanded-working", snap: pet_snap(42.0, true), mood: Mood::Working, view: View::default() });
}

#[test]
fn wizard_compact_on_fire() {
    render(Case { name: "wizard-compact-fire", snap: pet_snap(93.0, false), mood: Mood::Fire, view: compact_view() });
}

#[test]
fn wizard_pet_only() {
    render(Case { name: "wizard-pet-only", snap: pet_snap(42.0, false), mood: Mood::Idle, view: pet_only_view() });
}

#[test]
fn santa_expanded_working() {
    render(Case { name: "santa-expanded-working", snap: pet_snap(42.0, true), mood: Mood::Working, view: View::default() });
}

#[test]
fn santa_compact_on_fire() {
    render(Case { name: "santa-compact-fire", snap: pet_snap(93.0, false), mood: Mood::Fire, view: compact_view() });
}

#[test]
fn reindeer_expanded_working() {
    render(Case { name: "reindeer-expanded-working", snap: pet_snap(42.0, true), mood: Mood::Working, view: View::default() });
}

#[test]
fn reindeer_pet_only() {
    render(Case { name: "reindeer-pet-only", snap: pet_snap(42.0, false), mood: Mood::Idle, view: pet_only_view() });
}

#[test]
fn gingerbread_expanded() {
    render(Case { name: "gingerbread-expanded", snap: pet_snap(42.0, false), mood: Mood::Idle, view: View::default() });
}

#[test]
fn gingerbread_compact_on_fire() {
    render(Case { name: "gingerbread-compact-fire", snap: pet_snap(93.0, false), mood: Mood::Fire, view: compact_view() });
}

#[test]
fn pumpkin_expanded_working() {
    render(Case { name: "pumpkin-expanded-working", snap: pet_snap(42.0, true), mood: Mood::Working, view: View::default() });
}

#[test]
fn pumpkin_compact_on_fire() {
    render(Case { name: "pumpkin-compact-fire", snap: pet_snap(93.0, false), mood: Mood::Fire, view: compact_view() });
}

#[test]
fn astronaut_expanded() {
    render(Case { name: "astronaut-expanded", snap: pet_snap(42.0, false), mood: Mood::Idle, view: View::default() });
}

#[test]
fn astronaut_pet_only() {
    render(Case { name: "astronaut-pet-only", snap: pet_snap(42.0, false), mood: Mood::Idle, view: pet_only_view() });
}

#[test]
fn settings_picking_a_pet() {
    let draft = Prefs { pet: "wizard".into(), ..Prefs::default() };
    render_with(
        Case {
            name: "settings-pet-picker",
            snap: snapshot(usage(false, None), Some(limits(42.0, 68.0))),
            mood: Mood::Idle,
            view: View { settings: Some(settings::Tab::Appearance), pet_picker: true, ..View::default() },
        },
        draft,
    );
}

// Without the OS's cursor (Windows, Linux) the window's own pointer decides:
// moving up from the pet to the glance's button must not close the glance.
#[test]
fn pet_only_glance_stays_up_on_the_way_to_its_button() {
    freeze_clock();
    let snap = snapshot(usage(false, None), Some(limits(42.0, 68.0)));
    let view = pet_only_view();
    let hot: std::rc::Rc<std::cell::RefCell<Vec<Rect>>> = Default::default();
    let seen = hot.clone();
    let mut assets: Option<(Pet, Logos)> = None;
    let size: Vec2 = pet_only::size().into();
    let mut harness = Harness::builder().with_size(size).build_ui(|ui| {
        let ctx = ui.ctx().clone();
        let (pet, logos) = assets.get_or_insert_with(|| (Pet::frozen(&ctx, 0.3), Logos::load(&ctx)));
        let area = ctx.content_rect().shrink(super::MARGIN);
        let f = Frame {
            area,
            card: area,
            snap: &snap,
            mood: Mood::Idle,
            session: None,
            view: &view,
            pet,
            logos,
            menu_bar: false,
            heads_up: 80.0,
            bubble: None,
            update: Default::default(),
        };
        *seen.borrow_mut() = pet_only::draw(ui, &ctx, &f, None).hot;
    });
    harness.run();
    let body = hot.borrow()[0];
    harness.hover_at(body.center());
    harness.run();
    let up = hot.borrow()[0];
    assert!(up.top() < body.top(), "the glance comes up over the pet");
    // halfway between the glance and the head: off the pet, still on the way
    harness.hover_at(Pos2::new(body.center().x, (up.top() + body.top()) / 2.0));
    harness.run();
    assert_eq!(hot.borrow()[0], up, "the glance stays up");
    // and it goes once the pointer leaves both
    harness.hover_at(Pos2::new(1.0, size.y - 1.0));
    harness.run();
    assert_eq!(hot.borrow()[0], body);
}
