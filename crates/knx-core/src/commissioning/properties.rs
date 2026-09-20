//! The Property Identifiers and Object Indices the cited procedures name, and no others.
//!
//! Spec §3.1 lists the properties; §3.2 establishes that the *service's*
//! `object_index` field selects the Interface Object, not `PID_OBJECT_INDEX`.
//! Every number here is quoted from a source clause in its own doc comment,
//! because a wrong PID writes a correct payload into the wrong property and
//! §13 rates that as one of the worse outcomes in the document.
//!
//! There is no "all properties" table here on purpose: a constant nobody
//! needs is a constant nobody checked.

use std::fmt;

/// `PID_LOAD_STATE_CONTROL`.
///
/// `[D]` Spec §3.1 / RES §4.23; `03_07_03 Standardized Identifier Tables`
/// row `| 5: | PID_LOAD_STATE_CONTROL |`. Read gives a state, write is an
/// event (§5.1), which is why [`super::load_state`] has two enums.
pub const PID_LOAD_STATE_CONTROL: u8 = 5;

/// `PID_RUN_STATE_CONTROL`, named so it is not mistaken for the one above.
///
/// `[D]` Spec §3.3. This project never reads or writes it; the constant
/// exists so that a reader of a frame dump can recognise it.
pub const PID_RUN_STATE_CONTROL: u8 = 6;

/// `PID_TABLE_REFERENCE` — the base address design spec §7.2 step 3 calls
/// `PID_REFERENCE`.
///
/// `[D]` `03_07_03 Standardized Identifier Tables` row
/// `| 7: | PID_TABLE_REFERENCE |`, and RES §4.16.11.5.1.1's procedure
/// *"Get GrAT - Easy 2 Table pointer DMP_InterfaceObjectRead_R(object_index
/// = 01h, PID = PID_TABLE_REFERENCE, …)"*. That the design spec's
/// `PID_REFERENCE` is this property is stated by §7.6: *"Unload frees the
/// memory and zeroes `PID_TABLE_REFERENCE`."*
pub const PID_TABLE_REFERENCE: u8 = 7;

/// `PID_MANUFACTURER_ID`, the guard of design spec §7.1 step 04.
///
/// `[D]` `03_07_03 Standardized Identifier Tables` row
/// `| 12: | PID_MANUFACTURER_ID |`, and CP §3.5.2 step 03 *"MaC:
/// PropRead(DeviceObj, PID_MANUFACTURER_ID (PID = 12)"*.
pub const PID_MANUFACTURER_ID: u8 = 12;

/// `PID_PROGRAM_VERSION`, written by design spec §7.2 step 5.
///
/// `[D]` `03_07_03 Standardized Identifier Tables` row
/// `| 13: | PID_PROGRAM_VERSION | Application Version |`, confirmed by RES
/// §4.19's Application Program Object table *"Application Version | 13 =
/// PID_PROGRAM_VERSION | PDT_GENERIC_05"*.
pub const PID_PROGRAM_VERSION: u8 = 13;

/// `PID_DEVICE_CONTROL`, whose bit 2 is Verify Mode (design spec §6.3).
///
/// `[D]` RES §4.2.14, Table 11.
pub const PID_DEVICE_CONTROL: u8 = 14;

/// `PID_MCB_TABLE` / `PID_MCB`, the Memory Control Block whose CRC spec
/// §7.2 step 7 stores and §7.4 compares.
///
/// `[D]` Spec §3.1's table.
pub const PID_MCB_TABLE: u8 = 27;

/// `PID_ERROR_CODE`, read *before* any unload (design spec §5.5).
///
/// `[D]` RES §4.2.28. Its values are [`super::error_code`].
pub const PID_ERROR_CODE: u8 = 28;

/// `PID_OBJECT_INDEX`, which an object reports about itself.
///
/// `[D]` RES §4.2.29, and design spec §3.2's correction: this is **not** how an
/// object is addressed.
pub const PID_OBJECT_INDEX: u8 = 29;

/// `PID_DOWNLOAD_COUNTER`, RES §4.2.30, p. 41: `PDT_UNSIGNED_INT`,
/// `DPT_Value_2_Ucount` (7.010) — a two-octet unsigned counter.
///
/// `[D]` `03_07_03 Standardized Identifier Tables` row
/// `| 30: | PID_DOWNLOAD_COUNTER |`, and CP §3.12.4, p. 99: *"Read the
/// download counter."* RES §4.2.30.1, p. 41 makes it global and read-only
/// but stops short of mandatory: *"A device that has a Download Counter
/// shall at least have `PID_DOWNLOAD_COUNTER` in the Device Object"* —
/// conditioned on having one at all. Whether a System B device must have
/// one is Volume 6 Profiles Annex A's question, answered `[C18]`:
/// optional, not mandatory (pp. 138-140 omit PID 30 for every profile).
pub const PID_DOWNLOAD_COUNTER: u8 = 30;

/// `PID_MAX_APDU_LENGTH`, read from the Device Object only (design spec §6.4).
///
/// `[D]` RES §4.3.7.
pub const PID_MAX_APDU_LENGTH: u8 = 56;

/// `PID_HARDWARE_TYPE`, the optional capability check of CP §3.5.2 step 03.
///
/// `[D]` CP §3.5.2 *"Optional: MaC: Capability check: (e.g.) MaC:
/// PropRead(DeviceObj, PID_HARDWARE_TYPE (PID = 78)"*. Read-only, and
/// phase 2 only reads it.
pub const PID_HARDWARE_TYPE: u8 = 78;

/// Bit 2 of `PID_DEVICE_CONTROL`: *Verify Mode On*.
///
/// `[D]` RES §4.2.14.2, Table 11.
pub const DEVICE_CONTROL_VERIFY_MODE: u8 = 0b0000_0100;

/// Bit 0 of `PID_DEVICE_CONTROL`: *User stopped* (RES Table 11). Named so
/// that the read-modify-write of [`with_verify_mode`] is visibly preserving
/// something real rather than padding.
pub const DEVICE_CONTROL_USER_STOPPED: u8 = 0b0000_0001;

/// Bit 1 of `PID_DEVICE_CONTROL`: *Individual Address duplication*
/// (RES Table 11).
pub const DEVICE_CONTROL_ADDRESS_DUPLICATION: u8 = 0b0000_0010;

/// Bit 3 of `PID_DEVICE_CONTROL`: *Safe State On* (RES Table 11).
pub const DEVICE_CONTROL_SAFE_STATE: u8 = 0b0000_1000;

/// Whether a `PID_DEVICE_CONTROL` octet says Verify Mode is on.
pub fn verify_mode_active(device_control: u8) -> bool {
    device_control & DEVICE_CONTROL_VERIFY_MODE != 0
}

/// The octet to write back to set or clear Verify Mode, preserving every
/// other bit.
///
/// A blunt write of `04h` would clear *User stopped*, *Individual Address
/// duplication* and *Safe State On* (RES Table 11) in one go, which is
/// three pieces of device state destroyed to change one. RES §4.2.14.3
/// permits a client to *"reset the value by writing 0 to this Property"*,
/// so the property is writable as a whole — which is exactly why the
/// read-modify-write matters.
pub fn with_verify_mode(device_control: u8, on: bool) -> u8 {
    if on {
        device_control | DEVICE_CONTROL_VERIFY_MODE
    } else {
        device_control & !DEVICE_CONTROL_VERIFY_MODE
    }
}

/// Which Interface Object a property service addresses, via the service's
/// own `object_index` field (design spec §3.2).
///
/// This is a newtype and not a bare `u8` so that a PID and an object index
/// cannot be swapped at a call site: both are single octets, and the two
/// mistakes they produce are silent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectIndex(u8);

impl ObjectIndex {
    /// The Device Object.
    ///
    /// `[D]` CP §2.6.2.2: *"/* Read PID_MAX_APDU_LENGTH of the Device
    /// Object */ DMP_InterfaceObjectReadR(object_index = 0; PID =
    /// PID_MAX_APDU_LENGTH; …)"*, and CP's Coupler download: *"/* Set the
    /// Load State Machine of the Device Object to 'Unloaded'. */
    /// DMP_LoadStateMachineWrite_R_Co_IO(object_index = 0, data = {event =
    /// 04h})"*.
    pub const DEVICE: ObjectIndex = ObjectIndex(0);

    /// The Address Table Object.
    ///
    /// `[D]` RES §4.16.11.5.1.1: *"Get Group Address Table pointer
    /// DMP_InterfaceObjectRead_R(object_index = 01h, PID =
    /// PID_TABLE_REFERENCE, …)"*, and §4.16.11.5.1.3 drives its Load State
    /// Machine at the same index.
    pub const ADDRESS_TABLE: ObjectIndex = ObjectIndex(1);

    /// The Association Table Object.
    ///
    /// `[D]` RES §4.16.11.5.1.2: *"Get Group Association Table pointer
    /// DMP_InterfaceObjectRead_R(object_index = 02h, PID =
    /// PID_TABLE_REFERENCE, …)"*.
    pub const ASSOCIATION_TABLE: ObjectIndex = ObjectIndex(2);

    /// The Application Program Object, which is also where the Group
    /// Object Table and the parameter block are reached.
    ///
    /// `[D]` RES §4.17's procedures: *"Get Group Object Table Pointer
    /// DMP_InterfaceObjectRead_R(object_index = 03h, PID =
    /// PID_TABLE_REFERENCE, …)"* and *"Get Parameter Block Pointer
    /// DMP_InterfaceObjectRead_R(object_index = 03h, PID = PID_PARAM…)"*.
    ///
    /// **[A]** These four indices are the ones the Standard's own example
    /// procedures use. They are *not* guaranteed for an arbitrary device:
    /// CP's Coupler download addresses its objects as
    /// `OI = RouterObject2.ObjectIndex`, i.e. from a value the client
    /// obtained elsewhere. A real download takes the index from product
    /// data; these constants are the documented default and the
    /// simulator's layout.
    pub const APPLICATION_PROGRAM: ObjectIndex = ObjectIndex(3);

    /// An index a caller supplies from product data.
    pub fn new(index: u8) -> Self {
        ObjectIndex(index)
    }

    /// The octet the property service carries.
    pub fn octet(self) -> u8 {
        self.0
    }
}

impl fmt::Display for ObjectIndex {
    /// Renders the number plus, for the four documented indices, the object
    /// it refers to — an index alone is unreadable in a log, and a name
    /// alone would be a claim about a device nobody asked.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self.0 {
            0 => Some("Device Object"),
            1 => Some("Address Table Object"),
            2 => Some("Association Table Object"),
            3 => Some("Application Program Object"),
            _ => None,
        };
        match name {
            Some(name) => write!(f, "object index {} ({name})", self.0),
            None => write!(f, "object index {}", self.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec §3.1's table, transcribed once and asserted here so a later
    /// edit cannot quietly renumber a property.
    #[test]
    fn the_property_ids_are_the_ones_the_cited_clauses_print() {
        assert_eq!(PID_LOAD_STATE_CONTROL, 5);
        assert_eq!(PID_RUN_STATE_CONTROL, 6);
        assert_eq!(PID_TABLE_REFERENCE, 7);
        assert_eq!(PID_MANUFACTURER_ID, 12);
        assert_eq!(PID_PROGRAM_VERSION, 13);
        assert_eq!(PID_DEVICE_CONTROL, 14);
        assert_eq!(PID_MCB_TABLE, 27);
        assert_eq!(PID_ERROR_CODE, 28);
        assert_eq!(PID_OBJECT_INDEX, 29);
        assert_eq!(PID_DOWNLOAD_COUNTER, 30);
        assert_eq!(PID_MAX_APDU_LENGTH, 56);
        assert_eq!(PID_HARDWARE_TYPE, 78);
    }

    #[test]
    fn verify_mode_is_bit_two_and_setting_it_preserves_the_other_three() {
        let busy = DEVICE_CONTROL_USER_STOPPED
            | DEVICE_CONTROL_ADDRESS_DUPLICATION
            | DEVICE_CONTROL_SAFE_STATE;
        assert!(!verify_mode_active(busy));
        let set = with_verify_mode(busy, true);
        assert!(verify_mode_active(set));
        assert_eq!(set, busy | 0b0000_0100);
        assert_eq!(with_verify_mode(set, false), busy);
    }

    /// Reserved bits 4-7 are not this function's to tidy up: a device that
    /// reports them set is reporting something, and RES Table 11 does not
    /// say what.
    #[test]
    fn reserved_bits_survive_a_verify_mode_write() {
        assert_eq!(with_verify_mode(0b1111_0000, true), 0b1111_0100);
        assert_eq!(with_verify_mode(0b1111_0100, false), 0b1111_0000);
    }

    #[test]
    fn an_object_index_names_the_four_documented_objects() {
        assert_eq!(ObjectIndex::DEVICE.octet(), 0);
        assert_eq!(ObjectIndex::ADDRESS_TABLE.octet(), 1);
        assert_eq!(ObjectIndex::ASSOCIATION_TABLE.octet(), 2);
        assert_eq!(ObjectIndex::APPLICATION_PROGRAM.octet(), 3);
        assert_eq!(
            ObjectIndex::DEVICE.to_string(),
            "object index 0 (Device Object)"
        );
        assert_eq!(ObjectIndex::new(9).to_string(), "object index 9");
    }
}
