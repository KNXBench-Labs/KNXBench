//! The step list of a mask-`070nh` application download, decided before anything is sent.
//!
//! A download of a BIM M112 device is driven by the product's own load
//! procedure (`LdCtrl*` steps, ADR-0044). This module holds what such a
//! procedure becomes once every value in it is known. It is pure data, so a
//! plan can be shown to the operator in full before it goes near a bus. The
//! executor lives in `knx-net`; turning a product file into a plan is
//! `knx-productdb`'s job.
//!
//! Where each step's meaning comes from:
//!
//! - `[D]` The order and the data writes follow *Configuration Procedures*
//!   (`03_05_03` v02.01.01) §3.9.2.2, the Standard's download of a BIM M112
//!   device (mask 5705h). Unload all three parts, then per part: `Load`,
//!   allocate the segment, write its data (`DMP_MemWrite_RCoV`), define the
//!   task segment, `LoadCompleted`. Restart at the end.
//! - `[D]` The records are MP §3.31.2's, built by
//!   [`super::load_control_memory`].
//! - `[D]` CP §3.9.2.2 checks the device with `DMP_Identify_RCo2(Manufacturer
//!   Code, Hardware Type)`; [`MemoryDownloadStep::Connect`] carries the
//!   manufacturer half, the product's `LdCtrlCompareProp` the hardware half.
//! - `[A]` How a property is compared with a longer `InlineData`:
//!   [`property_matches`].
//! - `[A]` Octets whose product `Mask` octet is not `FFh` are not written:
//!   [`unmasked_runs`].

use std::fmt;

use super::load_control_memory::{MemoryLoadRecord, MemoryLoadStateMachine};
use super::load_state::{LoadEvent, MaskVersion};

/// One step of a memory-mapped download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryDownloadStep {
    /// `LdCtrlConnect`: connect, then check that the device's mask and
    /// manufacturer are the plan's.
    Connect,
    /// `LdCtrlCompareProp`: read one property and compare it with the
    /// product's `InlineData` ([`property_matches`]).
    CompareProperty {
        /// `ObjIdx`.
        object_index: u8,
        /// `PropId`.
        property_id: u8,
        /// `InlineData`, verbatim.
        inline_data: Vec<u8>,
    },
    /// An `LdCtrlUnload`/`Load`/`AbsSegment`/`TaskSegment`/`LoadCompleted`
    /// record, written to `0104h`.
    LoadRecord(MemoryLoadRecord),
    /// Segment data: one run of octets the product lets the tool write.
    WriteMemory {
        /// The first address.
        address: u16,
        /// The octets, in address order.
        octets: Vec<u8>,
    },
    /// Read one machine's memory-mapped load state and stop unless it is
    /// `Loaded`. Never in a product procedure: a partial download
    /// ([`super::partial_memory_download`]) adds it, because it only
    /// replaces parts of an application already loaded.
    RequireLoaded(MemoryLoadStateMachine),
    /// `LdCtrlRestart`: a Basic Restart.
    Restart,
    /// `LdCtrlDisconnect`.
    Disconnect,
}

/// A complete download, every value resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryDownloadPlan {
    /// The product's `MaskVersion`; the device must report the same.
    pub mask: MaskVersion,
    /// The product's manufacturer; the device must report the same.
    pub manufacturer: u16,
    /// The steps, in order.
    pub steps: Vec<MemoryDownloadStep>,
}

impl MemoryDownloadStep {
    /// Whether running this step changes the device: a load-state event,
    /// segment data, or a restart. Connect, compare and disconnect only
    /// read or manage the connection. A run that stopped after a step for
    /// which this is `true` may have left the device partially written.
    pub fn changes_device(&self) -> bool {
        matches!(
            self,
            Self::LoadRecord(_) | Self::WriteMemory { .. } | Self::Restart
        )
    }
}

impl MemoryDownloadPlan {
    /// How many octets the plan writes as segment data.
    pub fn data_octets(&self) -> usize {
        self.steps
            .iter()
            .map(|step| match step {
                MemoryDownloadStep::WriteMemory { octets, .. } => octets.len(),
                _ => 0,
            })
            .sum()
    }
}

/// Whether a property value read from the device matches a product's
/// `InlineData`.
///
/// `[A]` No PDF read says how `LdCtrlCompareProp` compares. The data can be
/// longer than the property: `A-0027-15-0BAC` compares `PID_HARDWARE_TYPE`,
/// which is `PDT_GENERIC_06` (*Resources* §4.3.28), with ten octets
/// `00 00 00 00 01 27 00 00 00 00`. The device answers six. The rule here
/// accepts only the reading that leaves nothing unexplained: the device's
/// octets are the start of the data, and every octet after them is zero.
/// An empty answer never matches.
pub fn property_matches(device: &[u8], inline_data: &[u8]) -> bool {
    !device.is_empty()
        && device.len() <= inline_data.len()
        && inline_data[..device.len()] == *device
        && inline_data[device.len()..].iter().all(|&octet| octet == 0)
}

/// The runs of a segment the product lets the tool write, as `(offset,
/// octets)`.
///
/// `[A]` A `Mask` octet other than `FFh` marks an octet the tool leaves
/// alone (docs/RESEARCH.md §19.1–19.2). No mask means every octet is
/// written.
/// Octets past the end of a shorter mask count as masked.
pub fn unmasked_runs<'a>(octets: &'a [u8], mask: Option<&[u8]>) -> Vec<(usize, &'a [u8])> {
    let writable = |offset: usize| mask.is_none_or(|mask| mask.get(offset) == Some(&0xFF));
    let mut runs = Vec::new();
    let mut start = None;
    for offset in 0..=octets.len() {
        match (start, offset < octets.len() && writable(offset)) {
            (None, true) => start = Some(offset),
            (Some(first), false) => {
                runs.push((first, &octets[first..offset]));
                start = None;
            }
            _ => {}
        }
    }
    runs
}

impl fmt::Display for MemoryDownloadStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryDownloadStep::Connect => f.write_str("connect; check mask and manufacturer"),
            MemoryDownloadStep::CompareProperty {
                object_index,
                property_id,
                inline_data,
            } => write!(
                f,
                "compare property {object_index}/{property_id} with {}",
                hex(inline_data)
            ),
            MemoryDownloadStep::LoadRecord(record) => write!(
                f,
                "A_Memory_Write 0104h: {} ({} {})",
                hex(record.octets()),
                event_name(record.event()),
                record.machine()
            ),
            MemoryDownloadStep::WriteMemory { address, octets } => write!(
                f,
                "A_Memory_Write {address:04X}h..{:04X}h, {} octets",
                usize::from(*address) + octets.len().saturating_sub(1),
                octets.len()
            ),
            MemoryDownloadStep::RequireLoaded(machine) => {
                write!(f, "check that the {machine} is loaded")
            }
            MemoryDownloadStep::Restart => f.write_str("A_Restart (basic)"),
            MemoryDownloadStep::Disconnect => f.write_str("disconnect"),
        }
    }
}

fn event_name(event: LoadEvent) -> &'static str {
    match event {
        LoadEvent::NoOperation => "no operation",
        LoadEvent::StartLoading => "load",
        LoadEvent::LoadCompleted => "load completed",
        LoadEvent::AdditionalLoadControls => "segment",
        LoadEvent::Unload => "unload",
    }
}

fn hex(octets: &[u8]) -> String {
    octets
        .iter()
        .map(|octet| format!("{octet:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The machines a plan addresses, for a caller that wants to read their
/// states after the download.
pub fn machines(plan: &MemoryDownloadPlan) -> Vec<MemoryLoadStateMachine> {
    let mut found: Vec<MemoryLoadStateMachine> = plan
        .steps
        .iter()
        .filter_map(|step| match step {
            MemoryDownloadStep::LoadRecord(record) => Some(record.machine()),
            _ => None,
        })
        .collect();
    found.sort();
    found.dedup();
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commissioning::load_control_memory::event_record;

    #[test]
    fn only_load_records_data_and_restart_change_the_device() {
        assert!(!MemoryDownloadStep::Connect.changes_device());
        assert!(!MemoryDownloadStep::Disconnect.changes_device());
        assert!(!MemoryDownloadStep::CompareProperty {
            object_index: 0,
            property_id: 78,
            inline_data: vec![0]
        }
        .changes_device());
        assert!(MemoryDownloadStep::WriteMemory {
            address: 0x4000,
            octets: vec![1]
        }
        .changes_device());
        assert!(MemoryDownloadStep::Restart.changes_device());
    }

    #[test]
    fn the_device_octets_are_the_start_of_the_data_and_the_rest_is_zero() {
        let inline = [0, 0, 0, 0, 0x01, 0x27, 0, 0, 0, 0];
        assert!(property_matches(&[0, 0, 0, 0, 0x01, 0x27], &inline));
        assert!(property_matches(&inline, &inline), "the same length");
    }

    #[test]
    fn another_hardware_type_does_not_match() {
        let inline = [0, 0, 0, 0, 0x01, 0x27, 0, 0, 0, 0];
        assert!(!property_matches(&[0, 0, 0, 0, 0x02, 0x39], &inline));
        assert!(!property_matches(&[0, 0, 0, 0, 0x01, 0x26], &inline));
    }

    #[test]
    fn data_left_over_that_is_not_zero_does_not_match() {
        assert!(!property_matches(&[0x01, 0x27], &[0x01, 0x27, 0x00, 0x05]));
    }

    #[test]
    fn a_longer_or_empty_answer_does_not_match() {
        assert!(!property_matches(&[0x01, 0x27, 0x00], &[0x01, 0x27]));
        assert!(!property_matches(&[], &[0x00, 0x00]));
        assert!(!property_matches(&[], &[]));
    }

    #[test]
    fn without_a_mask_the_whole_segment_is_one_run() {
        let octets = [1, 2, 3];
        assert_eq!(unmasked_runs(&octets, None), vec![(0, &octets[..])]);
    }

    #[test]
    fn masked_octets_split_the_segment() {
        // The MDT address table: its octets 1-2 (the individual address)
        // are masked.
        let octets = [0x02, 0x11, 0x43, 0x10, 0x35, 0x00];
        let mask = [0xFF, 0x00, 0x00, 0xFF, 0xFF, 0xFF];
        assert_eq!(
            unmasked_runs(&octets, Some(&mask)),
            vec![(0, &octets[..1]), (3, &octets[3..])]
        );
    }

    #[test]
    fn a_mask_at_either_end_leaves_no_empty_run() {
        let octets = [1, 2, 3, 4];
        let mask = [0x00, 0xFF, 0xFF, 0x00];
        assert_eq!(
            unmasked_runs(&octets, Some(&mask)),
            vec![(1, &octets[1..3])]
        );
        assert_eq!(
            unmasked_runs(&octets, Some(&[0, 0, 0, 0])),
            Vec::<(usize, &[u8])>::new()
        );
    }

    #[test]
    fn a_mask_octet_that_is_neither_ff_nor_zero_protects_its_octet() {
        let octets = [1, 2, 3];
        let mask = [0xFF, 0x7F, 0xFF];
        assert_eq!(
            unmasked_runs(&octets, Some(&mask)),
            vec![(0, &octets[..1]), (2, &octets[2..])]
        );
    }

    #[test]
    fn a_short_mask_protects_what_it_does_not_cover() {
        let octets = [1, 2, 3];
        assert_eq!(
            unmasked_runs(&octets, Some(&[0xFF])),
            vec![(0, &octets[..1])]
        );
    }

    #[test]
    fn a_step_reads_as_what_goes_on_the_wire() {
        let unload = event_record(MemoryLoadStateMachine::AddressTable, LoadEvent::Unload)
            .expect("an event record");
        assert_eq!(
            MemoryDownloadStep::LoadRecord(unload).to_string(),
            "A_Memory_Write 0104h: 14 00 00 00 00 00 00 00 00 00 00 (unload address table)"
        );
        assert_eq!(
            MemoryDownloadStep::WriteMemory {
                address: 0x4003,
                octets: vec![0; 510],
            }
            .to_string(),
            "A_Memory_Write 4003h..4200h, 510 octets"
        );
        assert_eq!(
            MemoryDownloadStep::CompareProperty {
                object_index: 0,
                property_id: 78,
                inline_data: vec![0, 0x27],
            }
            .to_string(),
            "compare property 0/78 with 00 27"
        );
    }

    #[test]
    fn a_plan_counts_its_data_and_names_its_machines() {
        let record = |machine, event| {
            MemoryDownloadStep::LoadRecord(event_record(machine, event).expect("record"))
        };
        let plan = MemoryDownloadPlan {
            mask: MaskVersion(0x0701),
            manufacturer: 0x0083,
            steps: vec![
                MemoryDownloadStep::Connect,
                record(
                    MemoryLoadStateMachine::ApplicationProgram,
                    LoadEvent::Unload,
                ),
                record(MemoryLoadStateMachine::AddressTable, LoadEvent::Unload),
                MemoryDownloadStep::WriteMemory {
                    address: 0x4000,
                    octets: vec![1, 2],
                },
                MemoryDownloadStep::WriteMemory {
                    address: 0x4400,
                    octets: vec![3],
                },
                record(
                    MemoryLoadStateMachine::AddressTable,
                    LoadEvent::StartLoading,
                ),
            ],
        };
        assert_eq!(plan.data_octets(), 3);
        assert_eq!(
            machines(&plan),
            vec![
                MemoryLoadStateMachine::AddressTable,
                MemoryLoadStateMachine::ApplicationProgram
            ]
        );
    }
}
