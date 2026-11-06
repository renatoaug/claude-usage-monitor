// Provider logos (the same glyphs the Electron widget uses), rasterized once
// at start-up into white masks and tinted when drawn.

use eframe::egui::{self, Color32, ColorImage, Rect, TextureHandle, TextureOptions};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Provider {
    Claude,
    Codex,
    Cursor,
}

impl Provider {
    pub fn name(self) -> &'static str {
        match self {
            Provider::Claude => "Claude",
            Provider::Codex => "Codex",
            Provider::Cursor => "Cursor",
        }
    }
    /// The logo tint on a selected row and in the activity line.
    pub fn tint(self) -> Color32 {
        match self {
            Provider::Claude => Color32::from_rgb(0xe4, 0xa1, 0x7c),
            Provider::Codex => Color32::from_rgb(0xb9, 0xc8, 0xb0),
            Provider::Cursor => Color32::from_rgb(0xb8, 0xc4, 0xdc),
        }
    }
    /// The usage bar.
    pub fn bar(self) -> Color32 {
        match self {
            Provider::Claude => Color32::from_rgb(0xd9, 0x77, 0x57),
            Provider::Codex => Color32::from_rgb(0xa7, 0xba, 0xa1),
            Provider::Cursor => Color32::from_rgb(0x9f, 0xb2, 0xd6),
        }
    }
    fn svg(self) -> &'static str {
        match self {
            Provider::Claude => include_str!("../assets/logos/claude.svg"),
            Provider::Codex => include_str!("../assets/logos/openai.svg"),
            Provider::Cursor => include_str!("../assets/logos/cursor.svg"),
        }
    }
}

pub struct Logos {
    textures: Vec<(Provider, TextureHandle)>,
}

// rasterized large enough to stay crisp on a Retina screen at 12-14 px
const PX: u32 = 64;

impl Logos {
    pub fn load(ctx: &egui::Context) -> Self {
        let textures = [Provider::Claude, Provider::Codex, Provider::Cursor]
            .into_iter()
            .map(|p| {
                let tree = resvg::usvg::Tree::from_str(p.svg(), &resvg::usvg::Options::default()).expect("embedded logo");
                let mut pm = resvg::tiny_skia::Pixmap::new(PX, PX).expect("logo pixmap");
                let k = PX as f32 / tree.size().width();
                resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(k, k), &mut pm.as_mut());
                let img = ColorImage::from_rgba_premultiplied([PX as usize, PX as usize], pm.data());
                (p, ctx.load_texture(format!("logo-{}", p.name()), img, TextureOptions::LINEAR))
            })
            .collect();
        Logos { textures }
    }

    pub fn draw(&self, painter: &egui::Painter, p: Provider, rect: Rect, color: Color32) {
        if let Some((_, tex)) = self.textures.iter().find(|(q, _)| *q == p) {
            let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            painter.image(tex.id(), rect, uv, color);
        }
    }
}
