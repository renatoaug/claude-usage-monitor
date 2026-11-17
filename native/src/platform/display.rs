// Linux: which display server the window opens on.
//
// On Wayland a client can't keep its window above the others or place it:
// winit ignores both requests there, so the widget sank under other windows
// (KDE Plasma, as on Bazzite). Under XWayland the compositor honours them,
// so a Wayland session opens the window through it when there is one.
// CLAUDDY_WAYLAND=1 keeps it native.

/// Whether to open on X11: in a Wayland session with XWayland running,
/// unless asked to stay on Wayland.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn prefer_x11(wayland: bool, xwayland: bool, stay_on_wayland: bool) -> bool {
    wayland && xwayland && !stay_on_wayland
}

#[cfg(target_os = "linux")]
pub fn event_loop() -> Option<eframe::EventLoopBuilderHook> {
    let set = |k: &str| std::env::var_os(k).is_some_and(|v| !v.is_empty());
    let x11 = prefer_x11(set("WAYLAND_DISPLAY"), set("DISPLAY"), std::env::var("CLAUDDY_WAYLAND").is_ok_and(|v| v == "1"));
    x11.then(|| -> eframe::EventLoopBuilderHook {
        Box::new(|b| {
            use winit::platform::x11::EventLoopBuilderExtX11;
            b.with_x11();
        })
    })
}

#[cfg(test)]
mod tests {
    use super::prefer_x11;

    #[test]
    fn a_wayland_session_opens_on_xwayland_unless_asked_not_to() {
        assert!(prefer_x11(true, true, false));
        // no XWayland to open on: stay on Wayland rather than fail to start
        assert!(!prefer_x11(true, false, false));
        assert!(!prefer_x11(true, true, true));
        // already X11
        assert!(!prefer_x11(false, true, false));
    }
}
