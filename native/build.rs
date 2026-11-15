// Windows: embed the icon and version info in clauddy.exe, so Explorer, the
// taskbar and Alt-Tab show Clauddy instead of a blank program icon. Keyed on
// the target, not the host: a cross build from Linux (mingw's windres) works.
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico").set("ProductName", "Clauddy").set("FileDescription", "Clauddy");
        // fails loudly: a release must never ship iconless again
        res.compile().expect("embedding the exe icon (needs the Windows SDK's rc.exe, or mingw's windres)");
    }
}
