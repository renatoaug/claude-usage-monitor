// What the OS has to answer: where the cursor is when the window can't see it.

pub mod cursor;
pub mod tray;

/// Bring Clauddy to the front. It lives outside the Dock, so a window it
/// shows from the menu bar doesn't get focus on its own.
pub fn activate() {
    #[cfg(target_os = "macos")]
    if let Some(mtm) = objc2::MainThreadMarker::new() {
        objc2_app_kit::NSApplication::sharedApplication(mtm).activate();
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
        app.activate();
    }
}
