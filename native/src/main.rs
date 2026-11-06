// Clauddy, native: a transparent, always-on-top widget with the pet and the
// account's real usage. CLAUDDY_STATE pins a mood (idle | working | fire |
// tired | sleeping) for development.

use eframe::egui;
use std::time::Instant;

mod accounts;
mod alerts;
mod app;
mod auth;
mod components;
mod format;
mod poller;
mod prefs;
mod screens;
mod ui;
mod usage;

fn main() -> eframe::Result {
    // `clauddy --usage`: print one reading of the local logs as JSON (dev aid)
    if std::env::args().any(|a| a == "--usage") {
        let path = std::env::home_dir().unwrap_or_default().join(".claude-usage-monitor/config.json");
        let cfg: usage::Config =
            std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let mut reader = usage::Reader::new(None);
        let t = Instant::now();
        let u = reader.get_usage(&cfg);
        let cold = t.elapsed();
        let t = Instant::now();
        reader.get_usage(&cfg);
        eprintln!("cold {cold:?}, warm {:?}", t.elapsed());
        println!("{}", serde_json::to_string_pretty(&u).unwrap_or_default());
        return Ok(());
    }
    // `clauddy --notify`: send a sample alert (dev aid: checks permission and look)
    if std::env::args().any(|a| a == "--notify") {
        alerts::send(&alerts::Note { title: "Session at 82%".into(), body: "2h 39m left · resets 6:50 PM".into(), sound: false });
        return Ok(());
    }
    // `clauddy --limits`: the active account's real % and identity (dev aid)
    if std::env::args().any(|a| a == "--limits") {
        let accts = accounts::Accounts::new(None);
        let id = accts.active_id();
        let mut auth = auth::Auth::new(Some(accts.data_dir_of(&id)), Box::new(auth::Ureq::default()));
        println!("account {id}, connected {}", auth.is_connected());
        match auth.fetch_profile() {
            Ok(p) => println!("profile {}", serde_json::to_string(&p).unwrap_or_default()),
            Err(e) => println!("profile error {} {}", e.status, e.message),
        }
        match auth.fetch_usage() {
            Ok(l) => println!("limits {}", serde_json::to_string_pretty(&l).unwrap_or_default()),
            Err(e) => println!("limits error {} {}", e.status, e.message),
        }
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Clauddy")
            .with_inner_size(app::initial_size())
            .with_transparent(true)
            .with_decorations(false)
            .with_has_shadow(false)
            .with_always_on_top()
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "Clauddy",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
