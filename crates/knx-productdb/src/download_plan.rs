//! Turns a download image and its product's load procedure into a memory download plan.
//!
//! The plan is [`knx_core::commissioning::memory_download`]'s: every record,
//! every data write and every check, decided before anything is sent. This
//! module only translates; it neither talks to a device nor chooses values.
//!
//! Where each rule comes from:
//!
//! - `[D]` *Configuration Procedures* (`03_05_03` v02.01.01) §3.9.2.2.2,
//!   pp. 67–68, the Standard's download of a BIM M112 device: a segment's
//!   data is written right after its allocation (`DMP_MemWrite_RCoV`) and
//!   before its task segment. The product's own `LdCtrlAbsSegment` is where
//!   that allocation happens, so the data write follows it.
//! - `[D]` The same procedure gives the task segment of the address and
//!   association tables `peitype=00h, appl_id=0000/0000/00`, and the
//!   application program's the PEI type and the *"Manufacturer Code, Device
//!   Type, Version"*.
//! - `[A]` The application identity comes from the program's
//!   `ApplicationNumber`, `ApplicationVersion` and `PeiType` attributes and
//!   the manufacturer from its `M-hhhh_` id prefix. `1.1.67` reported
//!   `00 83 00 27 15` for `A-0027-15-0BAC` (`ApplicationNumber="39"`,
//!   `ApplicationVersion="21"`), which agrees.
//! - `[A]` A segment the product masks keeps its masked octets: they are not
//!   written ([`unmasked_runs`]).
//!
//! Anything the translation does not understand is refused by name. A plan
//! that silently skipped a step would download something other than what
//! the product describes.

use std::collections::BTreeSet;
use std::fmt;

use knx_core::commissioning::load_control_memory::{
    abs_data_segment, abs_stack_segment, abs_task_segment, event_record, loads_through_memory,
    AbsoluteSegment as SegmentRecord, MemoryLoadRecordError, MemoryLoadStateMachine,
    SegmentMemoryType, TaskSegment,
};
use knx_core::commissioning::load_state::{LoadEvent, MaskVersion};
use knx_core::commissioning::memory_download::{
    unmasked_runs, MemoryDownloadPlan, MemoryDownloadStep,
};

use crate::code::{LoadStep, ProgramCode};
use crate::image::DownloadImage;

/// The `LoadProcedureStyle` this module translates.
const PRODUCT_PROCEDURE: &str = "ProductProcedure";
/// `SegType` of an absolute data segment and of a stack segment.
const SEGMENT_TYPE_DATA: u8 = 0;
const SEGMENT_TYPE_STACK: u8 = 1;
/// `SegFlags` bit 7, *"Checksum control enabled"* (MP §3.31.2). The other
/// bits are reserved.
const SEGMENT_FLAG_CHECKSUM: u8 = 0x80;

/// Why a plan was not built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadPlanError {
    /// The program's `LoadProcedureStyle` is not `ProductProcedure`.
    NotAProductProcedure(Option<String>),
    /// The program's mask does not load through memory (MP §3.31.2).
    NotMemoryMapped(Option<String>),
    /// A program attribute the task segment needs is missing or malformed.
    Attribute {
        /// The attribute.
        name: &'static str,
        /// What was there.
        value: Option<String>,
    },
    /// No manufacturer could be read from the program id.
    ProgramId(String),
    /// The program has no single load procedure without a merge id.
    ProcedureCount(usize),
    /// The procedure does not begin with `LdCtrlConnect`.
    DoesNotConnectFirst,
    /// A step this module does not translate.
    Unmodelled(String),
    /// An `LsmIdx` that names no memory-mapped machine, or one this module
    /// does not load.
    Lsm(u8),
    /// A `SegType` other than data or stack.
    SegmentType(u8),
    /// A `MemType` MP §3.31.2 does not define.
    MemoryType(u8),
    /// `SegFlags` with a reserved bit set.
    SegmentFlags(u8),
    /// An address that does not fit a BIM M112's 16-bit memory.
    Address(u32),
    /// The record builder refused.
    Record(MemoryLoadRecordError),
    /// The product allocates a different size than its segment has.
    SizeMismatch {
        /// The segment.
        address: u16,
        /// The procedure's `Size`.
        allocated: u16,
        /// The segment's octets.
        octets: usize,
    },
    /// A segment with data that the procedure never allocates.
    NeverWritten(String),
    /// A segment the procedure allocates twice.
    AllocatedTwice(u16),
}

impl std::error::Error for DownloadPlanError {}

impl fmt::Display for DownloadPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadPlanError::NotAProductProcedure(style) => write!(
                f,
                "LoadProcedureStyle {style:?} is not {PRODUCT_PROCEDURE}; no other style is translated"
            ),
            DownloadPlanError::NotMemoryMapped(mask) => {
                write!(f, "mask {mask:?} does not load through memory (MP §3.31.2)")
            }
            DownloadPlanError::Attribute { name, value } => {
                write!(f, "program attribute {name} is {value:?}")
            }
            DownloadPlanError::ProgramId(id) => write!(f, "no manufacturer in program id {id}"),
            DownloadPlanError::ProcedureCount(count) => write!(
                f,
                "{count} load procedures without a merge id; exactly one is translated"
            ),
            DownloadPlanError::DoesNotConnectFirst => {
                f.write_str("the load procedure does not begin with LdCtrlConnect")
            }
            DownloadPlanError::Unmodelled(name) => write!(f, "{name} is not translated"),
            DownloadPlanError::Lsm(index) => write!(f, "LsmIdx {index} is not loaded here"),
            DownloadPlanError::SegmentType(kind) => write!(f, "SegType {kind} is not translated"),
            DownloadPlanError::MemoryType(kind) => write!(f, "MemType {kind} is not defined"),
            DownloadPlanError::SegmentFlags(flags) => {
                write!(f, "SegFlags {flags:02X}h sets a reserved bit")
            }
            DownloadPlanError::Address(address) => {
                write!(f, "address {address:X}h does not fit 16 bits")
            }
            DownloadPlanError::Record(error) => write!(f, "{error}"),
            DownloadPlanError::SizeMismatch {
                address,
                allocated,
                octets,
            } => write!(
                f,
                "the segment at {address:04X}h is allocated as {allocated} octets but has {octets}"
            ),
            DownloadPlanError::NeverWritten(id) => {
                write!(f, "segment {id} has data but the procedure never allocates it")
            }
            DownloadPlanError::AllocatedTwice(address) => {
                write!(f, "the segment at {address:04X}h is allocated twice")
            }
        }
    }
}

impl From<MemoryLoadRecordError> for DownloadPlanError {
    fn from(error: MemoryLoadRecordError) -> Self {
        DownloadPlanError::Record(error)
    }
}

/// Refuses a program whose kind this module never translates: a mask that
/// does not load through memory, or another `LoadProcedureStyle`.
/// Cheap, and the first thing to ask: an image of such a program is not
/// worth building, and its refusal would name a symptom instead of this.
///
/// The mask is asked first. `[V]` Every `MergedProcedure` and
/// `DefaultProcedure` program in the local corpus (41 of 246) has a mask
/// that does not load through memory (`07B0h`, `0912h`, `091Ah`, `2920h`,
/// `0012h`, `0001h`); none is a `070nh` program. Naming the style first
/// would point at a translation that, once written, still could not load
/// such a device, so the mask is the reason that holds.
pub fn check_program_kind(code: &ProgramCode) -> Result<MaskVersion, DownloadPlanError> {
    let mask = code
        .mask_version
        .as_deref()
        .and_then(parse_mask)
        .filter(|mask| loads_through_memory(*mask))
        .ok_or_else(|| DownloadPlanError::NotMemoryMapped(code.mask_version.clone()))?;
    if code.load_procedure_style.as_deref() != Some(PRODUCT_PROCEDURE) {
        return Err(DownloadPlanError::NotAProductProcedure(
            code.load_procedure_style.clone(),
        ));
    }
    Ok(mask)
}

/// Builds the plan for `image`.
pub fn plan_memory_download(
    image: &DownloadImage,
) -> Result<MemoryDownloadPlan, DownloadPlanError> {
    let code = &image.code;
    let mask = check_program_kind(code)?;
    let manufacturer = manufacturer_of(&code.program_id)?;
    let application = TaskSegment {
        start: 0,
        pei_type: attribute(code, "PeiType")?,
        manufacturer,
        application: attribute(code, "ApplicationNumber")?,
        version: attribute(code, "ApplicationVersion")?,
    };

    let procedures: Vec<_> = code
        .load_procedures
        .iter()
        .filter(|procedure| procedure.merge_id.is_none())
        .collect();
    let [procedure] = procedures[..] else {
        return Err(DownloadPlanError::ProcedureCount(procedures.len()));
    };
    if procedure.steps.first() != Some(&LoadStep::Connect) {
        return Err(DownloadPlanError::DoesNotConnectFirst);
    }

    let mut steps = Vec::new();
    let mut allocated = BTreeSet::new();
    for step in &procedure.steps {
        match step {
            LoadStep::Connect => steps.push(MemoryDownloadStep::Connect),
            LoadStep::Disconnect => steps.push(MemoryDownloadStep::Disconnect),
            LoadStep::Restart => steps.push(MemoryDownloadStep::Restart),
            LoadStep::CompareProp {
                object_index,
                property_id,
                data,
            } => steps.push(MemoryDownloadStep::CompareProperty {
                object_index: *object_index,
                property_id: *property_id,
                inline_data: data.clone(),
            }),
            LoadStep::Unload { lsm } => steps.push(event(*lsm, LoadEvent::Unload)?),
            LoadStep::Load { lsm } => steps.push(event(*lsm, LoadEvent::StartLoading)?),
            LoadStep::LoadCompleted { lsm } => steps.push(event(*lsm, LoadEvent::LoadCompleted)?),
            LoadStep::AbsSegment {
                lsm,
                segment_type,
                address,
                size,
                access,
                memory_type,
                flags,
            } => {
                let machine = machine(*lsm)?;
                let segment = SegmentRecord {
                    start: *address,
                    length: *size,
                    access: *access,
                    memory_type: SegmentMemoryType::from_octet(*memory_type)
                        .ok_or(DownloadPlanError::MemoryType(*memory_type))?,
                    checksum_control: checksum_control(*flags)?,
                };
                let record = match *segment_type {
                    SEGMENT_TYPE_DATA => abs_data_segment(machine, segment)?,
                    SEGMENT_TYPE_STACK => abs_stack_segment(machine, segment)?,
                    other => return Err(DownloadPlanError::SegmentType(other)),
                };
                if !allocated.insert(*address) {
                    return Err(DownloadPlanError::AllocatedTwice(*address));
                }
                steps.push(MemoryDownloadStep::LoadRecord(record));
                steps.extend(data_writes(image, *address, *size)?);
            }
            LoadStep::TaskSegment { lsm, address } => {
                let machine = machine(*lsm)?;
                let task = match machine {
                    MemoryLoadStateMachine::ApplicationProgram => TaskSegment {
                        start: *address,
                        ..application
                    },
                    // CP §3.9.2.2.2: *"peitype=00h, appl_id=0000/0000/00"*.
                    _ => TaskSegment {
                        start: *address,
                        pei_type: 0,
                        manufacturer: 0,
                        application: 0,
                        version: 0,
                    },
                };
                steps.push(MemoryDownloadStep::LoadRecord(abs_task_segment(
                    machine, task,
                )));
            }
            LoadStep::Unmodelled { name, .. } => {
                return Err(DownloadPlanError::Unmodelled(name.clone()))
            }
        }
    }

    for segment in &image.segments {
        let address = u16::try_from(segment.address)
            .map_err(|_| DownloadPlanError::Address(segment.address))?;
        if !allocated.contains(&address) {
            return Err(DownloadPlanError::NeverWritten(segment.id.clone()));
        }
    }

    Ok(MemoryDownloadPlan {
        mask,
        manufacturer,
        steps,
    })
}

/// The writes for the segment allocated at `address`, if the image has one
/// there.
fn data_writes(
    image: &DownloadImage,
    address: u16,
    size: u16,
) -> Result<Vec<MemoryDownloadStep>, DownloadPlanError> {
    let Some(segment) = image
        .segments
        .iter()
        .find(|segment| segment.address == u32::from(address))
    else {
        return Ok(Vec::new());
    };
    if segment.octets.len() != usize::from(size) {
        return Err(DownloadPlanError::SizeMismatch {
            address,
            allocated: size,
            octets: segment.octets.len(),
        });
    }
    unmasked_runs(&segment.octets, segment.mask.as_deref())
        .into_iter()
        .map(|(offset, octets)| {
            // `offset < size`, and `address + size - 1` fits 16 bits or the
            // record builder above would have refused the allocation.
            let start = u32::from(address) + offset as u32;
            Ok(MemoryDownloadStep::WriteMemory {
                address: u16::try_from(start).map_err(|_| DownloadPlanError::Address(start))?,
                octets: octets.to_vec(),
            })
        })
        .collect()
}

fn event(lsm: u8, event: LoadEvent) -> Result<MemoryDownloadStep, DownloadPlanError> {
    Ok(MemoryDownloadStep::LoadRecord(event_record(
        machine(lsm)?,
        event,
    )?))
}

/// The machine for `lsm`. The PEI program's machine exists on a BIM M112,
/// but nothing here loads a PEI program, so it is refused rather than
/// driven with records nobody has checked.
fn machine(lsm: u8) -> Result<MemoryLoadStateMachine, DownloadPlanError> {
    match MemoryLoadStateMachine::from_lsm_index(lsm) {
        Some(MemoryLoadStateMachine::PeiProgram) | None => Err(DownloadPlanError::Lsm(lsm)),
        Some(machine) => Ok(machine),
    }
}

fn checksum_control(flags: u8) -> Result<bool, DownloadPlanError> {
    if flags & !SEGMENT_FLAG_CHECKSUM != 0 {
        return Err(DownloadPlanError::SegmentFlags(flags));
    }
    Ok(flags & SEGMENT_FLAG_CHECKSUM != 0)
}

/// `MV-0701` → `0701h`.
fn parse_mask(text: &str) -> Option<MaskVersion> {
    let digits = text.strip_prefix("MV-")?;
    (digits.len() == 4)
        .then(|| u16::from_str_radix(digits, 16).ok())
        .flatten()
        .map(MaskVersion)
}

/// `M-0083_A-…` → `0083h`.
fn manufacturer_of(program_id: &str) -> Result<u16, DownloadPlanError> {
    program_id
        .strip_prefix("M-")
        .and_then(|rest| rest.split('_').next())
        .filter(|digits| digits.len() == 4)
        .and_then(|digits| u16::from_str_radix(digits, 16).ok())
        .ok_or_else(|| DownloadPlanError::ProgramId(program_id.to_string()))
}

fn attribute<N: TryFrom<u32>>(
    code: &crate::code::ProgramCode,
    name: &'static str,
) -> Result<N, DownloadPlanError> {
    let value = code.program_attributes.get(name);
    value
        .and_then(|text| text.parse::<u32>().ok())
        .and_then(|number| N::try_from(number).ok())
        .ok_or_else(|| DownloadPlanError::Attribute {
            name,
            value: value.cloned(),
        })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::code::{LoadProcedure, ProgramCode};
    use crate::image::SegmentImage;

    fn abs(lsm: u8, address: u16, size: u16) -> LoadStep {
        LoadStep::AbsSegment {
            lsm,
            segment_type: 0,
            address,
            size,
            access: 0xFF,
            memory_type: 3,
            flags: 0x80,
        }
    }

    /// A two-table program in the shape of `A-0027-15-0BAC`'s procedure.
    fn image() -> DownloadImage {
        let attributes = [
            ("PeiType", "1"),
            ("ApplicationNumber", "39"),
            ("ApplicationVersion", "21"),
        ];
        DownloadImage {
            code: ProgramCode {
                program_id: "M-0083_A-0027-15-0BAC".into(),
                load_procedure_style: Some("ProductProcedure".into()),
                mask_version: Some("MV-0701".into()),
                load_procedures: vec![LoadProcedure {
                    merge_id: None,
                    steps: vec![
                        LoadStep::Connect,
                        LoadStep::CompareProp {
                            object_index: 0,
                            property_id: 78,
                            data: vec![0, 0, 0, 0, 1, 0x27],
                        },
                        LoadStep::Unload { lsm: 1 },
                        LoadStep::Unload { lsm: 3 },
                        LoadStep::Load { lsm: 1 },
                        abs(1, 0x4000, 4),
                        LoadStep::TaskSegment {
                            lsm: 1,
                            address: 0x4000,
                        },
                        LoadStep::LoadCompleted { lsm: 1 },
                        LoadStep::Load { lsm: 3 },
                        LoadStep::AbsSegment {
                            lsm: 3,
                            segment_type: 1,
                            address: 0x0798,
                            size: 1,
                            access: 0,
                            memory_type: 2,
                            flags: 0,
                        },
                        abs(3, 0x4400, 2),
                        LoadStep::TaskSegment {
                            lsm: 3,
                            address: 0x4400,
                        },
                        LoadStep::LoadCompleted { lsm: 3 },
                        LoadStep::Restart,
                        LoadStep::Disconnect,
                    ],
                }],
                program_attributes: attributes
                    .into_iter()
                    .map(|(name, value)| (name.to_string(), value.to_string()))
                    .collect(),
                ..ProgramCode::default()
            },
            segments: vec![
                SegmentImage {
                    id: "AS-4000".into(),
                    address: 0x4000,
                    octets: vec![0x01, 0x11, 0x43, 0x10],
                    mask: Some(vec![0xFF, 0x00, 0x00, 0xFF]),
                },
                SegmentImage {
                    id: "AS-4400".into(),
                    address: 0x4400,
                    octets: vec![0xAA, 0xBB],
                    mask: None,
                },
            ],
            parameters: BTreeMap::new(),
            texts: BTreeMap::new(),
            objects: Vec::new(),
            inferences: Vec::new(),
        }
    }

    fn records(plan: &MemoryDownloadPlan) -> Vec<String> {
        plan.steps.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_product_procedure_becomes_records_with_the_data_after_each_allocation() {
        let plan = plan_memory_download(&image()).expect("a plan");
        assert_eq!(plan.mask, MaskVersion(0x0701));
        assert_eq!(plan.manufacturer, 0x0083);
        assert_eq!(
            records(&plan),
            [
                "connect; check mask and manufacturer",
                "compare property 0/78 with 00 00 00 00 01 27",
                "A_Memory_Write 0104h: 14 00 00 00 00 00 00 00 00 00 00 (unload address table)",
                "A_Memory_Write 0104h: 34 00 00 00 00 00 00 00 00 00 00 (unload application program)",
                "A_Memory_Write 0104h: 11 00 00 00 00 00 00 00 00 00 00 (load address table)",
                "A_Memory_Write 0104h: 13 00 00 40 00 00 04 FF 03 80 00 (segment address table)",
                // The individual address at 4001h-4002h is masked.
                "A_Memory_Write 4000h..4000h, 1 octets",
                "A_Memory_Write 4003h..4003h, 1 octets",
                "A_Memory_Write 0104h: 13 02 00 40 00 00 00 00 00 00 00 (segment address table)",
                "A_Memory_Write 0104h: 12 00 00 00 00 00 00 00 00 00 00 (load completed address table)",
                "A_Memory_Write 0104h: 31 00 00 00 00 00 00 00 00 00 00 (load application program)",
                "A_Memory_Write 0104h: 33 01 00 07 98 00 01 00 02 00 00 (segment application program)",
                "A_Memory_Write 0104h: 33 00 00 44 00 00 02 FF 03 80 00 (segment application program)",
                "A_Memory_Write 4400h..4401h, 2 octets",
                // PEI type 1, manufacturer 0083h, application 0027h, version 15h.
                "A_Memory_Write 0104h: 33 02 00 44 00 01 00 83 00 27 15 (segment application program)",
                "A_Memory_Write 0104h: 32 00 00 00 00 00 00 00 00 00 00 (load completed application program)",
                "A_Restart (basic)",
                "disconnect",
            ]
        );
        assert_eq!(plan.data_octets(), 4);
    }

    #[test]
    fn masked_octets_are_never_in_the_plan() {
        let plan = plan_memory_download(&image()).expect("a plan");
        let written: Vec<u16> = plan
            .steps
            .iter()
            .filter_map(|step| match step {
                MemoryDownloadStep::WriteMemory { address, octets } => {
                    Some((0..octets.len() as u16).map(move |i| address + i))
                }
                _ => None,
            })
            .flatten()
            .collect();
        assert!(!written.contains(&0x4001) && !written.contains(&0x4002));
    }

    #[test]
    fn another_procedure_style_is_refused() {
        let mut image = image();
        image.code.load_procedure_style = Some("DefaultProcedure".into());
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::NotAProductProcedure(Some(
                "DefaultProcedure".into()
            )))
        );
    }

    #[test]
    fn a_mask_that_loads_through_properties_is_refused() {
        let mut image = image();
        image.code.mask_version = Some("MV-07B0".into());
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::NotMemoryMapped(Some("MV-07B0".into())))
        );
        image.code.mask_version = Some("0701".into());
        assert!(matches!(
            plan_memory_download(&image),
            Err(DownloadPlanError::NotMemoryMapped(_))
        ));
    }

    /// The mask decides first. Every `MergedProcedure`/`DefaultProcedure`
    /// program in the corpus has a mask that does not load through memory
    /// (`07B0h`, `0912h`, `091Ah`, `2920h`, `0012h`, `0001h`), so naming the
    /// style would point at a translation that, once written, still could
    /// not load that device. The mask is the reason that holds.
    #[test]
    fn a_non_memory_mask_is_named_before_the_procedure_style() {
        let mut image = image();
        image.code.mask_version = Some("MV-07B0".into());
        image.code.load_procedure_style = Some("MergedProcedure".into());
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::NotMemoryMapped(Some("MV-07B0".into())))
        );
    }

    #[test]
    fn an_unmodelled_step_is_refused_by_name() {
        let mut image = image();
        image.code.load_procedures[0].steps.insert(
            2,
            LoadStep::Unmodelled {
                name: "LdCtrlWriteMem".into(),
                attributes: BTreeMap::new(),
                has_children: false,
            },
        );
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::Unmodelled("LdCtrlWriteMem".into()))
        );
    }

    #[test]
    fn a_segment_whose_size_differs_from_its_allocation_is_refused() {
        let mut image = image();
        image.segments[1].octets.push(0xCC);
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::SizeMismatch {
                address: 0x4400,
                allocated: 2,
                octets: 3
            })
        );
    }

    #[test]
    fn a_segment_the_procedure_never_allocates_is_refused() {
        let mut image = image();
        image.segments.push(SegmentImage {
            id: "AS-4201".into(),
            address: 0x4201,
            octets: vec![0],
            mask: None,
        });
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::NeverWritten("AS-4201".into()))
        );
    }

    #[test]
    fn a_segment_allocated_twice_is_refused() {
        let mut image = image();
        image.code.load_procedures[0]
            .steps
            .insert(6, abs(1, 0x4000, 4));
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::AllocatedTwice(0x4000))
        );
    }

    #[test]
    fn the_pei_program_and_unknown_machines_are_refused() {
        for lsm in [0, 4, 5] {
            let mut image = image();
            image.code.load_procedures[0].steps[2] = LoadStep::Unload { lsm };
            assert_eq!(
                plan_memory_download(&image),
                Err(DownloadPlanError::Lsm(lsm)),
                "LsmIdx {lsm}"
            );
        }
    }

    #[test]
    fn reserved_segment_bits_and_types_are_refused() {
        let with = |segment_type, memory_type, flags| {
            let mut image = image();
            image.code.load_procedures[0].steps[5] = LoadStep::AbsSegment {
                lsm: 1,
                segment_type,
                address: 0x4000,
                size: 4,
                access: 0xFF,
                memory_type,
                flags,
            };
            plan_memory_download(&image)
        };
        assert_eq!(with(2, 3, 0x80), Err(DownloadPlanError::SegmentType(2)));
        assert_eq!(with(0, 4, 0x80), Err(DownloadPlanError::MemoryType(4)));
        assert_eq!(with(0, 3, 0x81), Err(DownloadPlanError::SegmentFlags(0x81)));
        assert!(with(0, 3, 0x00).is_ok(), "checksum control off is allowed");
    }

    #[test]
    fn a_stack_segment_for_a_table_is_refused_by_the_record_builder() {
        let mut image = image();
        image.code.load_procedures[0].steps[5] = LoadStep::AbsSegment {
            lsm: 1,
            segment_type: 1,
            address: 0x4000,
            size: 4,
            access: 0xFF,
            memory_type: 3,
            flags: 0x80,
        };
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::Record(
                MemoryLoadRecordError::StackSegmentNotAllowed(MemoryLoadStateMachine::AddressTable)
            ))
        );
    }

    #[test]
    fn a_procedure_must_connect_first_and_be_the_only_one() {
        let mut image = image();
        image.code.load_procedures[0].steps.remove(0);
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::DoesNotConnectFirst)
        );

        let mut image = self::image();
        let copy = image.code.load_procedures[0].clone();
        image.code.load_procedures.push(copy);
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::ProcedureCount(2))
        );
        image.code.load_procedures[1].merge_id = Some("1".into());
        assert!(
            plan_memory_download(&image).is_ok(),
            "a merge fragment is not a second procedure"
        );
    }

    #[test]
    fn identity_attributes_must_be_present_and_fit() {
        for (name, value) in [
            ("PeiType", Some("256")),
            ("ApplicationNumber", Some("65536")),
            ("ApplicationVersion", None),
        ] {
            let mut image = image();
            match value {
                Some(value) => {
                    image
                        .code
                        .program_attributes
                        .insert(name.into(), value.into());
                }
                None => {
                    image.code.program_attributes.remove(name);
                }
            }
            assert_eq!(
                plan_memory_download(&image),
                Err(DownloadPlanError::Attribute {
                    name,
                    value: value.map(str::to_string)
                })
            );
        }
        let mut image = image();
        image.code.program_id = "A-0027-15-0BAC".into();
        assert!(matches!(
            plan_memory_download(&image),
            Err(DownloadPlanError::ProgramId(_))
        ));
    }
}
