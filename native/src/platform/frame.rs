// Windows: nothing of the window's own shows around the card.
//
// The window is transparent (DWM blur-behind) and wgpu presents over it, but
// the window keeps its GDI redirection surface underneath, and Windows fills
// it white the first time the window shows. Wherever wgpu draws nothing (the
// margin around the card) that white showed through, and it stayed, anchored
// top-left at the window's first size, after the window grew or shrank.
// Painting that surface black clears it: under blur-behind, GDI's black is
// fully transparent.
//
// winit also keeps a caption style on an undecorated window and egui-winit
// asks for the "undecorated shadow", so DWM drew a frame, a border, a shadow
// and Windows 11's rounding around the whole transparent rectangle. DWM's
// non-client rendering is turned off; the card draws its own corners. That
// shadow is winit leaving the top pixel row out of the client area, which
// then showed as a light line: the window is subclassed so the client area
// is the whole window again.

#[cfg(windows)]
fn hwnd(window: &impl raw_window_handle::HasWindowHandle) -> Option<isize> {
    match window.window_handle().ok()?.as_raw() {
        raw_window_handle::RawWindowHandle::Win32(w) => Some(w.hwnd.get()),
        _ => None,
    }
}

/// Take DWM's frame off the window. Quiet on any failure (older Windows
/// lacks the corner and border attributes and just ignores them).
#[cfg(windows)]
pub fn remove(window: &impl raw_window_handle::HasWindowHandle) {
    let Some(hwnd) = hwnd(window) else { return };
    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(hwnd: isize, attribute: u32, value: *const core::ffi::c_void, size: u32) -> i32;
    }
    const DWMWA_NCRENDERING_POLICY: u32 = 2;
    const DWMNCRP_DISABLED: u32 = 1;
    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWCP_DONOTROUND: u32 = 1;
    const DWMWA_BORDER_COLOR: u32 = 34;
    const DWMWA_COLOR_NONE: u32 = 0xFFFF_FFFE;
    for (attribute, value) in
        [(DWMWA_NCRENDERING_POLICY, DWMNCRP_DISABLED), (DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND), (DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE)]
    {
        // SAFETY: a live window's handle, and a u32 the size we pass
        unsafe { DwmSetWindowAttribute(hwnd, attribute, (&raw const value).cast(), size_of::<u32>() as u32) };
    }

    #[link(name = "comctl32")]
    unsafe extern "system" {
        fn SetWindowSubclass(hwnd: isize, proc: SubclassProc, id: usize, data: usize) -> i32;
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetWindowPos(hwnd: isize, after: isize, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
    }
    const SWP_NOSIZE: u32 = 0x1;
    const SWP_NOMOVE: u32 = 0x2;
    const SWP_NOZORDER: u32 = 0x4;
    const SWP_NOACTIVATE: u32 = 0x10;
    const SWP_FRAMECHANGED: u32 = 0x20;
    const SWP_NOOWNERZORDER: u32 = 0x200;
    // SAFETY: on the window's own thread (the app is built on the event
    // loop's); the frame is then measured again with the new client area
    unsafe {
        if SetWindowSubclass(hwnd, whole_client, 1, 0) != 0 {
            SetWindowPos(hwnd, 0, 0, 0, 0, 0, SWP_NOSIZE | SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_NOOWNERZORDER);
        }
    }
}

#[cfg(windows)]
type SubclassProc = unsafe extern "system" fn(isize, u32, usize, isize, usize, usize) -> isize;

/// The window's client area is the whole window: answering WM_NCCALCSIZE
/// with 0 keeps the proposed rectangle as is. Everything else goes on to
/// winit.
#[cfg(windows)]
unsafe extern "system" fn whole_client(hwnd: isize, msg: u32, wparam: usize, lparam: isize, _id: usize, _data: usize) -> isize {
    #[link(name = "comctl32")]
    unsafe extern "system" {
        fn DefSubclassProc(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize;
    }
    const WM_NCCALCSIZE: u32 = 0x0083;
    if msg == WM_NCCALCSIZE {
        return 0;
    }
    // SAFETY: the message as Windows gave it, passed down the chain
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

/// Paint the redirection surface under wgpu's frames transparent black.
/// Cheap (one GDI fill), so the app does it every frame: whatever Windows
/// fills in on a show or a resize never lasts past the next one.
#[cfg(windows)]
pub fn clear_behind(window: &impl raw_window_handle::HasWindowHandle) {
    let Some(hwnd) = hwnd(window) else { return };
    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetDC(hwnd: isize) -> isize;
        fn ReleaseDC(hwnd: isize, dc: isize) -> i32;
        fn GetClientRect(hwnd: isize, rect: *mut Rect) -> i32;
        fn FillRect(dc: isize, rect: *const Rect, brush: isize) -> i32;
    }
    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn GetStockObject(kind: i32) -> isize;
    }
    const BLACK_BRUSH: i32 = 4;
    // SAFETY: a live window's handle; the DC is released before returning
    unsafe {
        let mut rect = Rect { left: 0, top: 0, right: 0, bottom: 0 };
        if GetClientRect(hwnd, &mut rect) == 0 {
            return;
        }
        let dc = GetDC(hwnd);
        if dc == 0 {
            return;
        }
        FillRect(dc, &rect, GetStockObject(BLACK_BRUSH));
        ReleaseDC(hwnd, dc);
    }
}

#[cfg(not(windows))]
pub fn remove<T>(_: &T) {}

#[cfg(not(windows))]
pub fn clear_behind<T>(_: &T) {}
