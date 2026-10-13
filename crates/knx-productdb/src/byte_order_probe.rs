//! Plans a byte-order probe: which device fields to read and what each should hold.
//!
//! KL-170, ADR-0119.
//!
//! The probe asks a device that ETS programmed which arrangement of each
//! multi-octet parameter value its memory holds
//! ([`knx_core::commissioning::byte_order_evidence`]). This module decides,
//! offline and from the product file and the project's values alone, which
//! fields to read: the ones [`crate::image::build_download_image`] would
//! write, activated by the same evaluation ([`crate::image::activate`]),
//! placed by the same rules, with the same value checks.
//!
//! Unlike the image builder it does not refuse a program for one field it
//! cannot read, and it does not refuse a declared `ParameterByteOrder`: the
//! probe exists for exactly those programs. Every field it leaves out is
//! named in [`ProbePlan::skipped`] with the reason, never dropped silently.
//!
//! Read, each in address order:
//!
//! - **Discriminating fields**: whole octets, at least two, whose value
//!   reads differently in the two orders. These answer the question.
//! - **Controls**: fields inside one octet. They have no octet order and
//!   show whether the device holds the project's values at all.
//!
//! Not read: values whose octets are the same in both orders (`0`, `0101h`);
//! fields across an octet boundary that are not whole octets (no arrangement
//! is defined, KL-170); texts; every value or placement the image builder
//! refuses; and segments above 16 bits, which a download refuses. The total
//! is bounded by [`MAX_PROBE_OCTETS`],
//! discriminating fields first.
//!
//! [`base_image_order`] runs the same planning at product defaults and
//! reads the fields from the program's own segment `Data` instead of a
//! device: what the base image suggests about the octet order, the basis of
//! a download's named warning (ADR-0120). A base image is product data, not
//! a device, so that is a suspicion, never a rule.
//!
//! Nothing here encodes a value for a download.

use std::collections::BTreeMap;
use std::fmt;

use knx_core::commissioning::byte_order_evidence::{
    covered_octets, read_field, FieldReading, Tally,
};
use knx_core::commissioning::load_state::MaskVersion;
use knx_core::commissioning::parameter_image::ParameterField;
use rusqlite::Connection;

use crate::code::{load_program_code, ProgramCode};
use crate::download_plan::{check_program_kind, manufacturer_of, DownloadPlanError};
use crate::image::{
    activate, field_of, superseded_union_members, Encoded, ImageError, ImageRequest, ParameterTypes,
};

/// The most octets one probe reads. A bus read carries about a dozen octets,
/// so this bounds a probe to a few hundred reads; a program with more
/// candidate fields has the excess listed in [`ProbePlan::skipped`].
pub const MAX_PROBE_OCTETS: usize = 4096;

/// One field to read, and the value the project says is there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeField {
    /// The `ParameterRef` whose value this is.
    pub parameter_ref: String,
    /// The code segment it lies in.
    pub segment: String,
    /// The device address of the octet at the field's offset.
    pub address: u32,
    /// The field inside its segment.
    pub field: ParameterField,
    /// The value.
    pub value: u64,
}

impl ProbeField {
    /// Whether the field can tell the two orders apart: whole octets, at
    /// least two, and a value whose octets differ in the two orders.
    pub fn discriminates(&self) -> bool {
        discriminates(self.field, self.value)
    }
}

fn discriminates(field: ParameterField, value: u64) -> bool {
    let octets = covered_octets(field);
    field.bit_offset == 0
        && field.size_in_bit.is_multiple_of(8)
        && octets >= 2
        && value.to_be_bytes()[8 - octets..] != value.to_le_bytes()[..octets]
}

/// A parameter the probe does not read, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeSkip {
    /// The `ParameterRef`.
    pub parameter_ref: String,
    /// Why.
    pub reason: SkipReason,
}

/// Why a parameter is not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// Its value reads the same in both orders.
    EitherOrder,
    /// It crosses an octet boundary without being whole octets.
    NoDefinedArrangement,
    /// A text, not a number.
    Text,
    /// A later union member holds its bits.
    SupersededUnionMember,
    /// Another active reference of the same parameter holds another value.
    ConflictingValues,
    /// It shares bits with another field the probe would read.
    Overlaps {
        /// The other `ParameterRef`.
        other: String,
    },
    /// It reaches past the end of its segment.
    OutsideSegment,
    /// The probe's read bound was reached.
    ReadBound,
    /// No value and no default.
    NoValue,
    /// The image builder refuses its value or placement, by this reason.
    Refused(String),
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkipReason::EitherOrder => f.write_str("its value reads the same in both orders"),
            SkipReason::NoDefinedArrangement => f.write_str(
                "it crosses an octet boundary without being whole octets; no arrangement is defined",
            ),
            SkipReason::Text => f.write_str("a text, not a number"),
            SkipReason::SupersededUnionMember => {
                f.write_str("a later member of its union holds its bits")
            }
            SkipReason::ConflictingValues => {
                f.write_str("two active references of its parameter hold different values")
            }
            SkipReason::Overlaps { other } => write!(f, "it shares bits with {other}"),
            SkipReason::OutsideSegment => f.write_str("it reaches past the end of its segment"),
            SkipReason::ReadBound => write!(f, "beyond the probe's bound of {MAX_PROBE_OCTETS} octets"),
            SkipReason::NoValue => f.write_str("no value, and no default"),
            SkipReason::Refused(why) => write!(f, "{why}"),
        }
    }
}

/// What to read from one device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbePlan {
    /// The application program.
    pub program_id: String,
    /// `Options/@ParameterByteOrder` as the program declares it, if at all.
    pub declared_order: Option<String>,
    /// The mask the device must report.
    pub mask: MaskVersion,
    /// The manufacturer the device must report.
    pub manufacturer: u16,
    /// The fields to read: discriminating ones first, each group in address
    /// order.
    pub fields: Vec<ProbeField>,
    /// The parameters not read, in parameter-tree order.
    pub skipped: Vec<ProbeSkip>,
}

impl ProbePlan {
    /// The memory to read: every field's covered octets, merged where they
    /// touch or overlap, in address order.
    pub fn runs(&self) -> Vec<(u32, usize)> {
        let mut spans: Vec<(u32, u32)> = self
            .fields
            .iter()
            .map(|field| {
                (
                    field.address,
                    field.address + covered_octets(field.field) as u32,
                )
            })
            .collect();
        spans.sort_unstable();
        let mut runs: Vec<(u32, u32)> = Vec::new();
        for (start, end) in spans {
            match runs.last_mut() {
                Some((_, last_end)) if start <= *last_end => *last_end = (*last_end).max(end),
                _ => runs.push((start, end)),
            }
        }
        runs.into_iter()
            .map(|(start, end)| (start, (end - start) as usize))
            .collect()
    }
}

/// Why no probe could be planned.
#[derive(Debug)]
pub enum ProbeError {
    /// The program is unknown, unreadable, or its evaluation undecided.
    Image(ImageError),
    /// The program is not one a memory download addresses.
    Program(DownloadPlanError),
}

impl std::error::Error for ProbeError {}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProbeError::Image(error) => write!(f, "{error}"),
            ProbeError::Program(error) => write!(f, "{error}"),
        }
    }
}

impl From<ImageError> for ProbeError {
    fn from(error: ImageError) -> Self {
        ProbeError::Image(error)
    }
}

/// A field the probe would read, before the bound and overlaps are applied.
struct Candidate {
    parameter: String,
    field: ProbeField,
}

/// Plans the probe of the device `request` describes.
pub fn plan_probe(conn: &Connection, request: &ImageRequest) -> Result<ProbePlan, ProbeError> {
    plan_fields(conn, request, MAX_PROBE_OCTETS).map(|(plan, _)| plan)
}

/// What a program's own base image says about the octet order of its
/// multi-octet product defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseImageOrder {
    /// `Options/@ParameterByteOrder` as the program states it; `None` when
    /// absent (the schema default, `BigEndian`).
    pub declared_order: Option<String>,
    /// The readings of every planned field against the segment `Data`.
    pub tally: Tally,
    /// Planned fields whose segment carries no `Data`: nothing to compare.
    pub without_data: usize,
}

/// Plans the probe of the program `request` names at product defaults (its
/// values, links and flag overrides are not used) and reads each field from
/// the program's segment `Data` instead of a device (ADR-0120).
///
/// A field the classifier cannot read counts as not classified; nothing is
/// bounded, because nothing is read from a bus.
pub fn base_image_order(
    conn: &Connection,
    request: &ImageRequest,
) -> Result<BaseImageOrder, ProbeError> {
    let defaults = ImageRequest {
        program_id: request.program_id.clone(),
        individual_address: request.individual_address,
        values: BTreeMap::new(),
        links: Vec::new(),
        flag_overrides: BTreeMap::new(),
    };
    let (plan, code) = plan_fields(conn, &defaults, usize::MAX)?;
    let mut readings = Vec::with_capacity(plan.fields.len());
    let mut without_data = 0;
    for probe in &plan.fields {
        let data = code
            .segment(&probe.segment)
            .and_then(|segment| segment.data.as_deref());
        let Some(data) = data else {
            without_data += 1;
            continue;
        };
        let start = probe.field.offset as usize;
        // The planner kept only fields inside their segment, and `Data` is
        // as long as the segment; `get` keeps that an assumption, not a
        // panic.
        let reading = data
            .get(start..start + covered_octets(probe.field))
            .and_then(|held| read_field(probe.field, probe.value, held).ok())
            .unwrap_or(FieldReading::NotClassified);
        readings.push(reading);
    }
    Ok(BaseImageOrder {
        declared_order: plan.declared_order,
        tally: Tally::of(&readings),
        without_data,
    })
}

/// [`plan_probe`] with the octet bound `bound`, and the program's code.
fn plan_fields(
    conn: &Connection,
    request: &ImageRequest,
    bound: usize,
) -> Result<(ProbePlan, ProgramCode), ProbeError> {
    let program_id = request.program_id.as_str();
    let code = load_program_code(conn, program_id)
        .map_err(ImageError::from)?
        .ok_or_else(|| ImageError::UnknownProgram(program_id.to_string()))?;
    let mask = check_program_kind(&code).map_err(ProbeError::Program)?;
    let manufacturer = manufacturer_of(program_id).map_err(ProbeError::Program)?;
    let types = ParameterTypes::load(conn, program_id, &code)?;
    let (values, activation) = activate(conn, request, &types)?;
    let (superseded, _) = superseded_union_members(&activation.parameter_refs, &types, &code);

    let mut skipped = Vec::new();
    let skip = |skipped: &mut Vec<ProbeSkip>, parameter_ref: &str, reason| {
        skipped.push(ProbeSkip {
            parameter_ref: parameter_ref.to_string(),
            reason,
        })
    };
    // By parameter: the candidate, or `None` once two refs disagree.
    let mut by_parameter: BTreeMap<String, Option<(String, u64)>> = BTreeMap::new();
    let mut candidates: Vec<Candidate> = Vec::new();
    for active in &activation.parameter_refs {
        let parameter_ref = active.ref_id.as_str();
        let parameter = types.parameter_of(parameter_ref)?;
        let Some(placement) = code.parameters.get(parameter) else {
            continue; // Lives nowhere in memory: nothing to read.
        };
        if superseded.contains(parameter) {
            skip(
                &mut skipped,
                parameter_ref,
                SkipReason::SupersededUnionMember,
            );
            continue;
        }
        let Some(raw) = values.get_unscoped(parameter_ref) else {
            skip(&mut skipped, parameter_ref, SkipReason::NoValue);
            continue;
        };
        let value = match types.value(parameter_ref, raw) {
            Ok(Encoded::Number(value)) => value,
            Ok(Encoded::Text { .. }) => {
                skip(&mut skipped, parameter_ref, SkipReason::Text);
                continue;
            }
            Err(error) => {
                skip(
                    &mut skipped,
                    parameter_ref,
                    SkipReason::Refused(error.to_string()),
                );
                continue;
            }
        };
        match by_parameter.get_mut(parameter) {
            Some(Some((_, earlier))) if *earlier == value => continue,
            Some(entry @ Some(_)) => {
                *entry = None;
                skip(&mut skipped, parameter_ref, SkipReason::ConflictingValues);
                continue;
            }
            Some(None) => {
                skip(&mut skipped, parameter_ref, SkipReason::ConflictingValues);
                continue;
            }
            None => {
                by_parameter.insert(
                    parameter.to_string(),
                    Some((parameter_ref.to_string(), value)),
                );
            }
        }
        let placed = types
            .size(parameter_ref)
            .and_then(|size| field_of(parameter, placement, size));
        let (segment_id, field) = match placed {
            Ok(placed) => placed,
            Err(error) => {
                skip(
                    &mut skipped,
                    parameter_ref,
                    SkipReason::Refused(error.to_string()),
                );
                continue;
            }
        };
        let Some(segment) = code.segment(segment_id) else {
            skip(
                &mut skipped,
                parameter_ref,
                SkipReason::Refused(format!("no segment {segment_id} in the program")),
            );
            continue;
        };
        if u64::from(field.offset) + covered_octets(field) as u64 > u64::from(segment.size) {
            skip(&mut skipped, parameter_ref, SkipReason::OutsideSegment);
            continue;
        }
        // A download refuses a segment above 16 bits
        // (`DownloadPlanError::Address`), so nothing there is read; this also
        // keeps every address below far from `u32` overflow.
        if u16::try_from(segment.address).is_err() {
            skip(
                &mut skipped,
                parameter_ref,
                SkipReason::Refused(DownloadPlanError::Address(segment.address).to_string()),
            );
            continue;
        }
        let single_octet = covered_octets(field) == 1;
        if !single_octet && !discriminates(field, value) {
            let reason = if field.bit_offset == 0 && field.size_in_bit.is_multiple_of(8) {
                SkipReason::EitherOrder
            } else {
                SkipReason::NoDefinedArrangement
            };
            skip(&mut skipped, parameter_ref, reason);
            continue;
        }
        candidates.push(Candidate {
            parameter: parameter.to_string(),
            field: ProbeField {
                parameter_ref: parameter_ref.to_string(),
                segment: segment_id.to_string(),
                address: segment.address + field.offset,
                field,
                value,
            },
        });
    }

    // A parameter two refs disagree about is not read under either.
    candidates.retain(|candidate| {
        let agreed = matches!(by_parameter.get(&candidate.parameter), Some(Some(_)));
        if !agreed {
            skip(
                &mut skipped,
                &candidate.field.parameter_ref,
                SkipReason::ConflictingValues,
            );
        }
        agreed
    });
    let candidates = without_overlaps(candidates, &mut skipped);

    // Discriminating fields first, then controls, each in address order,
    // up to the bound.
    let (mut fields, mut controls): (Vec<ProbeField>, Vec<ProbeField>) = candidates
        .into_iter()
        .map(|candidate| candidate.field)
        .partition(ProbeField::discriminates);
    fields.sort_by_key(|field| field.address);
    controls.sort_by_key(|field| field.address);
    fields.append(&mut controls);
    let kept = within_bound(fields, bound, &mut skipped);

    let plan = ProbePlan {
        program_id: program_id.to_string(),
        declared_order: code.options.get("ParameterByteOrder").cloned(),
        mask,
        manufacturer,
        fields: kept,
        skipped,
    };
    Ok((plan, code))
}

/// The leading `fields` whose covered octets add up to at most `bound`;
/// every later one is skipped. Once one field misses, smaller later ones are
/// skipped too, so the fields read are always a prefix of the order given.
fn within_bound(
    fields: Vec<ProbeField>,
    bound: usize,
    skipped: &mut Vec<ProbeSkip>,
) -> Vec<ProbeField> {
    let mut octets = 0usize;
    let mut full = false;
    let mut kept = Vec::with_capacity(fields.len());
    for field in fields {
        let length = covered_octets(field.field);
        full = full || octets + length > bound;
        if full {
            skipped.push(ProbeSkip {
                parameter_ref: field.parameter_ref,
                reason: SkipReason::ReadBound,
            });
            continue;
        }
        octets += length;
        kept.push(field);
    }
    kept
}

/// Drops both of every two candidates that share a bit: the device holds
/// at most one of them there, and the probe cannot know which.
fn without_overlaps(candidates: Vec<Candidate>, skipped: &mut Vec<ProbeSkip>) -> Vec<Candidate> {
    let span = |field: &ProbeField| {
        let start = u64::from(field.address) * 8 + u64::from(field.field.bit_offset);
        (start, start + u64::from(field.field.size_in_bit))
    };
    let mut overlapping: BTreeMap<usize, String> = BTreeMap::new();
    for (i, a) in candidates.iter().enumerate() {
        let (a_start, a_end) = span(&a.field);
        for (j, b) in candidates.iter().enumerate().skip(i + 1) {
            let (b_start, b_end) = span(&b.field);
            if a_start < b_end && b_start < a_end {
                overlapping
                    .entry(i)
                    .or_insert_with(|| b.field.parameter_ref.clone());
                overlapping
                    .entry(j)
                    .or_insert_with(|| a.field.parameter_ref.clone());
            }
        }
    }
    candidates
        .into_iter()
        .enumerate()
        .filter_map(|(i, candidate)| match overlapping.remove(&i) {
            Some(other) => {
                skipped.push(ProbeSkip {
                    parameter_ref: candidate.field.parameter_ref.clone(),
                    reason: SkipReason::Overlaps { other },
                });
                None
            }
            None => Some(candidate),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::commissioning::byte_order_evidence::{read_field, OctetOrder, Tally, Verdict};
    use knx_core::IndividualAddress;

    const ID: &str = "M-0002_A-0001-01-0000";

    /// A little-endian mask-`0701h` program with one parameter per probe
    /// decision. Segment `AS-4400` is 32 octets at `4400h`.
    const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0002">
<ApplicationPrograms><ApplicationProgram Id="M-0002_A-0001-01-0000" Name="P" ApplicationNumber="1"
  ApplicationVersion="1" MaskVersion="MV-0701" LoadProcedureStyle="ProductProcedure"><Static>
<Code>
  <AbsoluteSegment Id="AS-4400" Address="17408" Size="32"><Data>AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=</Data></AbsoluteSegment>
</Code>
<ParameterTypes>
  <ParameterType Id="PT-U16" Name="U16"><TypeNumber SizeInBit="16" Type="unsignedInt" minInclusive="0" maxInclusive="65535" /></ParameterType>
  <ParameterType Id="PT-U32" Name="U32"><TypeNumber SizeInBit="32" Type="unsignedInt" minInclusive="0" maxInclusive="4294967295" /></ParameterType>
  <ParameterType Id="PT-U8" Name="U8"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255" /></ParameterType>
  <ParameterType Id="PT-U3" Name="U3"><TypeNumber SizeInBit="3" Type="unsignedInt" minInclusive="0" maxInclusive="7" /></ParameterType>
  <ParameterType Id="PT-U12" Name="U12"><TypeNumber SizeInBit="12" Type="unsignedInt" minInclusive="0" maxInclusive="4095" /></ParameterType>
  <ParameterType Id="PT-S16" Name="S16"><TypeNumber SizeInBit="16" Type="signedInt" minInclusive="-100" maxInclusive="100" /></ParameterType>
  <ParameterType Id="PT-T" Name="T"><TypeText SizeInBit="16" /></ParameterType>
  <ParameterType Id="PT-F" Name="F"><TypeFloat Encoding="DPT 9" minInclusive="0" maxInclusive="100" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-W16" Name="W16" ParameterType="PT-U16" Value="300"><Memory CodeSegment="AS-4400" Offset="0" BitOffset="0" /></Parameter>
  <Parameter Id="P-W32" Name="W32" ParameterType="PT-U32" Value="16909060"><Memory CodeSegment="AS-4400" Offset="2" BitOffset="0" /></Parameter>
  <Parameter Id="P-Z16" Name="Z16" ParameterType="PT-U16" Value="0"><Memory CodeSegment="AS-4400" Offset="6" BitOffset="0" /></Parameter>
  <Parameter Id="P-B3" Name="B3" ParameterType="PT-U3" Value="5"><Memory CodeSegment="AS-4400" Offset="10" BitOffset="2" /></Parameter>
  <Parameter Id="P-X12" Name="X12" ParameterType="PT-U12" Value="1"><Memory CodeSegment="AS-4400" Offset="11" BitOffset="0" /></Parameter>
  <Parameter Id="P-T" Name="T" ParameterType="PT-T" Value="ab"><Memory CodeSegment="AS-4400" Offset="13" BitOffset="0" /></Parameter>
  <Parameter Id="P-F" Name="F" ParameterType="PT-F" Value="1"><Memory CodeSegment="AS-4400" Offset="15" BitOffset="0" /></Parameter>
  <Parameter Id="P-S" Name="S" ParameterType="PT-S16" Value="-1"><Memory CodeSegment="AS-4400" Offset="17" BitOffset="0" /></Parameter>
  <Parameter Id="P-O1" Name="O1" ParameterType="PT-U8" Value="1"><Memory CodeSegment="AS-4400" Offset="20" BitOffset="0" /></Parameter>
  <Parameter Id="P-O2" Name="O2" ParameterType="PT-U8" Value="2"><Memory CodeSegment="AS-4400" Offset="20" BitOffset="0" /></Parameter>
  <Parameter Id="P-C" Name="C" ParameterType="PT-U16" Value="258"><Memory CodeSegment="AS-4400" Offset="21" BitOffset="0" /></Parameter>
  <Union SizeInBit="16">
    <Memory CodeSegment="AS-4400" Offset="24" BitOffset="0" />
    <Parameter Id="UP-1" Name="Wide" ParameterType="PT-U16" Offset="0" BitOffset="0" Value="513" />
    <Parameter Id="UP-2" Name="Narrow" ParameterType="PT-U8" Offset="0" BitOffset="0" Value="9" />
  </Union>
  <Parameter Id="P-P" Name="Prop" ParameterType="PT-U8" Value="1"><Property ObjectIndex="1" PropertyId="2" Offset="0" BitOffset="0" /></Parameter>
  <Parameter Id="P-OUT" Name="Out" ParameterType="PT-U16" Value="772"><Memory CodeSegment="AS-4400" Offset="31" BitOffset="0" /></Parameter>
  <Parameter Id="P-NONE" Name="Nowhere" ParameterType="PT-U8" Value="1" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="P-W16_R-1" RefId="P-W16" />
  <ParameterRef Id="P-W32_R-1" RefId="P-W32" />
  <ParameterRef Id="P-Z16_R-1" RefId="P-Z16" />
  <ParameterRef Id="P-B3_R-1" RefId="P-B3" />
  <ParameterRef Id="P-X12_R-1" RefId="P-X12" />
  <ParameterRef Id="P-T_R-1" RefId="P-T" />
  <ParameterRef Id="P-F_R-1" RefId="P-F" />
  <ParameterRef Id="P-S_R-1" RefId="P-S" />
  <ParameterRef Id="P-O1_R-1" RefId="P-O1" />
  <ParameterRef Id="P-O2_R-1" RefId="P-O2" />
  <ParameterRef Id="P-C_R-1" RefId="P-C" />
  <ParameterRef Id="P-C_R-2" RefId="P-C" />
  <ParameterRef Id="UP-1_R-1" RefId="UP-1" />
  <ParameterRef Id="UP-2_R-1" RefId="UP-2" />
  <ParameterRef Id="P-P_R-1" RefId="P-P" />
  <ParameterRef Id="P-OUT_R-1" RefId="P-OUT" />
  <ParameterRef Id="P-NONE_R-1" RefId="P-NONE" />
</ParameterRefs>
</Static>
<Dynamic><Channel Id="CH" Text="C"><ParameterBlock Id="PB" Text="B">
  <ParameterRefRef RefId="P-W16_R-1" /><ParameterRefRef RefId="P-W32_R-1" />
  <ParameterRefRef RefId="P-Z16_R-1" /><ParameterRefRef RefId="P-B3_R-1" />
  <ParameterRefRef RefId="P-X12_R-1" /><ParameterRefRef RefId="P-T_R-1" />
  <ParameterRefRef RefId="P-F_R-1" /><ParameterRefRef RefId="P-S_R-1" />
  <ParameterRefRef RefId="P-O1_R-1" /><ParameterRefRef RefId="P-O2_R-1" />
  <ParameterRefRef RefId="P-C_R-1" /><ParameterRefRef RefId="P-C_R-2" />
  <ParameterRefRef RefId="UP-1_R-1" /><ParameterRefRef RefId="UP-2_R-1" />
  <ParameterRefRef RefId="P-P_R-1" /><ParameterRefRef RefId="P-OUT_R-1" />
  <ParameterRefRef RefId="P-NONE_R-1" />
</ParameterBlock></Channel></Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    fn db(xml: &str) -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().expect("tempdir");
        let conn = crate::open_and_migrate(&dir.path().join("products.sqlite")).expect("database");
        crate::ingest_file(&conn, "M-0002/A.xml", xml.as_bytes()).expect("ingests");
        (dir, conn)
    }

    fn little_endian() -> String {
        PROGRAM.replace(
            "<Code>",
            r#"<Options ParameterByteOrder="LittleEndian" /><Code>"#,
        )
    }

    fn request(values: &[(&str, &str)]) -> ImageRequest {
        ImageRequest {
            program_id: ID.to_string(),
            individual_address: IndividualAddress::new(1, 1, 5).expect("valid"),
            values: values
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            links: vec![],
            flag_overrides: BTreeMap::new(),
        }
    }

    fn reason<'a>(plan: &'a ProbePlan, parameter_ref: &str) -> Vec<&'a SkipReason> {
        plan.skipped
            .iter()
            .filter(|skip| skip.parameter_ref == parameter_ref)
            .map(|skip| &skip.reason)
            .collect()
    }

    fn planned() -> ProbePlan {
        let (_dir, conn) = db(&little_endian());
        plan_probe(&conn, &request(&[("P-C_R-1", "258"), ("P-C_R-2", "259")])).expect("plans")
    }

    #[test]
    fn a_little_endian_program_is_planned_not_refused() {
        let plan = planned();
        assert_eq!(plan.declared_order.as_deref(), Some("LittleEndian"));
        assert_eq!(plan.mask, MaskVersion(0x0701));
        assert_eq!(plan.manufacturer, 0x0002);
        let read: Vec<(&str, u32, u64)> = plan
            .fields
            .iter()
            .map(|field| (field.parameter_ref.as_str(), field.address, field.value))
            .collect();
        // Discriminating fields first, then the single-octet controls, each
        // in address order; the later union member wins its bits.
        assert_eq!(
            read,
            vec![
                ("P-W16_R-1", 0x4400, 300),
                ("P-W32_R-1", 0x4402, 0x0102_0304),
                ("P-B3_R-1", 0x440A, 5),
                ("UP-2_R-1", 0x4418, 9),
            ]
        );
    }

    #[test]
    fn every_parameter_left_out_is_named_with_its_reason() {
        let plan = planned();
        assert_eq!(reason(&plan, "P-Z16_R-1"), [&SkipReason::EitherOrder]);
        assert_eq!(
            reason(&plan, "P-X12_R-1"),
            [&SkipReason::NoDefinedArrangement]
        );
        assert_eq!(reason(&plan, "P-T_R-1"), [&SkipReason::Text]);
        assert!(
            matches!(reason(&plan, "P-F_R-1")[..], [SkipReason::Refused(why)] if why.contains("Float"))
        );
        assert!(
            matches!(reason(&plan, "P-S_R-1")[..], [SkipReason::Refused(why)] if why.contains("negative"))
        );
        assert_eq!(
            reason(&plan, "P-O1_R-1"),
            [&SkipReason::Overlaps {
                other: "P-O2_R-1".into()
            }]
        );
        assert_eq!(
            reason(&plan, "P-O2_R-1"),
            [&SkipReason::Overlaps {
                other: "P-O1_R-1".into()
            }]
        );
        assert_eq!(reason(&plan, "P-C_R-1"), [&SkipReason::ConflictingValues]);
        assert_eq!(reason(&plan, "P-C_R-2"), [&SkipReason::ConflictingValues]);
        assert_eq!(
            reason(&plan, "UP-1_R-1"),
            [&SkipReason::SupersededUnionMember]
        );
        assert!(
            matches!(reason(&plan, "P-P_R-1")[..], [SkipReason::Refused(why)] if why.contains("Property"))
        );
        assert_eq!(reason(&plan, "P-OUT_R-1"), [&SkipReason::OutsideSegment]);
        // A parameter that lives nowhere in memory has nothing to read and
        // nothing to explain.
        assert!(reason(&plan, "P-NONE_R-1").is_empty());
        // Nothing read is also skipped, and nothing is both.
        for field in &plan.fields {
            assert!(reason(&plan, &field.parameter_ref).is_empty());
        }
    }

    #[test]
    fn two_agreeing_references_are_read_once() {
        let (_dir, conn) = db(&little_endian());
        let plan =
            plan_probe(&conn, &request(&[("P-C_R-1", "258"), ("P-C_R-2", "258")])).expect("plans");
        let c: Vec<_> = plan
            .fields
            .iter()
            .filter(|field| field.parameter_ref.starts_with("P-C_"))
            .collect();
        assert_eq!(c.len(), 1);
        assert_eq!((c[0].address, c[0].value), (0x4415, 258));
        assert!(reason(&plan, "P-C_R-2").is_empty());
    }

    #[test]
    fn the_probe_reads_any_declared_order_and_the_image_still_refuses_little_endian() {
        let (_dir, conn) = db(&little_endian());
        let values = [("P-C_R-1", "258"), ("P-C_R-2", "258")];
        assert!(matches!(
            crate::image::build_download_image(&conn, &request(&values)),
            Err(ImageError::Unsupported { what }) if what.contains("ParameterByteOrder")
        ));
        let (_dir, conn) = db(PROGRAM);
        let plan = plan_probe(&conn, &request(&values)).expect("plans");
        assert_eq!(plan.declared_order, None);
        assert!(!plan.fields.is_empty());
    }

    #[test]
    fn a_program_no_memory_download_addresses_is_refused() {
        let (_dir, conn) = db(&PROGRAM.replace("MV-0701", "MV-07B0"));
        assert!(matches!(
            plan_probe(&conn, &request(&[])),
            Err(ProbeError::Program(DownloadPlanError::NotMemoryMapped(_)))
        ));
        let (_dir, conn) = db(PROGRAM);
        let mut unknown = request(&[]);
        unknown.program_id = "M-0002_A-FFFF-01-0000".into();
        assert!(matches!(
            plan_probe(&conn, &unknown),
            Err(ProbeError::Image(ImageError::UnknownProgram(_)))
        ));
    }

    #[test]
    fn runs_merge_touching_fields_and_keep_gaps() {
        let plan = planned();
        // 4400h+2 and 4402h+4 touch; 440Ah+1; 4418h+1.
        assert_eq!(plan.runs(), vec![(0x4400, 6), (0x440A, 1), (0x4418, 1)]);
    }

    #[test]
    fn the_bound_keeps_a_prefix_and_names_the_rest() {
        let field = |name: &str, address: u32, size_in_bit: u32| ProbeField {
            parameter_ref: name.into(),
            segment: "AS".into(),
            address,
            field: ParameterField {
                offset: 0,
                bit_offset: 0,
                size_in_bit,
            },
            value: 1,
        };
        let mut skipped = Vec::new();
        let kept = within_bound(
            vec![field("a", 0, 16), field("b", 2, 16), field("c", 4, 8)],
            3,
            &mut skipped,
        );
        assert_eq!(kept.len(), 1);
        // `c` would still fit, but the fields read stay a prefix.
        assert_eq!(
            skipped,
            vec![
                ProbeSkip {
                    parameter_ref: "b".into(),
                    reason: SkipReason::ReadBound
                },
                ProbeSkip {
                    parameter_ref: "c".into(),
                    reason: SkipReason::ReadBound
                },
            ]
        );
    }

    /// The plan and the classifier together: a memory laid out low octet
    /// first reads as such, one laid out high octet first as that, and the
    /// controls agree either way.
    #[test]
    fn a_planned_probe_tells_a_low_first_device_from_a_high_first_one() {
        let plan = planned();
        let device = |order: OctetOrder| {
            let mut memory = vec![0u8; 32];
            for field in &plan.fields {
                let at = (field.address - 0x4400) as usize;
                let n = covered_octets(field.field);
                if n == 1 {
                    let shift = 8 - u32::from(field.field.bit_offset) - field.field.size_in_bit;
                    memory[at] |= (field.value as u8) << shift;
                } else {
                    let octets = match order {
                        OctetOrder::HighFirst => field.value.to_be_bytes()[8 - n..].to_vec(),
                        OctetOrder::LowFirst => field.value.to_le_bytes()[..n].to_vec(),
                    };
                    memory[at..at + n].copy_from_slice(&octets);
                }
            }
            memory
        };
        for order in [OctetOrder::HighFirst, OctetOrder::LowFirst] {
            let memory = device(order);
            let readings: Vec<_> = plan
                .fields
                .iter()
                .map(|field| {
                    let at = (field.address - 0x4400) as usize;
                    read_field(
                        field.field,
                        field.value,
                        &memory[at..at + covered_octets(field.field)],
                    )
                    .expect("reads")
                })
                .collect();
            let tally = Tally::of(&readings);
            assert_eq!(tally.verdict(), Verdict::Consistent(order));
            assert_eq!(tally.single_octet_agree, 2);
            assert_eq!(tally.single_octet_disagree, 0);
        }
    }

    #[test]
    fn a_segment_a_download_cannot_address_is_named_not_read() {
        // A download refuses a segment above 16 bits
        // (`DownloadPlanError::Address`); the probe reads only what a
        // download writes, so it names every field there instead, and an
        // address near the top of `u32` must not overflow.
        let (_dir, conn) =
            db(&little_endian().replace(r#"Address="17408""#, r#"Address="4294967290""#));
        let plan = plan_probe(&conn, &request(&[])).expect("plans");
        assert_eq!(plan.fields, vec![], "nothing read");
        let refused = SkipReason::Refused("address FFFFFFFAh does not fit 16 bits".to_string());
        for parameter_ref in ["P-W16_R-1", "P-W32_R-1", "P-B3_R-1", "UP-2_R-1"] {
            assert_eq!(
                reason(&plan, parameter_ref),
                vec![&refused],
                "{parameter_ref}"
            );
        }
    }

    /// The base image with every planned multi-octet default of [`PROGRAM`]
    /// (`W16`, `W32`, `C`) in `order`, `W16` low first instead when `mixed`,
    /// and the controls `B3` and `UP-2` at their defaults.
    fn base_image(order: OctetOrder, mixed: bool) -> String {
        let mut data = [0u8; 32];
        let put = |data: &mut [u8; 32], at: usize, value: u64, octets: usize, order| {
            let bytes = match order {
                OctetOrder::HighFirst => value.to_be_bytes()[8 - octets..].to_vec(),
                OctetOrder::LowFirst => value.to_le_bytes()[..octets].to_vec(),
            };
            data[at..at + octets].copy_from_slice(&bytes);
        };
        put(
            &mut data,
            0,
            300,
            2,
            if mixed { OctetOrder::LowFirst } else { order },
        );
        put(&mut data, 2, 16_909_060, 4, order);
        put(&mut data, 21, 258, 2, order);
        // UP-1 (513) is superseded by the later UP-2: a one-octet control.
        data[24] = 9;
        data[10] = 5 << 3;
        use base64::Engine as _;
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        PROGRAM.replace("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=", &encoded)
    }

    #[test]
    fn a_base_image_holding_defaults_low_first_says_so() {
        let (_dir, conn) = db(&base_image(OctetOrder::LowFirst, false));
        // Project values are not what a base image holds: they are ignored.
        let order = base_image_order(&conn, &request(&[("P-W16_R-1", "7")])).expect("checks");
        assert_eq!(order.declared_order, None, "undeclared");
        assert_eq!(
            order.tally.verdict(),
            Verdict::Consistent(OctetOrder::LowFirst)
        );
        assert_eq!((order.tally.low_first, order.tally.high_first), (3, 0));
        assert_eq!(
            order.tally.single_octet_agree, 2,
            "B3 and UP-2 at their defaults"
        );
        assert_eq!(order.without_data, 0);
    }

    #[test]
    fn a_base_image_holding_defaults_high_first_or_mixed_says_that() {
        let (_dir, conn) = db(&base_image(OctetOrder::HighFirst, false));
        let order = base_image_order(&conn, &request(&[])).expect("checks");
        assert_eq!(
            order.tally.verdict(),
            Verdict::Consistent(OctetOrder::HighFirst)
        );
        let (_dir, conn) = db(&base_image(OctetOrder::HighFirst, true));
        let order = base_image_order(&conn, &request(&[])).expect("checks");
        assert_eq!(order.tally.verdict(), Verdict::Contradictory);
        assert_eq!((order.tally.low_first, order.tally.high_first), (1, 2));
    }

    #[test]
    fn a_zero_base_image_tells_nothing_and_no_data_is_counted() {
        let (_dir, conn) = db(PROGRAM);
        let order = base_image_order(&conn, &request(&[])).expect("checks");
        assert_eq!(order.tally.verdict(), Verdict::Inconclusive);
        assert_eq!(order.tally.neither, 3, "zeros hold neither arrangement");
        let without = PROGRAM.replace(
            "<Data>AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=</Data>",
            "",
        );
        let (_dir, conn) = db(&without);
        let order = base_image_order(&conn, &request(&[])).expect("checks");
        assert_eq!(order.tally, Tally::default());
        assert_eq!(
            order.without_data, 5,
            "three multi-octet fields, B3 and UP-2"
        );
    }
}
