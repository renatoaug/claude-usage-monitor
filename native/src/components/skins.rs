// The pets to pick from: Clauddy and its costumes, each a pair of sprite
// sheets in the same layout (working 4 x 2, moods 4 x 4), generated in the
// classic sprite's art style. Seasonal packs take over on their days: one pet
// per pack and year, the same all day and across restarts, picked by a hash
// and never stored (port of renderer/pets.js).

/// Where the art sits inside a frame, as fractions of it: (left, top, right, bottom).
pub type Box4 = [f32; 4];

pub struct Skin {
    pub id: &'static str,
    pub name: &'static str,
    pub pack: &'static str,
    pub working: &'static [u8],
    pub moods: &'static [u8],
    /// the desk scene, all of it
    pub work_box: Box4,
    /// on fire: the body and the flames above it
    pub fire_box: Box4,
    /// standing: the body alone, so switching moods doesn't move it
    pub stand_box: Box4,
    /// out on the screen (4 x 1): crouching, jumping, falling, dizzy; a pet
    /// without one plays with its moods
    pub play: Option<&'static [u8]>,
    /// its own number on the screen (4 x 1), when it has one (the Getaway's
    /// drive), and where the art sits in a frame
    pub act: Option<&'static [u8]>,
    pub act_box: Box4,
    /// how wide an act frame is drawn against a standing one, so the pet
    /// in it is the size it is everywhere else
    pub act_scale: f32,
}

pub const DEFAULT: &str = "clauddy";

/// The hats Clauddy can wear (the classic pet only: the others come
/// dressed), in the order of `assets/hats.png` (4 x 2): (id, name).
pub const HATS: [(&str, &str); 8] = [
    ("santa", "Santa"),
    ("party", "Party"),
    ("crown", "Crown"),
    ("viking", "Viking"),
    ("cowboy", "Cowboy"),
    ("top", "Top hat"),
    ("beanie", "Beanie"),
    ("grad", "Grad cap"),
];

pub const HATS_PNG: &[u8] = include_bytes!("../../assets/hats.png");

/// The presents Santa's sleigh drops (4 x 1), and where a box sits in a frame.
pub const GIFTS_PNG: &[u8] = include_bytes!("../../assets/gifts.png");
pub const GIFT_BOX: Box4 = [0.266, 0.273, 0.734, 0.781];

/// The pet that wears hats.
pub const HAT_WEARER: &str = "clauddy";

pub struct Pack {
    pub id: &'static str,
    /// (month, day) it takes over on
    pub days: &'static [(u32, u32)],
}

pub const PACKS: &[Pack] =
    &[Pack { id: "everyday", days: &[] }, Pack { id: "halloween", days: &[(10, 31)] }, Pack { id: "christmas", days: &[(12, 24), (12, 25)] }];

pub const SKINS: &[Skin] = &[
    Skin {
        id: "clauddy",
        name: "Clauddy",
        pack: "everyday",
        working: include_bytes!("../../assets/working-sheet.png"),
        moods: include_bytes!("../../assets/moods-sheet.png"),
        work_box: [0.047, 0.203, 0.961, 0.93],
        fire_box: [0.086, 0.062, 0.898, 0.812],
        stand_box: [0.18, 0.27, 0.84, 0.79],
        play: Some(include_bytes!("../../assets/clauddy-play.png")),
        act: Some(include_bytes!("../../assets/clauddy-act.png")),
        // up at the glass: looking, knocking, pressed flat, waving
        act_box: [0.1, 0.258, 0.86, 0.84],
        act_scale: 0.9,
    },
    Skin {
        id: "wizard",
        name: "Wizard",
        pack: "everyday",
        working: include_bytes!("../../assets/wizard-working.png"),
        moods: include_bytes!("../../assets/wizard-moods.png"),
        work_box: [0.047, 0.051, 0.961, 0.93],
        fire_box: [0.031, 0.012, 0.91, 0.844],
        // the hat, wand and owl stand with it
        stand_box: [0.059, 0.07, 0.852, 0.828],
        play: None,
        act: Some(include_bytes!("../../assets/wizard-act.png")),
        // the wand raised, sparks shot, a poof of smoke, a wave
        act_box: [0.2, 0.074, 0.84, 0.875],
        act_scale: 1.0,
    },
    Skin {
        id: "astronaut",
        name: "Astronaut",
        pack: "everyday",
        working: include_bytes!("../../assets/astronaut-working.png"),
        moods: include_bytes!("../../assets/astronaut-moods.png"),
        work_box: [0.047, 0.039, 0.961, 0.934],
        fire_box: [0.09, 0.062, 0.898, 0.883],
        // the helmet and the backpack stand with it
        stand_box: [0.164, 0.172, 0.891, 0.848],
        play: None,
        act: Some(include_bytes!("../../assets/astronaut-act.png")),
        // riding its rocket: two sizes of flame, cruising, a wave; the box
        // is the rocket and the rider (the flame trails off the left)
        act_box: [0.25, 0.11, 0.86, 0.88],
        act_scale: 1.1,
    },
    Skin {
        id: "getaway",
        name: "Getaway",
        pack: "everyday",
        working: include_bytes!("../../assets/getaway-working.png"),
        moods: include_bytes!("../../assets/getaway-moods.png"),
        // the car and the palm tree
        work_box: [0.031, 0.16, 0.973, 0.934],
        // the flames and the siren on top
        fire_box: [0.078, 0.043, 0.922, 0.867],
        // the shades and the jacket stand with it; the twirling keys swing out
        stand_box: [0.137, 0.133, 0.902, 0.867],
        play: None,
        act: Some(include_bytes!("../../assets/getaway-act.png")),
        // the car, wheels at the bottom (its exhaust and speed lines trail off the left)
        act_box: [0.113, 0.223, 0.918, 0.918],
        act_scale: 1.25,
    },
    Skin {
        id: "santa",
        name: "Santa",
        pack: "christmas",
        working: include_bytes!("../../assets/santa-working.png"),
        moods: include_bytes!("../../assets/santa-moods.png"),
        work_box: [0.043, 0.035, 0.965, 0.934],
        fire_box: [0.09, 0.074, 0.941, 0.875],
        // the hat, the gift and the sack stand with it
        stand_box: [0.141, 0.172, 0.941, 0.871],
        play: None,
        act: Some(include_bytes!("../../assets/santa-act.png")),
        // the sleigh and its two reindeer (4 x 1 at 512 x 256), the runners at the bottom
        act_box: [0.021, 0.043, 0.965, 0.887],
        act_scale: 1.5,
    },
    Skin {
        id: "reindeer",
        name: "Reindeer",
        pack: "christmas",
        working: include_bytes!("../../assets/reindeer-working.png"),
        moods: include_bytes!("../../assets/reindeer-moods.png"),
        work_box: [0.043, 0.07, 0.965, 0.934],
        fire_box: [0.086, 0.059, 0.906, 0.832],
        stand_box: [0.176, 0.152, 0.836, 0.812],
        play: None,
        act: Some(include_bytes!("../../assets/reindeer-act.png")),
        // a flying gallop (three frames) and a wave with its nose aglow
        act_box: [0.2, 0.15, 0.9, 0.78],
        act_scale: 1.0,
    },
    Skin {
        id: "gingerbread",
        name: "Gingerbread",
        pack: "christmas",
        working: include_bytes!("../../assets/gingerbread-working.png"),
        moods: include_bytes!("../../assets/gingerbread-moods.png"),
        work_box: [0.047, 0.145, 0.965, 0.938],
        fire_box: [0.082, 0.062, 0.91, 0.816],
        stand_box: [0.121, 0.238, 0.875, 0.793],
        play: None,
        act: Some(include_bytes!("../../assets/gingerbread-act.png")),
        // running (its crumbs trail off the left) and teasing, feet at the bottom
        act_box: [0.156, 0.2, 0.898, 0.812],
        act_scale: 0.9,
    },
    Skin {
        id: "pumpkin",
        name: "Pumpkin",
        pack: "halloween",
        working: include_bytes!("../../assets/pumpkin-working.png"),
        moods: include_bytes!("../../assets/pumpkin-moods.png"),
        work_box: [0.047, 0.109, 0.969, 0.934],
        fire_box: [0.082, 0.0, 0.91, 0.785],
        stand_box: [0.176, 0.16, 0.93, 0.785],
        play: None,
        act: Some(include_bytes!("../../assets/pumpkin-act.png")),
        // cackling; then its bat (two frames) and its ghost, drawn on their own
        act_box: [0.14, 0.22, 0.86, 0.742],
        act_scale: 1.0,
    },
];

pub fn find(id: &str) -> Option<&'static Skin> {
    SKINS.iter().find(|s| s.id == id)
}

// FNV-1a: a stable pick per year without storing anything
fn hash(text: &str) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in text.encode_utf16() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

/// The season's pet on its days, if its pack has any.
pub fn seasonal(year: i32, month: u32, day: u32) -> Option<&'static Skin> {
    let pack = PACKS.iter().find(|p| p.days.contains(&(month, day)))?;
    let pool: Vec<&Skin> = SKINS.iter().filter(|s| s.pack == pack.id).collect();
    (!pool.is_empty()).then(|| pool[hash(&format!("{year}:{}", pack.id)) as usize % pool.len()])
}

/// What the widget shows: the season's pet on its days, else the saved choice.
pub fn active(chosen: &str, year: i32, month: u32, day: u32) -> &'static Skin {
    seasonal(year, month, day).or_else(|| find(chosen)).unwrap_or(&SKINS[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_matches_the_electron_apps() {
        // FNV-1a over UTF-16 code units, as the JS loop does
        assert_eq!(hash(""), 0x811c_9dc5);
        assert_eq!(hash("a"), 0xe40c_292c);
    }

    #[test]
    fn the_saved_pet_shows_and_an_unknown_one_falls_back() {
        assert_eq!(active("clauddy", 2026, 11, 6).id, "clauddy");
        assert_eq!(active("nope", 2026, 11, 6).id, DEFAULT);
    }

    #[test]
    fn christmas_picks_one_of_its_pets_for_both_days() {
        let eve = seasonal(2026, 12, 24).unwrap();
        assert_eq!(eve.pack, "christmas");
        assert_eq!(seasonal(2026, 12, 25).unwrap().id, eve.id);
        assert_eq!(active("wizard", 2026, 12, 25).id, eve.id);
        assert_eq!(active("wizard", 2026, 12, 26).id, "wizard");
    }

    #[test]
    fn halloween_brings_the_pumpkin_and_other_days_nothing() {
        assert_eq!(seasonal(2026, 10, 31).unwrap().id, "pumpkin");
        assert!(seasonal(2026, 11, 6).is_none());
    }

    #[test]
    fn every_pet_has_both_sheets_in_their_grids() {
        for s in SKINS {
            for (bytes, size) in [(s.working, (1024, 512)), (s.moods, (1024, 1024))] {
                let r = png::Decoder::new(std::io::Cursor::new(bytes)).read_info().unwrap();
                assert_eq!((r.info().width, r.info().height), size, "{}", s.id);
            }
        }
    }
}
