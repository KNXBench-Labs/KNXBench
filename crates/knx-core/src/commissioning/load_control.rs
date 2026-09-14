//! The ten-octet payloads written to `PID_LOAD_STATE_CONTROL`, and the subtype each mask allows.
//!
//! Spec §7.3, from MP §3.31.3's `DM_LoadStateMachineWrite_RCo_IO` and
//! PROF Annex A.2.4.1 Table 7. Building a payload is pure arithmetic and
//! lives here; sending one is a write and lives behind the mutation API.

use std::fmt;

use super::load_state::{LoadEvent, MaskVersion};

/// `PID_LOAD_STATE_CONTROL`. `[D]` RES §4.23: property 5 of a loadable
/// part's Interface Object, `PDT_CONTROL`.
///
/// Re-exported rather than restated: the number lives with the rest of the
/// cited property identifiers in [`super::properties`], and one constant
/// with two definitions is one constant that can disagree with itself.
pub use super::properties::PID_LOAD_STATE_CONTROL;

/// `[D]` MP §3.31.3: the property write is *"exactly 10 octets"*, at
/// `start_index = 01h` with `nr_of_elem = 01h`. Not 1, not 2, and not
/// "however many the payload needs".
pub const LOAD_CONTROL_PAYLOAD_OCTETS: usize = 10;

/// The `start_index` MP §3.31.3 specifies for the write.
pub const LOAD_CONTROL_START_INDEX: u16 = 0x01;

/// The `nr_of_elem` MP §3.31.3 specifies for the write.
pub const LOAD_CONTROL_NR_OF_ELEM: u8 = 0x01;

/// A ten-octet `PID_LOAD_STATE_CONTROL` payload.
///
/// A newtype rather than a bare `[u8; 10]`, so that a payload cannot be
/// confused with any other ten octets in the system and so that the length
/// is a property of the type rather than of whoever remembered it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoadControlPayload([u8; LOAD_CONTROL_PAYLOAD_OCTETS]);

impl LoadControlPayload {
    /// The octets, in wire order.
    pub fn octets(&self) -> &[u8; LOAD_CONTROL_PAYLOAD_OCTETS] {
        &self.0
    }

    /// The first octet, which is the event.
    pub fn event_octet(&self) -> u8 {
        self.0[0]
    }
}

impl fmt::Display for LoadControlPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, octet) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(" ")?;
            }
            write!(f, "{octet:02X}")?;
        }
        Ok(())
    }
}

/// The payload for a plain event: the event octet then nine zeroes.
///
/// `[D]` MP §3.31.3's table: No Operation is all `00h`, Start Loading is
/// `01h` + 9 × `00h`, Load Completed `02h`, Unload `04h`. Additional Load
/// Control is *not* reachable here, because it needs a subtype — see
/// [`additional_load_control`].
pub fn event_payload(event: LoadEvent) -> LoadControlPayload {
    let mut octets = [0u8; LOAD_CONTROL_PAYLOAD_OCTETS];
    octets[0] = event.octet();
    LoadControlPayload(octets)
}

/// An `Additional Load Control` subtype, from MP §3.31.3's second table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LoadControlSubtype {
    /// `00h` AllocAbsDataSeg.
    AllocAbsDataSeg,
    /// `01h` AllocAbsStackSeg.
    AllocAbsStackSeg,
    /// `02h` AllocAbsTaskSeg.
    AllocAbsTaskSeg,
    /// `03h` TaskPtr.
    TaskPtr,
    /// `04h` TaskCtrl1.
    TaskCtrl1,
    /// `05h` TaskCtrl2.
    TaskCtrl2,
    /// `0Ah` Relative Allocation, whose field is a number of octets
    /// requested. `[D]` MP §3.31.3: *"If the requested number of octets is
    /// not supported by the Management Server (device) then the Load State
    /// Machine of the loadable part shall change to error."* — asking for
    /// too much is not an error code, it is a trap.
    RelativeAllocation,
    /// `0Bh` Data Relative Allocation: a 4-octet requested size plus a
    /// Mode/fill octet. The subtype CP §3.5.2 step 06 uses on System B.
    DataRelativeAllocation,
}

impl LoadControlSubtype {
    /// The subtype octet.
    pub fn octet(self) -> u8 {
        match self {
            LoadControlSubtype::AllocAbsDataSeg => 0x00,
            LoadControlSubtype::AllocAbsStackSeg => 0x01,
            LoadControlSubtype::AllocAbsTaskSeg => 0x02,
            LoadControlSubtype::TaskPtr => 0x03,
            LoadControlSubtype::TaskCtrl1 => 0x04,
            LoadControlSubtype::TaskCtrl2 => 0x05,
            LoadControlSubtype::RelativeAllocation => 0x0A,
            LoadControlSubtype::DataRelativeAllocation => 0x0B,
        }
    }

    /// Whether this subtype allocates memory at all, as opposed to
    /// describing a task or a segment record.
    pub fn is_allocation(self) -> bool {
        matches!(
            self,
            LoadControlSubtype::AllocAbsDataSeg
                | LoadControlSubtype::AllocAbsStackSeg
                | LoadControlSubtype::AllocAbsTaskSeg
                | LoadControlSubtype::RelativeAllocation
                | LoadControlSubtype::DataRelativeAllocation
        )
    }
}

impl fmt::Display for LoadControlSubtype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            LoadControlSubtype::AllocAbsDataSeg => "AllocAbsDataSeg",
            LoadControlSubtype::AllocAbsStackSeg => "AllocAbsStackSeg",
            LoadControlSubtype::AllocAbsTaskSeg => "AllocAbsTaskSeg",
            LoadControlSubtype::TaskPtr => "TaskPtr",
            LoadControlSubtype::TaskCtrl1 => "TaskCtrl1",
            LoadControlSubtype::TaskCtrl2 => "TaskCtrl2",
            LoadControlSubtype::RelativeAllocation => "Relative Allocation",
            LoadControlSubtype::DataRelativeAllocation => "Data Relative Allocation",
        };
        write!(f, "{name} ({:02X}h)", self.octet())
    }
}

/// The event octet for `Additional Load Control`.
const ADDITIONAL_LOAD_CONTROL: u8 = 0x03;

/// Builds an `Additional Load Control` payload: `03h`, the subtype, and
/// eight octets of subtype-specific fields.
///
/// The eight field octets are passed whole rather than assembled here,
/// because only two subtypes' layouts are transcribed in the spec and
/// inventing the others would be exactly the guess §12's `GAP-T30-02`
/// forbids. Use [`data_relative_allocation`] for the one System B needs.
pub fn additional_load_control(subtype: LoadControlSubtype, fields: [u8; 8]) -> LoadControlPayload {
    let mut octets = [0u8; LOAD_CONTROL_PAYLOAD_OCTETS];
    octets[0] = ADDITIONAL_LOAD_CONTROL;
    octets[1] = subtype.octet();
    octets[2..].copy_from_slice(&fields);
    LoadControlPayload(octets)
}

/// What subtype `0Bh`'s Mode octet asks the device to do with memory it
/// allocates.
///
/// `[D]` spec §7.3: *"4-octet requested memory size + Mode/fill octet;
/// Mode bit 0 selects keep vs fill"*. Bits 1–7 of that octet are not
/// specified here and are therefore sent as zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AllocationMode {
    /// Keep whatever the memory contained. Bit 0 clear.
    #[default]
    Keep,
    /// Fill the allocated memory. Bit 0 set.
    Fill,
}

impl AllocationMode {
    /// The Mode/fill octet.
    pub fn octet(self) -> u8 {
        match self {
            AllocationMode::Keep => 0x00,
            AllocationMode::Fill => 0x01,
        }
    }
}

/// Builds the `0Bh` Data Relative Allocation payload for `size` octets.
///
/// The size is four octets, most significant first, which is how every
/// other multi-octet field in these procedures is ordered.
pub fn data_relative_allocation(size: u32, mode: AllocationMode) -> LoadControlPayload {
    let size = size.to_be_bytes();
    let fields = [
        size[0],
        size[1],
        size[2],
        size[3],
        mode.octet(),
        0x00,
        0x00,
        0x00,
    ];
    additional_load_control(LoadControlSubtype::DataRelativeAllocation, fields)
}

/// Builds the `0Ah` Relative Allocation payload for `octets_requested`.
///
/// Kept for the masks whose profile requires `0Ah` (`0300h`), and
/// deliberately *not* reachable as a fallback from `0Bh`: §7.3 design rule
/// 2 says there is no fallback between allocation styles.
pub fn relative_allocation(octets_requested: u32) -> LoadControlPayload {
    let requested = octets_requested.to_be_bytes();
    let fields = [
        requested[0],
        requested[1],
        requested[2],
        requested[3],
        0x00,
        0x00,
        0x00,
        0x00,
    ];
    additional_load_control(LoadControlSubtype::RelativeAllocation, fields)
}

/// Mask `0300h`, whose profile requires `0Ah` Relative Allocation.
pub const MASK_0300: MaskVersion = MaskVersion(0x0300);
/// Mask `07B0h`, a System B mask whose profile requires `0Bh`.
pub const MASK_07B0: MaskVersion = MaskVersion(0x07B0);
/// Mask `17B0h`, a System B mask whose profile requires `0Bh`.
pub const MASK_17B0: MaskVersion = MaskVersion(0x17B0);
/// Mask `57B0h`, the sharpest case: `0Bh` mandatory, absolute allocation
/// `n/a`.
pub const MASK_57B0: MaskVersion = MaskVersion(0x57B0);

/// Why no allocation subtype could be chosen for a mask.
///
/// There is exactly one correct response to this value, and it is to
/// refuse the download. §7.3 design rule 2: *"There is no fallback between
/// allocation styles. Pick by mask, or refuse."*
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllocationSubtypeError {
    /// PROF Table 7 was not transcribed for this mask, because its cells
    /// did not extract reliably (§7.3 design rule 3). Not "unknown to the
    /// Standard" — unknown to this implementation, which is a refusal
    /// either way.
    MaskNotProfiled(MaskVersion),
    /// The caller asked for a style the mask's profile marks `n/a`. The
    /// `57B0h` absolute-allocation case.
    StyleNotApplicable {
        /// The mask asked about.
        mask: MaskVersion,
        /// The subtype whose cell is `n/a` for that mask.
        subtype: LoadControlSubtype,
    },
}

impl std::error::Error for AllocationSubtypeError {}

impl fmt::Display for AllocationSubtypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AllocationSubtypeError::MaskNotProfiled(mask) => write!(
                f,
                "mask {mask} has no transcribed allocation subtype in PROF Table 7, \
                 so no load control is emitted for it"
            ),
            AllocationSubtypeError::StyleNotApplicable { mask, subtype } => write!(
                f,
                "PROF Table 7 marks {subtype} as not applicable for mask {mask}, \
                 and there is no fallback between allocation styles"
            ),
        }
    }
}

/// The allocation subtype a mask's profile requires, or a refusal.
///
/// Only the rows spec §7.3 transcribes are represented. Everything else
/// returns [`AllocationSubtypeError::MaskNotProfiled`], because a `?` cell
/// in the extraction is an extraction artefact and not a documented
/// permission.
pub fn allocation_subtype_for(
    mask: MaskVersion,
) -> Result<LoadControlSubtype, AllocationSubtypeError> {
    match mask {
        MASK_07B0 | MASK_17B0 | MASK_57B0 => Ok(LoadControlSubtype::DataRelativeAllocation),
        MASK_0300 => Ok(LoadControlSubtype::RelativeAllocation),
        other => Err(AllocationSubtypeError::MaskNotProfiled(other)),
    }
}

/// Whether a mask's profile permits a given subtype at all.
///
/// Exists so that the `57B0h` / absolute-allocation combination refuses
/// loudly rather than being silently rerouted: a client that "falls back"
/// there sends a device something its own profile calls not applicable.
pub fn require_subtype(
    mask: MaskVersion,
    subtype: LoadControlSubtype,
) -> Result<(), AllocationSubtypeError> {
    let absolute_or_record = matches!(
        subtype,
        LoadControlSubtype::AllocAbsDataSeg
            | LoadControlSubtype::AllocAbsStackSeg
            | LoadControlSubtype::AllocAbsTaskSeg
            | LoadControlSubtype::TaskPtr
            | LoadControlSubtype::TaskCtrl1
            | LoadControlSubtype::TaskCtrl2
    );
    if mask == MASK_57B0 && absolute_or_record {
        return Err(AllocationSubtypeError::StyleNotApplicable { mask, subtype });
    }
    let profiled = allocation_subtype_for(mask)?;
    if subtype.is_allocation() && subtype != profiled {
        return Err(AllocationSubtypeError::StyleNotApplicable { mask, subtype });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec §5.3 and §7.3: the ten-octet payloads, octet for octet.
    #[test]
    fn every_plain_event_payload_is_the_event_octet_then_nine_zeroes() {
        let cases = [
            (LoadEvent::NoOperation, 0x00),
            (LoadEvent::StartLoading, 0x01),
            (LoadEvent::LoadCompleted, 0x02),
            (LoadEvent::AdditionalLoadControls, 0x03),
            (LoadEvent::Unload, 0x04),
        ];
        for (event, expected) in cases {
            let payload = event_payload(event);
            assert_eq!(payload.octets().len(), 10, "MP §3.31.3 says exactly ten");
            assert_eq!(payload.event_octet(), expected, "{event}");
            assert!(
                payload.octets()[1..].iter().all(|octet| *octet == 0),
                "{event} must pad with zeroes: {payload}"
            );
        }
    }

    #[test]
    fn the_property_write_parameters_are_the_ones_the_clause_states() {
        assert_eq!(PID_LOAD_STATE_CONTROL, 5);
        assert_eq!(LOAD_CONTROL_START_INDEX, 0x01);
        assert_eq!(LOAD_CONTROL_NR_OF_ELEM, 0x01);
        assert_eq!(LOAD_CONTROL_PAYLOAD_OCTETS, 10);
    }

    #[test]
    fn data_relative_allocation_carries_the_size_big_endian_then_the_mode() {
        let payload = data_relative_allocation(0x0001_2345, AllocationMode::Fill);
        assert_eq!(
            payload.octets(),
            &[0x03, 0x0B, 0x00, 0x01, 0x23, 0x45, 0x01, 0x00, 0x00, 0x00]
        );
        let keep = data_relative_allocation(0x0001_2345, AllocationMode::Keep);
        assert_eq!(keep.octets()[6], 0x00, "keep clears Mode bit 0");
    }

    #[test]
    fn keep_is_the_default_mode_because_filling_destroys_content() {
        assert_eq!(AllocationMode::default(), AllocationMode::Keep);
    }

    #[test]
    fn relative_allocation_carries_the_requested_octet_count() {
        let payload = relative_allocation(2048);
        assert_eq!(
            payload.octets(),
            &[0x03, 0x0A, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn an_additional_load_control_always_starts_with_03_then_the_subtype() {
        for subtype in [
            LoadControlSubtype::AllocAbsDataSeg,
            LoadControlSubtype::TaskCtrl2,
            LoadControlSubtype::RelativeAllocation,
            LoadControlSubtype::DataRelativeAllocation,
        ] {
            let payload = additional_load_control(subtype, [0xFF; 8]);
            assert_eq!(payload.octets()[0], 0x03);
            assert_eq!(payload.octets()[1], subtype.octet());
            assert_eq!(payload.octets().len(), 10);
        }
    }

    /// Spec §7.3 rules 1 and 3: the subtype is chosen by mask, and the wrong
    /// mask is a refusal rather than a fallback.
    #[test]
    fn the_system_b_masks_get_data_relative_allocation() {
        for mask in [MASK_07B0, MASK_17B0, MASK_57B0] {
            assert_eq!(
                allocation_subtype_for(mask),
                Ok(LoadControlSubtype::DataRelativeAllocation),
                "{mask}"
            );
        }
    }

    #[test]
    fn mask_0300_gets_relative_allocation_and_not_the_system_b_subtype() {
        assert_eq!(
            allocation_subtype_for(MASK_0300),
            Ok(LoadControlSubtype::RelativeAllocation)
        );
    }

    #[test]
    fn an_unprofiled_mask_is_refused_rather_than_defaulted() {
        let err = allocation_subtype_for(MaskVersion(0x0705)).unwrap_err();
        assert_eq!(
            err,
            AllocationSubtypeError::MaskNotProfiled(MaskVersion(0x0705))
        );
        assert!(
            err.to_string().contains("0705h"),
            "the refusal must name the mask: {err}"
        );
    }

    /// §7.3 design rule 2, the sharpest case: absolute allocation on
    /// `57B0h` is `n/a`, so there is no fallback to it.
    #[test]
    fn absolute_allocation_on_57b0_is_refused_and_the_message_says_why() {
        for subtype in [
            LoadControlSubtype::AllocAbsDataSeg,
            LoadControlSubtype::AllocAbsStackSeg,
            LoadControlSubtype::AllocAbsTaskSeg,
            LoadControlSubtype::TaskPtr,
            LoadControlSubtype::TaskCtrl1,
            LoadControlSubtype::TaskCtrl2,
        ] {
            let err = require_subtype(MASK_57B0, subtype).unwrap_err();
            assert_eq!(
                err,
                AllocationSubtypeError::StyleNotApplicable {
                    mask: MASK_57B0,
                    subtype
                }
            );
            assert!(err.to_string().contains("not applicable"), "{err}");
        }
    }

    #[test]
    fn the_profiled_allocation_subtype_is_the_only_allocation_a_mask_accepts() {
        assert!(require_subtype(MASK_57B0, LoadControlSubtype::DataRelativeAllocation).is_ok());
        assert!(require_subtype(MASK_0300, LoadControlSubtype::RelativeAllocation).is_ok());
        // Crossing the two styles over is the bug rule 2 exists to stop.
        assert!(require_subtype(MASK_0300, LoadControlSubtype::DataRelativeAllocation).is_err());
        assert!(require_subtype(MASK_07B0, LoadControlSubtype::RelativeAllocation).is_err());
    }

    #[test]
    fn the_subtype_octets_are_the_values_the_clause_lists() {
        assert_eq!(LoadControlSubtype::AllocAbsDataSeg.octet(), 0x00);
        assert_eq!(LoadControlSubtype::AllocAbsStackSeg.octet(), 0x01);
        assert_eq!(LoadControlSubtype::AllocAbsTaskSeg.octet(), 0x02);
        assert_eq!(LoadControlSubtype::TaskPtr.octet(), 0x03);
        assert_eq!(LoadControlSubtype::TaskCtrl1.octet(), 0x04);
        assert_eq!(LoadControlSubtype::TaskCtrl2.octet(), 0x05);
        assert_eq!(LoadControlSubtype::RelativeAllocation.octet(), 0x0A);
        assert_eq!(LoadControlSubtype::DataRelativeAllocation.octet(), 0x0B);
    }

    #[test]
    fn a_payload_renders_as_hex_octets_for_a_log() {
        assert_eq!(
            event_payload(LoadEvent::StartLoading).to_string(),
            "01 00 00 00 00 00 00 00 00 00"
        );
    }
}
