//! The KNX Serial Number: six octets that name one device, whatever its address.
//!
//! `[D]` RES §4.22.1.2, Figure 61, p. 291 (DPT_SerNum 221.001): two octets
//! of manufacturer code, then four octets *"incremented with each BAU"*.
//! AL §3.2.4, p. 21: *"KNX Serial numbers are administered by the KNX
//! Association."* MP §2.4/§2.5 address a device by it, without its
//! programming button.
//!
//! Written as `MMMM:NNNNNNNN` (the manufacturer code, a colon, the rest).
//! Parsing also takes the twelve hex digits without the colon. Anything
//! else is refused rather than padded or truncated: a serial number that is
//! off by one octet names a different device, or none.

use std::fmt;
use std::str::FromStr;

/// A KNX Serial Number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SerialNumber([u8; 6]);

impl SerialNumber {
    /// From the six octets as they travel on the bus.
    pub const fn from_octets(octets: [u8; 6]) -> Self {
        Self(octets)
    }

    /// The six octets as they travel on the bus.
    pub const fn octets(self) -> [u8; 6] {
        self.0
    }

    /// The manufacturer code in the first two octets.
    pub const fn manufacturer(self) -> u16 {
        u16::from_be_bytes([self.0[0], self.0[1]])
    }
}

impl fmt::Display for SerialNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d, e, g] = self.0;
        write!(f, "{a:02X}{b:02X}:{c:02X}{d:02X}{e:02X}{g:02X}")
    }
}

/// Why a text is not a KNX Serial Number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialNumberParseError;

impl fmt::Display for SerialNumberParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a KNX Serial Number is six octets: MMMM:NNNNNNNN or twelve hex digits")
    }
}

impl std::error::Error for SerialNumberParseError {}

impl FromStr for SerialNumber {
    type Err = SerialNumberParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let digits: String = match text.split_once(':') {
            Some((manufacturer, rest)) if manufacturer.len() == 4 && rest.len() == 8 => {
                format!("{manufacturer}{rest}")
            }
            Some(_) => return Err(SerialNumberParseError),
            None => text.to_owned(),
        };
        if digits.len() != 12 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(SerialNumberParseError);
        }
        let mut octets = [0; 6];
        for (index, octet) in octets.iter_mut().enumerate() {
            *octet = u8::from_str_radix(&digits[index * 2..index * 2 + 2], 16)
                .map_err(|_| SerialNumberParseError)?;
        }
        Ok(Self(octets))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_spellings_parse_to_the_same_octets_and_print_with_the_colon() {
        let with: SerialNumber = "0083:12345678".parse().unwrap();
        let without: SerialNumber = "00831234abcd".parse().unwrap();
        assert_eq!(with.octets(), [0x00, 0x83, 0x12, 0x34, 0x56, 0x78]);
        assert_eq!(with.manufacturer(), 0x0083);
        assert_eq!(with.to_string(), "0083:12345678");
        assert_eq!(without.to_string(), "0083:1234ABCD");
    }

    #[test]
    fn a_wrong_length_or_a_stray_character_is_refused_not_padded() {
        for text in [
            "",
            "0083:1234567",
            "0083:123456789",
            "083:12345678",
            "00831234567",
            "0083123456789",
            "0083:1234567G",
            "+083:12345678",
            "0083-12345678",
        ] {
            assert!(text.parse::<SerialNumber>().is_err(), "{text:?}");
        }
    }
}
