// The design tokens. The raw palette lives only in this file; everything else
// reads the semantic names (`color::*`, `text::*`, `radius::*`), so a new look
// changes this file alone. A test fails on a raw colour anywhere else.

use eframe::egui::{self, Color32};

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

// white at `alpha`, premultiplied (what Color32::from_white_alpha does)
const fn white(alpha: u8) -> Color32 {
    Color32::from_rgba_premultiplied(alpha, alpha, alpha, alpha)
}

// premultiplied: the channels already scaled by alpha
const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    Color32::from_rgba_premultiplied(r, g, b, a)
}

/// The raw palette. Only the semantic names below may use it.
mod palette {
    use super::*;

    pub const CLAY_900: Color32 = rgb(0x221913);
    pub const CLAY_EDGE: Color32 = rgba(0x1d, 0x17, 0x11, 0x20);
    pub const NIGHT_900: Color32 = rgb(0x0f1210);
    pub const SAND_50: Color32 = rgb(0xeeeae5);
    pub const SAND_100: Color32 = rgb(0xeee4d9);
    pub const SAND_200: Color32 = rgb(0xd0cac2);
    pub const SAND_250: Color32 = rgb(0xc4b9aa);
    pub const SAND_300: Color32 = rgb(0xbdb7af);
    pub const SAND_350: Color32 = rgb(0xb9afa3);
    pub const SAND_400: Color32 = rgb(0xaaa59d);
    pub const SAND_450: Color32 = rgb(0xa8a29a);
    pub const SAND_500: Color32 = rgb(0xa39f98);
    pub const SAND_600: Color32 = rgb(0xb4a18a);
    pub const SAND_700: Color32 = rgb(0x6b635a);
    pub const TAN_300: Color32 = rgb(0xc6a385);
    pub const CLAY_700: Color32 = rgb(0x30251e);
    pub const CORAL_350: Color32 = rgb(0xcb8a67);
    pub const TAN_400: Color32 = rgb(0xc2865e);
    pub const CORAL_300: Color32 = rgb(0xf3a38b);
    pub const CORAL_400: Color32 = rgb(0xe4a17c);
    pub const CORAL_450: Color32 = rgb(0xdf9777);
    pub const CORAL_500: Color32 = rgb(0xd97757);
    pub const RED_400: Color32 = rgb(0xee705e);
    pub const RED_500: Color32 = rgb(0xe0553f);
    pub const RED_300: Color32 = rgb(0xe08368);
    pub const AMBER_400: Color32 = rgb(0xe5b55d);
    pub const GREEN_400: Color32 = rgb(0x7ec77d);
    pub const MOSS_300: Color32 = rgb(0xabbd96);
    pub const MOSS_400: Color32 = rgb(0xa1b786);
    pub const MOSS_EDGE: Color32 = rgba(21, 24, 17, 33);
    pub const MOSS_WASH: Color32 = rgba(0, 1, 0, 8);
    pub const TAN_EDGE: Color32 = rgba(27, 22, 16, 38);
    pub const TAN_WASH: Color32 = rgba(1, 1, 0, 10);
    pub const STONE_300: Color32 = rgb(0xc2b4a6);
    pub const STONE_400: Color32 = rgb(0xb6a99c);
    pub const SAGE_300: Color32 = rgb(0xb9c8b0);
    pub const SAGE_400: Color32 = rgb(0xa7baa1);
    pub const SKY_300: Color32 = rgb(0xb8c4dc);
    pub const SKY_400: Color32 = rgb(0x9fb2d6);
    pub const HEAT_1: Color32 = rgb(0x6b4430);
    pub const HEAT_2: Color32 = rgb(0x9c6340);
    pub const HEAT_3: Color32 = rgb(0xcf8b52);
    pub const HEAT_4: Color32 = rgb(0xf0b464);
    pub const DOT_2: Color32 = rgb(0xc8a475);
    pub const DOT_3: Color32 = rgb(0x807769);
    pub const DOT_N: Color32 = rgb(0xb1967c);
    // the night sky behind the expanded pet
    pub const NIGHT_CLOUD: Color32 = rgb(0x3a3a3a);
    pub const NIGHT_STAR: Color32 = rgb(0xd8d8d8);
}

/// Semantic colours: what a colour is for, not what it looks like.
pub mod color {
    use super::*;
    use palette::*;

    // surfaces
    pub const SURFACE: Color32 = CLAY_900;
    /// the glow's heart: warm, a little translucent, fading to nothing
    pub const SURFACE_WARM: Color32 = rgba(0x16, 0x0c, 0x06, 0x24);
    pub const BORDER: Color32 = CLAY_EDGE;
    pub const PANE: Color32 = NIGHT_900;
    pub const PANE_EDGE: Color32 = white(6);
    pub const HAIR: Color32 = rgba(19, 19, 19, 19);
    pub const WASH: Color32 = white(3);
    pub const WASH_EDGE: Color32 = white(10);
    /// a placeholder while the first reading loads
    pub const SKELETON: Color32 = white(12);

    // text
    pub const FG: Color32 = SAND_50;
    pub const FG_STRONG: Color32 = SAND_100;
    pub const FG_ROW: Color32 = SAND_200;
    pub const FG_TOGGLE: Color32 = SAND_250;
    pub const FG_LABEL: Color32 = SAND_300;
    pub const FG_CAPTION: Color32 = SAND_350;
    pub const FG_SECTION: Color32 = SAND_400;
    pub const FG_SUB: Color32 = SAND_450;
    pub const FG_MUTED: Color32 = SAND_500;
    pub const FG_RESET: Color32 = SAND_600;

    pub const CLEAR: Color32 = Color32::TRANSPARENT;

    /// "No tint": draw a texture in its own colours.
    pub const NO_TINT: Color32 = Color32::WHITE;

    // controls
    pub const ICON: Color32 = SAND_400;
    pub const ICON_HOVER: Color32 = Color32::WHITE;
    pub const CONTROL: Color32 = white(4);
    pub const CONTROL_HOVER: Color32 = white(60);
    pub const CONTROL_EDGE: Color32 = white(9);

    // menus and fields
    pub const MENU: Color32 = CLAY_700;
    pub const MENU_EDGE: Color32 = white(20);
    pub const MENU_HOVER: Color32 = white(12);
    pub const ADD: Color32 = CORAL_350;
    pub const FIELD: Color32 = white(8);
    pub const FIELD_EDGE: Color32 = white(24);
    pub const ERROR: Color32 = RED_300;

    // tags
    pub const PLAN: Color32 = TAN_300;
    pub const PLAN_EDGE: Color32 = TAN_EDGE;
    pub const PLAN_WASH: Color32 = TAN_WASH;
    pub const LIVE: Color32 = MOSS_300;
    pub const LIVE_EDGE: Color32 = MOSS_EDGE;
    pub const LIVE_WASH: Color32 = MOSS_WASH;
    pub const SIGNED_IN: Color32 = MOSS_400;

    // meters: the 60/85 ramp
    pub const TRACK: Color32 = white(10);
    pub const TRACK_THIN: Color32 = rgba(0x0d, 0x0d, 0x0d, 0x0d);
    pub const TRACK_FAINT: Color32 = white(6);
    pub const ACCENT: Color32 = CORAL_500;
    pub const WARM: Color32 = AMBER_400;
    pub const HOT: Color32 = RED_400;
    pub const URGENT: Color32 = CORAL_300;
    pub const BREAKDOWN: Color32 = TAN_400;

    // moods
    pub const WORKING: Color32 = GREEN_400;
    pub const FIRE: Color32 = RED_500;
    pub const FIRE_TEXT: Color32 = RED_300;
    pub const TIRED: Color32 = STONE_400;
    pub const TIRED_TEXT: Color32 = STONE_300;
    pub const REST: Color32 = SAND_700;

    // providers: the logo tint and the bar
    pub const CLAUDE: Color32 = CORAL_400;
    pub const CLAUDE_WORKING: Color32 = CORAL_450;
    pub const CLAUDE_BAR: Color32 = CORAL_500;
    pub const CODEX: Color32 = SAGE_300;
    pub const CODEX_BAR: Color32 = SAGE_400;
    pub const CURSOR: Color32 = SKY_300;
    pub const CURSOR_BAR: Color32 = SKY_400;

    // the 30-day map, idle to busiest; the breakdown rows' dots
    pub const HEAT: [Color32; 5] = [rgba(13, 13, 13, 13), HEAT_1, HEAT_2, HEAT_3, HEAT_4];
    pub const DOTS: [Color32; 4] = [CORAL_500, DOT_2, DOT_3, DOT_N];

    // the night sky
    pub const SKY_DOT: Color32 = white(217);
    pub const CLOUD: Color32 = NIGHT_CLOUD;
    pub const STAR: Color32 = NIGHT_STAR;
}

/// Mix two colours, `t` of the way from `a` to `b` (premultiplied mix linearly).
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()), m(a.a(), b.a()))
}

/// Text sizes, by role.
pub mod text {
    pub const TINY: f32 = 8.0; // tags, the reset clock
    pub const CAPTION: f32 = 9.0; // the word under the compact pet, the map
    pub const SMALL: f32 = 10.0; // rows, subtitles, the account
    pub const BODY: f32 = 11.0; // labels, the status line
    pub const VALUE: f32 = 12.0; // a meter's %
    pub const DISPLAY: f32 = 15.0; // a compact row's %
    pub const HERO: f32 = 27.0; // the session's %
}

/// Corners, by role. No other radius.
pub mod radius {
    pub const TAG: u8 = 2; // labels and badges
    pub const CONTROL: u8 = 4; // buttons, bars, boxes inside a card
    pub const CARD: u8 = 10; // the widget's card
}

/// The system UI font, as the Electron panel used (-apple-system). Falls back
/// to egui's own where the system font isn't found.
pub fn install_font(ctx: &egui::Context) {
    let Ok(bytes) = std::fs::read("/System/Library/Fonts/SFNS.ttf") else { return };
    let mut fonts = egui::FontDefinitions::default();
    // egui sizes a font by its whole line (ascent to descent), CSS by its em:
    // SF's line is 1.19 em, so scale it up to read at the Electron sizes
    let data = egui::FontData::from_owned(bytes).tweak(egui::FontTweak { scale: 1.19, ..Default::default() });
    fonts.font_data.insert("system".into(), std::sync::Arc::new(data));
    fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "system".into());
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    // The only place a raw colour may appear is this file.
    #[test]
    fn no_raw_colour_outside_the_theme() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = vec![];
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            for e in std::fs::read_dir(&dir).unwrap().flatten() {
                let path = e.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().is_none_or(|x| x != "rs") || path.ends_with("ui/theme.rs") {
                    continue;
                }
                let body = std::fs::read_to_string(&path).unwrap();
                for (i, line) in body.lines().enumerate() {
                    if ["Color32::from_", "Color32::WHITE", "Color32::BLACK"].iter().any(|p| line.contains(p)) {
                        offenders.push(format!("{}:{}", path.display(), i + 1));
                    }
                }
            }
        }
        assert!(offenders.is_empty(), "raw colours outside ui/theme.rs:\n{}", offenders.join("\n"));
    }
}
