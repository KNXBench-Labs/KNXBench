//! Explicit CSV import encodings: UTF-8 or Windows-1252, chosen by the user and never guessed.
//!
//! The Windows-1252 table follows the Unicode Consortium's Microsoft mapping
//! (`MAPPINGS/VENDORS/MICSFT/WINDOWS/CP1252.TXT`): bytes `0x00`–`0x7F` and
//! `0xA0`–`0xFF` are the code points of the same value, `0x80`–`0x9F` follow
//! [`CP1252_HIGH`], and the five bytes that table leaves undefined (`0x81`,
//! `0x8D`, `0x8F`, `0x90`, `0x9D`) are refused with their offset rather than
//! mapped to C1 control characters.

use std::borrow::Cow;

/// The text encoding the user chose for a CSV import file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CsvEncoding {
    /// UTF-8, with or without a leading byte-order mark; the default.
    #[default]
    Utf8,
    /// The Windows-1252 single-byte code page that ETS6 was observed to write.
    Windows1252,
}

impl CsvEncoding {
    /// The canonical label used on every surface (`utf-8`, `windows-1252`).
    pub fn label(self) -> &'static str {
        match self {
            CsvEncoding::Utf8 => "utf-8",
            CsvEncoding::Windows1252 => "windows-1252",
        }
    }

    /// Parses a canonical label, ASCII case-insensitively. Anything else is
    /// `None`: no alias, code-page number or guess is accepted.
    pub fn from_label(label: &str) -> Option<Self> {
        if label.eq_ignore_ascii_case("utf-8") {
            Some(CsvEncoding::Utf8)
        } else if label.eq_ignore_ascii_case("windows-1252") {
            Some(CsvEncoding::Windows1252)
        } else {
            None
        }
    }
}

/// `0x80`–`0x9F` in Windows-1252; `None` marks the five undefined bytes.
const CP1252_HIGH: [Option<char>; 32] = [
    Some('\u{20AC}'), // 0x80 EURO SIGN
    None,             // 0x81 undefined
    Some('\u{201A}'), // 0x82 SINGLE LOW-9 QUOTATION MARK
    Some('\u{0192}'), // 0x83 LATIN SMALL LETTER F WITH HOOK
    Some('\u{201E}'), // 0x84 DOUBLE LOW-9 QUOTATION MARK
    Some('\u{2026}'), // 0x85 HORIZONTAL ELLIPSIS
    Some('\u{2020}'), // 0x86 DAGGER
    Some('\u{2021}'), // 0x87 DOUBLE DAGGER
    Some('\u{02C6}'), // 0x88 MODIFIER LETTER CIRCUMFLEX ACCENT
    Some('\u{2030}'), // 0x89 PER MILLE SIGN
    Some('\u{0160}'), // 0x8A LATIN CAPITAL LETTER S WITH CARON
    Some('\u{2039}'), // 0x8B SINGLE LEFT-POINTING ANGLE QUOTATION MARK
    Some('\u{0152}'), // 0x8C LATIN CAPITAL LIGATURE OE
    None,             // 0x8D undefined
    Some('\u{017D}'), // 0x8E LATIN CAPITAL LETTER Z WITH CARON
    None,             // 0x8F undefined
    None,             // 0x90 undefined
    Some('\u{2018}'), // 0x91 LEFT SINGLE QUOTATION MARK
    Some('\u{2019}'), // 0x92 RIGHT SINGLE QUOTATION MARK
    Some('\u{201C}'), // 0x93 LEFT DOUBLE QUOTATION MARK
    Some('\u{201D}'), // 0x94 RIGHT DOUBLE QUOTATION MARK
    Some('\u{2022}'), // 0x95 BULLET
    Some('\u{2013}'), // 0x96 EN DASH
    Some('\u{2014}'), // 0x97 EM DASH
    Some('\u{02DC}'), // 0x98 SMALL TILDE
    Some('\u{2122}'), // 0x99 TRADE MARK SIGN
    Some('\u{0161}'), // 0x9A LATIN SMALL LETTER S WITH CARON
    Some('\u{203A}'), // 0x9B SINGLE RIGHT-POINTING ANGLE QUOTATION MARK
    Some('\u{0153}'), // 0x9C LATIN SMALL LIGATURE OE
    None,             // 0x9D undefined
    Some('\u{017E}'), // 0x9E LATIN SMALL LETTER Z WITH CARON
    Some('\u{0178}'), // 0x9F LATIN CAPITAL LETTER Y WITH DIAERESIS
];

/// Maps one Windows-1252 byte to its character; `None` for an undefined byte.
pub(crate) fn cp1252_char(byte: u8) -> Option<char> {
    match byte {
        0x80..=0x9F => CP1252_HIGH[usize::from(byte - 0x80)],
        _ => Some(char::from(byte)),
    }
}

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

/// Decodes an import file's raw bytes in the encoding the user chose.
///
/// UTF-8 keeps the established refusal of invalid bytes. Windows-1252 maps
/// every defined byte and refuses an undefined one, naming its offset. A file
/// that starts with a UTF-8 byte-order mark is refused under Windows-1252,
/// because that choice contradicts the file's own marker; the user picks
/// UTF-8 instead. Nothing is guessed in either direction.
pub fn decode_csv_bytes(bytes: &[u8], encoding: CsvEncoding) -> Result<Cow<'_, str>, String> {
    match encoding {
        CsvEncoding::Utf8 => crate::read::decode_csv_text(bytes).map(Cow::Borrowed),
        CsvEncoding::Windows1252 => {
            if bytes.starts_with(UTF8_BOM) {
                return Err(
                    "file starts with a UTF-8 byte-order mark but Windows-1252 was \
                            chosen; import it as UTF-8 instead"
                        .to_string(),
                );
            }
            let mut text = String::with_capacity(bytes.len());
            for (offset, &byte) in bytes.iter().enumerate() {
                match cp1252_char(byte) {
                    Some(c) => text.push(c),
                    None => {
                        return Err(format!(
                            "byte 0x{byte:02X} at offset {offset} is not defined in Windows-1252; \
                             the file is not Windows-1252 text, so check the chosen encoding"
                        ))
                    }
                }
            }
            Ok(Cow::Owned(text))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNDEFINED: [u8; 5] = [0x81, 0x8D, 0x8F, 0x90, 0x9D];

    /// Independent spelling of the 0x80-0x9F block (code point values from
    /// CP1252.TXT), so a typo in the table above cannot pass by symmetry.
    const EXPECTED_HIGH: [(u8, u32); 27] = [
        (0x80, 0x20AC),
        (0x82, 0x201A),
        (0x83, 0x0192),
        (0x84, 0x201E),
        (0x85, 0x2026),
        (0x86, 0x2020),
        (0x87, 0x2021),
        (0x88, 0x02C6),
        (0x89, 0x2030),
        (0x8A, 0x0160),
        (0x8B, 0x2039),
        (0x8C, 0x0152),
        (0x8E, 0x017D),
        (0x91, 0x2018),
        (0x92, 0x2019),
        (0x93, 0x201C),
        (0x94, 0x201D),
        (0x95, 0x2022),
        (0x96, 0x2013),
        (0x97, 0x2014),
        (0x98, 0x02DC),
        (0x99, 0x2122),
        (0x9A, 0x0161),
        (0x9B, 0x203A),
        (0x9C, 0x0153),
        (0x9E, 0x017E),
        (0x9F, 0x0178),
    ];

    #[test]
    fn every_byte_from_0x80_to_0x9f_maps_as_cp1252_txt_says() {
        for byte in 0x80u8..=0x9F {
            let expected = EXPECTED_HIGH
                .iter()
                .find(|(b, _)| *b == byte)
                .map(|(_, cp)| char::from_u32(*cp).unwrap());
            assert_eq!(cp1252_char(byte), expected, "byte 0x{byte:02X}");
            assert_eq!(expected.is_none(), UNDEFINED.contains(&byte));
        }
    }

    #[test]
    fn ascii_and_latin1_ranges_map_to_the_same_code_point() {
        for byte in (0x00u8..=0x7F).chain(0xA0..=0xFF) {
            assert_eq!(
                cp1252_char(byte),
                Some(char::from(byte)),
                "byte 0x{byte:02X}"
            );
        }
        assert_eq!(cp1252_char(0xE4), Some('ä'));
        assert_eq!(cp1252_char(0xF6), Some('ö'));
        assert_eq!(cp1252_char(0xFC), Some('ü'));
        assert_eq!(cp1252_char(0xDF), Some('ß'));
        assert_eq!(cp1252_char(0xC4), Some('Ä'));
    }

    #[test]
    fn decoding_all_defined_bytes_yields_one_char_each() {
        let bytes: Vec<u8> = (0u8..=0xFF).filter(|b| !UNDEFINED.contains(b)).collect();
        let text = decode_csv_bytes(&bytes, CsvEncoding::Windows1252).unwrap();
        assert_eq!(text.chars().count(), 251);
    }

    #[test]
    fn each_undefined_byte_is_refused_with_its_offset() {
        for byte in UNDEFINED {
            let bytes = [b'a', b'b', byte, b'c'];
            let error = decode_csv_bytes(&bytes, CsvEncoding::Windows1252).unwrap_err();
            assert!(
                error.contains(&format!("0x{byte:02X}")) && error.contains("offset 2"),
                "{error}"
            );
        }
    }

    #[test]
    fn a_utf8_bom_is_refused_under_windows_1252_only() {
        let bytes = b"\xEF\xBB\xBFAddress,Name\n";
        let error = decode_csv_bytes(bytes, CsvEncoding::Windows1252).unwrap_err();
        assert!(error.contains("byte-order mark"), "{error}");
        assert!(decode_csv_bytes(bytes, CsvEncoding::Utf8).is_ok());
    }

    #[test]
    fn utf8_keeps_refusing_invalid_bytes_with_the_offset() {
        let error = decode_csv_bytes(b"ab\xE4c", CsvEncoding::Utf8).unwrap_err();
        assert!(error.contains("offset 2"), "{error}");
        assert!(matches!(
            decode_csv_bytes(b"plain", CsvEncoding::Utf8),
            Ok(Cow::Borrowed("plain"))
        ));
    }

    #[test]
    fn labels_parse_case_insensitively_and_nothing_else() {
        for encoding in [CsvEncoding::Utf8, CsvEncoding::Windows1252] {
            assert_eq!(CsvEncoding::from_label(encoding.label()), Some(encoding));
        }
        assert_eq!(CsvEncoding::from_label("UTF-8"), Some(CsvEncoding::Utf8));
        assert_eq!(
            CsvEncoding::from_label("Windows-1252"),
            Some(CsvEncoding::Windows1252)
        );
        for other in ["utf8", "cp1252", "latin1", "iso-8859-1", "", "auto"] {
            assert_eq!(CsvEncoding::from_label(other), None, "{other}");
        }
        assert_eq!(CsvEncoding::default(), CsvEncoding::Utf8);
    }
}
