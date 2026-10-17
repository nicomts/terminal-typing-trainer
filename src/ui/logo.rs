//! The block-letter logo on the menu, drawn with half-block characters.
//!
//! Each letter is 3 columns wide and 3 rows high. To change the word, edit
//! `WORD`; any new letter also needs a glyph in `glyph`.

/// The word the logo spells.
const WORD: &str = "TYPE TRAINER";

/// Rows of the logo.
pub const ROWS: u16 = 3;

/// The 3 rows of one letter. Every row is exactly 3 characters, so all
/// letters line up. A space is a narrower gap between words.
fn glyph(letter: char) -> [&'static str; 3] {
    match letter {
        'A' => ["█▀█", "█▀█", "▀ ▀"],
        'E' => ["█▀▀", "█▀▀", "▀▀▀"],
        'I' => ["▀█▀", " █ ", "▀▀▀"],
        'N' => ["█▀▄", "█ █", "▀ ▀"],
        'P' => ["█▀█", "█▀▀", "▀  "],
        'R' => ["█▀█", "█▀▄", "▀ ▀"],
        'T' => ["▀█▀", " █ ", " ▀ "],
        'Y' => ["█ █", "▀█▀", " ▀ "],
        ' ' => [" ", " ", " "],
        // Not reachable with the `WORD` above; a blank keeps the rows even.
        _ => ["   ", "   ", "   "],
    }
}

/// The logo as text: 3 strings of equal length, letters separated by one
/// column.
pub fn rows() -> [String; 3] {
    let mut rows = [String::new(), String::new(), String::new()];
    for (i, letter) in WORD.chars().enumerate() {
        if i > 0 {
            for row in &mut rows {
                row.push(' ');
            }
        }
        let glyph = glyph(letter);
        for (row, part) in rows.iter_mut().zip(glyph) {
            row.push_str(part);
        }
    }
    rows
}

/// Columns the logo takes.
pub fn width() -> u16 {
    let columns = rows()[0].chars().count();
    u16::try_from(columns).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_letter_has_a_glyph() {
        for letter in WORD.chars() {
            if letter != ' ' {
                assert_ne!(glyph(letter), glyph('?'), "no glyph for {letter:?}");
            }
        }
    }

    #[test]
    fn rows_have_equal_width() {
        let [a, b, c] = rows();
        assert_eq!(a.chars().count(), b.chars().count());
        assert_eq!(b.chars().count(), c.chars().count());
    }

    #[test]
    fn logo_fits_the_narrowest_panel() {
        assert!(width() <= super::super::MIN_WIDTH);
    }
}
