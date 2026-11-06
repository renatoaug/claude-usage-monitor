// A row of options where one is chosen: the settings tabs, "Floating /
// Menu bar".

use eframe::egui::{Align, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};

use super::controls::click_area;
use super::text::text_mid;
use super::motion;
use super::theme::{color, radius};

/// Options spread evenly across `rect`; returns the index clicked. Options
/// listed in `disabled` are faded and inert.
pub fn segmented(ui: &Ui, rect: Rect, id: &str, labels: &[&str], selected: usize, size: f32, disabled: &[usize]) -> Option<usize> {
    let p = ui.painter();
    p.rect(rect, CornerRadius::same(radius::CONTROL + 2), color::TABS, Stroke::new(1.0, color::HAIR), StrokeKind::Inside);
    let inner = rect.shrink(3.0);
    let w = inner.width() / labels.len() as f32;
    // the chosen option's plate slides over to a new choice
    let at = ui.ctx().animate_value_with_time(ui.scope_id().with((id, "plate")), selected as f32, motion::SLIDE_S);
    let plate = Rect::from_min_size(Pos2::new(inner.left() + at * w, inner.top()), Vec2::new(w, inner.height()));
    p.rect_filled(plate, CornerRadius::same(radius::CONTROL), color::TAB_ON);
    let mut pick = None;
    for (i, label) in labels.iter().enumerate() {
        let r = Rect::from_min_size(Pos2::new(inner.left() + i as f32 * w, inner.top()), Vec2::new(w, inner.height()));
        let off = disabled.contains(&i);
        let hovered = !off && {
            let resp = click_area(ui, r, (id, i));
            if resp.clicked() && i != selected {
                pick = Some(i);
            }
            resp.hovered()
        };
        let fg = if i == selected || hovered { color::FG } else { color::FG_MUTED };
        text_mid(p, r.center().x, r.center().y, Align::Center, label, size, if off { fg.gamma_multiply(0.4) } else { fg });
    }
    pick
}
