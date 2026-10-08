//! Windows-1252 decoding of EX-IM text, the payload charset assumed for legacy files.
//!
//! `[A]` Both measured files decode correctly as Windows-1252 and contain no
//! byte in `0x80`–`0x9F`, the only range where Windows-1252 and ISO-8859-1
//! differ, so the samples cannot tell the two apart. Bytes in that range are
//! decoded as Windows-1252 and counted, so the parser can report them.

use std::borrow::Cow;

/// Windows-1252 code points for `0x80`–`0x9F`. The five bytes Windows-1252
/// leaves undefined (`0x81`, `0x8D`, `0x8F`, `0x90`, `0x9D`) map to the C1
/// control with the same number, as the WHATWG Encoding Standard does.
const HIGH_CONTROL_RANGE: [char; 32] = [
    '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}', '\u{017D}', '\u{008F}',
    '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
];

/// Decodes `bytes` as Windows-1252. ASCII input is borrowed unchanged.
pub(crate) fn decode_windows_1252(bytes: &[u8]) -> Cow<'_, str> {
    if bytes.is_ascii() {
        // ASCII is valid UTF-8 byte for byte.
        return Cow::Borrowed(std::str::from_utf8(bytes).unwrap_or_default());
    }
    Cow::Owned(
        bytes
            .iter()
            .map(|&byte| match byte {
                0x80..=0x9F => HIGH_CONTROL_RANGE[usize::from(byte - 0x80)],
                _ => char::from(byte),
            })
            .collect(),
    )
}

/// Counts bytes in `0x80`–`0x9F`, where the charset assumption matters.
pub(crate) fn windows_1252_only_bytes(bytes: &[u8]) -> usize {
    bytes.iter().filter(|b| (0x80..=0x9F).contains(*b)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_is_borrowed_and_latin1_range_maps_one_to_one() {
        assert!(matches!(
            decode_windows_1252(b"plain"),
            Cow::Borrowed("plain")
        ));
        assert_eq!(
            decode_windows_1252(b"\xe4\xf6\xfc\xdf\xa0\xff"),
            "äöüß\u{a0}ÿ"
        );
    }

    #[test]
    fn the_windows_1252_range_decodes_to_its_own_code_points() {
        assert_eq!(decode_windows_1252(b"\x80\x84\x93\x94\x9f"), "€„“”Ÿ");
        assert_eq!(decode_windows_1252(b"\x81\x9d"), "\u{81}\u{9d}");
        assert_eq!(windows_1252_only_bytes(b"\x7f\x80\x9f\xa0"), 2);
    }
}
