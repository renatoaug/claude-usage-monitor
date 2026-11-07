// The pet's voice: port of renderer/voice.js. Two channels. The bubble
// carries the meaning: a short remark, only on a transition, and only when
// there's something specific to say (a pet that talks on a timer is
// Clippy). The sound carries the personality: chiptune blips, one
// square-wave note per letter, synthesized here, so no assets and no
// language baked into the sound.

pub const BASE_HZ: f64 = 400.0;
pub const SPREAD_HZ: f64 = 250.0;
pub const STEP_MS: f64 = 115.0;
/// a sentence is a babble, not a recital
pub const MAX_BLIPS: usize = 14;
const GAIN: f64 = 0.05;

/// The tone doubles as state: hot is faster and higher, sleepy slower and lower.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Normal,
    Fire,
    Sleepy,
}

impl Tone {
    fn speed_pitch(self) -> (f64, f64) {
        match self {
            Tone::Normal => (1.0, 1.0),
            Tone::Fire => (0.72, 1.25),
            Tone::Sleepy => (1.35, 0.8),
        }
    }
}

/// Letters and digits each get a fixed pitch, so the same word always
/// sounds the same; anything else is a rest.
pub fn pitch_of(ch: char) -> Option<f64> {
    let c = ch.to_ascii_lowercase();
    match c {
        'a'..='z' => Some(BASE_HZ + (c as u8 - b'a') as f64 / 25.0 * SPREAD_HZ),
        '0'..='9' => Some(BASE_HZ + (c as u8 - b'0') as f64 / 9.0 * SPREAD_HZ),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blip {
    pub at: u32,
    pub hz: u32,
    pub dur: u32,
}

/// The schedule for a line, in ms.
pub fn blip_plan(text: &str, tone: Tone) -> Vec<Blip> {
    let (speed, pitch) = tone.speed_pitch();
    let step = STEP_MS * speed;
    let mut plan = vec![];
    let mut t: f64 = 0.0;
    for ch in text.chars() {
        if plan.len() >= MAX_BLIPS {
            break;
        }
        if let Some(hz) = pitch_of(ch) {
            plan.push(Blip { at: t.round() as u32, hz: (hz * pitch).round() as u32, dur: (step * 0.6).round() as u32 });
            t += step;
        } else if ".,!?—:;".contains(ch) {
            t += step * 1.5; // a breath on punctuation
        } else if ch == ' ' {
            t += step * 0.4;
        }
    }
    plan
}

/// The blips as a mono 16-bit WAV: square waves with a quick attack and an
/// exponential decay, or every note clicks.
pub fn wav(plan: &[Blip]) -> Vec<u8> {
    const RATE: u32 = 22_050;
    let end_ms = plan.iter().map(|b| b.at + b.dur + 10).max().unwrap_or(0) + 20;
    let n = (end_ms as u64 * RATE as u64 / 1000) as usize;
    let mut buf = vec![0f64; n];
    for b in plan {
        let start = (b.at as u64 * RATE as u64 / 1000) as usize;
        let len = (b.dur as u64 * RATE as u64 / 1000) as usize;
        let attack = (0.008 * RATE as f64) as usize;
        for i in 0..len {
            let Some(s) = buf.get_mut(start + 20 * RATE as usize / 1000 + i) else { break };
            let phase = (i as f64 * b.hz as f64 / RATE as f64).fract();
            let wave = if phase < 0.5 { 1.0 } else { -1.0 };
            let env = if i < attack { i as f64 / attack as f64 } else { 0.0001f64.powf((i - attack) as f64 / (len - attack).max(1) as f64) };
            *s += wave * env * GAIN * 4.0;
        }
    }
    let mut out = Vec::with_capacity(44 + n * 2);
    let data_len = (n * 2) as u32;
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in buf {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * i16::MAX as f64) as i16).to_le_bytes());
    }
    out
}

/// Play a line's blips, without waiting. Quiet where there's no way to.
pub fn play(text: &str, tone: Tone) {
    let plan = blip_plan(text, tone);
    if plan.is_empty() {
        return;
    }
    #[cfg(target_os = "macos")]
    {
        use objc2::AnyThread;
        use objc2_app_kit::NSSound;
        use objc2_foundation::NSData;
        let data = NSData::with_bytes(&wav(&plan));
        if let Some(sound) = NSSound::initWithData(NSSound::alloc(), &data) {
            sound.play();
            // NSSound stops when released: keep it until it's done
            std::mem::forget(sound);
        }
    }
}

// ---- remarks ----------------------------------------------------------------
// Each returns a Line, or None when there's nothing worth saying. `text` is the
// full sentence for the expanded scene; `short` the glance the compact face has
// room for.

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub text: String,
    pub short: String,
}

fn line(text: impl Into<String>, short: impl Into<String>) -> Line {
    Line { text: text.into(), short: short.into() }
}

pub type Fmt<'a> = &'a dyn Fn(u64) -> String;
pub type Dur<'a> = &'a dyn Fn(i64) -> String;

/// the small hours get their own hello: "Good morning!" at 2am rings false
const LATE_UNTIL: u32 = 5;

fn salutation(hour: u32) -> &'static str {
    match hour {
        h if h < LATE_UNTIL => "Still up?",
        h if h < 12 => "Good morning!",
        h if h < 18 => "Good afternoon!",
        _ => "Good evening!",
    }
}

fn hello(hour: u32) -> &'static str {
    match hour {
        h if h < LATE_UNTIL => "Up late!",
        h if h < 12 => "Morning!",
        h if h < 18 => "Afternoon!",
        _ => "Evening!",
    }
}

/// `days` is the 30-day series, oldest first, today last.
pub fn greeting(days: &[u64], hour: u32, fmt: Fmt) -> Option<Line> {
    if days.len() < 2 {
        return None;
    }
    let y = days[days.len() - 2];
    if y == 0 {
        return None;
    }
    let week = &days[days.len().saturating_sub(8)..days.len() - 1];
    let worked: Vec<u64> = week.iter().copied().filter(|&v| v > 0).collect();
    let (hi, short) = (salutation(hour), format!("{} {} yesterday.", hello(hour), fmt(y)));
    if worked.len() > 1 && y >= week.iter().copied().max().unwrap_or(0) {
        return Some(line(format!("{hi} Yesterday was your heaviest day this week, {} tokens.", fmt(y)), format!("{} Big day yesterday.", hello(hour))));
    }
    let avg = worked.iter().sum::<u64>() as f64 / worked.len().max(1) as f64;
    if worked.len() > 2 && (y as f64) < avg * 0.5 {
        return Some(line(format!("{hi} Yesterday was a quiet one, just {} tokens.", fmt(y)), short));
    }
    Some(line(format!("{hi} You spent {} tokens yesterday.", fmt(y)), short))
}

pub fn fire(pct: f64, eta_ms: Option<i64>, reset_ms: Option<i64>, dur: Dur) -> Line {
    let p = pct.round();
    let short = format!("{p}%, getting warm!");
    match (eta_ms, reset_ms) {
        (Some(eta), _) => line(format!("Getting warm, {p}% already. At this pace you'll run out in about {}.", dur(eta)), short),
        (None, Some(r)) => line(format!("Getting warm, {p}% already. It resets in {}.", dur(r)), short),
        _ => line(format!("Getting warm, {p}% already."), short),
    }
}

pub fn reset(was: f64) -> Line {
    line(format!("Fresh window! The last one closed at {}%.", was.round()), "Fresh window!")
}

/// What the window that just closed was like.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Recap {
    pub was: f64,
    pub tokens: u64,
    pub active_ms: i64,
    pub top: Option<String>,
}

fn doing(activity: &str) -> Option<&'static str> {
    Some(match activity {
        "editing" => "mostly editing code",
        "reading" => "mostly reading",
        "planning" => "mostly planning",
        "running" => "mostly running commands",
        "researching" => "mostly researching",
        "delegating" => "mostly delegating to agents",
        "waiting" => "mostly waiting on you",
        _ => return None,
    })
}

/// The recap of the window that just closed, from whatever is known.
pub fn recap(r: &Recap, fmt: Fmt, dur: Dur) -> Line {
    if r.tokens == 0 {
        return reset(r.was);
    }
    let mut text = format!("Fresh window! That was {} tokens", fmt(r.tokens));
    if r.active_ms >= 60_000 {
        text += &format!(" over {}", dur(r.active_ms));
    }
    if let Some(d) = r.top.as_deref().and_then(doing) {
        text += &format!(", {d}");
    }
    line(format!("{text}."), format!("Fresh window! {} last time.", fmt(r.tokens)))
}

// Codex and Cursor speak through the same pet, so their lines say whose they are
pub fn ext_fire(pct: f64, reset_ms: Option<i64>, dur: Dur, name: &str) -> Line {
    let p = pct.round();
    let text = match reset_ms {
        Some(r) => format!("{name} is at {p}% now. It resets in {}.", dur(r)),
        None => format!("{name} is at {p}% now."),
    };
    line(text, format!("{name} at {p}%!"))
}

pub fn ext_maxed(reset_at: Option<&str>, name: &str) -> Line {
    let text = match reset_at {
        Some(at) => format!("{name} is maxed out. It's back at {at}."),
        None => format!("{name} is maxed out."),
    };
    line(text, format!("{name} is maxed out."))
}

/// Cursor's window is its billing month.
pub fn ext_reset(was: f64, name: &str, span: &str) -> Line {
    line(format!("{name} has a fresh {span}! The last one closed at {}%.", was.round()), format!("{name}: fresh {span}!"))
}

pub fn maxed(reset_at: Option<&str>) -> Line {
    match reset_at {
        Some(at) => line(format!("That's the limit. I'll be back at {at}."), format!("Maxed out till {at}.")),
        None => line("That's the limit. Nap time.", "Maxed out."),
    }
}

pub fn welcome(away_ms: i64, pct: Option<f64>, dur: Dur) -> Line {
    let away = dur(away_ms);
    let text = match pct {
        Some(p) => format!("Welcome back! You were gone {away}. The session's at {}%.", p.round()),
        None => format!("Welcome back! You were gone {away}."),
    };
    line(text, "Welcome back!")
}

pub fn streak(ms: i64, dur: Dur) -> Line {
    line(format!("You've been at it for {} straight. Stretch break?", dur(ms)), "Stretch break?")
}

/// a record only counts against a real history, past a floor that makes it more than noise
const RECORD_FLOOR: u64 = 1_000_000;

pub fn record(days: &[u64], fmt: Fmt) -> Option<Line> {
    if days.len() < 8 {
        return None;
    }
    let today = *days.last()?;
    let best = days[..days.len() - 1].iter().copied().max().unwrap_or(0);
    if best == 0 || today < RECORD_FLOOR || today <= best {
        return None;
    }
    Some(line(format!("New record! {} tokens today, your biggest day in a month.", fmt(today)), format!("New record: {}!", fmt(today))))
}

#[cfg(test)]
mod tests {
    use super::*;

    // the JS tests' formatters: "12M", "40m"
    fn fmt(n: u64) -> String {
        format!("{}M", n as f64 / 1e6)
    }
    fn dur(ms: i64) -> String {
        format!("{}m", (ms as f64 / 60_000.0).round())
    }

    #[test]
    fn letters_span_the_range_and_anything_else_rests() {
        assert_eq!(pitch_of('a'), Some(BASE_HZ));
        assert_eq!(pitch_of('Z'), Some(BASE_HZ + SPREAD_HZ));
        assert_eq!(pitch_of('9'), Some(BASE_HZ + SPREAD_HZ));
        assert_eq!(pitch_of(' '), None);
        assert_eq!(pitch_of('%'), None);
    }

    #[test]
    fn the_blip_schedule() {
        let plan = blip_plan("abc", Tone::Normal);
        assert_eq!(plan.iter().map(|b| b.at).collect::<Vec<_>>(), vec![0, 115, 230]);
        assert_eq!(blip_plan(&"x".repeat(80), Tone::Normal).len(), MAX_BLIPS);
        let at = |s: &str| blip_plan(s, Tone::Normal)[1].at;
        assert!(at("a b") > at("ab"));
        assert!(at("a. b") > at("a b"));
        let (n, f, s) = (blip_plan("aa", Tone::Normal)[1], blip_plan("aa", Tone::Fire)[1], blip_plan("aa", Tone::Sleepy)[1]);
        assert!(f.at < n.at && f.hz > n.hz);
        assert!(s.at > n.at && s.hz < n.hz);
    }

    #[test]
    fn the_wav_is_a_playable_file() {
        let w = wav(&blip_plan("hi", Tone::Normal));
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(&w[8..12], b"WAVE");
        assert!(w.len() > 44 + 1000);
    }

    fn week(y: u64, rest: [u64; 6]) -> Vec<u64> {
        let mut v = vec![0; 22];
        v.extend(rest);
        v.extend([y, 5_000_000]);
        v
    }

    #[test]
    fn the_greeting_names_yesterday() {
        let l = greeting(&week(12_000_000, [1, 2, 3, 4, 5, 6].map(|n| n * 1_000_000)), 9, &fmt).unwrap();
        assert_eq!(l.text, "Good morning! Yesterday was your heaviest day this week, 12M tokens.");
        assert_eq!(l.short, "Morning! Big day yesterday.");
        let l = greeting(&week(1_000_000, [8_000_000; 6]), 14, &fmt).unwrap();
        assert_eq!(l.text, "Good afternoon! Yesterday was a quiet one, just 1M tokens.");
        let ordinary = week(5_000_000, [4, 6, 5, 5, 5, 5].map(|n| n * 1_000_000));
        assert_eq!(greeting(&ordinary, 20, &fmt).unwrap().text, "Good evening! You spent 5M tokens yesterday.");
        assert_eq!(greeting(&ordinary, 2, &fmt).unwrap().short, "Up late! 5M yesterday.");
        assert_eq!(greeting(&ordinary, 5, &fmt).unwrap().short, "Morning! 5M yesterday.");
        assert!(greeting(&week(0, [1; 6]), 9, &fmt).is_none());
        assert!(greeting(&[], 9, &fmt).is_none());
    }

    #[test]
    fn the_other_remarks() {
        assert_eq!(fire(91.4, Some(40 * 60_000), Some(3_600_000), &dur).text, "Getting warm, 91% already. At this pace you'll run out in about 40m.");
        assert_eq!(fire(91.0, None, Some(90 * 60_000), &dur).text, "Getting warm, 91% already. It resets in 90m.");
        assert_eq!(fire(91.0, None, None, &dur).short, "91%, getting warm!");
        assert_eq!(reset(92.6), line("Fresh window! The last one closed at 93%.", "Fresh window!"));
        assert_eq!(maxed(Some("14:35")), line("That's the limit. I'll be back at 14:35.", "Maxed out till 14:35."));
        assert_eq!(maxed(None).text, "That's the limit. Nap time.");
        assert_eq!(welcome(180 * 60_000, Some(12.0), &dur).text, "Welcome back! You were gone 180m. The session's at 12%.");
        assert_eq!(streak(95 * 60_000, &dur), line("You've been at it for 95m straight. Stretch break?", "Stretch break?"));
    }

    #[test]
    fn a_record_needs_a_history_a_floor_and_to_beat_it() {
        let mut days = vec![2_000_000; 29];
        days.push(3_000_000);
        assert_eq!(record(&days, &fmt), Some(line("New record! 3M tokens today, your biggest day in a month.", "New record: 3M!")));
        days[29] = 2_000_000;
        assert!(record(&days, &fmt).is_none());
        let mut empty = vec![0; 29];
        empty.push(3_000_000);
        assert!(record(&empty, &fmt).is_none());
        assert!(record(&[1, 2], &fmt).is_none());
    }

    #[test]
    fn the_recap_is_built_from_what_is_known() {
        let all = Recap { was: 92.0, tokens: 34_000_000, active_ms: 252 * 60_000, top: Some("editing".into()) };
        assert_eq!(recap(&all, &fmt, &dur), line("Fresh window! That was 34M tokens over 252m, mostly editing code.", "Fresh window! 34M last time."));
        let short = Recap { was: 92.0, tokens: 5_000_000, active_ms: 30_000, top: Some("nope".into()) };
        assert_eq!(recap(&short, &fmt, &dur).text, "Fresh window! That was 5M tokens.");
        assert_eq!(recap(&Recap { was: 92.0, ..Default::default() }, &fmt, &dur).text, "Fresh window! The last one closed at 92%.");
    }

    #[test]
    fn codex_and_cursor_lines_say_whose_they_are() {
        assert_eq!(ext_fire(91.2, Some(3_600_000), &dur, "Codex"), line("Codex is at 91% now. It resets in 60m.", "Codex at 91%!"));
        assert_eq!(ext_maxed(Some("14:35"), "Codex").text, "Codex is maxed out. It's back at 14:35.");
        assert_eq!(ext_reset(88.0, "Codex", "window"), line("Codex has a fresh window! The last one closed at 88%.", "Codex: fresh window!"));
        assert_eq!(ext_reset(90.0, "Cursor", "month").text, "Cursor has a fresh month! The last one closed at 90%.");
    }
}
