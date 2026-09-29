//! Writing parameter values into a segment's memory image.
//!
//! An application program places each parameter at a `Memory` location:
//! a code segment, an octet `Offset` and a `BitOffset`. A download writes the
//! segment's image, which is its base data with the configured parameter
//! values laid over it. This module does that laying-over, and nothing else.
//! Which parameters to write, and with which values, is the caller's
//! business.
//!
//! `[D]` *Project Schema23 v01.00.00.pdf*, §1.1.3.17 `BitOffset_t`
//! (pp. 29–30): *"The bit offset is the distance of the most significant bit
//! of the parameter from the most significant bit of the first octet in
//! memory"*, range 0–7.
//!
//! `[V]` Multi-octet values are written high octet first. That PDF does not
//! define `ParameterByteOrder`, and the MDT product file does not declare
//! it either: its `Options` carries only one legacy flag. Only 26 of 310
//! corpus product files declare the attribute, all `BigEndian`. ETS
//! materialises it as `BigEndian` in all 20 mask-`0701h` `Options` of the
//! corpus projects. A real device (`1.1.67`, `A-0027-15-0BAC`) settles it:
//! its enumerated 16-bit parameters read back `00 32` (50 ms) and `01 90`
//! (0.4 s), and a low-octet-first reading would give values the
//! enumerations do not allow (docs/RESEARCH.md §19.1).
//! A caller must refuse an application that declares another order. This
//! module has no little-endian mode, because no source says how a bit
//! offset combines with it.
//!
//! Supported fields are 1 to 64 bits at a bit offset of 0–7, laid out as
//! the definition above states: the value's most significant bit sits
//! `bit_offset` bits below the first octet's MSB, and every further bit
//! follows MSB-first, continuing into the next octet's MSB when one octet
//! is full. That covers a field inside one octet, whole octets, and a field
//! that crosses an octet boundary alike.
//!
//! `[D]` A field across an octet boundary is written that way because the
//! KNX Standard numbers bit offsets on through consecutive octets, MSB
//! first: *Configuration Procedures* `03_05_03` v02.01.01 §8.5.4
//! (pp. 197–198) — *"The position of the parameters inside the parameter
//! block are not restricted to the boundary of the parameters itself"*,
//! with a two-octet block whose offsets run `0…7` through octet 0 and
//! `8…15` through octet 1 — and *Resources* `03_05_01` v01.10.01
//! §4.18.5.2.5 `PID_EXT_GRPOBJREFERENCE` (p. 268): *"Bit offset shall start from "left" / MSB"*, where
//! bit 0 of the third octet of a `U16B8` is *"Bit Offset = 23"*. A field
//! of the product's `Memory` is the same bit string; no PDF shows one that
//! crosses a boundary, and no device has confirmed it (RESEARCH §19.11).
//!
//! Wider than 64 bits, zero width, and a bit offset above 7 are refused.

use std::fmt;

/// Where one parameter lives in its segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParameterField {
    /// Octet offset from the start of the segment.
    pub offset: u32,
    /// Distance of the value's most significant bit from the most
    /// significant bit of the octet at `offset`, 0–7.
    pub bit_offset: u8,
    /// Width of the value in bits. An integer field has at most 64; an
    /// octet string ([`ParameterImage::write_octets`]) may be wider.
    pub size_in_bit: u32,
}

/// Why a value could not be written. The image is left unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterImageError {
    /// A field shape this module does not write (see the module
    /// documentation).
    UnsupportedField {
        /// The field.
        field: ParameterField,
    },
    /// The field reaches past the end of the segment.
    OutOfSegment {
        /// The field.
        field: ParameterField,
        /// The segment's length in octets.
        segment_octets: usize,
    },
    /// The value needs more bits than the field has.
    ValueTooWide {
        /// The field.
        field: ParameterField,
        /// The value.
        value: u64,
    },
    /// A signed value outside the field's two's-complement range.
    SignedValueOutOfRange {
        /// The width.
        size_in_bit: u8,
        /// The value.
        value: i64,
    },
    /// The field shares bits with one written before. Two parameters that
    /// are both written must not overlap; overlapping `Union` members are
    /// alternatives, and only the active one may be written.
    Overlap {
        /// The field.
        field: ParameterField,
        /// The field it overlaps.
        earlier: ParameterField,
    },
}

impl std::error::Error for ParameterImageError {}

impl fmt::Display for ParameterImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterImageError::UnsupportedField { field } => write!(
                f,
                "unsupported parameter field: {} bits at offset {} bit {}",
                field.size_in_bit, field.offset, field.bit_offset
            ),
            ParameterImageError::OutOfSegment {
                field,
                segment_octets,
            } => write!(
                f,
                "parameter field at offset {} ({} bits) reaches past the {segment_octets}-octet segment",
                field.offset, field.size_in_bit
            ),
            ParameterImageError::ValueTooWide { field, value } => write!(
                f,
                "value {value} does not fit {} bits at offset {}",
                field.size_in_bit, field.offset
            ),
            ParameterImageError::SignedValueOutOfRange { size_in_bit, value } => {
                write!(f, "signed value {value} does not fit {size_in_bit} bits")
            }
            ParameterImageError::Overlap { field, earlier } => write!(
                f,
                "parameter field at offset {} bit {} overlaps the one at offset {} bit {}",
                field.offset, field.bit_offset, earlier.offset, earlier.bit_offset
            ),
        }
    }
}

/// A segment image being filled with parameter values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterImage {
    octets: Vec<u8>,
    written: Vec<ParameterField>,
}

impl ParameterField {
    /// The bits the field covers, counted MSB-first from the start of the
    /// segment.
    fn span(self) -> (u64, u64) {
        let start = u64::from(self.offset) * 8 + u64::from(self.bit_offset);
        (start, start + u64::from(self.size_in_bit))
    }

    /// Whether [`ParameterImage::write`] supports the shape: 1 to 64 bits
    /// at a bit offset of 0–7.
    fn is_integer_shape(self) -> bool {
        self.bit_offset <= 7 && (1..=64).contains(&self.size_in_bit)
    }

    /// The octets the field touches.
    fn octet_len(self) -> usize {
        (u32::from(self.bit_offset) + self.size_in_bit).div_ceil(8) as usize
    }
}

impl ParameterImage {
    /// Starts from the segment's base data.
    pub fn new(base: Vec<u8>) -> ParameterImage {
        ParameterImage {
            octets: base,
            written: Vec::new(),
        }
    }

    /// Checks what every write needs: the field lies in the segment and
    /// shares no bit with a field written before.
    fn check_free(&self, field: ParameterField) -> Result<(), ParameterImageError> {
        if field.offset as usize + field.octet_len() > self.octets.len() {
            return Err(ParameterImageError::OutOfSegment {
                field,
                segment_octets: self.octets.len(),
            });
        }
        let (start, end) = field.span();
        if let Some(earlier) = self.written.iter().copied().find(|earlier| {
            let (s, e) = earlier.span();
            s < end && start < e
        }) {
            return Err(ParameterImageError::Overlap { field, earlier });
        }
        Ok(())
    }

    /// Writes `value` into `field`. Refuses, and leaves the image
    /// unchanged, if the field is unsupported, out of the segment, too
    /// narrow for the value, or overlaps a field written before.
    pub fn write(&mut self, field: ParameterField, value: u64) -> Result<(), ParameterImageError> {
        if !field.is_integer_shape() {
            return Err(ParameterImageError::UnsupportedField { field });
        }
        self.check_free(field)?;
        if field.size_in_bit < 64 && value >> field.size_in_bit != 0 {
            return Err(ParameterImageError::ValueTooWide { field, value });
        }

        let first = field.offset as usize;
        // Bit `i` of the field (0 = its most significant) is segment bit
        // `bit_offset + i`, counted MSB-first from the octet at `offset`.
        let size = field.size_in_bit;
        for i in 0..size {
            let bit = (value >> (size - 1 - i)) & 1;
            let at = u32::from(field.bit_offset) + i;
            let octet = &mut self.octets[first + (at / 8) as usize];
            let mask = 0x80u8 >> (at % 8);
            *octet = if bit == 1 {
                *octet | mask
            } else {
                *octet & !mask
            };
        }
        self.written.push(field);
        Ok(())
    }

    /// Copies `octets` into `field`, first octet first: a value that is an
    /// octet string, not a number (a text parameter). The field must start
    /// on an octet and be exactly as long as `octets`; how the value became
    /// these octets is the caller's business. Refuses, and leaves the image
    /// unchanged, like [`Self::write`].
    pub fn write_octets(
        &mut self,
        field: ParameterField,
        octets: &[u8],
    ) -> Result<(), ParameterImageError> {
        if field.bit_offset != 0
            || field.size_in_bit == 0
            || field.size_in_bit as usize != octets.len() * 8
        {
            return Err(ParameterImageError::UnsupportedField { field });
        }
        self.check_free(field)?;
        let first = field.offset as usize;
        self.octets[first..first + octets.len()].copy_from_slice(octets);
        self.written.push(field);
        Ok(())
    }

    /// The image.
    pub fn octets(&self) -> &[u8] {
        &self.octets
    }

    /// Consumes the image and returns its octets.
    pub fn into_octets(self) -> Vec<u8> {
        self.octets
    }
}

/// The two's-complement bit pattern of `value` in `size_in_bit` bits, for a
/// signed parameter.
pub fn signed_bits(value: i64, size_in_bit: u8) -> Result<u64, ParameterImageError> {
    let out_of_range = ParameterImageError::SignedValueOutOfRange { size_in_bit, value };
    match size_in_bit {
        0 | 65.. => Err(out_of_range),
        64 => Ok(value as u64),
        width => {
            let min = -(1i64 << (width - 1));
            let max = (1i64 << (width - 1)) - 1;
            if value < min || value > max {
                return Err(out_of_range);
            }
            Ok((value as u64) & ((1u64 << width) - 1))
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
    fn the_base_data_is_kept_where_nothing_is_written() {
        let image = ParameterImage::new(vec![0xAA, 0x55, 0x0F]);
        assert_eq!(image.octets(), &[0xAA, 0x55, 0x0F]);
    }

    /// The MDT push button's *Subfunction* of button 1 (`UP-5501`) is 16
    /// bits at offset 266 of `AS-4400`; `1 = Toggle by push`.
    #[test]
    fn a_whole_octet_value_is_written_high_octet_first() {
        let mut image = ParameterImage::new(vec![0; 4]);
        image.write(field(1, 0, 16), 0x0102).unwrap();
        assert_eq!(image.octets(), &[0x00, 0x01, 0x02, 0x00]);
        image.write(field(3, 0, 8), 0xFE).unwrap();
        assert_eq!(image.into_octets(), vec![0x00, 0x01, 0x02, 0xFE]);
    }

    #[test]
    fn a_32_bit_value_is_written_high_octet_first() {
        let mut image = ParameterImage::new(vec![0; 4]);
        image.write(field(0, 0, 32), 0x1234_5678).unwrap();
        assert_eq!(image.octets(), &[0x12, 0x34, 0x56, 0x78]);
    }

    /// BitOffset counts from the octet's most significant bit, and names
    /// the value's most significant bit.
    #[test]
    fn a_sub_octet_value_is_placed_from_the_msb_and_neighbours_survive() {
        let mut image = ParameterImage::new(vec![0b1000_0001]);
        // 1 bit at bit offset 5: the octet's 0b0000_0100.
        image.write(field(0, 5, 1), 1).unwrap();
        assert_eq!(image.octets(), &[0b1000_0101]);
        // 3 bits at bit offset 1: 0b0111_0000.
        image.write(field(0, 1, 3), 0b101).unwrap();
        assert_eq!(image.octets(), &[0b1101_0101]);
    }

    /// Writing a 0 clears the field's bits in the base data.
    #[test]
    fn a_zero_clears_the_fields_bits() {
        let mut image = ParameterImage::new(vec![0xFF, 0xFF, 0xFF]);
        image.write(field(0, 2, 4), 0).unwrap();
        image.write(field(1, 0, 16), 0).unwrap();
        assert_eq!(image.octets(), &[0b1100_0011, 0x00, 0x00]);
    }

    #[test]
    fn a_value_wider_than_its_field_is_refused() {
        let mut image = ParameterImage::new(vec![0; 2]);
        assert_eq!(
            image.write(field(0, 4, 2), 4),
            Err(ParameterImageError::ValueTooWide {
                field: field(0, 4, 2),
                value: 4
            })
        );
        assert_eq!(
            image.write(field(0, 0, 8), 0x100),
            Err(ParameterImageError::ValueTooWide {
                field: field(0, 0, 8),
                value: 0x100
            })
        );
        // The largest value that fits is fine, including 64 bits.
        image.write(field(0, 4, 2), 3).unwrap();
        let mut wide = ParameterImage::new(vec![0; 8]);
        wide.write(field(0, 0, 64), u64::MAX).unwrap();
        assert_eq!(wide.octets(), &[0xFF; 8]);
    }

    #[test]
    fn a_field_past_the_segment_end_is_refused() {
        let mut image = ParameterImage::new(vec![0; 3]);
        assert_eq!(
            image.write(field(2, 0, 16), 1),
            Err(ParameterImageError::OutOfSegment {
                field: field(2, 0, 16),
                segment_octets: 3
            })
        );
        assert_eq!(
            image.write(field(3, 0, 1), 1),
            Err(ParameterImageError::OutOfSegment {
                field: field(3, 0, 1),
                segment_octets: 3
            })
        );
    }

    /// A field that crosses an octet boundary continues in the next octet's
    /// most significant bits: bit offsets count on from the first octet's
    /// MSB (module documentation). 6 bits at bit 5: the value's upper three
    /// bits end octet 0, its lower three start octet 1; the neighbours keep
    /// their bits.
    #[test]
    fn a_field_across_an_octet_boundary_continues_in_the_next_octets_msbs() {
        let mut image = ParameterImage::new(vec![0xFF, 0xFF]);
        image.write(field(0, 5, 6), 0b101_011).unwrap();
        assert_eq!(image.octets(), &[0b1111_1101, 0b0111_1111]);
    }

    /// 11 bits at bit 5 end on the octet boundary; 16 bits at bit 4 touch
    /// three octets. Both are written high bits first.
    #[test]
    fn wider_fields_across_octet_boundaries_are_written_high_bits_first() {
        let mut image = ParameterImage::new(vec![0; 2]);
        image.write(field(0, 5, 11), 0x5A5).unwrap();
        assert_eq!(image.octets(), &[0b0000_0101, 0xA5]);
        let mut image = ParameterImage::new(vec![0; 3]);
        image.write(field(0, 4, 16), 0xABCD).unwrap();
        assert_eq!(image.octets(), &[0x0A, 0xBC, 0xD0]);
        let mut image = ParameterImage::new(vec![0; 2]);
        image.write(field(0, 0, 12), 0xABC).unwrap();
        assert_eq!(image.octets(), &[0xAB, 0xC0]);
    }

    /// A crossing field shares the overlap check: it may not claim a bit a
    /// field written before holds, on either side of the boundary.
    #[test]
    fn a_field_across_an_octet_boundary_is_checked_for_overlap() {
        let mut image = ParameterImage::new(vec![0; 2]);
        image.write(field(1, 2, 1), 1).unwrap();
        let before = image.clone();
        assert_eq!(
            image.write(field(0, 5, 6), 0),
            Err(ParameterImageError::Overlap {
                field: field(0, 5, 6),
                earlier: field(1, 2, 1)
            })
        );
        assert_eq!(image, before);
        // The bits beside it are free.
        image.write(field(0, 5, 5), 0b11111).unwrap();
        assert_eq!(image.octets(), &[0b0000_0111, 0b1110_0000]);
    }

    /// Shapes the schema's definition excludes are refused, not guessed: a
    /// bit offset above 7, zero width, and more than 64 bits.
    #[test]
    fn unsupported_shapes_are_refused() {
        for shape in [field(0, 8, 1), field(0, 0, 0), field(0, 0, 72)] {
            let mut image = ParameterImage::new(vec![0; 16]);
            assert_eq!(
                image.write(shape, 0),
                Err(ParameterImageError::UnsupportedField { field: shape }),
                "{shape:?}"
            );
        }
    }

    /// Two union members at the same place are alternatives. Writing both
    /// would let the later silently win, so it is refused.
    #[test]
    fn an_overlapping_write_is_refused_and_the_image_is_unchanged() {
        let mut image = ParameterImage::new(vec![0; 4]);
        image.write(field(1, 0, 16), 0x0001).unwrap();
        let before = image.clone();
        assert_eq!(
            image.write(field(2, 5, 1), 1),
            Err(ParameterImageError::Overlap {
                field: field(2, 5, 1),
                earlier: field(1, 0, 16)
            })
        );
        assert_eq!(
            image.write(field(1, 0, 8), 0),
            Err(ParameterImageError::Overlap {
                field: field(1, 0, 8),
                earlier: field(1, 0, 16)
            })
        );
        assert_eq!(image, before);
    }

    /// Neighbouring bits in one octet are not an overlap.
    #[test]
    fn adjacent_bits_in_one_octet_do_not_overlap() {
        let mut image = ParameterImage::new(vec![0; 1]);
        image.write(field(0, 5, 1), 1).unwrap();
        image.write(field(0, 6, 1), 1).unwrap();
        image.write(field(0, 7, 1), 0).unwrap();
        image.write(field(0, 0, 5), 0b10001).unwrap();
        assert_eq!(image.octets(), &[0b1000_1110]);
    }

    #[test]
    fn a_refused_write_leaves_the_image_unchanged() {
        let mut image = ParameterImage::new(vec![0x12, 0x34]);
        let before = image.clone();
        let _ = image.write(field(0, 0, 8), 0x1FF);
        let _ = image.write(field(1, 0, 16), 1);
        assert_eq!(image, before);
        // And a refused field does not block a later, valid one.
        image.write(field(0, 0, 8), 0xFF).unwrap();
        assert_eq!(image.octets(), &[0xFF, 0x34]);
    }

    /// An octet string (a text value) is copied as is, wider than any
    /// integer field may be.
    #[test]
    fn an_octet_string_is_copied_into_its_whole_octet_field() {
        let mut image = ParameterImage::new(vec![0xAA; 12]);
        image
            .write_octets(field(1, 0, 80), b"Hi\0\0\0\0\0\0\0\0")
            .unwrap();
        assert_eq!(
            image.octets(),
            &[0xAA, b'H', b'i', 0, 0, 0, 0, 0, 0, 0, 0, 0xAA]
        );
    }

    #[test]
    fn an_octet_string_must_fill_its_field_exactly_and_start_on_an_octet() {
        let mut image = ParameterImage::new(vec![0; 8]);
        let before = image.clone();
        assert_eq!(
            image.write_octets(field(0, 0, 32), b"abc"),
            Err(ParameterImageError::UnsupportedField {
                field: field(0, 0, 32)
            })
        );
        assert_eq!(
            image.write_octets(field(0, 2, 16), b"ab"),
            Err(ParameterImageError::UnsupportedField {
                field: field(0, 2, 16)
            })
        );
        assert_eq!(
            image.write_octets(field(6, 0, 32), b"abcd"),
            Err(ParameterImageError::OutOfSegment {
                field: field(6, 0, 32),
                segment_octets: 8
            })
        );
        assert_eq!(image, before);
    }

    /// Octet strings and integers share one overlap check, both ways.
    #[test]
    fn an_octet_string_and_an_integer_may_not_overlap() {
        let mut image = ParameterImage::new(vec![0; 16]);
        image.write_octets(field(2, 0, 96), &[1; 12]).unwrap();
        assert_eq!(
            image.write(field(13, 4, 4), 1),
            Err(ParameterImageError::Overlap {
                field: field(13, 4, 4),
                earlier: field(2, 0, 96)
            })
        );
        image.write(field(0, 0, 16), 7).unwrap();
        assert_eq!(
            image.write_octets(field(1, 0, 8), &[9]),
            Err(ParameterImageError::Overlap {
                field: field(1, 0, 8),
                earlier: field(0, 0, 16)
            })
        );
        image.write(field(14, 0, 8), 5).unwrap();
    }

    #[test]
    fn signed_values_become_twos_complement_of_the_width() {
        assert_eq!(signed_bits(-1, 8), Ok(0xFF));
        assert_eq!(signed_bits(-128, 8), Ok(0x80));
        assert_eq!(signed_bits(127, 8), Ok(0x7F));
        assert_eq!(signed_bits(-2, 16), Ok(0xFFFE));
        assert_eq!(signed_bits(-1, 4), Ok(0xF));
        assert_eq!(signed_bits(i64::MIN, 64), Ok(0x8000_0000_0000_0000));
        assert_eq!(
            signed_bits(128, 8),
            Err(ParameterImageError::SignedValueOutOfRange {
                size_in_bit: 8,
                value: 128
            })
        );
        assert_eq!(
            signed_bits(-129, 8),
            Err(ParameterImageError::SignedValueOutOfRange {
                size_in_bit: 8,
                value: -129
            })
        );
        assert_eq!(
            signed_bits(0, 0),
            Err(ParameterImageError::SignedValueOutOfRange {
                size_in_bit: 0,
                value: 0
            })
        );
    }
}
