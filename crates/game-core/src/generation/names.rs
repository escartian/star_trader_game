//! Names for generated stars, planets, moons, asteroids and stations: built
//! from syllables by the map's seed, unique within a galaxy, never numbered or
//! generic ("Inner world"). Stations take a proper name and a port word
//! ("Port Avar", "Halden Exchange").
use super::next;
use std::collections::HashSet;

/// Name beginnings; with a linking vowel and an ending they make words that
/// read as places (about 9,000 combinations).
const PREFIXES: [&str; 36] = [
    "Al", "Bel", "Cal", "Dor", "El", "Fen", "Gal", "Hal", "Is", "Jor", "Kal", "Lor", "Mar", "Nor",
    "Or", "Per", "Ral", "Sel", "Tal", "Ul", "Var", "Wen", "Zar", "Cor", "Ser", "Ar", "Bren", "Cas",
    "Del", "Mir", "Ost", "Tes", "Vel", "Ath", "Ky", "Rho",
];
/// Optional links between beginning and ending.
const LINKS: [&str; 10] = ["", "", "", "", "a", "e", "i", "o", "an", "er"];
const ENDINGS: [&str; 26] = [
    "a", "is", "on", "ar", "ia", "or", "us", "en", "ara", "ion", "ene", "ith", "ora", "ea", "ius",
    "ane", "ova", "ere", "ir", "une", "ax", "ell", "ys", "anth", "os", "eda",
];
/// Station name patterns: `{}` is the station's own name.
const STATIONS: [&str; 6] = [
    "Port {}",
    "{} Station",
    "{} Exchange",
    "{} Yards",
    "{} Terminal",
    "{} Hub",
];

/// Hands out unused names for one galaxy.
pub(super) struct Namer {
    used: HashSet<String>,
}

impl Namer {
    /// `taken` names (the authored systems and bodies) are never reused.
    pub(super) fn new<'a>(taken: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            used: taken.into_iter().map(str::to_owned).collect(),
        }
    }

    /// A new place name: a beginning, perhaps a linking vowel, and an ending.
    pub(super) fn place(&mut self, state: &mut u64) -> String {
        loop {
            let word = format!(
                "{}{}{}",
                pick(&PREFIXES, state),
                pick(&LINKS, state),
                pick(&ENDINGS, state)
            );
            let name = capitalize(&tidy(&word.to_lowercase()));
            if (4..=10).contains(&name.len()) && self.used.insert(name.clone()) {
                return name;
            }
        }
    }

    /// A station's name: its own name in a port pattern.
    pub(super) fn station(&mut self, state: &mut u64) -> String {
        let own = self.place(state);
        pick(&STATIONS, state).replace("{}", &own)
    }
}

fn pick<'a>(options: &[&'a str], state: &mut u64) -> &'a str {
    options[(next(state) % options.len() as u64) as usize]
}

/// Drops a letter that would repeat the one before it at a join ("Ala" +
/// "ara" → "Alara") and any third vowel in a row.
fn tidy(word: &str) -> String {
    let mut out: Vec<char> = Vec::new();
    for letter in word.chars() {
        let n = out.len();
        let vowel = |c: char| "aeiouy".contains(c);
        let doubled = n >= 1 && out[n - 1] == letter && vowel(letter);
        let vowel_run = n >= 2 && vowel(letter) && vowel(out[n - 1]) && vowel(out[n - 2]);
        if !doubled && !vowel_run {
            out.push(letter);
        }
    }
    out.into_iter().collect()
}

fn capitalize(word: &str) -> String {
    let mut letters = word.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_capitalized_words_without_digits() {
        let mut namer = Namer::new(["Sol", "Meridian"]);
        let mut state = 7;
        let mut seen = HashSet::new();
        for _ in 0..1_500 {
            let name = namer.place(&mut state);
            assert!(name.chars().next().unwrap().is_uppercase(), "{name}");
            assert!(name.chars().all(char::is_alphabetic), "{name}");
            assert!(seen.insert(name.clone()), "{name} repeated");
            assert!(name != "Sol" && name != "Meridian");
        }
        let station = namer.station(&mut state);
        assert!(!station.chars().any(|c| c.is_ascii_digit()), "{station}");
        assert!(station.split(' ').count() == 2, "{station}");
    }
}
