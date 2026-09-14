//! Typed KNX addresses: dedicated types, not integers, with pure parsing and
//! formatting so that address handling is exhaustively unit-testable and
//! formatting decisions stay out of the UI (DATA_MODEL §9).

use std::fmt;
use std::str::FromStr;

/// A device's individual address: area (4 bits), line (4 bits), device
/// (8 bits), packed as ETS does.
///
/// Ordered by the packed value, which is area-then-line-then-device, so a
/// sorted list of addresses reads topologically. `GroupAddress` below is
/// ordered the same way and for the same reason: deterministic output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

    pub const fn from_raw(raw: u16) -> Self {
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

/// Individual addresses this installation forbids all bus traffic to,
/// in one place, shared by every layer that can reach a bus.
///
/// `1.1.220` is a live alarm panel on the installation this code will
/// eventually run against. Spec §2.1 (R-SAFE-1) requires that it "must
/// never be read, never be written, never be probed, and never appear
/// inside any address range, scan plan, iteration, retry list or
/// diagnostic sweep", and that the guard sit at "the lowest layer that
/// knows what an individual address is" rather than being re-checked per
/// caller. This constant is that one place; [`ContactableAddress`] is
/// that guard.
///
/// Deliberately not a caller-supplied argument: a caller that can pass
/// the list can pass a list with the alarm panel missing from it.
pub const EXCLUDED_INDIVIDUAL_ADDRESSES: &[IndividualAddress] =
    &[IndividualAddress::from_raw(0x11DC)];

/// Whether `address` is on the project exclusion list of
/// [`EXCLUDED_INDIVIDUAL_ADDRESSES`].
pub fn is_project_excluded(address: IndividualAddress) -> bool {
    EXCLUDED_INDIVIDUAL_ADDRESSES.contains(&address)
}

/// An individual address that has passed the project exclusion guard,
/// and the only way to name a bus target in the commissioning layers.
///
/// Constructing one is the check: there is no field access, no `Default`,
/// no `From<IndividualAddress>` and no unchecked constructor outside
/// `test-support`, so "we forgot to check" is not a reachable state. A
/// function that takes a `ContactableAddress` cannot be handed an
/// excluded address at all, which is spec §2.1's "every higher layer
/// inherits the refusal instead of repeating it".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContactableAddress(IndividualAddress);

impl ContactableAddress {
    /// Admits `address` unless it is on the project exclusion list. The
    /// refusal carries the address, because a silent refusal and an
    /// absent guard look identical from outside (§2.1).
    pub fn new(address: IndividualAddress) -> Result<Self, ExcludedAddress> {
        if is_project_excluded(address) {
            return Err(ExcludedAddress(address));
        }
        Ok(Self(address))
    }

    /// The address, once the guard has admitted it.
    pub fn address(self) -> IndividualAddress {
        self.0
    }
}

impl fmt::Display for ContactableAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// An operation named an address on the project exclusion list and was
/// refused. Reported, never swallowed: per `CLAUDE.md`'s "never silently
/// discard information", the caller learns which address was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExcludedAddress(pub IndividualAddress);

impl std::error::Error for ExcludedAddress {}

impl fmt::Display for ExcludedAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "individual address {} is on the project exclusion list and \
             must never be contacted",
            self.0
        )
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

    #[test]
    fn the_alarm_panel_is_on_the_shared_exclusion_list() {
        // §14 item 11, the guard half: the list is a constant in one
        // place, not an argument a caller can forget to pass.
        let alarm_panel = IndividualAddress::new(1, 1, 220).unwrap();
        assert!(is_project_excluded(alarm_panel));
        assert_eq!(EXCLUDED_INDIVIDUAL_ADDRESSES, &[alarm_panel]);
    }

    #[test]
    fn an_excluded_address_cannot_become_contactable_and_the_refusal_names_it() {
        let alarm_panel = IndividualAddress::new(1, 1, 220).unwrap();
        let err = ContactableAddress::new(alarm_panel).unwrap_err();
        assert_eq!(err, ExcludedAddress(alarm_panel));
        // Reported, not silent: the message names the address, so a
        // refusal is distinguishable from a guard that never ran.
        assert!(err.to_string().contains("1.1.220"));
    }

    #[test]
    fn a_neighbour_of_the_excluded_address_is_contactable() {
        // The guard excludes exactly one address, not a neighbourhood:
        // 1.1.219 and 1.1.221 must stay reachable or the guard is wrong
        // in the other direction.
        for device in [219u8, 221] {
            let address = IndividualAddress::new(1, 1, device).unwrap();
            assert_eq!(
                ContactableAddress::new(address).map(ContactableAddress::address),
                Ok(address)
            );
        }
    }
}
