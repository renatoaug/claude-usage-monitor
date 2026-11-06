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
}

pub const DEFAULT: &str = "clauddy";

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
    fn a_pack_without_pets_leaves_the_choice_alone() {
        let halloween = SKINS.iter().any(|s| s.pack == "halloween");
        assert_eq!(seasonal(2026, 10, 31).is_some(), halloween);
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
