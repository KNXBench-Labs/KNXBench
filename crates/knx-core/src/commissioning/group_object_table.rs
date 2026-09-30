//! The Group Object Table of a mask-`0701h` device, rewritten over the product's base image.
//!
//! `[D]` Resources `03_05_01` v01.10.01 §4.18.9.1, p. 276 (*GrOT – Easy 3*,
//! "used by … mask 0701h in E-Mode"):
//!
//! ```text
//!   1 octet   TableAddress       Current Size
//!   2 octets  TableAddress + 1   RAM-Flags-Table Pointer
//!   4 octets  TableAddress + 3   Group Object Descriptor 0
//!   4 octets  TableAddress + 7   Group Object Descriptor 1
//!   …
//! ```
//!
//! *"The current size shall indicate the number of Group Objects in the
//! GrOT."* The clause stops there: it does not say what the four octets of
//! an Easy-3 descriptor are. The octets this module writes are therefore
//! taken from the neighbouring clauses and pinned by a device, not by the
//! Easy-3 clause:
//!
//! - `[D]` §4.18.3.1.2.1, pp. 262–265 (Type 1) defines a *Config Octet* and a
//!   *Type Octet*. Config bits: 6 transmit enable, 5 segment selector,
//!   4 write enable, 3 read enable, 2 communication enable, 1–0 transmission
//!   priority (`00` system, `10` urgent, `01` normal, `11` low). Type octet:
//!   bits 7–6 reserved `0`, value types 0–14.
//! - `[D]` §4.18.4.1, p. 265 (Type 2) makes bit 7 *Update Enable*; the other
//!   fields are *"identical as above"*.
//! - `[V]` The descriptor's third octet is that Config Octet in its Type 2
//!   form, and the fourth is the Type Octet. Octets 0–1 (by position, a data
//!   pointer) are **not written**: they are copied from the product's base
//!   image, so no reading of them is needed or claimed. Evidence: a
//!   read-only dump of a real `0701h` device (`1.1.67`, MDT
//!   `A-0027-15-0BAC`, 2026-09-28) is reproduced octet for octet, all 64
//!   descriptors, from the product's `AS-4400` `<Data>` by these rules:
//!   - an **active** object's config octet is its product flags and
//!     priority, with bit 5 (segment selector) kept from the base image,
//!     and its type octet is its value type;
//!   - an **inactive** object keeps its base octets except that
//!     communication enable (bit 2) is cleared, so the device ignores it.
//!
//! Which objects are active, and with what flags, is the product database's
//! business (the `Dynamic` tree evaluation). This module only encodes.

use std::collections::BTreeSet;
use std::fmt;

/// Octets before the first descriptor: Current Size and the two-octet
/// RAM-Flags-Table Pointer (§4.18.9.1).
pub const HEADER_OCTETS: usize = 3;

/// Octets per Group Object Descriptor (§4.18.9.1).
pub const DESCRIPTOR_OCTETS: usize = 4;

const CONFIG_OCTET: usize = 2;
const TYPE_OCTET: usize = 3;

const UPDATE_ENABLE: u8 = 0x80;
const TRANSMIT_ENABLE: u8 = 0x40;
const SEGMENT_SELECTOR: u8 = 0x20;
const WRITE_ENABLE: u8 = 0x10;
const READ_ENABLE: u8 = 0x08;
const COMMUNICATION_ENABLE: u8 = 0x04;

/// §4.18.3.1.2.1's transmission priority, bits 1–0 of the config octet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransmissionPriority {
    /// `00`.
    System,
    /// `10`.
    Urgent,
    /// `01`.
    Normal,
    /// `11`.
    Low,
}

impl TransmissionPriority {
    /// The two priority bits.
    pub fn bits(self) -> u8 {
        match self {
            TransmissionPriority::System => 0b00,
            TransmissionPriority::Urgent => 0b10,
            TransmissionPriority::Normal => 0b01,
            TransmissionPriority::Low => 0b11,
        }
    }

    /// The priority two bits name; only the low two bits of `bits` are
    /// read. The same codes as a frame's Ctrl1 priority field (Data Link
    /// Layer General v01.03.02 AS §2.2.3), which is why `knx-net` decodes
    /// received frames with it.
    pub fn from_bits(bits: u8) -> TransmissionPriority {
        match bits & 0b11 {
            0b00 => TransmissionPriority::System,
            0b10 => TransmissionPriority::Urgent,
            0b01 => TransmissionPriority::Normal,
            _ => TransmissionPriority::Low,
        }
    }
}

/// §4.18.3.1.2.1's value type, the type octet: 0–14.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ValueType(u8);

impl ValueType {
    /// The largest code the Type Octet table lists (14 octets).
    pub const MAX_CODE: u8 = 14;

    /// A code from the table, or `None` outside 0–14.
    pub fn from_code(code: u8) -> Option<ValueType> {
        (code <= Self::MAX_CODE).then_some(ValueType(code))
    }

    /// The value type for a value of `bits` bits. `[D]` The table lists
    /// 1–7 bits and 1, 2, 3, 4, 6, 8, 10 and 14 octets; every other size has
    /// no code and is refused (`None`) rather than rounded up.
    pub fn for_bits(bits: u32) -> Option<ValueType> {
        let code = match bits {
            1..=7 => bits - 1,
            8 => 7,
            16 => 8,
            24 => 9,
            32 => 10,
            48 => 11,
            64 => 12,
            80 => 13,
            112 => 14,
            _ => return None,
        };
        u8::try_from(code).ok().and_then(ValueType::from_code)
    }

    /// The type octet.
    pub fn code(self) -> u8 {
        self.0
    }
}

/// The flags and priority of one active group object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectFlags {
    /// Update enable (Type 2 bit 7).
    pub update: bool,
    /// Transmit enable.
    pub transmit: bool,
    /// Write enable.
    pub write: bool,
    /// Read enable.
    pub read: bool,
    /// Communication enable.
    pub communication: bool,
    /// Transmission priority.
    pub priority: TransmissionPriority,
}

impl ObjectFlags {
    /// The config octet without the segment selector, which is not a flag
    /// of the object but of where its value lives.
    fn config_bits(self) -> u8 {
        let flag = |on: bool, bit: u8| if on { bit } else { 0 };
        flag(self.update, UPDATE_ENABLE)
            | flag(self.transmit, TRANSMIT_ENABLE)
            | flag(self.write, WRITE_ENABLE)
            | flag(self.read, READ_ENABLE)
            | flag(self.communication, COMMUNICATION_ENABLE)
            | self.priority.bits()
    }
}

/// One object the application uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActiveObject {
    /// The group object number, the descriptor's index.
    pub number: u8,
    /// Its flags.
    pub flags: ObjectFlags,
    /// Its value type.
    pub value_type: ValueType,
}

/// Why the table could not be written. Nothing is clamped or skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupObjectTableError {
    /// The base image is shorter than the header.
    NoHeader {
        /// Its length.
        octets: usize,
    },
    /// The base image is shorter than its own Current Size says.
    Truncated {
        /// Octets the table needs.
        needed: usize,
        /// Octets the base image has.
        octets: usize,
    },
    /// An active object has no descriptor in the table.
    ObjectOutOfTable {
        /// The object.
        number: u8,
        /// The table's Current Size.
        size: u8,
    },
    /// The same object is listed twice.
    DuplicateObject {
        /// The object.
        number: u8,
    },
}

impl std::error::Error for GroupObjectTableError {}

impl fmt::Display for GroupObjectTableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GroupObjectTableError::NoHeader { octets } => write!(
                f,
                "the group object table base is {octets} octets, shorter than its \
                 {HEADER_OCTETS}-octet header"
            ),
            GroupObjectTableError::Truncated { needed, octets } => write!(
                f,
                "the group object table needs {needed} octets for its descriptors, \
                 but the base has {octets}"
            ),
            GroupObjectTableError::ObjectOutOfTable { number, size } => write!(
                f,
                "group object {number} has no descriptor: the table holds {size}"
            ),
            GroupObjectTableError::DuplicateObject { number } => {
                write!(f, "group object {number} is listed twice")
            }
        }
    }
}

/// Rewrites the descriptors of `base` (the table and anything after it, as
/// the product's segment `<Data>` holds them) for exactly `active` objects.
/// Octets outside the descriptors' config and type octets are returned
/// unchanged.
pub fn write_group_object_table(
    base: &[u8],
    active: &[ActiveObject],
) -> Result<Vec<u8>, GroupObjectTableError> {
    if base.len() < HEADER_OCTETS {
        return Err(GroupObjectTableError::NoHeader { octets: base.len() });
    }
    let size = base[0];
    let needed = HEADER_OCTETS + usize::from(size) * DESCRIPTOR_OCTETS;
    if base.len() < needed {
        return Err(GroupObjectTableError::Truncated {
            needed,
            octets: base.len(),
        });
    }
    let mut seen = BTreeSet::new();
    for object in active {
        if object.number >= size {
            return Err(GroupObjectTableError::ObjectOutOfTable {
                number: object.number,
                size,
            });
        }
        if !seen.insert(object.number) {
            return Err(GroupObjectTableError::DuplicateObject {
                number: object.number,
            });
        }
    }

    let mut table = base.to_vec();
    for number in 0..usize::from(size) {
        let at = HEADER_OCTETS + number * DESCRIPTOR_OCTETS;
        table[at + CONFIG_OCTET] &= !COMMUNICATION_ENABLE;
    }
    for object in active {
        let at = HEADER_OCTETS + usize::from(object.number) * DESCRIPTOR_OCTETS;
        let selector = base[at + CONFIG_OCTET] & SEGMENT_SELECTOR;
        table[at + CONFIG_OCTET] = object.flags.config_bits() | selector;
        table[at + TYPE_OCTET] = object.value_type.code();
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `A-0027-15-0BAC`'s `AS-4400` `<Data>`, the table part (3 + 64 × 4).
    const PRODUCT_BASE: &str = concat!(
        "4007000740df030748df030750df000758df000760df000744df03074cdf0307",
        "54df00075cdf000761df000762df000764df000766df000763df000765df0007",
        "67df000768df000769df00076adf00076bdf00076cdf00076ddf00076edf0007",
        "6fdf000770df000771df000772df000773df000774df000775df000776df0007",
        "77df000778df000779df00077adf00077bdf00077cdf00077ddf00077edf0007",
        "7fdf000780df000781df000782df000783df000784df000785df000786df0007",
        "87df000788df000789df00078adf00078bdf00078cdf00078ddf00078edf0007",
        "8fdf000790df000791df000792df000793df000794df000795df000796df0007",
        "97df00",
    );

    /// The same 259 octets read back from `1.1.67` on 2026-09-28
    /// (`live_memory_readonly`), configured as a shutter pair on objects
    /// 0/1 plus the LED orientation light on object 18.
    const DEVICE_READBACK: &str = concat!(
        "40070007404f0007484f000750db000758db000760db000744db03074cdb0307",
        "54db00075cdb000761db000762db000764db000766db000763db000765db0007",
        "67db000768db000769db00076ad700076bdb00076cdb00076ddb00076edb0007",
        "6fdb000770db000771db000772db000773db000774db000775db000776db0007",
        "77db000778db000779db00077adb00077bdb00077cdb00077ddb00077edb0007",
        "7fdb000780db000781db000782db000783db000784db000785db000786db0007",
        "87db000788db000789db00078adb00078bdb00078cdb00078ddb00078edb0007",
        "8fdb000790db000791db000792db000793db000794db000795db000796db0007",
        "97db00",
    );

    fn hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("valid hex"))
            .collect()
    }

    fn one_bit() -> ValueType {
        ValueType::from_code(0).expect("code 0 is 1 bit")
    }

    /// The product's flags for a push-button sender: read, communication,
    /// transmit, low priority (the `ComObject`s carry no `Priority`).
    fn sender() -> ObjectFlags {
        ObjectFlags {
            update: false,
            transmit: true,
            write: false,
            read: true,
            communication: true,
            priority: TransmissionPriority::Low,
        }
    }

    /// Write, communication, transmit and update, low priority.
    fn listener() -> ObjectFlags {
        ObjectFlags {
            update: true,
            transmit: true,
            write: true,
            read: false,
            communication: true,
            priority: TransmissionPriority::Low,
        }
    }

    fn object(number: u8, flags: ObjectFlags) -> ActiveObject {
        ActiveObject {
            number,
            flags,
            value_type: one_bit(),
        }
    }

    #[test]
    fn rebuilds_the_real_devices_table_octet_for_octet() {
        let table = write_group_object_table(
            &hex(PRODUCT_BASE),
            &[
                object(0, sender()),
                object(1, sender()),
                object(18, listener()),
            ],
        )
        .expect("the device's own configuration is valid");
        assert_eq!(table, hex(DEVICE_READBACK));
    }

    #[test]
    fn a_toggle_on_button_1_changes_only_descriptors_1_and_18() {
        let table = write_group_object_table(
            &hex(PRODUCT_BASE),
            &[object(0, sender()), object(1, listener())],
        )
        .expect("valid");
        let device = hex(DEVICE_READBACK);
        let changed: Vec<usize> = (0..table.len())
            .filter(|&i| table[i] != device[i])
            .collect();
        // Descriptor 1's config octet (3 + 4 + 2) and descriptor 18's (3 + 72 + 2).
        assert_eq!(changed, vec![9, 77]);
        assert_eq!(&table[3..11], &hex("07404f000748d700")[..]);
        assert_eq!(table[77], 0xDB, "object 18 is switched off");
    }

    #[test]
    fn an_inactive_object_loses_only_its_communication_flag() {
        let base = hex(PRODUCT_BASE);
        let table = write_group_object_table(&base, &[]).expect("valid");
        for number in 0..64usize {
            let at = HEADER_OCTETS + number * DESCRIPTOR_OCTETS;
            assert_eq!(table[at..at + 2], base[at..at + 2], "pointer of {number}");
            assert_eq!(table[at + 2], base[at + 2] & !COMMUNICATION_ENABLE);
            assert_eq!(table[at + 3], base[at + 3], "type of {number}");
        }
    }

    #[test]
    fn the_header_is_never_rewritten() {
        let base = hex(PRODUCT_BASE);
        let table = write_group_object_table(&base, &[object(5, listener())]).expect("valid");
        assert_eq!(table[..HEADER_OCTETS], base[..HEADER_OCTETS]);
    }

    #[test]
    fn the_segment_selector_comes_from_the_base_image() {
        let mut base = hex(PRODUCT_BASE);
        base[HEADER_OCTETS + CONFIG_OCTET] |= SEGMENT_SELECTOR;
        let table = write_group_object_table(&base, &[object(0, sender())]).expect("valid");
        assert_eq!(table[HEADER_OCTETS + CONFIG_OCTET], 0x4F | SEGMENT_SELECTOR);
    }

    #[test]
    fn octets_after_the_table_are_left_alone() {
        let mut base = hex(PRODUCT_BASE);
        base.extend_from_slice(&[0xAA, 0x55]);
        let table = write_group_object_table(&base, &[]).expect("valid");
        assert_eq!(&table[table.len() - 2..], &[0xAA, 0x55]);
        assert_eq!(table.len(), base.len());
    }

    #[test]
    fn every_flag_lands_on_its_own_bit() {
        let base = hex(PRODUCT_BASE);
        let cases = [
            (
                ObjectFlags {
                    update: true,
                    ..none()
                },
                UPDATE_ENABLE,
            ),
            (
                ObjectFlags {
                    transmit: true,
                    ..none()
                },
                TRANSMIT_ENABLE,
            ),
            (
                ObjectFlags {
                    write: true,
                    ..none()
                },
                WRITE_ENABLE,
            ),
            (
                ObjectFlags {
                    read: true,
                    ..none()
                },
                READ_ENABLE,
            ),
            (
                ObjectFlags {
                    communication: true,
                    ..none()
                },
                COMMUNICATION_ENABLE,
            ),
        ];
        for (flags, bit) in cases {
            let table = write_group_object_table(&base, &[object(2, flags)]).expect("valid");
            let config = table[HEADER_OCTETS + 2 * DESCRIPTOR_OCTETS + CONFIG_OCTET];
            assert_eq!(config, bit, "{flags:?}");
        }
    }

    fn none() -> ObjectFlags {
        ObjectFlags {
            update: false,
            transmit: false,
            write: false,
            read: false,
            communication: false,
            priority: TransmissionPriority::System,
        }
    }

    #[test]
    fn priorities_follow_the_resources_table() {
        assert_eq!(TransmissionPriority::System.bits(), 0b00);
        assert_eq!(TransmissionPriority::Urgent.bits(), 0b10);
        assert_eq!(TransmissionPriority::Normal.bits(), 0b01);
        assert_eq!(TransmissionPriority::Low.bits(), 0b11);
        for priority in [
            TransmissionPriority::System,
            TransmissionPriority::Urgent,
            TransmissionPriority::Normal,
            TransmissionPriority::Low,
        ] {
            assert_eq!(TransmissionPriority::from_bits(priority.bits()), priority);
            assert_eq!(
                TransmissionPriority::from_bits(priority.bits() | 0b1111_1100),
                priority
            );
        }
        let base = hex(PRODUCT_BASE);
        let flags = ObjectFlags {
            priority: TransmissionPriority::Urgent,
            ..none()
        };
        let table = write_group_object_table(&base, &[object(3, flags)]).expect("valid");
        assert_eq!(
            table[HEADER_OCTETS + 3 * DESCRIPTOR_OCTETS + CONFIG_OCTET],
            0b10
        );
    }

    #[test]
    fn the_type_octet_is_the_value_type() {
        let base = hex(PRODUCT_BASE);
        let two_octets = ActiveObject {
            number: 4,
            flags: sender(),
            value_type: ValueType::for_bits(16).expect("2 octets"),
        };
        let table = write_group_object_table(&base, &[two_octets]).expect("valid");
        assert_eq!(table[HEADER_OCTETS + 4 * DESCRIPTOR_OCTETS + TYPE_OCTET], 8);
    }

    #[test]
    fn value_types_cover_exactly_the_listed_sizes() {
        let listed: Vec<(u32, u8)> = vec![
            (1, 0),
            (2, 1),
            (3, 2),
            (4, 3),
            (5, 4),
            (6, 5),
            (7, 6),
            (8, 7),
            (16, 8),
            (24, 9),
            (32, 10),
            (48, 11),
            (64, 12),
            (80, 13),
            (112, 14),
        ];
        for (bits, code) in &listed {
            assert_eq!(
                ValueType::for_bits(*bits).map(ValueType::code),
                Some(*code),
                "{bits}"
            );
        }
        for bits in [0, 9, 40, 56, 72, 88, 96, 104, 120, 128] {
            assert_eq!(ValueType::for_bits(bits), None, "{bits} bits has no code");
        }
        assert_eq!(ValueType::from_code(14).map(ValueType::code), Some(14));
        assert_eq!(ValueType::from_code(15), None);
    }

    #[test]
    fn an_object_beyond_the_table_is_refused() {
        let err = write_group_object_table(&hex(PRODUCT_BASE), &[object(64, sender())])
            .expect_err("64 objects: 0-63");
        assert_eq!(
            err,
            GroupObjectTableError::ObjectOutOfTable {
                number: 64,
                size: 64
            }
        );
    }

    #[test]
    fn a_duplicate_object_is_refused() {
        let err = write_group_object_table(
            &hex(PRODUCT_BASE),
            &[object(7, sender()), object(7, listener())],
        )
        .expect_err("twice");
        assert_eq!(err, GroupObjectTableError::DuplicateObject { number: 7 });
    }

    #[test]
    fn a_base_without_header_is_refused() {
        assert_eq!(
            write_group_object_table(&[0x01, 0x00], &[]),
            Err(GroupObjectTableError::NoHeader { octets: 2 })
        );
    }

    #[test]
    fn a_base_shorter_than_its_size_is_refused() {
        let base = hex(PRODUCT_BASE);
        assert_eq!(
            write_group_object_table(&base[..258], &[]),
            Err(GroupObjectTableError::Truncated {
                needed: 259,
                octets: 258
            })
        );
    }
}
