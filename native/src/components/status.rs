// What the pet is up to: the expanded status line and the compact caption.

use eframe::egui::{Align, Color32, Context, Painter, Pos2, Rect, Vec2};

use super::logos::{Logos, Provider};
use super::pet::Mood;
use crate::format::tokens;
use crate::ui::skeleton::sk_line;
use crate::ui::tag::{TagStyle, tag};
use crate::ui::text::{bold_mid, text_mid, width};
use crate::ui::theme::{color, text};

pub const STATUS_H: f32 = 34.0;

/// The word for the mood; while working, what Claude is doing.
pub fn status_word(m: Mood, activity: Option<&str>) -> String {
    match m {
        Mood::Working => activity.unwrap_or("working").to_string(),
        Mood::Fire => "on fire".into(),
        Mood::Tired => "maxed out".into(),
        Mood::Sleeping => "sleeping".into(),
        Mood::Idle => "idle".into(),
    }
}

/// (dot, word) colours for a mood.
pub fn mood_colors(m: Mood) -> (Color32, Color32) {
    match m {
        Mood::Working => (color::WORKING, color::WORKING),
        Mood::Fire => (color::FIRE, color::FIRE_TEXT),
        Mood::Tired => (color::TIRED, color::TIRED_TEXT),
        _ => (color::REST, color::FG),
    }
}

/// Expanded: dot, word, today's tokens, and the live tag when the % is real.
pub fn status_line(p: &Painter, left: f32, right: f32, y: f32, mood: Mood, activity: Option<&str>, today: Option<u64>, live: bool) {
    let (dot, word_color) = mood_colors(mood);
    p.circle_filled(Pos2::new(left + 3.5, y), 3.5, dot);
    let r = bold_mid(p, left + 13.0, y, Align::Min, &status_word(mood, activity), text::BODY, word_color);
    if today.is_none() {
        // still loading: hold the tokens' place
        sk_line(p, r.right() + 10.0, y, 110.0, 8.0);
    }
    if let Some(n) = today {
        let sep = text_mid(p, r.right() + 6.0, y, Align::Min, "·", text::BODY, color::FG_MUTED.gamma_multiply(0.5));
        text_mid(p, sep.right() + 6.0, y, Align::Min, &format!("{} tokens today", tokens(n)), text::SMALL, color::FG_MUTED);
    }
    if live {
        let style = TagStyle { fg: color::LIVE, edge: color::LIVE_EDGE, wash: color::LIVE_WASH, pad: Vec2::new(5.0, 3.0) };
        tag(p, right, y, Align::Max, "live", &style);
    }
}

/// Compact: under the pet, the workers' logos (or a dot) and the word,
/// centred. Claude alone says what it's doing; Codex, or both, say "working".
pub fn caption(p: &Painter, ctx: &Context, logos: &Logos, center_x: f32, y: f32, mood: Mood, activity: Option<&str>, workers: &[Provider]) {
    let working = mood == Mood::Working && !workers.is_empty();
    let word = if working && workers != [Provider::Claude] { "working".to_string() } else { status_word(mood, activity) };
    let mark = if working { workers.len() as f32 * 13.0 - 3.0 } else { 4.0 };
    let left = center_x - (mark + 5.0 + width(ctx, &word, text::CAPTION)) / 2.0;
    if working {
        for (i, w) in workers.iter().enumerate() {
            let at = Rect::from_min_size(Pos2::new(left + i as f32 * 13.0, y - 5.0), Vec2::splat(10.0));
            logos.draw(p, *w, at, if *w == Provider::Claude { color::CLAUDE_WORKING } else { w.tint() });
        }
    } else {
        p.circle_filled(Pos2::new(left + 2.0, y), 2.0, mood_colors(mood).0);
    }
    text_mid(p, left + mark + 5.0, y, Align::Min, &word, text::CAPTION, color::FG_CAPTION);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_word_is_the_activity_while_working() {
        assert_eq!(status_word(Mood::Working, Some("running")), "running");
        assert_eq!(status_word(Mood::Working, None), "working");
        assert_eq!(status_word(Mood::Idle, Some("running")), "idle");
    }
}
