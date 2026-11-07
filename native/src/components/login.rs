// Finishing a browser login: the browser shows a code; paste it here.

use eframe::egui::{self, Align2, CornerRadius, FontId, Pos2, Rect, Stroke, StrokeKind, TextEdit, Ui, Vec2};

use crate::ui::controls::click_area;
use crate::ui::theme::{color, radius, text};

pub const LOGIN_H: f32 = 112.0;

pub enum LoginPick {
    Submit,
    Reopen,
}

/// The login panel between `left` and `right` from `y`. `code` is the field.
pub fn login_panel(ui: &mut Ui, left: f32, right: f32, y: f32, code: &mut String, error: Option<&str>) -> Option<LoginPick> {
    let mut pick = None;
    let boxr = Rect::from_min_size(Pos2::new(left, y + 12.0), Vec2::new(right - left, LOGIN_H - 24.0));
    let p = ui.painter().clone();
    p.rect(boxr, CornerRadius::same(radius::CONTROL), color::WASH, Stroke::new(1.0, color::WASH_EDGE), StrokeKind::Inside);
    let x = boxr.left() + 10.0;
    p.text(Pos2::new(x, boxr.top() + 14.0), Align2::LEFT_CENTER, "Finish logging in", FontId::proportional(text::BODY), color::FG);
    let hint = error.unwrap_or("Paste the code the browser shows");
    p.text(
        Pos2::new(x, boxr.top() + 30.0),
        Align2::LEFT_CENTER,
        hint,
        FontId::proportional(text::SMALL),
        if error.is_some() { color::ERROR } else { color::FG_MUTED },
    );

    // the field and the button share a row
    let field = Rect::from_min_size(Pos2::new(x, boxr.top() + 42.0), Vec2::new(boxr.width() - 20.0 - 70.0, 24.0));
    p.rect(field, CornerRadius::same(radius::CONTROL), color::FIELD, Stroke::new(1.0, color::FIELD_EDGE), StrokeKind::Inside);
    let edit = TextEdit::singleline(code)
        .hint_text("code#state")
        .font(FontId::proportional(text::SMALL))
        .text_color(color::FG)
        .frame(egui::Frame::NONE)
        .margin(Vec2::new(6.0, 5.0));
    let resp = ui.put(field, edit);
    let entered = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

    let btn = Rect::from_min_size(Pos2::new(field.right() + 6.0, field.top()), Vec2::new(64.0, 24.0));
    let b = click_area(ui, btn, "login-submit");
    let (bg, fg) = if b.hovered() { (color::CONTROL_HOVER, color::ICON_HOVER) } else { (color::CONTROL, color::FG) };
    p.rect(btn, CornerRadius::same(radius::CONTROL), bg, Stroke::new(1.0, color::CONTROL_EDGE), StrokeKind::Inside);
    p.text(btn.center(), Align2::CENTER_CENTER, "Connect", FontId::proportional(text::SMALL), fg);
    if (b.clicked() || entered) && !code.trim().is_empty() {
        pick = Some(LoginPick::Submit);
    }

    let again = Rect::from_min_size(Pos2::new(x, field.bottom() + 4.0), Vec2::new(150.0, 14.0));
    let a = click_area(ui, again, "login-reopen");
    let c = if a.hovered() { color::FG } else { color::ADD };
    p.text(Pos2::new(x, again.center().y), Align2::LEFT_CENTER, "Open the browser again", FontId::proportional(text::TINY + 1.0), c);
    if a.clicked() {
        pick = Some(LoginPick::Reopen);
    }
    pick
}
