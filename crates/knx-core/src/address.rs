//! Typed KNX addresses: dedicated types, not integers, with pure parsing and
//! formatting so that address handling is exhaustively unit-testable and
//! formatting decisions stay out of the UI (DATA_MODEL §9).

use std::fmt;
use std::str::FromStr;

/// A device's individual address: area (4 bits), line (4 bits), device
/// (8 bits), packed as ETS does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IndividualAddress(u16);

impl IndividualAddress {
    pub fn new(area: u8, line: u8, device: u8) -> Result<Self, AddressError> {
        if area > 15 {
            return Err(AddressError::OutOfRange {
                field: "area",
                value: area as u32,
                max: 15,
            });
        }
        if line > 15 {
            return Err(AddressError::OutOfRange {
                field: "line",
                value: line as u32,
                max: 15,
            });
        }
        Ok(Self::from_raw(
            ((area as u16) << 12) | ((line as u16) << 8) | device as u16,
        ))
    }

    pub fn from_raw(raw: u16) -> Self {
        Self(raw)
    }

    pub fn raw(self) -> u16 {
        self.0
    }

    pub fn area(self) -> u8 {
        (self.0 >> 12) as u8 & 0x0F
    }

    pub fn line(self) -> u8 {
        (self.0 >> 8) as u8 & 0x0F
    }

    pub fn device(self) -> u8 {
        (self.0 & 0xFF) as u8
    }
}

impl fmt::Display for IndividualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.area(), self.line(), self.device())
    }
}

impl FromStr for IndividualAddress {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        let [area, line, device] = parts[..] else {
            return Err(AddressError::MalformedIndividual(s.to_string()));
        };
        let parse_part = |p: &str| {
            p.parse::<u8>()
                .map_err(|_| AddressError::MalformedIndividual(s.to_string()))
        };
        IndividualAddress::new(parse_part(area)?, parse_part(line)?, parse_part(device)?)
    }
}

/// How a project renders group addresses. Chosen project-wide; the raw
/// 16-bit value never changes, only its textual representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupAddressStyle {
    /// Plain decimal, 0–65535.
    Free,
    /// `main/sub`, main 0–31 (5 bits), sub 0–2047 (11 bits).
    TwoLevel,
    /// `main/middle/sub`, main 0–31 (5 bits), middle 0–7 (3 bits), sub 0–255
    /// (8 bits).
    ThreeLevel,
}

/// A group address's raw 16-bit value. Rendered according to the project's
/// `GroupAddressStyle`, never carries a style of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GroupAddress(u16);

impl GroupAddress {
    pub fn from_raw(raw: u16) -> Self {
        Self(raw)
    }

    pub fn raw(self) -> u16 {
        self.0
    }

    pub fn parse(s: &str, style: GroupAddressStyle) -> Result<Self, AddressError> {
        let malformed = || AddressError::MalformedGroup(s.to_string());
        let raw = match style {
            GroupAddressStyle::Free => s.parse::<u16>().map_err(|_| malformed())?,
            GroupAddressStyle::TwoLevel => {
                let [main, sub] = s.split('/').collect::<Vec<_>>()[..] else {
                    return Err(malformed());
                };
                let main: u16 = main.parse().map_err(|_| malformed())?;
                let sub: u16 = sub.parse().map_err(|_| malformed())?;
                if main > 31 {
                    return Err(AddressError::OutOfRange {
                        field: "main",
                        value: main as u32,
                        max: 31,
                    });
                }
                if sub > 2047 {
                    return Err(AddressError::OutOfRange {
                        field: "sub",
                        value: sub as u32,
                        max: 2047,
                    });
                }
                (main << 11) | sub
            }
            GroupAddressStyle::ThreeLevel => {
                let [main, middle, sub] = s.split('/').collect::<Vec<_>>()[..] else {
                    return Err(malformed());
                };
                let main: u16 = main.parse().map_err(|_| malformed())?;
                let middle: u16 = middle.parse().map_err(|_| malformed())?;
                let sub: u16 = sub.parse().map_err(|_| malformed())?;
                if main > 31 {
                    return Err(AddressError::OutOfRange {
                        field: "main",
                        value: main as u32,
                        max: 31,
                    });
                }
                if middle > 7 {
                    return Err(AddressError::OutOfRange {
                        field: "middle",
                        value: middle as u32,
                        max: 7,
                    });
                }
                if sub > 255 {
                    return Err(AddressError::OutOfRange {
                        field: "sub",
                        value: sub as u32,
                        max: 255,
                    });
                }
                (main << 11) | (middle << 8) | sub
            }
        };
        Ok(Self(raw))
    }

    pub fn format(self, style: GroupAddressStyle) -> String {
        match style {
            GroupAddressStyle::Free => self.0.to_string(),
            GroupAddressStyle::TwoLevel => {
                let main = self.0 >> 11;
                let sub = self.0 & 0x07FF;
                format!("{main}/{sub}")
            }
            GroupAddressStyle::ThreeLevel => {
                let main = self.0 >> 11;
                let middle = (self.0 >> 8) & 0x07;
                let sub = self.0 & 0xFF;
                format!("{main}/{middle}/{sub}")
            }
        }
    }
}

/// A typed address failed to parse or fell outside its valid range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressError {
    OutOfRange {
        field: &'static str,
        value: u32,
        max: u32,
    },
    MalformedIndividual(String),
    MalformedGroup(String),
}

impl std::error::Error for AddressError {}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressError::OutOfRange { field, value, max } => {
                write!(f, "{field} {value} exceeds maximum {max}")
            }
            AddressError::MalformedIndividual(s) => {
                write!(f, "malformed individual address: {s:?}")
            }
            AddressError::MalformedGroup(s) => write!(f, "malformed group address: {s:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn individual_address_roundtrips_through_display_and_parse() {
        let a = IndividualAddress::new(1, 2, 3).unwrap();
        assert_eq!(a.to_string(), "1.2.3");
        assert_eq!("1.2.3".parse::<IndividualAddress>().unwrap(), a);
    }

    #[test]
    fn individual_address_rejects_area_above_15() {
        assert!(matches!(
            IndividualAddress::new(16, 0, 0),
            Err(AddressError::OutOfRange { field: "area", .. })
        ));
    }

    #[test]
    fn group_address_free_style_is_plain_decimal() {
        let ga = GroupAddress::parse("1234", GroupAddressStyle::Free).unwrap();
        assert_eq!(ga.raw(), 1234);
        assert_eq!(ga.format(GroupAddressStyle::Free), "1234");
    }

    #[test]
    fn group_address_three_level_roundtrips() {
        let ga = GroupAddress::parse("4/2/100", GroupAddressStyle::ThreeLevel).unwrap();
        assert_eq!(ga.format(GroupAddressStyle::ThreeLevel), "4/2/100");
        assert_eq!(ga.raw(), (4u16 << 11) | (2u16 << 8) | 100u16);
    }

    #[test]
    fn group_address_two_level_roundtrips() {
        let ga = GroupAddress::parse("4/612", GroupAddressStyle::TwoLevel).unwrap();
        assert_eq!(ga.format(GroupAddressStyle::TwoLevel), "4/612");
        assert_eq!(ga.raw(), (4u16 << 11) | 612u16);
    }

    #[test]
    fn group_address_rejects_out_of_range_middle() {
        assert!(GroupAddress::parse("1/8/1", GroupAddressStyle::ThreeLevel).is_err());
    }
}
