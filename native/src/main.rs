// no console window behind the widget on Windows
#![cfg_attr(windows, windows_subsystem = "windows")]

// Clauddy, native: a transparent, always-on-top widget with the pet and the
// account's real usage. CLAUDDY_STATE pins a mood (idle | working | fire |
// tired | sleeping) for development.

use eframe::egui;
use std::time::Instant;

mod accounts;
mod alerts;
mod app;
mod auth;
mod codex;
mod components;
mod cursor;
mod format;
mod platform;
mod poller;
mod prefs;
mod reminders;
mod screens;
mod talk;
mod ui;
mod updates;
mod usage;
mod voice;

fn main() -> eframe::Result {
    // `clauddy --usage`: print one reading of the local logs as JSON (dev aid)
    if std::env::args().any(|a| a == "--usage") {
        let path = std::env::home_dir().unwrap_or_default().join(".claude-usage-monitor/config.json");
        let cfg: usage::Config = std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let mut reader = usage::Reader::new(None);
        reader.set_wsl(prefs::Prefs::load().wsl);
        let t = Instant::now();
        let u = reader.get_usage(&cfg);
        let cold = t.elapsed();
        let t = Instant::now();
        reader.get_usage(&cfg);
        eprintln!("cold {cold:?}, warm {:?}", t.elapsed());
        println!("{}", serde_json::to_string_pretty(&u).unwrap_or_default());
        return Ok(());
    }
    // `clauddy --codex`: one reading of Codex's logs as JSON (dev aid; no live read)
    if std::env::args().any(|a| a == "--codex") {
        let mut c = codex::Codex::new(None);
        let u = c.get_usage(usage::now_ms());
        println!("{}", serde_json::to_string_pretty(&u).unwrap_or_default());
        return Ok(());
    }
    // `clauddy --cursor`: Cursor's sign-in, limits and activity as JSON (dev aid)
    if std::env::args().any(|a| a == "--cursor") {
        let mut c = cursor::Cursor::new();
        eprintln!("detect {:?}", c.detect());
        c.refresh_blocking(usage::now_ms());
        let u = c.get_usage(usage::now_ms());
        println!("{}", serde_json::to_string_pretty(&u).unwrap_or_default());
        return Ok(());
    }
    // `clauddy --say`: play a line's chiptune blips (dev aid: checks the sound)
    if std::env::args().any(|a| a == "--say") {
        voice::play("Getting warm, 91% already.", voice::Tone::Fire);
        std::thread::sleep(std::time::Duration::from_secs(3));
        return Ok(());
    }
    // `clauddy --notify`: send a sample alert (dev aid: checks permission and look)
    if std::env::args().any(|a| a == "--notify") {
        alerts::ask_permission();
        alerts::send(&alerts::Note { title: "Session at 82%".into(), body: "2h 39m left · resets 6:50 PM".into(), sound: false });
        std::thread::sleep(std::time::Duration::from_secs(3)); // let it be delivered before exiting
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
        viewport: with_app_icon(
            egui::ViewportBuilder::default()
                .with_title("Clauddy")
                .with_inner_size(app::initial_size())
                // living in the menu bar, the panel waits for a click on the icon
                .with_visible(!prefs::Prefs::load().menu_bar)
                .with_transparent(true)
                .with_decorations(false)
                .with_has_shadow(false)
                .with_always_on_top()
                .with_resizable(false),
        ),
        // Linux: XWayland in a Wayland session, so it can stay on top
        #[cfg(target_os = "linux")]
        event_loop_builder: platform::display::event_loop(),
        ..Default::default()
    };
    eframe::run_native(
        "Clauddy",
        options,
        Box::new(|cc| {
            platform::frame::remove(cc);
            Ok(Box::new(app::App::new(cc)))
        }),
    )
}

/// The taskbar's and Alt-Tab's icon: the .ico's 256 px image, decoded at
/// launch. macOS takes the bundle's, and its binary carries neither.
#[cfg(not(target_os = "macos"))]
fn with_app_icon(viewport: egui::ViewportBuilder) -> egui::ViewportBuilder {
    match largest_png(include_bytes!("../assets/icon.ico")).and_then(|png| eframe::icon_data::from_png_bytes(png).ok()) {
        Some(icon) => viewport.with_icon(icon),
        None => viewport,
    }
}

#[cfg(target_os = "macos")]
fn with_app_icon(viewport: egui::ViewportBuilder) -> egui::ViewportBuilder {
    viewport
}

/// The largest image in a .ico whose images are PNGs, as `scripts/ico.py`
/// writes them.
#[cfg_attr(target_os = "macos", allow(dead_code))]
fn largest_png(ico: &[u8]) -> Option<&[u8]> {
    let u16_at = |i: usize| Some(u16::from_le_bytes(ico.get(i..i + 2)?.try_into().ok()?));
    let u32_at = |i: usize| Some(u32::from_le_bytes(ico.get(i..i + 4)?.try_into().ok()?) as usize);
    let count = u16_at(4)? as usize;
    let mut best: Option<(usize, &[u8])> = None;
    for i in 0..count {
        let entry = 6 + 16 * i;
        // a width byte of 0 means 256
        let width = match *ico.get(entry)? {
            0 => 256,
            w => w as usize,
        };
        let (size, offset) = (u32_at(entry + 8)?, u32_at(entry + 12)?);
        let image = ico.get(offset..offset.checked_add(size)?)?;
        if image.starts_with(b"\x89PNG") && best.is_none_or(|(w, _)| width > w) {
            best = Some((width, image));
        }
    }
    best.map(|(_, png)| png)
}

#[cfg(test)]
mod tests {
    use super::largest_png;

    #[test]
    fn the_window_icon_is_the_ico_s_256_px_png() {
        let png = largest_png(include_bytes!("../assets/icon.ico")).expect("a PNG in the .ico");
        // the IHDR's width and height, right after the signature and chunk header
        assert_eq!(&png[16..24], &[0, 0, 1, 0, 0, 0, 1, 0]);
    }

    #[test]
    fn a_broken_ico_has_no_icon() {
        assert!(largest_png(b"").is_none());
        assert!(largest_png(&[0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 32, 0, 255, 255, 0, 0, 22, 0, 0, 0]).is_none());
    }
}
