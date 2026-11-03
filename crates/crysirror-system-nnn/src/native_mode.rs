// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Reviewed original mode identities, independent of which mode actors have
//! native consumers. The old module has fewer named modes than later sources.
use anyhow::Result;
const MODES: &[(i32, &[u8])] = &[
    (1, b"nothing"),
    (2, b"logo"),
    (3, b"opening"),
    (4, b"title"),
    (5, b"load"),
    (6, b"save"),
    (7, b"backlog"),
    (8, b"listenbgm"),
    (9, b"selectmessage"),
    (10, b"printmessage"),
    (11, b"draw"),
    (12, b"config"),
    (13, b"systemmenu"),
    (14, b"printoverrap"),
    (15, b"omake"),
    (16, b"selectcgchara"),
    (17, b"selectcg"),
    (18, b"selectscene"),
    (20, b"selectscenechara"),
    (22, b"selectplace"),
    (23, b"printcg"),
    (24, b"selectzukan"),
    (25, b"selectmoviechara"),
    (26, b"selectmovie"),
    (27, b"printmovie"),
    (29, b"nameinput"),
    (30, b"printcalendar"),
    (31, b"selectheroin"),
    (32, b"codecerror"),
    (33, b"printresult"),
    (34, b"selectchart"),
    (35, b"selectplace2"),
    (40, b"minigame"),
];
/// Original GetModeNumberByName uses an ASCII case-insensitive name table,
/// then atoi's decimal prefix (including unknown nonnumeric strings ->0).
/// Returning a number never claims that its mode actor is implemented.
pub(crate) fn number(name: &[u8]) -> Result<i32> {
    if let Some((id, _)) = MODES
        .iter()
        .find(|(_, value)| value.eq_ignore_ascii_case(name))
    {
        return Ok(*id);
    }
    crate::config::integer(name)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_mode_identity_keeps_numeric_prefix_and_old_named_table_distinct_from_consumers() {
        for (name, id) in [
            (b"nothing".as_slice(), 1),
            (b"SELECTMESSAGE", 9),
            (b"printmovie", 27),
            (b"selectplace2", 35),
            (b"minigame", 40),
        ] {
            assert_eq!(number(name).unwrap(), id);
        }
        for (name, id) in [
            (b" +27suffix".as_slice(), 27),
            (b"-1", -1),
            (b"0", 0),
            (b"unknown", 0),
            (b"printhint", 0),
            (b"shop", 0),
        ] {
            assert_eq!(number(name).unwrap(), id);
        }
        assert!(number(b"2147483648").is_err());
        assert!(
            crate::native_shell::NativeMode::named(b"5").is_err(),
            "a correct load identity is not a fabricated load actor"
        );
        assert_eq!(
            crate::native_shell::NativeMode::named(b"27").unwrap(),
            crate::native_shell::NativeMode::Movie
        );
    }
}
