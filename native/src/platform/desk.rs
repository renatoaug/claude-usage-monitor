// The desk the pet plays on: where other apps' windows are, so it (and the
// snow) can land on their top edges, and the screen around them. Only where
// each window is, never its title or what it shows, so nothing asks for a
// permission. macOS and Windows; elsewhere there are no windows to land on,
// only the screen's floor.

use eframe::egui::{Pos2, Rect};

/// The visible stretch of a window's top edge: something to stand on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ledge {
    pub y: f32,
    pub x0: f32,
    pub x1: f32,
}

impl Ledge {
    pub fn holds(&self, x: f32) -> bool {
        (self.x0..=self.x1).contains(&x)
    }
}

// a stretch narrower than this isn't worth standing on
const MIN_LEDGE: f32 = 60.0;
// nor a window smaller than this (tooltips, badges, the pet's own kind)
const MIN_WINDOW: [f32; 2] = [140.0, 80.0];

/// Other apps' windows on screen, front to back, in screen points from the
/// primary screen's top-left (as egui places windows). `ppp`: the pixels
/// per point, for an OS that answers in pixels.
pub fn windows(ppp: f32) -> Vec<Rect> {
    imp::windows(ppp).into_iter().filter(|r| r.width() >= MIN_WINDOW[0] && r.height() >= MIN_WINDOW[1]).collect()
}

/// The parts of each window's top edge that no window in front covers.
pub fn ledges(windows: &[Rect]) -> Vec<Ledge> {
    let mut out = vec![];
    for (i, w) in windows.iter().enumerate() {
        let mut spans = vec![(w.left(), w.right())];
        for f in &windows[..i] {
            if f.top() <= w.top() && w.top() <= f.bottom() {
                spans = spans.into_iter().flat_map(|s| cut(s, (f.left(), f.right()))).collect();
            }
        }
        out.extend(spans.into_iter().filter(|(a, b)| b - a >= MIN_LEDGE).map(|(x0, x1)| Ledge { y: w.top(), x0, x1 }));
    }
    out
}

// `span` less `hole`: none, one or two pieces
fn cut((a, b): (f32, f32), (c, d): (f32, f32)) -> Vec<(f32, f32)> {
    [(a, b.min(c)), (a.max(d), b)].into_iter().filter(|(x, y)| y > x).collect()
}

/// The screen around `p`, all of it and the part apps may use (no menu bar
/// or Dock), in screen points. None where the OS isn't asked.
pub fn screen_at(p: Pos2) -> Option<(Rect, Rect)> {
    imp::screen_at(p)
}

#[cfg(target_os = "macos")]
mod imp {
    use eframe::egui::{Pos2, Rect};
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use objc2_core_foundation::CFString;
    use objc2_core_graphics::{CGWindowListCopyWindowInfo, CGWindowListOption, kCGWindowAlpha, kCGWindowBounds, kCGWindowLayer, kCGWindowOwnerPID};

    // CF and Foundation objects are the same objects (toll-free bridged):
    // read the list as an NSArray of NSDictionary
    fn get<'a>(d: &'a AnyObject, key: &CFString) -> Option<&'a AnyObject> {
        let key = key as *const CFString as *const AnyObject;
        // SAFETY: `d` is an NSDictionary, `key` an NSString
        let v: *mut AnyObject = unsafe { msg_send![d, objectForKey: key] };
        // SAFETY: owned by `d`, which outlives the borrow
        unsafe { v.as_ref() }
    }

    fn num(d: &AnyObject, key: &CFString) -> Option<f64> {
        // SAFETY: the window list's numbers are NSNumbers
        get(d, key).map(|v| unsafe { msg_send![v, doubleValue] })
    }

    fn named(d: &AnyObject, key: &str) -> Option<f64> {
        num(d, &CFString::from_str(key))
    }

    pub fn windows(_ppp: f32) -> Vec<Rect> {
        let Some(list) = CGWindowListCopyWindowInfo(CGWindowListOption::OptionOnScreenOnly | CGWindowListOption::ExcludeDesktopElements, 0) else {
            return vec![];
        };
        // SAFETY: a CFArray is an NSArray
        let list: &AnyObject = unsafe { &*(&*list as *const _ as *const AnyObject) };
        let n: usize = unsafe { msg_send![list, count] };
        let me = std::process::id() as f64;
        let mut out = vec![];
        for i in 0..n {
            // SAFETY: in bounds
            let d: &AnyObject = unsafe { msg_send![list, objectAtIndex: i] };
            // SAFETY: the keys are CoreGraphics' own constants
            let (layer, owner, alpha) = unsafe { (num(d, kCGWindowLayer), num(d, kCGWindowOwnerPID), num(d, kCGWindowAlpha)) };
            // ordinary app windows only (the menu bar, the Dock and panels
            // sit on other layers), and never Clauddy's own
            if layer != Some(0.0) || owner == Some(me) || alpha.is_some_and(|a| a < 0.1) {
                continue;
            }
            // SAFETY: as above
            let Some(b) = (unsafe { get(d, kCGWindowBounds) }) else { continue };
            let (Some(x), Some(y), Some(w), Some(h)) = (named(b, "X"), named(b, "Y"), named(b, "Width"), named(b, "Height")) else { continue };
            out.push(Rect::from_min_size(Pos2::new(x as f32, y as f32), eframe::egui::vec2(w as f32, h as f32)));
        }
        out
    }

    pub fn screen_at(p: Pos2) -> Option<(Rect, Rect)> {
        use objc2_app_kit::NSScreen;
        let mtm = objc2::MainThreadMarker::new()?;
        let screens = NSScreen::screens(mtm);
        // Cocoa counts up from the primary screen's bottom; windows count down from its top
        let primary = screens.firstObject()?.frame().size.height;
        let flip = |r: objc2_foundation::NSRect| {
            Rect::from_min_size(
                Pos2::new(r.origin.x as f32, (primary - r.origin.y - r.size.height) as f32),
                eframe::egui::vec2(r.size.width as f32, r.size.height as f32),
            )
        };
        let all: Vec<(Rect, Rect)> = screens.iter().map(|s| (flip(s.frame()), flip(s.visibleFrame()))).collect();
        all.iter().find(|(f, _)| f.contains(p)).or(all.first()).copied()
    }
}

#[cfg(windows)]
mod imp {
    use eframe::egui::{Pos2, Rect};

    #[repr(C)]
    #[derive(Default)]
    struct WinRect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumWindows(cb: unsafe extern "system" fn(isize, isize) -> i32, param: isize) -> i32;
        fn IsWindowVisible(hwnd: isize) -> i32;
        fn IsIconic(hwnd: isize) -> i32;
        fn GetWindowRect(hwnd: isize, rect: *mut WinRect) -> i32;
        fn GetWindowTextLengthW(hwnd: isize) -> i32;
        fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
    }

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmGetWindowAttribute(hwnd: isize, attribute: u32, value: *mut core::ffi::c_void, size: u32) -> i32;
    }

    // DWMWA_EXTENDED_FRAME_BOUNDS: the window as drawn, without the
    // invisible resize border; DWMWA_CLOAKED: hidden apps that still say visible
    const FRAME_BOUNDS: u32 = 9;
    const CLOAKED: u32 = 14;

    unsafe extern "system" fn each(hwnd: isize, param: isize) -> i32 {
        // SAFETY: `param` is the Vec handed to EnumWindows below
        let out = unsafe { &mut *(param as *mut Vec<isize>) };
        out.push(hwnd);
        1
    }

    pub fn windows(ppp: f32) -> Vec<Rect> {
        let mut all: Vec<isize> = vec![];
        // SAFETY: the callback only pushes into `all`, alive for the call
        unsafe { EnumWindows(each, &mut all as *mut Vec<isize> as isize) };
        let me = std::process::id();
        let mut out = vec![];
        for hwnd in all {
            // SAFETY: handles straight from EnumWindows; plain reads
            unsafe {
                if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 || GetWindowTextLengthW(hwnd) == 0 {
                    continue;
                }
                let mut pid = 0u32;
                GetWindowThreadProcessId(hwnd, &mut pid);
                let mut cloaked = 0u32;
                DwmGetWindowAttribute(hwnd, CLOAKED, &mut cloaked as *mut u32 as *mut _, 4);
                if pid == me || cloaked != 0 {
                    continue;
                }
                let mut r = WinRect::default();
                if DwmGetWindowAttribute(hwnd, FRAME_BOUNDS, &mut r as *mut WinRect as *mut _, std::mem::size_of::<WinRect>() as u32) != 0 {
                    GetWindowRect(hwnd, &mut r);
                }
                out.push(Rect::from_min_max(Pos2::new(r.left as f32 / ppp, r.top as f32 / ppp), Pos2::new(r.right as f32 / ppp, r.bottom as f32 / ppp)));
            }
        }
        out
    }

    pub fn screen_at(_: Pos2) -> Option<(Rect, Rect)> {
        None
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
mod imp {
    use eframe::egui::{Pos2, Rect};

    pub fn windows(_: f32) -> Vec<Rect> {
        vec![]
    }

    pub fn screen_at(_: Pos2) -> Option<(Rect, Rect)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(Pos2::new(x, y), eframe::egui::vec2(w, h))
    }

    #[test]
    fn a_window_in_front_hides_part_of_the_edge_behind() {
        // front: 300..700 from y 50 down; behind: 100..900 at y 200
        let l = ledges(&[r(300.0, 50.0, 400.0, 600.0), r(100.0, 200.0, 800.0, 400.0)]);
        assert_eq!(l, vec![Ledge { y: 50.0, x0: 300.0, x1: 700.0 }, Ledge { y: 200.0, x0: 100.0, x1: 300.0 }, Ledge { y: 200.0, x0: 700.0, x1: 900.0 }]);
    }

    #[test]
    fn a_window_below_the_edge_hides_nothing_and_slivers_go() {
        // the front window starts under the back one's top edge
        let l = ledges(&[r(0.0, 300.0, 500.0, 200.0), r(0.0, 100.0, 500.0, 400.0)]);
        assert_eq!(l.len(), 2);
        // a 40-point gap is too narrow to stand on
        let l = ledges(&[r(0.0, 0.0, 480.0, 900.0), r(0.0, 100.0, 520.0, 300.0)]);
        assert_eq!(l, vec![Ledge { y: 0.0, x0: 0.0, x1: 480.0 }]);
    }
}
