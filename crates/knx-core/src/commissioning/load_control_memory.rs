//! The eleven-octet load records mask `070nh` takes at `0104h` instead of a property write.
//!
//! `[D]` MP §3.31.2 `DMP_LoadStateMachineWrite_RCo_Mem`: on the BIM M112
//! (mask `070nh`) the Load State Machines are *"located in the memory of the
//! Management Server"*. An event is one `A_Memory_Write` of `0Bh` octets to
//! the management control at `0104h`. The resulting state is read back as one
//! octet from a per-machine address (`B6EAh`–`B6EDh`).
//!
//! MP writes the first octet only as the symbols `L1`–`L4`, without values.
//! The values come from `[D]` *Test Suite Supplement G — Load State Machines
//! Tests* (`08_TSSG` v01.02.01 AS). It pairs every property event with its
//! memory-mapped twin: the first octet is `(machine type << 4) | event`, with
//! the machine type numbered as in `DM_LoadStateMachineWrite`'s
//! `stateMachineType` (MP §3.31.1) and the event as in RES Table 93. TSSG
//! contradicts itself on the record length: three of its data-segment records
//! carry a twelfth octet. This module follows MP's `0Bh`; see
//! `docs/RESEARCH.md` §19.
//!
//! Building a record is pure arithmetic and lives here. Sending one is a
//! write and belongs behind the mutation API, like
//! [`super::load_control`]'s ten-octet property payloads.

use std::fmt;

use super::load_state::{LoadEvent, MaskVersion};

/// `[D]` MP §3.31.2: *"The address of the management control is 0104h."*
pub const MANAGEMENT_CONTROL_ADDRESS: u16 = 0x0104;

/// `[D]` MP §3.31.2: `A_Memory_Write (addr = 0104h, length = 0Bh, …)`.
pub const MEMORY_LOAD_RECORD_OCTETS: usize = 11;

/// `[D]` MP §3.31.2: the client reads the load state back *"until loadstate
/// is correct (for max. 3 times)"*.
pub const LOAD_STATE_READ_ATTEMPTS: u8 = 3;

/// Whether a device of this mask takes load events through memory.
///
/// `[D]` MP §3.31.2: *"This Management Procedure shall only be used with
/// device model for mask version 070nh (BIM M112)."* `n` is the lowest
/// nibble; every other mask uses the property procedure (MP §3.31.3).
pub fn loads_through_memory(mask: MaskVersion) -> bool {
    mask.0 & 0xFFF0 == 0x0700
}

/// Which Load State Machine a record addresses.
///
/// `[D]` MP §3.31.1 numbers them `0001` to `0004` as `stateMachineType`, and
/// MP §3.31.2 gives each one its load-state address. The BIM M112 has only
/// one machine of each type (*"shall support only one state machine of each
/// type"*), so the type alone identifies it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MemoryLoadStateMachine {
    /// Type 1, load state at `B6EAh`.
    AddressTable,
    /// Type 2, load state at `B6EBh`.
    AssociationTable,
    /// Type 3, load state at `B6ECh`.
    ApplicationProgram,
    /// Type 4, load state at `B6EDh`.
    PeiProgram,
}

impl MemoryLoadStateMachine {
    /// Every machine, for exhaustive tests.
    pub const ALL: [MemoryLoadStateMachine; 4] = [
        MemoryLoadStateMachine::AddressTable,
        MemoryLoadStateMachine::AssociationTable,
        MemoryLoadStateMachine::ApplicationProgram,
        MemoryLoadStateMachine::PeiProgram,
    ];

    /// `stateMachineType` from MP §3.31.1, the upper nibble of octet 0.
    pub fn type_number(self) -> u8 {
        match self {
            MemoryLoadStateMachine::AddressTable => 1,
            MemoryLoadStateMachine::AssociationTable => 2,
            MemoryLoadStateMachine::ApplicationProgram => 3,
            MemoryLoadStateMachine::PeiProgram => 4,
        }
    }

    /// The address MP §3.31.2 names for this machine's load state.
    pub fn load_state_address(self) -> u16 {
        match self {
            MemoryLoadStateMachine::AddressTable => 0xB6EA,
            MemoryLoadStateMachine::AssociationTable => 0xB6EB,
            MemoryLoadStateMachine::ApplicationProgram => 0xB6EC,
            MemoryLoadStateMachine::PeiProgram => 0xB6ED,
        }
    }

    /// Octet 0 of a record: the machine type in the upper nibble, the
    /// RES Table 93 event in the lower one.
    fn first_octet(self, event: LoadEvent) -> u8 {
        (self.type_number() << 4) | event.octet()
    }

    /// The machine for an `LdCtrl*` `LsmIdx`. The product schema numbers the
    /// machines with the same numbers as `stateMachineType`; that
    /// correspondence is `[A]`. The corpus (`A-0027-15-0BAC`) puts the
    /// address table's segment `AS-4000` under `LsmIdx="1"` and the
    /// association table's `AS-4201` under `LsmIdx="2"`, which agrees with it.
    pub fn from_lsm_index(index: u8) -> Option<MemoryLoadStateMachine> {
        MemoryLoadStateMachine::ALL
            .into_iter()
            .find(|machine| machine.type_number() == index)
    }
}

impl fmt::Display for MemoryLoadStateMachine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            MemoryLoadStateMachine::AddressTable => "address table",
            MemoryLoadStateMachine::AssociationTable => "association table",
            MemoryLoadStateMachine::ApplicationProgram => "application program",
            MemoryLoadStateMachine::PeiProgram => "PEI program",
        };
        f.write_str(name)
    }
}

/// An eleven-octet record for `0104h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLoadRecord([u8; MEMORY_LOAD_RECORD_OCTETS]);

impl MemoryLoadRecord {
    /// The octets, in wire order.
    pub fn octets(&self) -> &[u8; MEMORY_LOAD_RECORD_OCTETS] {
        &self.0
    }

    /// The machine octet 0 addresses.
    pub fn machine(&self) -> MemoryLoadStateMachine {
        MemoryLoadStateMachine::from_lsm_index(self.0[0] >> 4)
            .expect("a record is only built by this module, always with a machine type of 1-4")
    }

    /// The RES Table 93 event octet 0 carries.
    pub fn event(&self) -> LoadEvent {
        let event = self.0[0] & 0x0F;
        LoadEvent::ALL
            .into_iter()
            .find(|candidate| candidate.octet() == event)
            .expect("a record is only built by this module, always with a RES Table 93 event")
    }
}

impl fmt::Display for MemoryLoadRecord {
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

/// The memory type octet of an absolute segment. `[D]` MP §3.31.2: bits 0–2
/// are the type (*"1 Zero page RAM, 2 RAM, 3 EEPROM"*), and bits 3–7 are
/// *"Reserved. Shall be zero"*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SegmentMemoryType {
    /// `1`.
    ZeroPageRam,
    /// `2`.
    Ram,
    /// `3`.
    Eeprom,
}

impl SegmentMemoryType {
    /// The octet, reserved bits zero.
    pub fn octet(self) -> u8 {
        match self {
            SegmentMemoryType::ZeroPageRam => 1,
            SegmentMemoryType::Ram => 2,
            SegmentMemoryType::Eeprom => 3,
        }
    }

    /// Decodes a product database `MemType`. Anything outside 1–3 is refused:
    /// MP defines no other type, and the reserved bits must be zero.
    pub fn from_octet(octet: u8) -> Option<SegmentMemoryType> {
        match octet {
            1 => Some(SegmentMemoryType::ZeroPageRam),
            2 => Some(SegmentMemoryType::Ram),
            3 => Some(SegmentMemoryType::Eeprom),
            _ => None,
        }
    }
}

/// An `AllocAbsDataSeg` (segment type 0) or `AllocAbsStackSeg` (type 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AbsoluteSegment {
    /// `SSSS`, the first address of the segment.
    pub start: u16,
    /// The segment length. MP writes the field as `EEEE - SSSS + 1`, so a
    /// segment is at least one octet long and must end at or below `FFFFh`.
    pub length: u16,
    /// `AA`: bits 0–3 write access level, bits 4–7 read access level.
    pub access: u8,
    /// `TT`.
    pub memory_type: SegmentMemoryType,
    /// `MM` bit 7: *"Checksum control enabled"*. Bits 0–6 are reserved and
    /// always sent as zero.
    pub checksum_control: bool,
}

/// An `AllocAbsTaskSeg` (segment type 2): the segment's start, the PEI type,
/// and the 5-octet application identity `MM MM TT TT VV`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskSegment {
    /// `SSSS`.
    pub start: u16,
    /// `PP`.
    pub pei_type: u8,
    /// `MM MM`, the software manufacturer.
    pub manufacturer: u16,
    /// `TT TT`, the manufacturer-specific application software id.
    pub application: u16,
    /// `VV`.
    pub version: u8,
}

/// Why a record was not built. Every refusal names the rule it enforces, so
/// nothing is silently clamped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryLoadRecordError {
    /// `Additional Load Controls` needs a segment. Use
    /// [`abs_data_segment`], [`abs_stack_segment`] or [`abs_task_segment`].
    EventNeedsSegment,
    /// The segment would be empty. `EEEE - SSSS + 1` is at least 1.
    EmptySegment,
    /// The segment runs past `FFFFh`, which a 2-octet `EEEE` cannot express.
    SegmentPastAddressSpace { start: u16, length: u16 },
    /// `[D]` MP §3.31.1's event table: `AllocAbsStackSeg` exists only for the
    /// application program and the PEI program.
    StackSegmentNotAllowed(MemoryLoadStateMachine),
}

impl std::error::Error for MemoryLoadRecordError {}

impl fmt::Display for MemoryLoadRecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryLoadRecordError::EventNeedsSegment => {
                f.write_str("Additional Load Controls needs a segment record, not a bare event")
            }
            MemoryLoadRecordError::EmptySegment => f.write_str("a segment of zero octets"),
            MemoryLoadRecordError::SegmentPastAddressSpace { start, length } => write!(
                f,
                "a segment of {length} octets at {start:04X}h runs past FFFFh"
            ),
            MemoryLoadRecordError::StackSegmentNotAllowed(machine) => write!(
                f,
                "MP §3.31.1 allows AllocAbsStackSeg only for the application and PEI programs, not the {machine}"
            ),
        }
    }
}

/// The record for a plain event: `Unload` (`L4`), `Start Loading` (`L1`),
/// `Load Completed` (`L2`) or `No Operation`, then ten reserved `00h`.
pub fn event_record(
    machine: MemoryLoadStateMachine,
    event: LoadEvent,
) -> Result<MemoryLoadRecord, MemoryLoadRecordError> {
    if event == LoadEvent::AdditionalLoadControls {
        return Err(MemoryLoadRecordError::EventNeedsSegment);
    }
    let mut octets = [0u8; MEMORY_LOAD_RECORD_OCTETS];
    octets[0] = machine.first_octet(event);
    Ok(MemoryLoadRecord(octets))
}

/// `AllocAbsDataSeg`: `L3 00h 00h SSSS LLLL AA TT MM 00h`.
pub fn abs_data_segment(
    machine: MemoryLoadStateMachine,
    segment: AbsoluteSegment,
) -> Result<MemoryLoadRecord, MemoryLoadRecordError> {
    absolute_segment_record(machine, SEGMENT_TYPE_DATA, segment)
}

/// `AllocAbsStackSeg`: the same layout as a data segment, segment type `01h`.
pub fn abs_stack_segment(
    machine: MemoryLoadStateMachine,
    segment: AbsoluteSegment,
) -> Result<MemoryLoadRecord, MemoryLoadRecordError> {
    if matches!(
        machine,
        MemoryLoadStateMachine::AddressTable | MemoryLoadStateMachine::AssociationTable
    ) {
        return Err(MemoryLoadRecordError::StackSegmentNotAllowed(machine));
    }
    absolute_segment_record(machine, SEGMENT_TYPE_STACK, segment)
}

/// `AllocAbsTaskSeg`: `L3 02h 00h SSSS PP MMMM TTTT VV`.
pub fn abs_task_segment(machine: MemoryLoadStateMachine, task: TaskSegment) -> MemoryLoadRecord {
    let start = task.start.to_be_bytes();
    let manufacturer = task.manufacturer.to_be_bytes();
    let application = task.application.to_be_bytes();
    MemoryLoadRecord([
        machine.first_octet(LoadEvent::AdditionalLoadControls),
        SEGMENT_TYPE_TASK,
        SEGMENT_ID,
        start[0],
        start[1],
        task.pei_type,
        manufacturer[0],
        manufacturer[1],
        application[0],
        application[1],
        task.version,
    ])
}

/// Segment type octets, `[D]` MP §3.31.2's `LoadEvent` headings.
const SEGMENT_TYPE_DATA: u8 = 0x00;
const SEGMENT_TYPE_STACK: u8 = 0x01;
const SEGMENT_TYPE_TASK: u8 = 0x02;

/// `[D]` MP §3.31.2: the segment ID is `00h` in every record it shows; the
/// procedure *"shall support only one state machine of each type"*.
const SEGMENT_ID: u8 = 0x00;

/// `[D]` MP §3.31.2: memory attributes bit 7, *"Checksum control enabled"*.
const CHECKSUM_CONTROL: u8 = 0x80;

fn absolute_segment_record(
    machine: MemoryLoadStateMachine,
    segment_type: u8,
    segment: AbsoluteSegment,
) -> Result<MemoryLoadRecord, MemoryLoadRecordError> {
    if segment.length == 0 {
        return Err(MemoryLoadRecordError::EmptySegment);
    }
    let end = u32::from(segment.start) + u32::from(segment.length) - 1;
    if end > u32::from(u16::MAX) {
        return Err(MemoryLoadRecordError::SegmentPastAddressSpace {
            start: segment.start,
            length: segment.length,
        });
    }
    let start = segment.start.to_be_bytes();
    let length = segment.length.to_be_bytes();
    let attributes = if segment.checksum_control {
        CHECKSUM_CONTROL
    } else {
        0
    };
    Ok(MemoryLoadRecord([
        machine.first_octet(LoadEvent::AdditionalLoadControls),
        segment_type,
        SEGMENT_ID,
        start[0],
        start[1],
        length[0],
        length[1],
        segment.access,
        segment.memory_type.octet(),
        attributes,
        0x00,
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(record: &MemoryLoadRecord) -> String {
        record.to_string()
    }

    /// `[D]` TSSG §2.1: the four unload records, `14h` `24h` `34h` `44h`, and
    /// the four read-back addresses `B6EAh`..`B6EDh` paired with them.
    #[test]
    fn unload_is_the_machine_type_over_event_four_then_ten_zeroes() {
        let expected = [
            (MemoryLoadStateMachine::AddressTable, 0x14, 0xB6EA),
            (MemoryLoadStateMachine::AssociationTable, 0x24, 0xB6EB),
            (MemoryLoadStateMachine::ApplicationProgram, 0x34, 0xB6EC),
            (MemoryLoadStateMachine::PeiProgram, 0x44, 0xB6ED),
        ];
        for (machine, first, state_address) in expected {
            let record = event_record(machine, LoadEvent::Unload).unwrap();
            let mut octets = [0u8; MEMORY_LOAD_RECORD_OCTETS];
            octets[0] = first;
            assert_eq!(record.octets(), &octets, "{machine}");
            assert_eq!(machine.load_state_address(), state_address, "{machine}");
        }
    }

    /// `[D]` TSSG §2.2: `21h` start loading, `22h` load completed and `20h`
    /// no operation on the association table.
    #[test]
    fn the_lower_nibble_is_the_res_table_93_event() {
        let machine = MemoryLoadStateMachine::AssociationTable;
        let first = |event| event_record(machine, event).unwrap().octets()[0];
        assert_eq!(first(LoadEvent::StartLoading), 0x21);
        assert_eq!(first(LoadEvent::LoadCompleted), 0x22);
        assert_eq!(first(LoadEvent::NoOperation), 0x20);
    }

    #[test]
    fn only_masks_070n_load_through_memory() {
        for mask in [0x0700, 0x0701, 0x0705, 0x070F] {
            assert!(loads_through_memory(MaskVersion(mask)), "{mask:04X}h");
        }
        for mask in [0x07B0, 0x0710, 0x0012, 0x0705 | 0x1000, 0x5705, 0x0000] {
            assert!(!loads_through_memory(MaskVersion(mask)), "{mask:04X}h");
        }
    }

    #[test]
    fn a_record_names_its_machine_and_event() {
        let unload = event_record(MemoryLoadStateMachine::PeiProgram, LoadEvent::Unload).unwrap();
        assert_eq!(unload.machine(), MemoryLoadStateMachine::PeiProgram);
        assert_eq!(unload.event(), LoadEvent::Unload);
        let task = abs_task_segment(
            MemoryLoadStateMachine::ApplicationProgram,
            TaskSegment {
                start: 0x4400,
                pei_type: 1,
                manufacturer: 0x0083,
                application: 0x0027,
                version: 0x15,
            },
        );
        assert_eq!(task.machine(), MemoryLoadStateMachine::ApplicationProgram);
        assert_eq!(task.event(), LoadEvent::AdditionalLoadControls);
    }

    #[test]
    fn a_bare_additional_load_control_is_refused() {
        assert_eq!(
            event_record(
                MemoryLoadStateMachine::ApplicationProgram,
                LoadEvent::AdditionalLoadControls
            ),
            Err(MemoryLoadRecordError::EventNeedsSegment)
        );
    }

    /// `[D]` TSSG §2.3.4, the eleven-octet form:
    /// `23 00 00 42 00 00 10 FF 03 80 00` — start `4200h`, length `0010h`,
    /// no privilege, EEPROM, checksum on.
    #[test]
    fn a_data_segment_matches_the_tssg_vector() {
        let record = abs_data_segment(
            MemoryLoadStateMachine::AssociationTable,
            AbsoluteSegment {
                start: 0x4200,
                length: 0x0010,
                access: 0xFF,
                memory_type: SegmentMemoryType::Eeprom,
                checksum_control: true,
            },
        )
        .unwrap();
        assert_eq!(hex(&record), "23 00 00 42 00 00 10 FF 03 80 00");
    }

    /// `[D]` TSSG §2.3.4: `23 02 00 42 00 80 00 02 A0 4A 10` — start `4200h`,
    /// PEI type `80h`, manufacturer `0002h`, application `A04Ah`, version
    /// `10h`. (TSSG's prose annotation reads `0A4Ah`; the bytes are
    /// authoritative.)
    #[test]
    fn a_task_segment_matches_the_tssg_vector() {
        let record = abs_task_segment(
            MemoryLoadStateMachine::AssociationTable,
            TaskSegment {
                start: 0x4200,
                pei_type: 0x80,
                manufacturer: 0x0002,
                application: 0xA04A,
                version: 0x10,
            },
        );
        assert_eq!(hex(&record), "23 02 00 42 00 80 00 02 A0 4A 10");
    }

    /// `A-0027-15-0BAC`'s stack segment:
    /// `LdCtrlAbsSegment LsmIdx="3" SegType="1" Address="1944" Size="1"
    /// Access="0" MemType="2" SegFlags="0"`.
    #[test]
    fn a_stack_segment_is_segment_type_one() {
        let record = abs_stack_segment(
            MemoryLoadStateMachine::ApplicationProgram,
            AbsoluteSegment {
                start: 0x0798,
                length: 1,
                access: 0x00,
                memory_type: SegmentMemoryType::Ram,
                checksum_control: false,
            },
        )
        .unwrap();
        assert_eq!(hex(&record), "33 01 00 07 98 00 01 00 02 00 00");
    }

    #[test]
    fn a_stack_segment_on_a_table_machine_is_refused() {
        let segment = AbsoluteSegment {
            start: 0x0798,
            length: 1,
            access: 0,
            memory_type: SegmentMemoryType::Ram,
            checksum_control: false,
        };
        for machine in [
            MemoryLoadStateMachine::AddressTable,
            MemoryLoadStateMachine::AssociationTable,
        ] {
            assert_eq!(
                abs_stack_segment(machine, segment),
                Err(MemoryLoadRecordError::StackSegmentNotAllowed(machine))
            );
        }
        assert!(abs_stack_segment(MemoryLoadStateMachine::PeiProgram, segment).is_ok());
    }

    #[test]
    fn an_empty_segment_is_refused() {
        let segment = AbsoluteSegment {
            start: 0x4000,
            length: 0,
            access: 0xFF,
            memory_type: SegmentMemoryType::Eeprom,
            checksum_control: true,
        };
        assert_eq!(
            abs_data_segment(MemoryLoadStateMachine::AddressTable, segment),
            Err(MemoryLoadRecordError::EmptySegment)
        );
    }

    #[test]
    fn a_segment_ending_exactly_at_ffff_is_accepted_and_one_past_is_refused() {
        let at = |start, length| AbsoluteSegment {
            start,
            length,
            access: 0xFF,
            memory_type: SegmentMemoryType::Eeprom,
            checksum_control: false,
        };
        let machine = MemoryLoadStateMachine::ApplicationProgram;
        assert!(abs_data_segment(machine, at(0xFF00, 0x0100)).is_ok());
        assert_eq!(
            abs_data_segment(machine, at(0xFF00, 0x0101)),
            Err(MemoryLoadRecordError::SegmentPastAddressSpace {
                start: 0xFF00,
                length: 0x0101
            })
        );
    }

    #[test]
    fn lsm_indices_one_to_four_map_and_nothing_else_does() {
        assert_eq!(
            MemoryLoadStateMachine::ALL
                .map(|m| MemoryLoadStateMachine::from_lsm_index(m.type_number())),
            MemoryLoadStateMachine::ALL.map(Some)
        );
        assert_eq!(MemoryLoadStateMachine::from_lsm_index(0), None);
        assert_eq!(MemoryLoadStateMachine::from_lsm_index(5), None);
    }

    #[test]
    fn memory_types_outside_one_to_three_are_refused() {
        assert_eq!(SegmentMemoryType::from_octet(0), None);
        assert_eq!(SegmentMemoryType::from_octet(4), None);
        assert_eq!(SegmentMemoryType::from_octet(0x83), None);
        assert_eq!(
            SegmentMemoryType::from_octet(3),
            Some(SegmentMemoryType::Eeprom)
        );
    }
}
