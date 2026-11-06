// Surfaces: the widget's card, a pane inside it, the hairline between parts.

use eframe::egui::{Context, CornerRadius, Mesh, Painter, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, ViewportCommand};

use super::theme::{color, mix, radius};

/// The card: the panel colour, its border and the warm glow from the top
/// left. The whole card drags the window; anything drawn later sits on top.
pub fn card(ui: &Ui, ctx: &Context, rect: Rect) {
    let p = ui.painter();
    let round = CornerRadius::same(radius::CARD);
    p.rect_filled(rect, round, color::SURFACE);
    p.add(Shape::mesh(glow_mesh(rect, rect.left_top() + Vec2::new(rect.width() * 0.2, 0.0), GLOW_REACH)));
    // the border goes last: its anti-aliased edge covers the mesh's
    p.rect_stroke(rect, round, Stroke::new(1.0, color::BORDER), StrokeKind::Inside);
    let drag = ui.interact(rect, ui.scope_id().with("card-drag"), Sense::click_and_drag());
    if drag.drag_started() {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }
}

// how far the warm glow reaches from its centre on the top edge
const GLOW_REACH: f32 = 140.0;

/// Points around a rounded rect, clockwise from the top-left arc, about
/// `step` apart along the straight edges.
fn rounded_outline(rect: Rect, r: f32, step: f32) -> Vec<Pos2> {
    let mut pts = vec![];
    let corners = [
        (Pos2::new(rect.left() + r, rect.top() + r), std::f32::consts::PI),
        (Pos2::new(rect.right() - r, rect.top() + r), 1.5 * std::f32::consts::PI),
        (Pos2::new(rect.right() - r, rect.bottom() - r), 0.0),
        (Pos2::new(rect.left() + r, rect.bottom() - r), 0.5 * std::f32::consts::PI),
    ];
    for (i, (c, start)) in corners.iter().enumerate() {
        for k in 0..=8 {
            let a = start + std::f32::consts::FRAC_PI_2 * k as f32 / 8.0;
            pts.push(*c + Vec2::new(a.cos(), a.sin()) * r);
        }
        // the straight edge to the next corner
        let from = *pts.last().unwrap();
        let (nc, ns) = corners[(i + 1) % 4];
        let to = nc + Vec2::new(ns.cos(), ns.sin()) * r;
        let n = ((to - from).length() / step).floor() as usize;
        for k in 1..n {
            pts.push(from + (to - from) * (k as f32 / n as f32));
        }
    }
    pts
}

/// A radial glow shaped exactly like the card: a fan from `center` to the
/// rounded outline, with a ring at `reach` where it fades out, so the warmth
/// falls off with distance and never spills past the rounded corners.
fn glow_mesh(rect: Rect, center: Pos2, reach: f32) -> Mesh {
    let mut m = Mesh::default();
    let warm = color::SURFACE_WARM;
    let clear = color::CLEAR;
    let outline = rounded_outline(rect.shrink(0.5), radius::CARD as f32 - 0.5, 4.0);
    let c = m.vertices.len() as u32;
    m.colored_vertex(center, warm);
    let n = outline.len();
    for p in &outline {
        let d = *p - center;
        let ring = if d.length() > reach { center + d.normalized() * reach } else { *p };
        let t = (ring - center).length() / reach;
        m.colored_vertex(ring, mix(warm, clear, t));
        m.colored_vertex(*p, clear);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let (ri, oi, rj, oj) = (c + 1 + 2 * i as u32, c + 2 + 2 * i as u32, c + 1 + 2 * j as u32, c + 2 + 2 * j as u32);
        m.add_triangle(c, ri, rj);
        m.add_triangle(ri, oi, oj);
        m.add_triangle(ri, oj, rj);
    }
    m
}

/// The card's solid fill alone, drawn before content that may be fading so
/// the card itself never goes see-through.
pub fn card_backdrop(p: &Painter, rect: Rect) {
    p.rect_filled(rect, CornerRadius::same(radius::CARD), color::SURFACE);
}

/// A dark pane inside the card (the expanded pet's stage).
pub fn pane(p: &Painter, rect: Rect) {
    p.rect(rect, CornerRadius::same(radius::CONTROL), color::PANE, Stroke::new(1.0, color::PANE_EDGE), StrokeKind::Inside);
}

/// A soft round glow behind something (the compact pet).
pub fn halo(p: &Painter, rect: Rect, tint: eframe::egui::Color32) {
    for (r, a) in [(30.0, 0.5), (22.0, 0.8), (14.0, 1.0)] {
        p.circle_filled(rect.center(), r, tint.gamma_multiply(a));
    }
}

/// The 1 px rule between parts of the panel.
pub fn hairline(p: &Painter, left: f32, right: f32, y: f32) {
    p.hline(left..=right, y, Stroke::new(1.0, color::HAIR));
}
