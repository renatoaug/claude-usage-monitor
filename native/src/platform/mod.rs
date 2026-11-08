// What the OS has to answer: where the cursor is when the window can't see it.

pub mod cursor;
pub mod leftovers;
pub mod login;
pub mod tray;

/// Bring Clauddy to the front. It lives outside the Dock, so a window it
/// shows from the menu bar doesn't get focus on its own.
pub fn activate() {
    #[cfg(target_os = "macos")]
    if let Some(mtm) = objc2::MainThreadMarker::new() {
        force_activate(&objc2_app_kit::NSApplication::sharedApplication(mtm));
    }
}

/// `activate()` only asks since macOS 14, and a menu-bar app asking from its
/// own menu is turned down: the window shows as key, but the first click on
/// it hands focus back to the app before. The old call still takes it.
#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn force_activate(app: &objc2_app_kit::NSApplication) {
    app.activateIgnoringOtherApps(true);
}

/// `wake` once the menu-bar menu has closed. Shown while the menu is still
/// closing, the window looks focused but macOS hands the next click to the
/// app that was in front, and the click is lost. A timer in the default run
/// loop mode doesn't fire during menu tracking, so it waits for the menu.
pub fn wake_after_menu(ctx: eframe::egui::Context) {
    #[cfg(target_os = "macos")]
    {
        let block = block2::RcBlock::new(move |_: std::ptr::NonNull<objc2_foundation::NSTimer>| {
            wake();
            ctx.request_repaint();
        });
        // SAFETY: the block only touches the main thread's app and a Send context
        let _ = unsafe { objc2_foundation::NSTimer::scheduledTimerWithTimeInterval_repeats_block(0.05, false, &block) };
    }
    #[cfg(not(target_os = "macos"))]
    {
        wake();
        ctx.request_repaint();
    }
}

/// Put the window back on screen from outside a frame. A hidden window gets
/// no frames, so the app can't answer a menu-bar click until it shows again:
/// the menu's handler calls this, and the next frame does the rest.
pub fn wake() {
    #[cfg(target_os = "macos")]
    if let Some(mtm) = objc2::MainThreadMarker::new() {
        let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
        for w in app.windows().iter() {
            w.orderFrontRegardless();
        }
        force_activate(&app);
    }
}
