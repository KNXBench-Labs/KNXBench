//! The Domain Address of an open medium: two octets on Powerline, six on RF.
//!
//! `[D]` AL §3.3.3, p. 34: *"The Domain Address shall be encoded according
//! to the Domain Address format used on the medium of the communication
//! partner either as a 2 octet value (KNX-PL110) or a 6 octet value
//! (KNX-RF)."* RF v02.03.01 §6.1.1.4: *"The RF Domain Address shall be a 6
//! octet number. It shall be guaranteed during the Configuration procedures
//! that the RF Domain Address is a unique number."*
//!
//! The four- and 21-octet KNX IP forms (AL §3.3.7, Figures 31/32) exist only
//! inside `A_DomainAddressSerialNumber_Write` and the 21-octet one only as
//! KNX Data Security S-A_Data. Neither is modelled here: a PDU carrying them
//! stays an undecoded `Other` frame with its octets intact
//! (KNOWN_LIMITATIONS §143).
//!
//! Written as hex: four digits for PL110, twelve for RF. A text of any other
//! length is refused rather than padded: a domain address that is one octet
//! short names another installation.

use std::fmt;
use std::str::FromStr;

/// A Domain Address, in the format of the medium it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DomainAddress {
    /// KNX-PL110: two octets.
    Powerline(u16),
    /// KNX-RF: six octets.
    Rf([u8; 6]),
}

impl DomainAddress {
    /// The octets as they travel in the PDU, most significant first.
    pub fn octets(&self) -> Vec<u8> {
        match self {
            DomainAddress::Powerline(value) => value.to_be_bytes().to_vec(),
            DomainAddress::Rf(octets) => octets.to_vec(),
        }
    }

    /// From the PDU's octets, by their count: two is PL110, six is RF.
    /// Any other count is not a domain address this model knows.
    pub fn from_octets(octets: &[u8]) -> Option<Self> {
        match octets {
            [hi, lo] => Some(DomainAddress::Powerline(u16::from_be_bytes([*hi, *lo]))),
            [a, b, c, d, e, f] => Some(DomainAddress::Rf([*a, *b, *c, *d, *e, *f])),
            _ => None,
        }
    }

    /// Whether both are of the same medium's format. A device only takes a
    /// domain address in its own format (AL §3.3.3: encoded *"according to
    /// the Domain Address format used on the medium"*).
    pub fn same_format(&self, other: &DomainAddress) -> bool {
        matches!(
            (self, other),
            (DomainAddress::Powerline(_), DomainAddress::Powerline(_))
                | (DomainAddress::Rf(_), DomainAddress::Rf(_))
        )
    }
}

impl fmt::Display for DomainAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for octet in self.octets() {
            write!(f, "{octet:02X}")?;
        }
        Ok(())
    }
}

/// Why a text is not a Domain Address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainAddressParseError;

impl fmt::Display for DomainAddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a Domain Address is four hex digits (KNX-PL110) or twelve hex digits (KNX-RF)")
    }
}

impl std::error::Error for DomainAddressParseError {}

impl FromStr for DomainAddress {
    type Err = DomainAddressParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        if !(text.len() == 4 || text.len() == 12) || !text.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(DomainAddressParseError);
        }
        let octets: Vec<u8> = (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| DomainAddressParseError))
            .collect::<Result<_, _>>()?;
        DomainAddress::from_octets(&octets).ok_or(DomainAddressParseError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_octet_count_names_the_medium() {
        assert_eq!(
            DomainAddress::from_octets(&[0x12, 0x34]),
            Some(DomainAddress::Powerline(0x1234))
        );
        assert_eq!(
            DomainAddress::from_octets(&[1, 2, 3, 4, 5, 6]),
            Some(DomainAddress::Rf([1, 2, 3, 4, 5, 6]))
        );
        // The KNX IP forms (4 and 21 octets) are not modelled, and nothing
        // else is a domain address at all.
        for len in [0, 1, 3, 4, 5, 7, 21] {
            assert_eq!(DomainAddress::from_octets(&vec![0; len]), None, "{len}");
        }
    }

    #[test]
    fn octets_round_trip_most_significant_first() {
        for doa in [
            DomainAddress::Powerline(0xBEEF),
            DomainAddress::Rf([0x00, 0xFA, 0x12, 0x34, 0x56, 0x78]),
        ] {
            assert_eq!(DomainAddress::from_octets(&doa.octets()), Some(doa));
        }
        assert_eq!(DomainAddress::Powerline(0xBEEF).octets(), vec![0xBE, 0xEF]);
    }

    #[test]
    fn text_is_four_or_twelve_hex_digits_and_nothing_else() {
        assert_eq!(
            "00FA12345678".parse(),
            Ok(DomainAddress::Rf([0x00, 0xFA, 0x12, 0x34, 0x56, 0x78]))
        );
        assert_eq!("beef".parse(), Ok(DomainAddress::Powerline(0xBEEF)));
        for bad in [
            "",
            "BEE",
            "00FA1234567",
            "00FA123456789",
            "00FA1234567G",
            "12:34",
        ] {
            assert_eq!(
                bad.parse::<DomainAddress>(),
                Err(DomainAddressParseError),
                "{bad}"
            );
        }
        assert_eq!(
            DomainAddress::Rf([0x00, 0xFA, 0x12, 0x34, 0x56, 0x78]).to_string(),
            "00FA12345678"
        );
    }

    #[test]
    fn a_device_takes_only_its_own_format() {
        let rf = DomainAddress::Rf([0; 6]);
        let pl = DomainAddress::Powerline(0);
        assert!(rf.same_format(&DomainAddress::Rf([1; 6])));
        assert!(!rf.same_format(&pl));
        assert!(!pl.same_format(&rf));
    }
}
