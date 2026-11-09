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
    task_control_1, AbsoluteSegment as SegmentRecord, MemoryLoadRecordError,
    MemoryLoadStateMachine, SegmentMemoryType, TaskControl1, TaskSegment,
};
use knx_core::commissioning::load_state::{LoadEvent, MaskVersion};
use knx_core::commissioning::memory_download::{
    unmasked_runs, MemoryDownloadPlan, MemoryDownloadStep,
};

use crate::code::{CustomerAdjustableCounts, LoadStep, ProgramCode};
use crate::image::DownloadImage;
use crate::inference::Inference;

mod property_steps;
pub use property_steps::{
    DECLARED_SIZE_WITHOUT_COUNT, INLINE_DATA_FILLS_PROPERTY, MASTER_RESET_AS_FUNCTION_PROPERTY,
    PROPERTY_WITHOUT_PARAMETERS_NOT_SENT, SHORT_INLINE_DATA_READS_DEVICE,
};

/// Apply write-only proof to an already validated regular plan; source masks stay untouched.
/// Callers must first run the existing image and complete-plan validation. Never refuses:
/// only bytes proved exclusively inactive are omitted; every other byte is written as before.
pub fn select_inactive_whole_bytes(
    conn: &crate::Connection,
    request: &crate::image::ImageRequest,
    image: &DownloadImage,
    mut plan: MemoryDownloadPlan,
) -> MemoryDownloadPlan {
    let selection = crate::image::inactive_selection::selection(conn, request, image);
    let mut omitted = BTreeSet::new();
    for (id, address, permissions) in &selection {
        for (offset, permitted) in permissions.iter().enumerate() {
            let absolute = *address + offset as u32;
            // A byte inside another segment has another possible owner: written as before.
            let shared = image.segments.iter().any(|s| {
                s.id != *id && s.address <= absolute && absolute < s.address + s.octets.len() as u32
            });
            if !permitted && !shared {
                omitted.insert(absolute);
            }
        }
    }
    let mut steps = Vec::with_capacity(plan.steps.len());
    for step in plan.steps {
        match step {
            MemoryDownloadStep::WriteMemory { address, octets } => {
                let mut offset = 0;
                while offset < octets.len() {
                    if omitted.contains(&(u32::from(address) + offset as u32)) {
                        offset += 1;
                        continue;
                    }
                    let start = offset;
                    while offset < octets.len()
                        && !omitted.contains(&(u32::from(address) + offset as u32))
                    {
                        offset += 1;
                    }
                    steps.push(MemoryDownloadStep::WriteMemory {
                        address: address + start as u16,
                        octets: octets[start..offset].to_vec(),
                    });
                }
            }
            control => steps.push(control),
        }
    }
    plan.steps = steps;
    plan
}

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
    /// The program declares a `MergeId` load procedure beside its one regular
    /// procedure. A conservative refusal of an unsupported shape: what the
    /// fragment adds is not translated, and no merged meaning is assumed.
    MergeFragment {
        /// The fragment's 1-based position among all of the program's load
        /// procedures, in source order.
        position: usize,
        /// Its `MergeId`, verbatim.
        merge_id: String,
    },
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
    /// Two segments with data start at the allocated address: the image does
    /// not say whose octets belong there.
    DuplicateSegmentStart {
        /// The shared start.
        address: u16,
        /// The first segment with data there, in image order.
        first: String,
        /// The next one.
        second: String,
    },
    /// The program declares an ETS download plug-in (its GUID): the vendor
    /// code writes what the product data does not describe (KL-006).
    VendorDownloadPlugin(String),
    /// Presence-only conservative regular-write refusal; no source payload is exposed.
    CustomerAdjustableParameters(CustomerAdjustableCounts),
    /// An `LdCtrlMasterReset` with no observed translation (ADR-0161).
    MasterReset {
        /// `EraseCode`.
        erase_code: u8,
        /// `ChannelNumber`.
        channel: u8,
    },
    /// A property's parameters cannot be written as observed (ADR-0161).
    PropertyParameters {
        /// `ObjIdx`.
        object_index: u8,
        /// `PropId`.
        property_id: u8,
        /// Why.
        reason: String,
    },
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
            DownloadPlanError::MergeFragment { position, merge_id } => write!(
                f,
                "load procedure {position} declares MergeId {merge_id:?}; a MergeId fragment is not \
                 translated, so the program is not planned"
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
            DownloadPlanError::DuplicateSegmentStart {
                address,
                first,
                second,
            } => write!(
                f,
                "segments {first} and {second} both have data at {address:04X}h; \
                 the plan cannot tell which one the allocation takes"
            ),
            DownloadPlanError::CustomerAdjustableParameters(counts) => write!(
                f,
                "customer-adjustable-parameters: {} uninterpreted declarations \
                 (root parameters: {}, root Union members: {}, module parameters: {}, \
                 module Union members: {}, root Options: {}); no regular download scope is \
                 offered because preservation semantics are not implemented. Read-only \
                 compare and diagnostics stay available",
                counts.total(), counts.root_parameters, counts.root_union_parameters,
                counts.module_parameters, counts.module_union_parameters, counts.root_options
            ),
            DownloadPlanError::MasterReset {
                erase_code,
                channel,
            } => write!(
                f,
                "LdCtrlMasterReset EraseCode={erase_code} ChannelNumber={channel} has no \
                 observed translation; only erase codes 5 and 6 on channel 0 of mask 0701h do"
            ),
            DownloadPlanError::PropertyParameters {
                object_index,
                property_id,
                reason,
            } => write!(f, "property {object_index}/{property_id}: {reason}"),
            DownloadPlanError::VendorDownloadPlugin(plugin) => write!(
                f,
                "the application program declares the ETS download plug-in {plugin}, which writes \
                 data the product file does not describe; KNXBench does not run vendor plug-ins, \
                 so no download scope is offered (KNOWN_LIMITATIONS KL-006). Read-only compare \
                 and diagnostics stay available"
            ),
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

/// Refuses a program that declares an ETS download plug-in
/// (`Static/Extension/@EtsDownloadPlugin`, KL-006), for every download
/// scope. The rule is the declaration, whoever the manufacturer is.
///
/// `[V]` A real ETS-programmed device of such a program did not hold its
/// product's base data: one planned segment was not readable at all, and
/// the readable part differed in octets no parameter owns. A plan built from
/// the product file alone would write something other than what ETS wrote.
///
/// Not part of [`check_program_kind`] or the plan translation: a read-only
/// comparison prepares the same plan to know what to read, and must stay
/// possible. Every caller that writes to a device asks this first.
pub fn check_download_plugin(code: &ProgramCode) -> Result<(), DownloadPlanError> {
    match &code.download_plugin {
        Some(plugin) => Err(DownloadPlanError::VendorDownloadPlugin(plugin.clone())),
        None => Ok(()),
    }
}

/// Refuses every present declaration in the bounded customer-adjustable source policy.
/// Values are uniformly uninterpreted. This checker is write admission, not plan translation;
/// a read-only plan is not proof of write permission. Absence is not general safety evidence.
pub fn check_customer_adjustable(code: &ProgramCode) -> Result<(), DownloadPlanError> {
    let counts = code.customer_adjustable.counts();
    if counts.total() == 0 {
        Ok(())
    } else {
        Err(DownloadPlanError::CustomerAdjustableParameters(counts))
    }
}

/// Builds the plan for `image`.
pub fn plan_memory_download(
    image: &DownloadImage,
) -> Result<MemoryDownloadPlan, DownloadPlanError> {
    plan_memory_download_with_inferences(image).map(|(plan, _)| plan)
}

/// [`Inference::rule`]: a machine-5 event after the final restart is not
/// sent (RESEARCH §19.12).
pub const MACHINE_5_AFTER_RESTART: &str = "machine-5-after-restart";

/// [`Inference::rule`]: a segment longer than its allocation is written up
/// to the allocation only. `[V]` ETS 6.4 wrote exactly the allocated prefix
/// of such a segment (docs/research/2026-11-07-ldctrlwriteprop-and-byte-order.md);
/// KNXBench does so only when every written parameter and table lies in it.
pub const SEGMENT_BEYOND_ALLOCATION: &str = "segment-beyond-allocation-not-written";

/// [`Inference::rule`]: a property comparison the procedure places after a
/// fixed property write is run before it. The executor checks the device
/// before it changes anything; a comparison is only a read, so moving it
/// first can at worst refuse a download, never write to the wrong device.
pub const COMPARE_BEFORE_PROPERTY_WRITE: &str = "compare-before-property-write";

/// `LsmIdx` of the fifth machine some converted procedures name.
const MACHINE_5: u8 = 5;

/// Builds the plan for `image`, with every inference it rests on: the
/// image's, then the procedure's (ADR-0086).
pub fn plan_memory_download_with_inferences(
    image: &DownloadImage,
) -> Result<(MemoryDownloadPlan, Vec<Inference>), DownloadPlanError> {
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
    if let Some((index, merge_id)) = code
        .load_procedures
        .iter()
        .enumerate()
        .find_map(|(index, procedure)| procedure.merge_id.as_ref().map(|id| (index, id)))
    {
        return Err(DownloadPlanError::MergeFragment {
            position: index + 1,
            merge_id: merge_id.clone(),
        });
    }
    if procedure.steps.first() != Some(&LoadStep::Connect) {
        return Err(DownloadPlanError::DoesNotConnectFirst);
    }

    let mut inferences = image.inferences.clone();
    let mut steps = Vec::new();
    let mut allocated = BTreeSet::new();
    let mut restarted = false;
    let mut properties = property_steps::PropertySteps::new(image, mask);
    for step in &procedure.steps {
        match step {
            // `[D]` Cookbook *Load Controls* (`02_03_01` v01.00.02) §2.3: a
            // task segment of machine 5 is `AbsCObjSeg`, "for
            // MT-information only. ETS ignores it and will hence not be
            // transmitted on the bus."
            LoadStep::TaskSegment { lsm: MACHINE_5, .. } => {}
            // `[A]` (ADR-0086) After the restart, which "also closes the
            // transport layer connection" (Cookbook §2.2), a machine-5 event
            // is not sent: a BIM M112 has no fifth machine (MP §3.31.2).
            // bussard's captures of ETS show nothing sent after the restart.
            LoadStep::Load { lsm: MACHINE_5 }
            | LoadStep::Unload { lsm: MACHINE_5 }
            | LoadStep::LoadCompleted { lsm: MACHINE_5 }
                if restarted =>
            {
                inferences.push(Inference {
                    rule: MACHINE_5_AFTER_RESTART,
                    detail: format!(
                        "{} LsmIdx=5 after LdCtrlRestart is not sent: the restart closed the \
                         connection and a BIM M112 has no fifth load state machine",
                        element_name(step)
                    ),
                    reference: "RESEARCH §19.12",
                });
            }
            LoadStep::Connect => steps.push(MemoryDownloadStep::Connect),
            // `[V]` ETS 6.4 waited the declared time before the restart.
            LoadStep::Delay { milliseconds } => steps.push(MemoryDownloadStep::Delay {
                milliseconds: *milliseconds,
            }),
            LoadStep::Disconnect => steps.push(MemoryDownloadStep::Disconnect),
            LoadStep::Restart => {
                restarted = true;
                steps.push(MemoryDownloadStep::Restart);
            }
            LoadStep::CompareProp {
                object_index,
                property_id,
                data,
            } => steps.push(MemoryDownloadStep::CompareProperty {
                object_index: *object_index,
                property_id: *property_id,
                inline_data: data.clone(),
            }),
            // `[V]` An ETS 6.4 download against KNXBench's simulator sends
            // exactly this write where the procedure has it, between the
            // compare and the unloads and again before the restart.
            LoadStep::WriteProp {
                object_index,
                property_id,
                data,
            } => steps.push(MemoryDownloadStep::WriteProperty {
                object_index: *object_index,
                property_id: *property_id,
                inline_data: data.clone(),
            }),
            LoadStep::MasterReset {
                erase_code,
                channel,
            } => steps.push(properties.master_reset(*erase_code, *channel, &mut inferences)?),
            // Sends nothing: the next parameter-filled write relies on it.
            LoadStep::DeclarePropDesc {
                object_index,
                property_id,
                property_type,
                max_elements,
                writable,
                ..
            } => properties.declare(
                (*object_index, *property_id),
                property_type,
                *max_elements,
                *writable,
            )?,
            LoadStep::WriteParameterProp {
                object_index,
                property_id,
                count,
                inline_data,
            } => steps.extend(properties.write(
                *object_index,
                *property_id,
                *count,
                inline_data.as_deref(),
            )?),
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
                let (writes, beyond) = data_writes(image, *address, *size)?;
                steps.extend(writes);
                if let Some(beyond) = beyond {
                    inferences.push(Inference {
                        rule: SEGMENT_BEYOND_ALLOCATION,
                        detail: format!(
                            "the segment at {address:04X}h holds {beyond} octets past its \
                             allocation of {size}; nothing written lies there, so they are not \
                             sent, as ETS 6.4 does"
                        ),
                        reference: "docs/research/2026-11-07-ldctrlwriteprop-and-byte-order.md",
                    });
                }
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
            LoadStep::TaskCtrl1 {
                lsm,
                address,
                count,
            } => steps.push(MemoryDownloadStep::LoadRecord(task_control_1(
                machine(*lsm)?,
                TaskControl1 {
                    interface_objects: *address,
                    count: *count,
                },
            )?)),
            LoadStep::TaskCtrl2 { .. } => {
                // Typed product data is not a verified BIM M112 translation.
                return Err(DownloadPlanError::Unmodelled("LdCtrlTaskCtrl2".into()));
            }
            LoadStep::Unmodelled { name, .. } => {
                return Err(DownloadPlanError::Unmodelled(name.clone()))
            }
        }
    }

    properties.finish(&mut inferences)?;
    for segment in &image.segments {
        let address = u16::try_from(segment.address)
            .map_err(|_| DownloadPlanError::Address(segment.address))?;
        if !allocated.contains(&address) {
            return Err(DownloadPlanError::NeverWritten(segment.id.clone()));
        }
    }
    if let Some(moved) = compare_before_property_writes(&mut steps) {
        inferences.push(Inference {
            rule: COMPARE_BEFORE_PROPERTY_WRITE,
            detail: format!(
                "{moved} LdCtrlCompareProp after a leading LdCtrlWriteProp run before it: the \
                 device is checked before anything is written to it"
            ),
            reference: "docs/research/2026-11-07-ldctrlwriteprop-and-byte-order.md",
        });
    }

    Ok((
        MemoryDownloadPlan {
            mask,
            manufacturer,
            steps,
        },
        inferences,
    ))
}

/// Moves every comparison in the leading run of property writes and
/// comparisons (before any other device change) in front of the first
/// write, keeping their order. Returns how many moved, if any.
fn compare_before_property_writes(steps: &mut [MemoryDownloadStep]) -> Option<usize> {
    let first_write = steps
        .iter()
        .position(|step| matches!(step, MemoryDownloadStep::WriteProperty { .. }))?;
    let lead_end = steps[first_write..]
        .iter()
        .position(|step| {
            !matches!(
                step,
                MemoryDownloadStep::WriteProperty { .. }
                    | MemoryDownloadStep::CompareProperty { .. }
            )
        })
        .map_or(steps.len(), |offset| first_write + offset);
    let lead = &mut steps[first_write..lead_end];
    let moved = lead
        .iter()
        .filter(|step| matches!(step, MemoryDownloadStep::CompareProperty { .. }))
        .count();
    // A stable partition: comparisons first, writes after, each in order.
    lead.sort_by_key(|step| !matches!(step, MemoryDownloadStep::CompareProperty { .. }));
    (moved > 0).then_some(moved)
}

/// The product element a step was read from, for an inference's text.
fn element_name(step: &LoadStep) -> &'static str {
    match step {
        LoadStep::Load { .. } => "LdCtrlLoad",
        LoadStep::Unload { .. } => "LdCtrlUnload",
        LoadStep::LoadCompleted { .. } => "LdCtrlLoadCompleted",
        _ => "a load control",
    }
}

/// The writes for the segment allocated at `address`, if the image has one
/// there, and how many of its octets lie past the allocation unsent. Two
/// segments with data at one start are refused: nothing in the product says
/// which one an allocation takes, equal octets or not. A segment shorter
/// than its allocation, or longer with a written parameter or table past it,
/// is refused ([`SEGMENT_BEYOND_ALLOCATION`]).
fn data_writes(
    image: &DownloadImage,
    address: u16,
    size: u16,
) -> Result<(Vec<MemoryDownloadStep>, Option<usize>), DownloadPlanError> {
    let mut here = image
        .segments
        .iter()
        .filter(|segment| segment.address == u32::from(address));
    let Some(segment) = here.next() else {
        return Ok((Vec::new(), None));
    };
    if let Some(other) = here.next() {
        return Err(DownloadPlanError::DuplicateSegmentStart {
            address,
            first: segment.id.clone(),
            second: other.id.clone(),
        });
    }
    let allocated = usize::from(size);
    let mismatch = || DownloadPlanError::SizeMismatch {
        address,
        allocated: size,
        octets: segment.octets.len(),
    };
    if segment.octets.len() < allocated || written_end(image, &segment.id) > allocated {
        return Err(mismatch());
    }
    let beyond = (segment.octets.len() > allocated).then(|| segment.octets.len() - allocated);
    let mask = segment
        .mask
        .as_deref()
        .map(|mask| &mask[..allocated.min(mask.len())]);
    let writes = unmasked_runs(&segment.octets[..allocated], mask)
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
        .collect::<Result<Vec<_>, DownloadPlanError>>()?;
    Ok((writes, beyond))
}

/// The end, in octets from the segment's start, of everything the image
/// builder wrote into segment `id`: parameter fields and placed tables.
fn written_end(image: &DownloadImage, id: &str) -> usize {
    let fields = image
        .parameter_fields
        .iter()
        .filter(|written| written.segment == id)
        .map(|written| {
            let bits = u64::from(written.field.bit_offset) + u64::from(written.field.size_in_bit);
            usize::try_from(u64::from(written.field.offset) + bits.div_ceil(8))
                .unwrap_or(usize::MAX)
        });
    let tables = image
        .table_writes
        .iter()
        .filter(|table| table.segment_id == id)
        .map(|table| table.end);
    fields.chain(tables).max().unwrap_or(0)
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

/// The kind refusal a program's stored mask settles on its own: `Some` for
/// a recorded mask that does not load through memory, the refusal
/// [`check_program_kind`] would give first. For callers whose program code
/// cannot be read, so that an unreadable code does not hide the more
/// fundamental refusal (RESEARCH §19.22). A missing program or mask is
/// `None`: the caller's own refusal stands.
pub fn stored_mask_refusal(
    conn: &crate::Connection,
    program_id: &str,
) -> Result<Option<DownloadPlanError>, crate::ProductDbError> {
    use rusqlite::OptionalExtension;
    let mask: Option<Option<String>> = conn
        .query_row(
            "SELECT mask_version FROM application_program WHERE id = ?1",
            [program_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(match mask.flatten() {
        Some(mask)
            if parse_mask(&mask)
                .filter(|m| loads_through_memory(*m))
                .is_none() =>
        {
            Some(DownloadPlanError::NotMemoryMapped(Some(mask)))
        }
        _ => None,
    })
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
pub(crate) fn manufacturer_of(program_id: &str) -> Result<u16, DownloadPlanError> {
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
    use crate::image::{AttributedValue, ParameterAttribution, SegmentImage, TableWrite};
    use knx_core::commissioning::parameter_image::ParameterField;

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
    /// KL-006: the declaration refuses; it does not touch the kind check or
    /// the plan translation, which a read-only comparison still uses.
    #[test]
    fn a_declared_download_plugin_is_refused_by_its_own_check_only() {
        let mut image = image();
        assert_eq!(check_download_plugin(&image.code), Ok(()));
        image.code.download_plugin = Some("{G}".into());
        let refused = check_download_plugin(&image.code).unwrap_err();
        assert_eq!(
            refused,
            DownloadPlanError::VendorDownloadPlugin("{G}".into())
        );
        let shown = refused.to_string();
        assert!(shown.contains("{G}") && shown.contains("KL-006"), "{shown}");
        assert!(check_program_kind(&image.code).is_ok());
        assert!(plan_memory_download(&image).is_ok());
    }

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
            parameter_fields: Vec::new(),
            table_writes: Vec::new(),
            properties: Vec::new(),
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
    fn task_control_2_is_not_silently_translated_by_the_memory_planner() {
        let mut image = image();
        image.code.load_procedures[0].steps.insert(
            1,
            LoadStep::TaskCtrl2 {
                lsm: 3,
                callback: 0,
                address: 0x0400,
                segment_0: 0,
                segment_1: 0,
            },
        );
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::Unmodelled("LdCtrlTaskCtrl2".into()))
        );
        image.code.mask_version = Some("MV-0025".into());
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::NotMemoryMapped(Some("MV-0025".into())))
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
    fn a_segment_shorter_than_its_allocation_is_refused() {
        let mut image = image();
        image.segments[1].octets.pop();
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::SizeMismatch {
                address: 0x4400,
                allocated: 2,
                octets: 1
            })
        );
    }

    /// A comparison behind a leading property write runs first; a write
    /// later in the procedure stays where it is.
    /// [`image`] with the steps ETS 6.4 was observed with for a property
    /// program (ADR-0161) inserted after the comparison, and one property
    /// carrying parameters.
    fn property_image(steps: Vec<LoadStep>) -> DownloadImage {
        let mut image = image();
        let procedure = &mut image.code.load_procedures[0].steps;
        for (offset, step) in steps.into_iter().enumerate() {
            procedure.insert(2 + offset, step);
        }
        image.properties = vec![crate::image::property::PropertyImage {
            object_index: 8,
            property_id: 52,
            octets: vec![0x01, 0x78, 0x00],
            owned: vec![0x01, 0xFF, 0xFF],
        }];
        image
    }

    fn declare(property_type: &str) -> LoadStep {
        declare_at(8, property_type)
    }

    fn declare_at(object_index: u8, property_type: &str) -> LoadStep {
        LoadStep::DeclarePropDesc {
            object_index,
            property_id: 52,
            property_type: property_type.into(),
            max_elements: 4,
            read_access: 0,
            write_access: 0,
            writable: true,
        }
    }

    fn write_parameters(count: u16) -> LoadStep {
        write_inline(8, count, None)
    }

    fn write_inline(object_index: u8, count: u16, inline_data: Option<Vec<u8>>) -> LoadStep {
        LoadStep::WriteParameterProp {
            object_index,
            property_id: 52,
            count: Some(count),
            inline_data,
        }
    }

    fn write_without_count(object_index: u8) -> LoadStep {
        LoadStep::WriteParameterProp {
            object_index,
            property_id: 52,
            count: None,
            inline_data: None,
        }
    }

    /// `[V]` ETS 6.4 against KNXBench's simulator (ADR-0167): without `Count`
    /// the declared `MaxElements` are written, every bit no parameter owns as
    /// zero, and nothing is read; a property without parameters is refused.
    #[test]
    fn a_write_without_count_writes_the_declared_elements_and_reads_nothing() {
        // `declare_at` declares four elements; the image places three octets.
        let (plan, inferences) = plan_memory_download_with_inferences(&property_image(vec![
            declare("PDT_UNSIGNED_CHAR"),
            write_without_count(8),
        ]))
        .unwrap();
        assert!(plan
            .steps
            .contains(&MemoryDownloadStep::WritePropertyParameters {
                object_index: 8,
                property_id: 52,
                elements: 4,
                octets: vec![0x01, 0x78, 0x00, 0x00],
                owned: vec![0xFF; 4],
            }));
        let declared = inferences
            .iter()
            .find(|inference| inference.rule == DECLARED_SIZE_WITHOUT_COUNT)
            .expect("the rule is named");
        assert!(declared.detail.starts_with("1 "), "{}", declared.detail);

        let reason = |steps: Vec<LoadStep>| match plan_memory_download(&property_image(steps)) {
            Err(DownloadPlanError::PropertyParameters { reason, .. }) => reason,
            other => panic!("{other:?}"),
        };
        // A property without parameters: unobserved.
        assert!(reason(vec![
            declare("PDT_UNSIGNED_CHAR"),
            write_parameters(3),
            declare_at(10, "PDT_UNSIGNED_CHAR"),
            write_without_count(10),
        ])
        .contains("holds no parameter"));
        // Parameters past the declared elements.
        let mut small = declare("PDT_UNSIGNED_CHAR");
        if let LoadStep::DeclarePropDesc { max_elements, .. } = &mut small {
            *max_elements = 2;
        }
        assert!(reason(vec![small, write_without_count(8)]).contains("MaxElements"));
    }

    /// `[V]` ETS 6.4 against KNXBench's simulator (ADR-0162): an all-zero
    /// `InlineData` covering `Count` is the value under the parameters, and
    /// nothing is read; a shorter one is not used; a property no parameter
    /// reaches is not written.
    #[test]
    fn inline_data_fills_covered_writes_and_a_write_without_parameters_sends_nothing() {
        let plan_of =
            |steps: Vec<LoadStep>| plan_memory_download_with_inferences(&property_image(steps));
        let rules = |inferences: &[Inference]| -> Vec<&str> {
            inferences.iter().map(|inference| inference.rule).collect()
        };

        let (plan, inferences) = plan_of(vec![
            declare("PDT_UNSIGNED_CHAR"),
            // Exactly as long as Count: still covering.
            write_inline(8, 6, Some(vec![0; 6])),
            declare_at(10, "PDT_UNSIGNED_CHAR"),
            write_inline(10, 11, Some(vec![0; 10])),
            // Unobserved: covering InlineData on a property without
            // parameters. Sending nothing is the safe reading.
            declare_at(11, "PDT_UNSIGNED_CHAR"),
            write_inline(11, 2, Some(vec![0; 10])),
        ])
        .unwrap();
        let parameters: Vec<_> = plan
            .steps
            .iter()
            .filter(|step| matches!(step, MemoryDownloadStep::WritePropertyParameters { .. }))
            .collect();
        assert_eq!(
            parameters,
            [&MemoryDownloadStep::WritePropertyParameters {
                object_index: 8,
                property_id: 52,
                elements: 6,
                octets: vec![0x01, 0x78, 0x00, 0x00, 0x00, 0x00],
                owned: vec![0xFF; 6],
            }]
        );
        let found = rules(&inferences);
        assert!(found.contains(&INLINE_DATA_FILLS_PROPERTY), "{found:?}");
        let unsent = inferences
            .iter()
            .find(|inference| inference.rule == PROPERTY_WITHOUT_PARAMETERS_NOT_SENT)
            .expect("the unsent writes are named");
        assert!(unsent.detail.starts_with("2 "), "{}", unsent.detail);
        assert!(
            !found.contains(&SHORT_INLINE_DATA_READS_DEVICE),
            "{found:?}"
        );

        // Shorter than Count: merged into what the device holds, as without.
        let (plan, inferences) = plan_of(vec![
            declare("PDT_UNSIGNED_CHAR"),
            write_inline(8, 5, Some(vec![0; 2])),
        ])
        .unwrap();
        assert!(plan
            .steps
            .contains(&MemoryDownloadStep::WritePropertyParameters {
                object_index: 8,
                property_id: 52,
                elements: 5,
                octets: vec![0x01, 0x78, 0x00, 0x00, 0x00],
                owned: vec![0x01, 0xFF, 0xFF, 0x00, 0x00],
            }));
        assert!(rules(&inferences).contains(&SHORT_INLINE_DATA_READS_DEVICE));

        // A non-zero InlineData was never observed.
        let refused = plan_memory_download(&property_image(vec![
            declare("PDT_UNSIGNED_CHAR"),
            write_inline(8, 6, Some(vec![0, 0, 1])),
        ]))
        .unwrap_err();
        assert!(
            matches!(&refused, DownloadPlanError::PropertyParameters { reason, .. }
                if reason.contains("non-zero")),
            "{refused:?}"
        );
        // Without InlineData a property no parameter reaches is still written.
        let (plan, _) = plan_of(vec![
            declare("PDT_UNSIGNED_CHAR"),
            write_parameters(3),
            declare_at(10, "PDT_UNSIGNED_CHAR"),
            write_inline(10, 2, None),
        ])
        .unwrap();
        assert!(plan.steps.iter().any(|step| matches!(
            step,
            MemoryDownloadStep::WritePropertyParameters { object_index: 10, owned, .. }
                if owned == &[0, 0]
        )));
    }

    #[test]
    fn master_resets_and_parameter_properties_become_the_observed_commands() {
        let image = property_image(vec![
            LoadStep::MasterReset {
                erase_code: 6,
                channel: 0,
            },
            LoadStep::MasterReset {
                erase_code: 5,
                channel: 0,
            },
            declare("PDT_UNSIGNED_CHAR"),
            write_parameters(5),
        ]);
        let (plan, inferences) = plan_memory_download_with_inferences(&image).unwrap();
        assert_eq!(
            plan.steps[2..5],
            [
                MemoryDownloadStep::MasterResetCommand {
                    erase_code: 6,
                    object_index: 5,
                    property_id: 52,
                    command: 0x04,
                },
                MemoryDownloadStep::MasterResetCommand {
                    erase_code: 5,
                    object_index: 5,
                    property_id: 52,
                    command: 0x10,
                },
                // `Count` elements, the image zero-padded and owning nothing past it.
                MemoryDownloadStep::WritePropertyParameters {
                    object_index: 8,
                    property_id: 52,
                    elements: 5,
                    octets: vec![0x01, 0x78, 0x00, 0x00, 0x00],
                    owned: vec![0x01, 0xFF, 0xFF, 0x00, 0x00],
                },
            ]
        );
        assert_eq!(
            inferences
                .iter()
                .filter(|inference| inference.rule == MASTER_RESET_AS_FUNCTION_PROPERTY)
                .count(),
            2
        );
    }

    #[test]
    fn property_steps_without_an_observed_translation_are_refused_by_name() {
        let refused = |steps: Vec<LoadStep>| {
            plan_memory_download(&property_image(steps)).expect_err("refused")
        };
        for (erase_code, channel) in [(1, 0), (7, 0), (5, 1)] {
            assert_eq!(
                refused(vec![LoadStep::MasterReset {
                    erase_code,
                    channel
                }]),
                DownloadPlanError::MasterReset {
                    erase_code,
                    channel
                }
            );
        }
        let reason = |error: DownloadPlanError| match error {
            DownloadPlanError::PropertyParameters { reason, .. } => reason,
            other => panic!("{other:?}"),
        };
        assert!(reason(refused(vec![
            declare("PDT_UNSIGNED_INT"),
            write_parameters(3)
        ]))
        .contains("PDT_UNSIGNED_INT"));
        assert!(reason(refused(vec![write_parameters(3)])).contains("without a declared"));
        assert!(reason(refused(vec![
            declare("PDT_UNSIGNED_CHAR"),
            write_parameters(2)
        ]))
        .contains("past the elements"));
        assert!(reason(refused(vec![declare("PDT_UNSIGNED_CHAR")])).contains("never writes"));
        // Another mask has no observed mapping either.
        let mut other_mask = property_image(vec![LoadStep::MasterReset {
            erase_code: 6,
            channel: 0,
        }]);
        other_mask.code.mask_version = Some("MV-0705".into());
        other_mask.properties.clear();
        assert!(matches!(
            plan_memory_download(&other_mask),
            Err(DownloadPlanError::MasterReset { .. })
        ));
    }

    #[test]
    fn a_comparison_behind_a_leading_property_write_runs_before_it() {
        let mut image = image();
        let write = |data: u8| LoadStep::WriteProp {
            object_index: 5,
            property_id: 51,
            data: vec![data, 0],
        };
        image.code.load_procedures[0].steps.insert(1, write(1));
        let restart = image.code.load_procedures[0]
            .steps
            .iter()
            .position(|step| *step == LoadStep::Restart)
            .unwrap();
        image.code.load_procedures[0]
            .steps
            .insert(restart, write(2));
        let (plan, inferences) = plan_memory_download_with_inferences(&image).unwrap();
        let property_write = |data: u8| MemoryDownloadStep::WriteProperty {
            object_index: 5,
            property_id: 51,
            inline_data: vec![data, 0],
        };
        assert!(matches!(
            plan.steps[1],
            MemoryDownloadStep::CompareProperty { .. }
        ));
        assert_eq!(plan.steps[2], property_write(1));
        assert_eq!(plan.steps[plan.steps.len() - 3], property_write(2));
        assert!(inferences
            .iter()
            .any(|inference| inference.rule == COMPARE_BEFORE_PROPERTY_WRITE));
        // Without a leading write nothing moves and nothing is said.
        let (_, plain) = plan_memory_download_with_inferences(&super::tests::image()).unwrap();
        assert!(!plain
            .iter()
            .any(|inference| inference.rule == COMPARE_BEFORE_PROPERTY_WRITE));
    }

    /// ETS writes the allocated prefix; so does the plan, but only while no
    /// written parameter or table reaches past it.
    #[test]
    fn a_longer_segment_writes_its_allocation_while_nothing_written_lies_past_it() {
        let mut image = image();
        image.segments[1].octets.extend([0xCC, 0xDD]);
        let (plan, inferences) = plan_memory_download_with_inferences(&image).unwrap();
        let writes: Vec<_> = plan
            .steps
            .iter()
            .filter_map(|step| match step {
                MemoryDownloadStep::WriteMemory { address, octets } if *address >= 0x4400 => {
                    Some((*address, octets.clone()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(writes, [(0x4400, vec![0xAA, 0xBB])]);
        assert!(inferences
            .iter()
            .any(|inference| inference.rule == SEGMENT_BEYOND_ALLOCATION));

        let refused = |image: &DownloadImage| {
            assert_eq!(
                plan_memory_download(image),
                Err(DownloadPlanError::SizeMismatch {
                    address: 0x4400,
                    allocated: 2,
                    octets: 4
                })
            );
        };
        let mut table = image.clone();
        table.table_writes.push(TableWrite {
            segment_id: "AS-4400".into(),
            start: 1,
            end: 3,
        });
        refused(&table);
        let mut parameter = image.clone();
        parameter.parameter_fields.push(ParameterAttribution {
            reference: "P-1_R-1".into(),
            name: "P".into(),
            segment: "AS-4400".into(),
            field: ParameterField {
                offset: 1,
                bit_offset: 4,
                size_in_bit: 8,
            },
            value: AttributedValue::Number {
                value: 1,
                nonnegative_signed: false,
                low_octet_first: false,
            },
        });
        refused(&parameter);
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

    fn twin(id: &str, octets: Vec<u8>, mask: Option<Vec<u8>>) -> SegmentImage {
        SegmentImage {
            id: id.into(),
            address: 0x4400,
            octets,
            mask,
        }
    }

    fn duplicate(address: u16, first: &str, second: &str) -> DownloadPlanError {
        DownloadPlanError::DuplicateSegmentStart {
            address,
            first: first.into(),
            second: second.into(),
        }
    }

    /// Exact-address matching has no precedence between two segments with data:
    /// equal octets, disjoint masks and full protection do not make one win.
    #[test]
    fn two_segments_with_data_at_one_allocated_start_are_refused_whatever_they_hold() {
        let rows = [
            (vec![0xCC, 0xDD], None),
            (vec![0xAA, 0xBB], None),
            (vec![0xCC, 0xDD], Some(vec![0x00, 0xFF])),
            (vec![0xCC, 0xDD], Some(vec![0x00, 0x00])),
        ];
        for (octets, mask) in rows {
            let mut image = image();
            image.segments.push(twin("AS-4400-B", octets, mask));
            image.segments[1].mask = Some(vec![0xFF, 0x00]);
            let error = plan_memory_download(&image).unwrap_err();
            assert_eq!(error, duplicate(0x4400, "AS-4400", "AS-4400-B"));
            let shown = error.to_string();
            assert!(
                shown.contains("AS-4400") && shown.contains("AS-4400-B") && shown.contains("4400h"),
                "{shown}"
            );
        }
    }

    #[test]
    fn the_duplicate_start_names_the_segments_in_image_order() {
        let mut image = image();
        image
            .segments
            .insert(1, twin("AS-4400-B", vec![0xCC, 0xDD], None));
        assert_eq!(
            plan_memory_download(&image),
            Err(duplicate(0x4400, "AS-4400-B", "AS-4400"))
        );
    }

    #[test]
    fn a_duplicate_start_that_is_never_allocated_is_still_never_written() {
        let mut image = image();
        for id in ["AS-4300-A", "AS-4300-B"] {
            image.segments.push(SegmentImage {
                address: 0x4300,
                ..twin(id, vec![0], None)
            });
        }
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::NeverWritten("AS-4300-A".into()))
        );
    }

    #[test]
    fn an_ambiguous_start_is_named_before_a_size_mismatch_of_either_segment() {
        let mut image = image();
        image.segments.push(twin("AS-4400-B", vec![0xCC], None));
        assert_eq!(
            plan_memory_download(&image),
            Err(duplicate(0x4400, "AS-4400", "AS-4400-B"))
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

    /// Index of `LoadCompleted { lsm: 3 }` and of `Restart` in [`image`].
    const APPLICATION_COMPLETED: usize = 12;
    const RESTART: usize = 13;

    /// `[D]` MP §3.31.2 segment type 4, after the application's task
    /// segment as in `M-006A_A-0702-10-7779`.
    #[test]
    fn a_task_control_1_becomes_its_record() {
        let mut image = image();
        image.code.load_procedures[0].steps.insert(
            APPLICATION_COMPLETED,
            LoadStep::TaskCtrl1 {
                lsm: 3,
                address: 0x404F,
                count: 1,
            },
        );
        let shown = records(&plan_memory_download(&image).expect("a plan"));
        let task = shown
            .iter()
            .position(|record| record.contains(": 33 02 00"))
            .expect("the task segment");
        assert_eq!(
            shown[task + 1],
            "A_Memory_Write 0104h: 33 04 00 40 4F 01 00 00 00 00 00 (segment application program)"
        );
    }

    #[test]
    fn a_task_control_1_on_a_table_machine_is_refused() {
        let mut image = image();
        image.code.load_procedures[0].steps.insert(
            APPLICATION_COMPLETED,
            LoadStep::TaskCtrl1 {
                lsm: 1,
                address: 0x404F,
                count: 1,
            },
        );
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::Record(
                MemoryLoadRecordError::TaskControlNotAllowed(MemoryLoadStateMachine::AddressTable)
            ))
        );
    }

    /// `[D]` Cookbook *Load Controls* `02_03_01` §2.3: a task segment of
    /// machine 5 is `AbsCObjSeg`, *"for MT-information only. ETS ignores it
    /// and will hence not be transmitted on the bus."* The presence
    /// detectors `M-006A_A-0001-22` carry it before `LoadCompleted`.
    #[test]
    fn the_machine_5_task_segment_is_not_sent_and_needs_no_inference() {
        let mut image = image();
        image.code.load_procedures[0].steps.insert(
            APPLICATION_COMPLETED,
            LoadStep::TaskSegment {
                lsm: 5,
                address: 0x4400,
            },
        );
        let (plan, inferences) = plan_memory_download_with_inferences(&image).expect("a plan");
        assert_eq!(plan, plan_memory_download(&self::image()).unwrap());
        assert!(inferences.is_empty());
    }

    /// The tail of `M-000C_A-5701-10-DCC4` and `M-006A_A-0702-10-7779`:
    /// after the restart, a machine-5 task segment and `Load`. The restart
    /// closed the connection (Cookbook §2.2) and a BIM M112 has no fifth
    /// machine (MP §3.31.2), so nothing is sent — an inference, disclosed.
    #[test]
    fn machine_5_steps_after_the_restart_are_not_sent_and_disclosed() {
        let mut image = image();
        let steps = &mut image.code.load_procedures[0].steps;
        steps.insert(
            RESTART + 1,
            LoadStep::TaskSegment {
                lsm: 5,
                address: 0x4400,
            },
        );
        steps.insert(RESTART + 2, LoadStep::Load { lsm: 5 });
        let (plan, inferences) = plan_memory_download_with_inferences(&image).expect("a plan");
        assert_eq!(plan, plan_memory_download(&self::image()).unwrap());
        assert_eq!(
            inferences,
            vec![Inference {
                rule: MACHINE_5_AFTER_RESTART,
                detail: "LdCtrlLoad LsmIdx=5 after LdCtrlRestart is not sent: the restart \
                         closed the connection and a BIM M112 has no fifth load state machine"
                    .into(),
                reference: "RESEARCH §19.12",
            }]
        );
    }

    #[test]
    fn the_image_inferences_come_with_the_plan() {
        let mut image = image();
        let inference = Inference {
            rule: "union-later-member",
            detail: "UP-1 is not written".into(),
            reference: "RESEARCH §19.12",
        };
        image.inferences.push(inference.clone());
        let (_, inferences) = plan_memory_download_with_inferences(&image).expect("a plan");
        assert_eq!(inferences, vec![inference]);
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
        assert_eq!(
            plan_memory_download(&image),
            Err(DownloadPlanError::MergeFragment {
                position: 2,
                merge_id: "1".into()
            }),
            "a merge fragment is an untranslated shape, not a second procedure"
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
