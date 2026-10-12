//! Which octet order a device's memory holds its parameter values in.
//!
//! KL-170: a program may declare `Options/@ParameterByteOrder="LittleEndian"`,
//! and no source available to KNXBench defines what that means per parameter
//! type (docs/research/parameter-byte-order.md). [`super::parameter_image`]
//! writes high octet first only, and the image builder refuses every other
//! declared order. This module answers the question a device can settle: for
//! a device that ETS programmed with known values, which arrangement of each
//! value do its memory octets hold?
//!
//! It only *reads*. Nothing here encodes a value for a download, and nothing
//! here may feed [`super::parameter_image`]: a reading is evidence for a
//! reviewed decision, not a write rule.
//!
//! Only two arrangements are tested, and only where they are well defined:
//!
//! - A field of whole octets (bit offset 0, a multiple of 8 bits, at least
//!   16) holds its value either most significant octet first
//!   ([`OctetOrder::HighFirst`]) or least significant octet first
//!   ([`OctetOrder::LowFirst`]). A value whose octets read the same both
//!   ways (`0`, `0101h`) says nothing ([`FieldReading::EitherOrder`]).
//! - A field inside one octet has no octet order at all. It is read MSB-first
//!   at its bit offset (*Project Schema23*, §1.1.3.17 `BitOffset_t`), and only
//!   says whether device and project agree ([`FieldReading::SingleOctet`]):
//!   the control that tells a device holding other values apart from one
//!   holding the same values in another order.
//!
//! A field that crosses an octet boundary without being whole octets has no
//! defined little-endian arrangement anywhere (KL-170), so it is not
//! classified ([`FieldReading::NotClassified`]); guessing one would be the
//! "octet swap on suspicion" KL-170 forbids.

use std::fmt;

use super::parameter_image::ParameterField;

/// The two arrangements of a multi-octet value in memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OctetOrder {
    /// The most significant octet at the lowest address ("big endian").
    HighFirst,
    /// The least significant octet at the lowest address ("little endian").
    LowFirst,
}

impl fmt::Display for OctetOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            OctetOrder::HighFirst => "high octet first",
            OctetOrder::LowFirst => "low octet first",
        })
    }
}

/// What one field's octets in device memory say about the project's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldReading {
    /// Whole octets, and the value reads differently in the two orders: the
    /// device holds it in this one.
    Holds(OctetOrder),
    /// Whole octets, but the value's octets are the same in both orders.
    EitherOrder,
    /// Whole octets, and the device holds neither arrangement: device and
    /// project disagree about the value, or the field is elsewhere.
    Neither,
    /// A field inside one octet: whether the device holds the value there.
    SingleOctet {
        /// Device and project agree.
        agrees: bool,
    },
    /// Crosses an octet boundary without being whole octets: no arrangement
    /// is defined, so none is tested.
    NotClassified,
}

/// Why a field could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadingError {
    /// The octets given are not the octets the field covers.
    WrongLength {
        /// The field.
        field: ParameterField,
        /// The octets it covers.
        expected: usize,
        /// The octets given.
        got: usize,
    },
    /// A shape no parameter has: zero or more than 64 bits, or a bit offset
    /// above 7.
    UnsupportedField {
        /// The field.
        field: ParameterField,
    },
    /// The value needs more bits than the field has.
    ValueTooWide {
        /// The field.
        field: ParameterField,
        /// The value.
        value: u64,
    },
}

impl std::error::Error for ReadingError {}

impl fmt::Display for ReadingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadingError::WrongLength {
                field,
                expected,
                got,
            } => write!(
                f,
                "the field at offset {} ({} bits) covers {expected} octets, {got} were given",
                field.offset, field.size_in_bit
            ),
            ReadingError::UnsupportedField { field } => write!(
                f,
                "no parameter has {} bits at bit offset {}",
                field.size_in_bit, field.bit_offset
            ),
            ReadingError::ValueTooWide { field, value } => {
                write!(f, "value {value} does not fit {} bits", field.size_in_bit)
            }
        }
    }
}

/// The number of octets `field` touches, from the octet at its offset.
pub fn covered_octets(field: ParameterField) -> usize {
    (u32::from(field.bit_offset) + field.size_in_bit).div_ceil(8) as usize
}

/// Reads `field` from `held`, the device's octets the field covers (from
/// the octet at `field.offset`, [`covered_octets`] long), against `value`,
/// the value the project says the device was programmed with.
pub fn read_field(
    field: ParameterField,
    value: u64,
    held: &[u8],
) -> Result<FieldReading, ReadingError> {
    if field.bit_offset > 7 || !(1..=64).contains(&field.size_in_bit) {
        return Err(ReadingError::UnsupportedField { field });
    }
    if field.size_in_bit < 64 && value >> field.size_in_bit != 0 {
        return Err(ReadingError::ValueTooWide { field, value });
    }
    let expected = covered_octets(field);
    if held.len() != expected {
        return Err(ReadingError::WrongLength {
            field,
            expected,
            got: held.len(),
        });
    }
    if expected == 1 {
        let shift = 8 - u32::from(field.bit_offset) - field.size_in_bit;
        let mask = ((1u16 << field.size_in_bit) - 1) as u8;
        let found = (held[0] >> shift) & mask;
        return Ok(FieldReading::SingleOctet {
            agrees: u64::from(found) == value,
        });
    }
    if field.bit_offset != 0 || !field.size_in_bit.is_multiple_of(8) {
        return Ok(FieldReading::NotClassified);
    }
    let high_first = &value.to_be_bytes()[8 - expected..];
    let low_first = &value.to_le_bytes()[..expected];
    Ok(if high_first == low_first {
        FieldReading::EitherOrder
    } else if held == high_first {
        FieldReading::Holds(OctetOrder::HighFirst)
    } else if held == low_first {
        FieldReading::Holds(OctetOrder::LowFirst)
    } else {
        FieldReading::Neither
    })
}

/// The readings of one device, counted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    /// [`FieldReading::Holds`] high octet first.
    pub high_first: usize,
    /// [`FieldReading::Holds`] low octet first.
    pub low_first: usize,
    /// [`FieldReading::EitherOrder`].
    pub either_order: usize,
    /// [`FieldReading::Neither`].
    pub neither: usize,
    /// [`FieldReading::SingleOctet`] that agree.
    pub single_octet_agree: usize,
    /// [`FieldReading::SingleOctet`] that disagree.
    pub single_octet_disagree: usize,
    /// [`FieldReading::NotClassified`].
    pub not_classified: usize,
}

impl Tally {
    /// Counts `readings`.
    pub fn of<'a>(readings: impl IntoIterator<Item = &'a FieldReading>) -> Tally {
        let mut tally = Tally::default();
        for reading in readings {
            match reading {
                FieldReading::Holds(OctetOrder::HighFirst) => tally.high_first += 1,
                FieldReading::Holds(OctetOrder::LowFirst) => tally.low_first += 1,
                FieldReading::EitherOrder => tally.either_order += 1,
                FieldReading::Neither => tally.neither += 1,
                FieldReading::SingleOctet { agrees: true } => tally.single_octet_agree += 1,
                FieldReading::SingleOctet { agrees: false } => tally.single_octet_disagree += 1,
                FieldReading::NotClassified => tally.not_classified += 1,
            }
        }
        tally
    }

    /// What the fields that can tell the orders apart say, and nothing more.
    /// The disagreeing counts ([`Self::neither`],
    /// [`Self::single_octet_disagree`]) are not folded in: they measure how
    /// far the project describes the device at all, and a reader weighs them
    /// next to the verdict.
    pub fn verdict(&self) -> Verdict {
        match (self.high_first, self.low_first) {
            (0, 0) => Verdict::Inconclusive,
            (_, 0) => Verdict::Consistent(OctetOrder::HighFirst),
            (0, _) => Verdict::Consistent(OctetOrder::LowFirst),
            _ => Verdict::Contradictory,
        }
    }
}

/// What the discriminating fields of one device say together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Every discriminating field holds its value in this order.
    Consistent(OctetOrder),
    /// Some fields hold one order and some the other.
    Contradictory,
    /// No field told the orders apart.
    Inconclusive,
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Verdict::Consistent(order) => {
                write!(f, "every discriminating field holds its value {order}")
            }
            Verdict::Contradictory => f.write_str(
                "contradictory: some fields hold their value high octet first, others low octet first",
            ),
            Verdict::Inconclusive => f.write_str("inconclusive: no field told the two orders apart"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(offset: u32, bit_offset: u8, size_in_bit: u32) -> ParameterField {
        ParameterField {
            offset,
            bit_offset,
            size_in_bit,
        }
    }

    #[test]
    fn a_two_octet_value_is_read_in_either_order() {
        let f = field(4, 0, 16);
        assert_eq!(
            read_field(f, 300, &[0x01, 0x2C]),
            Ok(FieldReading::Holds(OctetOrder::HighFirst))
        );
        assert_eq!(
            read_field(f, 300, &[0x2C, 0x01]),
            Ok(FieldReading::Holds(OctetOrder::LowFirst))
        );
        assert_eq!(read_field(f, 300, &[0x2C, 0x02]), Ok(FieldReading::Neither));
    }

    #[test]
    fn wider_fields_reverse_every_octet_not_just_halves() {
        let f = field(0, 0, 32);
        assert_eq!(
            read_field(f, 0x0102_0304, &[1, 2, 3, 4]),
            Ok(FieldReading::Holds(OctetOrder::HighFirst))
        );
        assert_eq!(
            read_field(f, 0x0102_0304, &[4, 3, 2, 1]),
            Ok(FieldReading::Holds(OctetOrder::LowFirst))
        );
        // Swapped 16-bit halves ("word swap") is neither order.
        assert_eq!(
            read_field(f, 0x0102_0304, &[3, 4, 1, 2]),
            Ok(FieldReading::Neither)
        );
        let f24 = field(0, 0, 24);
        assert_eq!(
            read_field(f24, 0x0A_0B0C, &[0x0C, 0x0B, 0x0A]),
            Ok(FieldReading::Holds(OctetOrder::LowFirst))
        );
        let f64 = field(0, 0, 64);
        assert_eq!(
            read_field(
                f64,
                u64::MAX - 1,
                &[0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
            ),
            Ok(FieldReading::Holds(OctetOrder::LowFirst))
        );
    }

    #[test]
    fn a_palindromic_value_tells_nothing() {
        let f = field(0, 0, 16);
        assert_eq!(read_field(f, 0, &[0, 0]), Ok(FieldReading::EitherOrder));
        assert_eq!(
            read_field(f, 0x0101, &[1, 1]),
            Ok(FieldReading::EitherOrder)
        );
        // Even when the device holds something else: no order is told apart,
        // and the mismatch is not hidden as a verdict either.
        assert_eq!(read_field(f, 0, &[0, 7]), Ok(FieldReading::EitherOrder));
        let f32 = field(0, 0, 32);
        assert_eq!(
            read_field(f32, 0x0102_0201, &[1, 2, 2, 1]),
            Ok(FieldReading::EitherOrder)
        );
    }

    #[test]
    fn a_field_inside_one_octet_is_a_control_read_msb_first() {
        // 3 bits at bit offset 2: bits 5..3 of the octet (MSB = bit 7).
        let f = field(0, 2, 3);
        assert_eq!(
            read_field(f, 0b101, &[0b0010_1000]),
            Ok(FieldReading::SingleOctet { agrees: true })
        );
        assert_eq!(
            read_field(f, 0b101, &[0b0000_0101]),
            Ok(FieldReading::SingleOctet { agrees: false })
        );
        // Neighbouring bits do not matter.
        assert_eq!(
            read_field(f, 0b101, &[0b1110_1111]),
            Ok(FieldReading::SingleOctet { agrees: true })
        );
        let whole = field(9, 0, 8);
        assert_eq!(
            read_field(whole, 0xA5, &[0xA5]),
            Ok(FieldReading::SingleOctet { agrees: true })
        );
        let last_bit = field(0, 7, 1);
        assert_eq!(
            read_field(last_bit, 1, &[0x01]),
            Ok(FieldReading::SingleOctet { agrees: true })
        );
    }

    #[test]
    fn a_field_across_an_octet_boundary_is_not_classified() {
        // 12 bits at bit offset 0, and 8 bits at bit offset 4: KL-170 has no
        // little-endian reading for either, so no hypothesis is tested.
        assert_eq!(
            read_field(field(0, 0, 12), 0xABC, &[0xAB, 0xC0]),
            Ok(FieldReading::NotClassified)
        );
        assert_eq!(
            read_field(field(0, 4, 8), 0xAB, &[0x0A, 0xB0]),
            Ok(FieldReading::NotClassified)
        );
        assert_eq!(
            read_field(field(0, 1, 16), 1, &[0, 0, 0]),
            Ok(FieldReading::NotClassified)
        );
    }

    #[test]
    fn malformed_input_is_refused_not_classified() {
        assert!(matches!(
            read_field(field(0, 0, 16), 1, &[0]),
            Err(ReadingError::WrongLength {
                expected: 2,
                got: 1,
                ..
            })
        ));
        assert!(matches!(
            read_field(field(0, 0, 16), 0x1_0000, &[0, 0]),
            Err(ReadingError::ValueTooWide { .. })
        ));
        assert!(matches!(
            read_field(field(0, 8, 1), 0, &[0]),
            Err(ReadingError::UnsupportedField { .. })
        ));
        assert!(matches!(
            read_field(field(0, 0, 0), 0, &[]),
            Err(ReadingError::UnsupportedField { .. })
        ));
        assert!(matches!(
            read_field(field(0, 0, 65), 0, &[0; 9]),
            Err(ReadingError::UnsupportedField { .. })
        ));
    }

    #[test]
    fn the_verdict_counts_only_discriminating_fields() {
        let low = FieldReading::Holds(OctetOrder::LowFirst);
        let high = FieldReading::Holds(OctetOrder::HighFirst);
        let controls = [
            FieldReading::EitherOrder,
            FieldReading::Neither,
            FieldReading::SingleOctet { agrees: false },
            FieldReading::SingleOctet { agrees: true },
            FieldReading::NotClassified,
        ];
        assert_eq!(Tally::of(&controls).verdict(), Verdict::Inconclusive);
        let mut readings = controls.to_vec();
        readings.push(low);
        let tally = Tally::of(&readings);
        assert_eq!(tally.verdict(), Verdict::Consistent(OctetOrder::LowFirst));
        assert_eq!(
            tally,
            Tally {
                high_first: 0,
                low_first: 1,
                either_order: 1,
                neither: 1,
                single_octet_agree: 1,
                single_octet_disagree: 1,
                not_classified: 1,
            }
        );
        assert_eq!(
            Tally::of(&[high, high]).verdict(),
            Verdict::Consistent(OctetOrder::HighFirst)
        );
        assert_eq!(Tally::of(&[high, low]).verdict(), Verdict::Contradictory);
    }

    #[test]
    fn covered_octets_counts_partial_octets() {
        assert_eq!(covered_octets(field(0, 0, 1)), 1);
        assert_eq!(covered_octets(field(0, 7, 1)), 1);
        assert_eq!(covered_octets(field(0, 7, 2)), 2);
        assert_eq!(covered_octets(field(0, 0, 16)), 2);
        assert_eq!(covered_octets(field(0, 4, 16)), 3);
    }
}
