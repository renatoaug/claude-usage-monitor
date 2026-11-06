// Motion: how long things take to settle, and their easing. Short and
// gentle, so a change reads as a change rather than a cut.

use std::time::Instant;

/// New content comes up over this long, from part-way (never from nothing).
pub const ENTER_S: f32 = 0.18;
/// The card opens or closes to a new size over this long.
pub const MORPH_S: f32 = 0.22;
/// The panel fades away over this long (going to the menu bar).
pub const LEAVE_S: f32 = 0.22;
/// A selection slides between options over this long.
pub const SLIDE_S: f32 = 0.18;

/// Ease-out cubic: quick to start, soft to land.
pub fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// How far along a motion started at `start` and lasting `secs` is, eased.
pub fn progress(start: Instant, secs: f32) -> f32 {
    ease_out(start.elapsed().as_secs_f32() / secs)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eases_out_from_zero_to_one() {
        assert_eq!(ease_out(0.0), 0.0);
        assert_eq!(ease_out(1.0), 1.0);
        assert!(ease_out(0.5) > 0.5); // most of the way by half time
        assert_eq!(ease_out(2.0), 1.0);
    }
}
