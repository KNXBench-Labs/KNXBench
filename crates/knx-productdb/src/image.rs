//! Segment images for an application download, built from the product file and the chosen values.
//!
//! This follows ADR-0044. The Cookbook *Load Controls* (`02_03_01`
//! v01.00.02, pp. 6–7) describes what a tool writes: the product's
//! *"default memory image"*, modified by *"1) group objects 2) group
//! addresses 3) device parameters"*. This module makes those three changes,
//! and nothing else, to the segment `Data` that [`crate::code`] reads:
//!
//! 1. **Parameters.** Every parameter the `Dynamic` tree activates for the
//!    chosen values, and that has a memory placement, is written over the
//!    segment's base data with [`knx_core::commissioning::parameter_image`].
//!    Its width comes from its type's `SizeInBit`, its value from the value
//!    map.
//! 2. **Group objects.** The activated `ComObjectRef`s become the active
//!    descriptors of the group object table
//!    ([`knx_core::commissioning::group_object_table`]).
//! 3. **Group addresses.** The links become the group address table and the
//!    association table ([`knx_core::commissioning::group_tables`]), with the
//!    device's individual address in the address table's first slot.
//!
//! What is not documented is refused, never guessed:
//!
//! - **Evaluation.** A `Diagnostic` from the `Dynamic` evaluation refuses
//!   the build, with one exception. A `choose` whose controlling value is
//!   one its parameter's type allows, but which no `when` covers,
//!   activates nothing. That is the evaluator's reading (RESEARCH §4.3:
//!   5570 of 8732 default-less `choose` elements have such a value), and
//!   `[V]` it rebuilds a real device's parameter segment, where ten such
//!   `choose`s are evaluated. A value the type does not allow is refused.
//! - **Placement.** A `Property` placement, or any placement
//!   [`crate::code`] kept as unmodelled, is refused by name.
//! - **Union members.** `[V]` A union member lies at the union's placement
//!   plus its own `@Offset`/`@BitOffset`. No PDF states this. The rule
//!   rebuilds a real device's parameter segment octet for octet
//!   (docs/RESEARCH.md §19.1).
//! - **Values.** `TypeRestriction` (base `Value`) and unsigned `TypeNumber`
//!   values are written as numbers. A signed `TypeNumber` is written only at
//!   or above zero: `[V]` 213 signed fields of the corpus's `070nh` base
//!   images hold their non-negative default high octet first and none low
//!   octet first, but no base image holds a negative default and no PDF
//!   says how one is stored (RESEARCH §19.9). `TypeText` is written as its
//!   characters' octets in the program's `Options/@TextParameterEncoding`,
//!   first character first, zero-filled to the field: `[V]` 262 text fields
//!   of those base images hold their default exactly so, 8 of them filling
//!   the field with no terminator. Only `iso-8859-1` and `iso-8859-15`, the
//!   two those programs declare, are understood; without a declaration only
//!   ASCII, whose octets every declared encoding shares, is written.
//!   `TypeFloat` is refused: 48 `DPT 9` fields match that encoding only at
//!   zero, and the 15 non-zero ones contradict it. Every other type is
//!   refused by name.
//! - **Byte order.** A program whose `Options/@ParameterByteOrder` is
//!   anything but `BigEndian` is refused: numbers are written high octet
//!   first (`knx_core::commissioning::parameter_image`).
//! - **Priority.** `[A]` *Project Schema23* lists `Low`, `High` and `Alert`
//!   (§1.1.2.4), and *Resources* lists `System`, `Urgent`, `Normal` and
//!   `Low` (§4.18.3.1.2.1). No PDF maps one set onto the other, so only an
//!   absent priority and `Low` are written, as `Low`, the reading the
//!   device confirms. `High` and `Alert` are refused.
//! - **Mask.** `[A]` A segment's `Mask` is read as "the tool does not
//!   write these octets" (ADR-0044). Each [`SegmentImage`] carries the mask,
//!   and a writer must skip every octet whose mask octet is not `FFh`. The
//!   image still holds the intended value there. The one masked octets the
//!   image may change are the address table's individual-address slot: the
//!   device keeps its own address there, and a real device's slot holds it
//!   (`[V]`, docs/RESEARCH.md §19.1). Any other change to a masked octet is
//!   refused.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use knx_core::commissioning::group_object_table::{
    write_group_object_table, ActiveObject, GroupObjectTableError, ObjectFlags,
    TransmissionPriority, ValueType,
};
use knx_core::commissioning::group_tables::{
    build_group_tables, GroupTableError, TableCapacity, TableLink,
};
use knx_core::commissioning::parameter_image::{
    ParameterField, ParameterImage, ParameterImageError,
};
use knx_core::IndividualAddress;
use rusqlite::Connection;

use crate::code::{
    load_program_code, AbsoluteSegment, CodeError, MemoryPlacement, ParameterPlacement,
    ProgramCode, TablePlacement,
};
use crate::dynamic::{evaluate, load_program_trees, resolve_values, Diagnostic, ScopedDiagnostic};
use crate::ProductDbError;

/// One group address link: object `object` (its `Number`) uses
/// `group_address`; `sending` marks the address it transmits on.
pub type Link = TableLink;

/// What to build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRequest {
    /// The application program.
    pub program_id: String,
    /// The device's individual address, for the address table.
    pub individual_address: IndividualAddress,
    /// Chosen parameter values by `ParameterRef` id. Everything absent
    /// takes its product default.
    pub values: BTreeMap<String, String>,
    /// The group address links.
    pub links: Vec<Link>,
}

/// One segment's finished image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentImage {
    /// The segment id.
    pub id: String,
    /// Its first memory address.
    pub address: u32,
    /// The image, as long as the segment.
    pub octets: Vec<u8>,
    /// The product's `Mask`, if any: an octet whose mask octet is not
    /// `FFh` must not be written.
    pub mask: Option<Vec<u8>>,
}

/// The images of every segment the product ships data for, in the order
/// the product file lists them, and what went into them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadImage {
    /// The program's code, as read from its product file.
    pub code: ProgramCode,
    /// One image per segment with `Data`.
    pub segments: Vec<SegmentImage>,
    /// The numeric parameters written, by `ParameterRef` id, with their
    /// values.
    pub parameters: BTreeMap<String, u64>,
    /// The text parameters written, by `ParameterRef` id, with their
    /// values.
    pub texts: BTreeMap<String, String>,
    /// The active group objects, by number.
    pub objects: Vec<ActiveObject>,
}

impl DownloadImage {
    /// The image of the segment with this id.
    pub fn segment(&self, id: &str) -> Option<&SegmentImage> {
        self.segments.iter().find(|segment| segment.id == id)
    }
}

/// Why no image was built. Every variant names what it refused.
#[derive(Debug)]
pub enum ImageError {
    /// The database failed.
    Database(ProductDbError),
    /// The program's code could not be read.
    Code(CodeError),
    /// The database does not know the program.
    UnknownProgram(String),
    /// The `Dynamic` evaluation could not decide everything.
    Evaluation(Vec<ScopedDiagnostic>),
    /// A product structure this module does not write.
    Unsupported {
        /// What it is, naming the parameter, object or segment.
        what: String,
    },
    /// A value that does not fit its parameter.
    Value {
        /// The `ParameterRef`.
        parameter_ref: String,
        /// Why.
        cause: String,
    },
    /// Two active `ParameterRef`s of one parameter disagree.
    ConflictingValues {
        /// The parameter.
        parameter: String,
    },
    /// The parameter image refused a write.
    Parameter {
        /// The `ParameterRef`.
        parameter_ref: String,
        /// The refusal.
        error: ParameterImageError,
    },
    /// The group object table refused.
    GroupObjects(GroupObjectTableError),
    /// The group tables refused.
    GroupTables(GroupTableError),
    /// A link names a group object the chosen values do not activate.
    InactiveObject {
        /// The object number.
        object: u8,
    },
    /// A change would hit an octet the segment's `Mask` marks.
    Masked {
        /// The segment.
        segment: String,
        /// The octet offset in it.
        offset: usize,
    },
}

impl std::error::Error for ImageError {}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::Database(error) => write!(f, "{error}"),
            ImageError::Code(error) => write!(f, "{error}"),
            ImageError::UnknownProgram(id) => write!(f, "{id}: not in the product database"),
            ImageError::Evaluation(diagnostics) => write!(
                f,
                "the parameter tree could not be evaluated completely ({} diagnostics, first: {:?})",
                diagnostics.len(),
                diagnostics.first().map(|d| &d.diagnostic)
            ),
            ImageError::Unsupported { what } => write!(f, "not supported for a download: {what}"),
            ImageError::Value {
                parameter_ref,
                cause,
            } => write!(f, "{parameter_ref}: {cause}"),
            ImageError::ConflictingValues { parameter } => {
                write!(f, "{parameter}: two active references hold different values")
            }
            ImageError::Parameter {
                parameter_ref,
                error,
            } => write!(f, "{parameter_ref}: {error}"),
            ImageError::GroupObjects(error) => write!(f, "{error}"),
            ImageError::GroupTables(error) => write!(f, "{error}"),
            ImageError::InactiveObject { object } => write!(
                f,
                "group object {object} is linked, but the chosen parameters do not activate it"
            ),
            ImageError::Masked { segment, offset } => write!(
                f,
                "{segment}: the change at octet {offset} hits an octet the product's Mask marks"
            ),
        }
    }
}

impl From<ProductDbError> for ImageError {
    fn from(error: ProductDbError) -> Self {
        ImageError::Database(error)
    }
}

impl From<rusqlite::Error> for ImageError {
    fn from(error: rusqlite::Error) -> Self {
        ImageError::Database(ProductDbError::Sqlite(error))
    }
}

impl From<CodeError> for ImageError {
    fn from(error: CodeError) -> Self {
        ImageError::Code(error)
    }
}

/// Builds every segment image for `request`.
pub fn build_download_image(
    conn: &Connection,
    request: &ImageRequest,
) -> Result<DownloadImage, ImageError> {
    let program_id = request.program_id.as_str();
    let code = load_program_code(conn, program_id)?
        .ok_or_else(|| ImageError::UnknownProgram(program_id.to_string()))?;
    if let Some(order) = code
        .options
        .get("ParameterByteOrder")
        .filter(|order| *order != "BigEndian")
    {
        return Err(ImageError::Unsupported {
            what: format!("ParameterByteOrder {order:?}: numbers are written high octet first"),
        });
    }
    let types = ParameterTypes::load(conn, program_id, &code)?;

    // Every chosen value is checked before evaluation, so that a value the
    // type does not allow is reported as such, not as an undecided branch.
    for (parameter_ref, value) in &request.values {
        types.value(parameter_ref, value)?;
    }
    let supplied = request.values.clone().into_iter().collect();
    let values = resolve_values(conn, program_id, &supplied)?;
    let activation = evaluate(&load_program_trees(conn, program_id)?, &values);
    let mut undecided = Vec::new();
    for scoped in &activation.diagnostics {
        match &scoped.diagnostic {
            Diagnostic::NoBranchMatched {
                param_ref: Some(param_ref),
                observed_value,
                ..
            } if scoped.scope.is_none() => {
                // Nothing is active under a legal value no `when` covers;
                // an illegal one is refused here.
                types.value(param_ref, observed_value)?;
            }
            // A `Rename`/`ParameterBlockRename` leaf only retitles a
            // `ParameterBlock` (corpus: 326 of them, every one an empty
            // leaf under a `when` naming a `ParameterBlock`). The image
            // holds no titles, so the walk leaving it unrecognized decides
            // nothing here. Anything *below* one is still named by its own
            // `RefBelowSkippedNode` and still refuses the image.
            Diagnostic::UnrecognizedNode { kind, .. }
                if kind == "Rename" || kind == "ParameterBlockRename" => {}
            _ => undecided.push(scoped.clone()),
        }
    }
    if !undecided.is_empty() {
        return Err(ImageError::Evaluation(undecided));
    }
    if let Some(active) = activation
        .parameter_refs
        .iter()
        .chain(&activation.com_object_refs)
        .find(|active| active.scope.is_some())
    {
        return Err(ImageError::Unsupported {
            what: format!("{}: a module instance", active.ref_id),
        });
    }

    let mut images: Vec<(&AbsoluteSegment, ParameterImage)> = code
        .segments
        .iter()
        .filter_map(|segment| {
            let data = segment.data.clone()?;
            Some((segment, ParameterImage::new(data)))
        })
        .collect();

    // 1. Parameters.
    let group_objects = com_object_span(&code)?;
    let mut written: BTreeMap<String, (String, Encoded)> = BTreeMap::new();
    let mut parameters = BTreeMap::new();
    let mut texts = BTreeMap::new();
    for active in &activation.parameter_refs {
        let parameter_ref = active.ref_id.as_str();
        let parameter = types.parameter_of(parameter_ref)?;
        let Some(placement) = code.parameters.get(parameter) else {
            continue; // Lives nowhere in memory.
        };
        let raw = values
            .get_unscoped(parameter_ref)
            .ok_or_else(|| ImageError::Value {
                parameter_ref: parameter_ref.to_string(),
                cause: "no value, and no default".to_string(),
            })?;
        let value = types.value(parameter_ref, raw)?;
        match written.get(parameter) {
            Some((_, earlier)) if *earlier == value => continue,
            Some(_) => {
                return Err(ImageError::ConflictingValues {
                    parameter: parameter.to_string(),
                })
            }
            None => {}
        }
        let (segment_id, field) = field_of(parameter, placement, types.size(parameter_ref)?)?;
        let field_start = field.offset as usize;
        let field_end =
            field_start + (u32::from(field.bit_offset) + field.size_in_bit).div_ceil(8) as usize;
        if group_objects.as_ref().is_some_and(|(id, start, end)| {
            id == segment_id && field_start < *end && *start < field_end
        }) {
            return Err(ImageError::Unsupported {
                what: format!("{parameter}: placed inside the group object table"),
            });
        }
        let image = images
            .iter_mut()
            .find(|(segment, _)| segment.id == segment_id)
            .map(|(_, image)| image)
            .ok_or_else(|| ImageError::Unsupported {
                what: format!("{parameter}: segment {segment_id} ships no Data"),
            })?;
        match &value {
            Encoded::Number(number) => image.write(field, *number),
            Encoded::Text { octets, .. } => image.write_octets(field, octets),
        }
        .map_err(|error| ImageError::Parameter {
            parameter_ref: parameter_ref.to_string(),
            error,
        })?;
        match &value {
            Encoded::Number(number) => parameters.insert(parameter_ref.to_string(), *number),
            Encoded::Text { text, .. } => {
                texts.insert(parameter_ref.to_string(), text.clone());
                None
            }
        };
        written.insert(parameter.to_string(), (parameter_ref.to_string(), value));
    }
    let mut segments: Vec<SegmentImage> = images
        .into_iter()
        .map(|(segment, image)| SegmentImage {
            id: segment.id.clone(),
            address: segment.address,
            octets: image.into_octets(),
            mask: segment.mask.clone(),
        })
        .collect();

    // 2. Group objects.
    let ref_ids: Vec<&str> = activation
        .com_object_refs
        .iter()
        .map(|active| active.ref_id.as_str())
        .collect();
    let views = crate::query::com_object_views(conn, program_id, &ref_ids, None)?;
    let mut objects = Vec::with_capacity(ref_ids.len());
    for ref_id in &ref_ids {
        let view = views.get(*ref_id).ok_or_else(|| ImageError::Unsupported {
            what: format!("{ref_id}: no such ComObjectRef"),
        })?;
        objects.push(active_object(ref_id, view)?);
    }
    objects.sort_by_key(|object| object.number);
    match &group_objects {
        Some((segment_id, start, _)) => {
            let image = segment_mut(&mut segments, segment_id)?;
            let table = write_group_object_table(&image.octets[*start..], &objects)
                .map_err(ImageError::GroupObjects)?;
            image.octets[*start..].copy_from_slice(&table);
        }
        None if !objects.is_empty() => {
            return Err(ImageError::Unsupported {
                what: "active group objects, but no ComObjectTable placement".to_string(),
            })
        }
        None => {}
    }

    // 3. Group addresses.
    let numbers: BTreeSet<u8> = objects.iter().map(|object| object.number).collect();
    if let Some(link) = request
        .links
        .iter()
        .find(|link| !numbers.contains(&link.object))
    {
        return Err(ImageError::InactiveObject {
            object: link.object,
        });
    }
    match (&code.address_table, &code.association_table) {
        (Some(addresses), Some(associations)) => {
            let capacity = TableCapacity {
                group_addresses: max_entries(addresses),
                associations: max_entries(associations),
            };
            let tables = build_group_tables(request.individual_address, &request.links, capacity)
                .map_err(ImageError::GroupTables)?;
            place(
                &mut segments,
                addresses,
                "AddressTable",
                &tables.address_table,
            )?;
            place(
                &mut segments,
                associations,
                "AssociationTable",
                &tables.association_table,
            )?;
        }
        _ if !request.links.is_empty() => {
            return Err(ImageError::Unsupported {
                what: "group address links, but no AddressTable/AssociationTable placement"
                    .to_string(),
            })
        }
        _ => {}
    }

    check_masks(&code, &segments)?;
    Ok(DownloadImage {
        code,
        segments,
        parameters,
        texts,
        objects,
    })
}

/// The segment and field a parameter's placement puts it at.
fn field_of<'a>(
    parameter: &str,
    placement: &'a ParameterPlacement,
    size_in_bit: u32,
) -> Result<(&'a str, ParameterField), ImageError> {
    let field = |memory: &MemoryPlacement, offset: u32, bit_offset: u8| ParameterField {
        offset: memory.offset + offset,
        bit_offset,
        size_in_bit,
    };
    match placement {
        ParameterPlacement::Memory(memory) => Ok((
            memory.code_segment.as_str(),
            field(memory, 0, memory.bit_offset),
        )),
        // `[V]` The union's placement plus the member's own offsets
        // (module documentation). A member at 0/0 simply is the union's
        // placement; any other member of a union that itself starts inside
        // an octet has no attested reading.
        ParameterPlacement::UnionMember {
            union,
            offset: 0,
            bit_offset: 0,
        } => Ok((
            union.code_segment.as_str(),
            field(union, 0, union.bit_offset),
        )),
        ParameterPlacement::UnionMember {
            union,
            offset,
            bit_offset,
        } if union.bit_offset == 0 => Ok((
            union.code_segment.as_str(),
            field(union, *offset, *bit_offset),
        )),
        ParameterPlacement::UnionMember { .. } => Err(ImageError::Unsupported {
            what: format!(
                "{parameter}: a union member offset inside a union that starts mid-octet"
            ),
        }),
        ParameterPlacement::Unmodelled { name, .. } => Err(ImageError::Unsupported {
            what: format!("{parameter}: placed by {name}"),
        }),
    }
}

/// `(segment, first octet, end)` of the group object table, from its
/// placement and its size octet.
fn com_object_span(code: &ProgramCode) -> Result<Option<(String, usize, usize)>, ImageError> {
    let Some(placement) = &code.com_object_table else {
        return Ok(None);
    };
    let (segment, start) = placed(code, placement, "ComObjectTable")?;
    let data = segment.data.as_deref().unwrap_or_default();
    let size = data
        .get(start)
        .copied()
        .ok_or_else(|| ImageError::Unsupported {
            what: format!("ComObjectTable: offset {start} is past {}", segment.id),
        })?;
    let end = start
        + knx_core::commissioning::group_object_table::HEADER_OCTETS
        + usize::from(size) * knx_core::commissioning::group_object_table::DESCRIPTOR_OCTETS;
    Ok(Some((segment.id.clone(), start, end)))
}

/// The segment (with `Data`) and octet offset a table placement names.
fn placed<'a>(
    code: &'a ProgramCode,
    placement: &TablePlacement,
    table: &str,
) -> Result<(&'a AbsoluteSegment, usize), ImageError> {
    let unsupported = |why: &str| ImageError::Unsupported {
        what: format!("{table}: {why}"),
    };
    let id = placement
        .code_segment
        .as_deref()
        .ok_or_else(|| unsupported("no CodeSegment"))?;
    let offset = placement.offset.ok_or_else(|| unsupported("no Offset"))?;
    let segment = code
        .segment(id)
        .filter(|segment| segment.data.is_some())
        .ok_or_else(|| unsupported(&format!("segment {id} ships no Data")))?;
    Ok((segment, offset as usize))
}

fn segment_mut<'a>(
    segments: &'a mut [SegmentImage],
    id: &str,
) -> Result<&'a mut SegmentImage, ImageError> {
    segments
        .iter_mut()
        .find(|segment| segment.id == id)
        .ok_or_else(|| ImageError::Unsupported {
            what: format!("segment {id} ships no Data"),
        })
}

/// Writes a group table at its placement; refuses one that does not fit.
fn place(
    segments: &mut [SegmentImage],
    placement: &TablePlacement,
    table: &str,
    octets: &[u8],
) -> Result<(), ImageError> {
    let unsupported = |why: String| ImageError::Unsupported {
        what: format!("{table}: {why}"),
    };
    let id = placement
        .code_segment
        .as_deref()
        .ok_or_else(|| unsupported("no CodeSegment".to_string()))?;
    let start = placement
        .offset
        .ok_or_else(|| unsupported("no Offset".to_string()))? as usize;
    let image = segment_mut(segments, id)?;
    let end = start + octets.len();
    if end > image.octets.len() {
        return Err(unsupported(format!(
            "{} octets do not fit segment {id} at offset {start}",
            octets.len()
        )));
    }
    image.octets[start..end].copy_from_slice(octets);
    Ok(())
}

fn max_entries(placement: &TablePlacement) -> usize {
    placement.max_entries.map_or(usize::MAX, |n| n as usize)
}

/// Refuses a change to an octet the product's `Mask` marks, except the
/// address table's individual-address slot (module documentation).
fn check_masks(code: &ProgramCode, segments: &[SegmentImage]) -> Result<(), ImageError> {
    let address_slot = code.address_table.as_ref().and_then(|placement| {
        let offset = placement.offset? as usize;
        Some((placement.code_segment.clone()?, offset + 1..offset + 3))
    });
    for image in segments {
        let (Some(mask), Some(base)) = (
            image.mask.as_deref(),
            code.segment(&image.id).and_then(|s| s.data.as_deref()),
        ) else {
            continue;
        };
        for (offset, ((&mark, &now), &was)) in mask.iter().zip(&image.octets).zip(base).enumerate()
        {
            let exempt = address_slot
                .as_ref()
                .is_some_and(|(id, slot)| *id == image.id && slot.contains(&offset));
            if mark != 0xFF && now != was && !exempt {
                return Err(ImageError::Masked {
                    segment: image.id.clone(),
                    offset,
                });
            }
        }
    }
    Ok(())
}

/// One activated `ComObjectRef` as a group object table descriptor.
fn active_object(
    ref_id: &str,
    view: &crate::query::ComObjectView,
) -> Result<ActiveObject, ImageError> {
    let unsupported = |why: String| ImageError::Unsupported {
        what: format!("{ref_id}: {why}"),
    };
    let number = view
        .number
        .and_then(|n| u8::try_from(n).ok())
        .ok_or_else(|| unsupported(format!("object number {:?}", view.number)))?;
    let bits = view
        .object_size
        .as_deref()
        .and_then(object_size_bits)
        .ok_or_else(|| unsupported(format!("ObjectSize {:?}", view.object_size)))?;
    let value_type = ValueType::for_bits(bits).ok_or_else(|| {
        unsupported(format!(
            "ObjectSize {:?} has no value type",
            view.object_size
        ))
    })?;
    let priority = match view.priority.as_deref() {
        None | Some("Low") => TransmissionPriority::Low,
        Some(other) => return Err(unsupported(format!("Priority {other:?}"))),
    };
    let flag = |name: &str, value: &Option<String>| match value.as_deref() {
        Some("Enabled") => Ok(true),
        Some("Disabled") => Ok(false),
        other => Err(unsupported(format!("{name} {other:?}"))),
    };
    match view.read_on_init.as_deref() {
        None | Some("Disabled") => {}
        other => return Err(unsupported(format!("ReadOnInitFlag {other:?}"))),
    }
    Ok(ActiveObject {
        number,
        flags: ObjectFlags {
            update: flag("UpdateFlag", &view.update)?,
            transmit: flag("TransmitFlag", &view.transmit)?,
            write: flag("WriteFlag", &view.write)?,
            read: flag("ReadFlag", &view.read)?,
            communication: flag("CommunicationFlag", &view.communication)?,
            priority,
        },
        value_type,
    })
}

/// `ComObjectSize_t` (*Project Schema23* §1.1.2.5) in bits.
fn object_size_bits(size: &str) -> Option<u32> {
    let (count, unit) = size.split_once(' ')?;
    let count: u32 = count.parse().ok()?;
    match unit {
        "Bit" if (1..=7).contains(&count) => Some(count),
        "Byte" if count == 1 => Some(8),
        "Bytes" if count > 1 => count.checked_mul(8),
        _ => None,
    }
}

/// A value as it goes into memory.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Encoded {
    /// A number, written by width ([`ParameterImage::write`]).
    Number(u64),
    /// A text, written as `octets` ([`ParameterImage::write_octets`]).
    Text {
        /// The value as chosen.
        text: String,
        /// Its encoding, zero-filled to the field.
        octets: Vec<u8>,
    },
}

/// What a value check needs to know about one `ParameterRef`.
struct RefType {
    parameter: String,
    kind: Option<String>,
    size_in_bit: Option<i64>,
    base: Option<String>,
    number_type: Option<String>,
    min: Option<String>,
    max: Option<String>,
    parameter_type: Option<String>,
}

/// The program's parameter refs and their types, loaded once.
struct ParameterTypes {
    refs: BTreeMap<String, RefType>,
    enums: BTreeMap<String, BTreeSet<String>>,
    /// `Options/@TextParameterEncoding`, if declared.
    text_encoding: Option<String>,
}

/// A text encoding this module writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextEncoding {
    /// ISO/IEC 8859-1: every character up to `U+00FF` is its own octet.
    Latin1,
    /// ISO/IEC 8859-15: ISO/IEC 8859-1 with eight positions replaced.
    Latin9,
}

impl TextEncoding {
    /// The encoding a declared name stands for. Encoding names are
    /// case-insensitive (RFC 2978).
    fn named(name: &str) -> Option<TextEncoding> {
        if name.eq_ignore_ascii_case("iso-8859-1") {
            Some(TextEncoding::Latin1)
        } else if name.eq_ignore_ascii_case("iso-8859-15") {
            Some(TextEncoding::Latin9)
        } else {
            None
        }
    }

    /// The octet for `c`, if the encoding has one.
    fn octet(self, c: char) -> Option<u8> {
        // ISO/IEC 8859-15 reassigns A4h, A6h, A8h, B4h, B8h, BCh, BDh, BEh.
        const LATIN_9: [(char, u8); 8] = [
            ('€', 0xA4),
            ('Š', 0xA6),
            ('š', 0xA8),
            ('Ž', 0xB4),
            ('ž', 0xB8),
            ('Œ', 0xBC),
            ('œ', 0xBD),
            ('Ÿ', 0xBE),
        ];
        let code = u32::from(c);
        match self {
            TextEncoding::Latin1 => u8::try_from(code).ok(),
            TextEncoding::Latin9 => {
                if let Some((_, octet)) = LATIN_9.iter().find(|(known, _)| *known == c) {
                    return Some(*octet);
                }
                u8::try_from(code)
                    .ok()
                    .filter(|octet| !LATIN_9.iter().any(|(_, taken)| taken == octet))
            }
        }
    }
}

impl ParameterTypes {
    fn load(conn: &Connection, program_id: &str, code: &ProgramCode) -> Result<Self, ImageError> {
        let mut stmt = conn.prepare(
            "SELECT pr.id, p.id, pt.kind, pt.size_in_bit, pt.base, pt.number_type,
                    pt.min_inclusive, pt.max_inclusive, pt.id
             FROM parameter_ref pr
             JOIN parameter p ON p.program_id = pr.program_id AND p.id = pr.parameter_id
             LEFT JOIN parameter_type pt
               ON pt.program_id = p.program_id AND pt.id = p.parameter_type_id
             WHERE pr.program_id = ?1",
        )?;
        let refs = stmt
            .query_map([program_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    RefType {
                        parameter: r.get(1)?,
                        kind: r.get(2)?,
                        size_in_bit: r.get(3)?,
                        base: r.get(4)?,
                        number_type: r.get(5)?,
                        min: r.get(6)?,
                        max: r.get(7)?,
                        parameter_type: r.get(8)?,
                    },
                ))
            })?
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let mut stmt = conn.prepare(
            "SELECT parameter_type_id, value FROM parameter_type_enum
             WHERE program_id = ?1 AND value IS NOT NULL",
        )?;
        let mut enums: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for row in stmt.query_map([program_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })? {
            let (parameter_type, value) = row?;
            enums.entry(parameter_type).or_default().insert(value);
        }
        Ok(ParameterTypes {
            refs,
            enums,
            text_encoding: code.options.get("TextParameterEncoding").cloned(),
        })
    }

    fn get(&self, parameter_ref: &str) -> Result<&RefType, ImageError> {
        self.refs
            .get(parameter_ref)
            .ok_or_else(|| ImageError::Value {
                parameter_ref: parameter_ref.to_string(),
                cause: "no such ParameterRef in this program".to_string(),
            })
    }

    fn parameter_of(&self, parameter_ref: &str) -> Result<&str, ImageError> {
        Ok(self.get(parameter_ref)?.parameter.as_str())
    }

    /// The field width: 1 to 64 bits for a number, whole octets for a text.
    fn size(&self, parameter_ref: &str) -> Result<u32, ImageError> {
        let found = self.get(parameter_ref)?;
        let text = found.kind.as_deref() == Some("Text");
        found
            .size_in_bit
            .and_then(|size| u32::try_from(size).ok())
            .filter(|size| {
                if text {
                    *size > 0 && size.is_multiple_of(8)
                } else {
                    (1..=64).contains(size)
                }
            })
            .ok_or_else(|| ImageError::Unsupported {
                what: format!("{}: SizeInBit {:?}", found.parameter, found.size_in_bit),
            })
    }

    /// The value `raw` stands for, if the ref's type allows it (module
    /// documentation, *Values*). Every other type is refused by name.
    fn value(&self, parameter_ref: &str, raw: &str) -> Result<Encoded, ImageError> {
        let found = self.get(parameter_ref)?;
        let invalid = |cause: String| ImageError::Value {
            parameter_ref: parameter_ref.to_string(),
            cause,
        };
        let unsigned = || -> Result<u64, ImageError> {
            raw.trim()
                .parse()
                .map_err(|_| invalid(format!("{raw:?} is not an unsigned number")))
        };
        match (
            found.kind.as_deref(),
            found.base.as_deref(),
            found.number_type.as_deref(),
        ) {
            (Some("Restriction"), Some("Value"), _) => {
                let value = unsigned()?;
                let allowed = found
                    .parameter_type
                    .as_ref()
                    .and_then(|id| self.enums.get(id));
                if !allowed
                    .is_some_and(|values| values.iter().any(|v| v.trim().parse() == Ok(value)))
                {
                    return Err(invalid(format!(
                        "{value} is not one of the enumeration's values"
                    )));
                }
                Ok(Encoded::Number(value))
            }
            (Some("Number"), _, Some("unsignedInt")) => {
                let value = unsigned()?;
                let bound = |text: &Option<String>| {
                    text.as_deref().and_then(|t| t.trim().parse::<u64>().ok())
                };
                if bound(&found.min).is_some_and(|min| value < min)
                    || bound(&found.max).is_some_and(|max| value > max)
                {
                    return Err(invalid(format!(
                        "{value} is outside {:?}..={:?}",
                        found.min, found.max
                    )));
                }
                Ok(Encoded::Number(value))
            }
            (Some("Number"), _, Some("signedInt")) => {
                let value: i64 = raw
                    .trim()
                    .parse()
                    .map_err(|_| invalid(format!("{raw:?} is not an integer")))?;
                let bound = |text: &Option<String>| {
                    text.as_deref().and_then(|t| t.trim().parse::<i64>().ok())
                };
                if bound(&found.min).is_some_and(|min| value < min)
                    || bound(&found.max).is_some_and(|max| value > max)
                {
                    return Err(invalid(format!(
                        "{value} is outside {:?}..={:?}",
                        found.min, found.max
                    )));
                }
                if value < 0 {
                    return Err(invalid(format!(
                        "{value} is negative: no source says how a signed parameter is \
                         stored, and no product image shows one (RESEARCH §19.9)"
                    )));
                }
                // At or above zero every sign representation agrees, as long
                // as the sign bit stays clear.
                let size = self.size(parameter_ref)?;
                if size < 64 && value >> (size - 1) != 0 {
                    return Err(invalid(format!(
                        "{value} does not fit a signed {size}-bit field"
                    )));
                }
                Ok(Encoded::Number(value as u64))
            }
            (Some("Text"), _, _) => self.text(parameter_ref, found, raw),
            (kind, base, number_type) => Err(ImageError::Unsupported {
                what: format!(
                    "{}: parameter type {kind:?} (base {base:?}, number type {number_type:?})",
                    found.parameter
                ),
            }),
        }
    }

    /// A `TypeText` value in the program's declared encoding, zero-filled
    /// to its field.
    fn text(&self, parameter_ref: &str, found: &RefType, raw: &str) -> Result<Encoded, ImageError> {
        let invalid = |cause: String| ImageError::Value {
            parameter_ref: parameter_ref.to_string(),
            cause,
        };
        let encoding = match self.text_encoding.as_deref() {
            None => None,
            Some(name) => {
                Some(
                    TextEncoding::named(name).ok_or_else(|| ImageError::Unsupported {
                        what: format!(
                            "{}: TextParameterEncoding {name:?} is not one this crate writes",
                            found.parameter
                        ),
                    })?,
                )
            }
        };
        let mut octets = Vec::with_capacity(raw.len());
        for c in raw.chars() {
            if c == '\0' {
                return Err(invalid(
                    "a NUL character would end the text early".to_string(),
                ));
            }
            let octet = match encoding {
                Some(encoding) => encoding.octet(c).ok_or_else(|| {
                    invalid(format!(
                        "{c:?} has no octet in {}",
                        self.text_encoding.as_deref().unwrap_or_default()
                    ))
                })?,
                None if c.is_ascii() => c as u8,
                None => {
                    return Err(invalid(format!(
                        "{c:?} is not ASCII, and the program declares no TextParameterEncoding"
                    )))
                }
            };
            octets.push(octet);
        }
        let field = (self.size(parameter_ref)? / 8) as usize;
        if octets.len() > field {
            return Err(invalid(format!(
                "{} octets do not fit the {field}-octet field",
                octets.len()
            )));
        }
        octets.resize(field, 0);
        Ok(Encoded::Text {
            text: raw.to_string(),
            octets,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::load_program_code;
    use knx_core::GroupAddress;

    const ID: &str = "M-0001_A-0001-01-0000";

    /// A program shaped like a mask-`0701h` one: a group address table with
    /// a masked individual-address slot, an association table, and a
    /// parameter segment that starts with the group object table.
    ///
    /// `P-1` selects: 0 activates `P-3` (a `Property` placement), 1 activates
    /// `P-2`, `UP-1` and object 0, 2 activates `UP-2` (through two refs) and
    /// object 1. Its enumeration also allows 3, which no `when` covers.
    const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001">
<ApplicationPrograms><ApplicationProgram Id="M-0001_A-0001-01-0000" Name="P" ApplicationNumber="1"
  ApplicationVersion="1" MaskVersion="MV-0701" LoadProcedureStyle="ProductProcedure"><Static>
<Code>
  <AbsoluteSegment Id="AS-4000" Address="16384" Size="8"><Data>AwAAGQAZAQA=</Data><Mask>/wAA//////8=</Mask></AbsoluteSegment>
  <AbsoluteSegment Id="AS-4100" Address="16640" Size="6"><Data>AgEAAgUA</Data></AbsoluteSegment>
  <AbsoluteSegment Id="AS-4400" Address="17408" Size="16"><Data>AgAAB0BMAAdBbAeqAAu4AA==</Data></AbsoluteSegment>
  <AbsoluteSegment Id="AS-0700" Address="1792" Size="4" />
</Code>
<ParameterTypes>
  <ParameterType Id="PT-E" Name="E"><TypeRestriction Base="Value" SizeInBit="2">
    <Enumeration Id="PT-E_EN-0" Text="zero" Value="0" /><Enumeration Id="PT-E_EN-1" Text="one" Value="1" />
    <Enumeration Id="PT-E_EN-2" Text="two" Value="2" /><Enumeration Id="PT-E_EN-3" Text="three" Value="3" />
  </TypeRestriction></ParameterType>
  <ParameterType Id="PT-N" Name="N"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="100" /></ParameterType>
  <ParameterType Id="PT-W" Name="W"><TypeRestriction Base="Value" SizeInBit="16">
    <Enumeration Id="PT-W_EN-0" Text="a" Value="0" /><Enumeration Id="PT-W_EN-2" Text="b" Value="2" />
    <Enumeration Id="PT-W_EN-255" Text="c" Value="255" />
  </TypeRestriction></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="Mode" ParameterType="PT-E" Value="1"><Memory CodeSegment="AS-4400" Offset="11" BitOffset="0" /></Parameter>
  <Parameter Id="P-2" Name="Time" ParameterType="PT-N" Value="50"><Memory CodeSegment="AS-4400" Offset="12" BitOffset="0" /></Parameter>
  <Parameter Id="P-3" Name="Prop" ParameterType="PT-N" Value="1"><Property ObjectIndex="1" PropertyId="2" Offset="0" BitOffset="0" /></Parameter>
  <Union SizeInBit="16">
    <Memory CodeSegment="AS-4400" Offset="13" BitOffset="0" />
    <Parameter Id="UP-1" Name="Wide" ParameterType="PT-W" Offset="0" BitOffset="0" Value="255" />
    <Parameter Id="UP-2" Name="Narrow" ParameterType="PT-E" Offset="1" BitOffset="5" Value="2" />
  </Union>
</Parameters>
<ParameterRefs>
  <ParameterRef Id="P-1_R-1" RefId="P-1" />
  <ParameterRef Id="P-2_R-2" RefId="P-2" />
  <ParameterRef Id="P-3_R-6" RefId="P-3" />
  <ParameterRef Id="UP-1_R-3" RefId="UP-1" />
  <ParameterRef Id="UP-2_R-4" RefId="UP-2" />
  <ParameterRef Id="UP-2_R-5" RefId="UP-2" />
</ParameterRefs>
<ComObjectTable CodeSegment="AS-4400" Offset="0">
  <ComObject Id="O-0" Name="Switch" Number="0" ObjectSize="1 Bit" ReadFlag="Enabled" WriteFlag="Disabled"
    CommunicationFlag="Enabled" TransmitFlag="Enabled" UpdateFlag="Disabled" ReadOnInitFlag="Disabled" />
  <ComObject Id="O-1" Name="Value" Number="1" ObjectSize="1 Byte" ReadFlag="Disabled" WriteFlag="Disabled"
    CommunicationFlag="Enabled" TransmitFlag="Disabled" UpdateFlag="Enabled" ReadOnInitFlag="Disabled" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="O-0_R-1" RefId="O-0" />
  <ComObjectRef Id="O-1_R-2" RefId="O-1" WriteFlag="Enabled" />
</ComObjectRefs>
<AddressTable CodeSegment="AS-4000" Offset="0" MaxEntries="4" />
<AssociationTable CodeSegment="AS-4100" Offset="0" MaxEntries="2" />
</Static>
<Dynamic><Channel Id="CH" Text="C"><ParameterBlock Id="PB" Text="B">
  <ParameterRefRef RefId="P-1_R-1" />
  <choose ParamRefId="P-1_R-1">
    <when test="0"><ParameterRefRef RefId="P-3_R-6" /></when>
    <when test="1"><ParameterRefRef RefId="P-2_R-2" /><ParameterRefRef RefId="UP-1_R-3" /><ComObjectRefRef RefId="O-0_R-1" /></when>
    <when test="2"><ParameterRefRef RefId="UP-2_R-4" /><ParameterRefRef RefId="UP-2_R-5" /><ComObjectRefRef RefId="O-1_R-2" /></when>
  </choose>
</ParameterBlock></Channel></Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    fn db(xml: &str) -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().expect("tempdir");
        let conn = crate::open_and_migrate(&dir.path().join("products.sqlite")).expect("database");
        crate::ingest_file(&conn, "M-0001/A.xml", xml.as_bytes()).expect("ingests");
        (dir, conn)
    }

    fn device() -> IndividualAddress {
        IndividualAddress::new(1, 1, 67).expect("valid")
    }

    fn link(object: u8, raw: u16, sending: bool) -> Link {
        Link {
            object,
            group_address: GroupAddress::from_raw(raw),
            sending,
        }
    }

    fn request(values: &[(&str, &str)], links: Vec<Link>) -> ImageRequest {
        ImageRequest {
            program_id: ID.to_string(),
            individual_address: device(),
            values: values
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            links,
        }
    }

    fn build(
        xml: &str,
        values: &[(&str, &str)],
        links: Vec<Link>,
    ) -> Result<DownloadImage, ImageError> {
        let (_dir, conn) = db(xml);
        build_download_image(&conn, &request(values, links))
    }

    fn octets<'a>(image: &'a DownloadImage, id: &str) -> &'a [u8] {
        &image.segment(id).expect("segment").octets
    }

    #[test]
    fn product_defaults_write_parameters_objects_and_tables() {
        // 2/0/53 = 1035h.
        let image = build(PROGRAM, &[], vec![link(0, 0x1035, true)]).expect("builds");
        assert_eq!(
            octets(&image, "AS-4400"),
            &[
                0x02, 0x00, 0x00, // table size and RAM-flags pointer, untouched
                0x07, 0x40, 0x4F, 0x00, // object 0: T, R, C, low priority; 1 bit
                0x07, 0x41, 0x68, 0x07, // object 1 inactive: only C cleared
                0x6A, // P-1 = 1 in the top two bits of the base's AAh; the other bits kept
                0x32, // P-2 = 50
                0x00, 0xFF, // UP-1 = 255, high octet first
                0x00,
            ][..]
        );
        assert_eq!(
            octets(&image, "AS-4000"),
            &[0x02, 0x11, 0x43, 0x10, 0x35, 0x19, 0x01, 0x00][..]
        );
        assert_eq!(
            octets(&image, "AS-4100"),
            &[0x01, 0x01, 0x00, 0x02, 0x05, 0x00][..]
        );
        let ids: Vec<&str> = image.segments.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["AS-4000", "AS-4100", "AS-4400"],
            "only segments with Data"
        );
        assert_eq!(
            image.segment("AS-4000").and_then(|s| s.mask.clone()),
            Some(vec![0xFF, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
        );
        assert_eq!(
            image.parameters,
            BTreeMap::from([
                ("P-1_R-1".to_string(), 1),
                ("P-2_R-2".to_string(), 50),
                ("UP-1_R-3".to_string(), 255),
            ])
        );
    }

    #[test]
    fn a_chosen_value_switches_the_branch_and_a_ref_override_wins() {
        let image =
            build(PROGRAM, &[("P-1_R-1", "2")], vec![link(1, 0x1035, false)]).expect("builds");
        let segment = octets(&image, "AS-4400");
        assert_eq!(segment[5], 0x48, "object 0 inactive: C cleared from 4Ch");
        // Object 1: U, W (from the ref), C; low priority; the base's segment
        // selector (bit 5) kept; 1 octet is type 7.
        assert_eq!(&segment[9..11], &[0xB7, 0x07]);
        assert_eq!(
            segment[11], 0xAA,
            "P-1 = 2: the base already holds 10b there"
        );
        assert_eq!(segment[12], 0x00, "P-2 inactive: base data");
        assert_eq!(
            &segment[13..15],
            &[0x0B, 0xBC],
            "UP-2 = 2 at octet 14, bits 5-6"
        );
        assert_eq!(octets(&image, "AS-4100")[..3], [0x01, 0x01, 0x01]);
    }

    #[test]
    fn a_value_outside_the_enumeration_is_refused() {
        let error = build(PROGRAM, &[("P-1_R-1", "5")], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Value { parameter_ref, .. } if parameter_ref == "P-1_R-1"),
            "{error}"
        );
    }

    #[test]
    fn a_number_outside_its_bounds_is_refused() {
        let error = build(PROGRAM, &[("P-2_R-2", "101")], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Value { parameter_ref, .. } if parameter_ref == "P-2_R-2"),
            "{error}"
        );
    }

    #[test]
    fn a_value_for_an_unknown_ref_is_refused() {
        let error = build(PROGRAM, &[("P-9_R-9", "1")], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Value { parameter_ref, .. } if parameter_ref == "P-9_R-9"),
            "{error}"
        );
    }

    #[test]
    fn two_refs_of_one_parameter_must_agree() {
        let error = build(
            PROGRAM,
            &[("P-1_R-1", "2"), ("UP-2_R-4", "1"), ("UP-2_R-5", "2")],
            vec![],
        )
        .unwrap_err();
        assert!(
            matches!(&error, ImageError::ConflictingValues { parameter } if parameter == "UP-2"),
            "{error}"
        );
    }

    #[test]
    fn two_agreeing_refs_are_written_once() {
        let image = build(PROGRAM, &[("P-1_R-1", "2"), ("UP-2_R-5", "2")], vec![]).expect("builds");
        assert_eq!(image.parameters.get("UP-2_R-4"), Some(&2));
        assert_eq!(image.parameters.get("UP-2_R-5"), None);
    }

    #[test]
    fn a_legal_value_no_when_covers_activates_nothing() {
        let image = build(PROGRAM, &[("P-1_R-1", "3")], vec![]).expect("builds");
        assert_eq!(
            image.parameters,
            BTreeMap::from([("P-1_R-1".to_string(), 3)]),
            "only the selector itself"
        );
        assert!(image.objects.is_empty());
        let segment = octets(&image, "AS-4400");
        assert_eq!(
            &segment[3..11],
            &[0x07, 0x40, 0x48, 0x00, 0x07, 0x41, 0x68, 0x07]
        );
        assert_eq!(segment[11], 0xEA, "P-1 = 3");
    }

    #[test]
    fn a_default_the_type_does_not_allow_is_refused() {
        let xml = PROGRAM.replace(
            r#"ParameterType="PT-E" Value="1""#,
            r#"ParameterType="PT-E" Value="7""#,
        );
        let error = build(&xml, &[], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Value { parameter_ref, .. } if parameter_ref == "P-1_R-1"),
            "{error}"
        );
    }

    #[test]
    fn a_hidden_selector_with_a_value_its_type_does_not_allow_is_refused() {
        // P-1 is no longer shown itself, so only the `choose` sees its value.
        let xml = PROGRAM
            .replace(
                r#"ParameterType="PT-E" Value="1""#,
                r#"ParameterType="PT-E" Value="7""#,
            )
            .replace(
                r#"  <ParameterRefRef RefId="P-1_R-1" />
  <choose"#,
                "  <choose",
            );
        let error = build(&xml, &[], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Value { parameter_ref, .. } if parameter_ref == "P-1_R-1"),
            "{error}"
        );
    }

    #[test]
    fn objects_are_ordered_by_number_not_by_activation() {
        let xml = PROGRAM.replace(
            r#"<ComObjectRefRef RefId="O-0_R-1" /></when>"#,
            r#"<ComObjectRefRef RefId="O-1_R-2" /><ComObjectRefRef RefId="O-0_R-1" /></when>"#,
        );
        let image = build(&xml, &[], vec![]).expect("builds");
        let numbers: Vec<u8> = image.objects.iter().map(|object| object.number).collect();
        assert_eq!(numbers, vec![0, 1]);
    }

    #[test]
    fn each_flag_comes_from_its_own_attribute() {
        // Object 0: update on, write off; nothing else may borrow either.
        let xml = PROGRAM.replace(
            r#"TransmitFlag="Enabled" UpdateFlag="Disabled""#,
            r#"TransmitFlag="Enabled" UpdateFlag="Enabled""#,
        );
        let image = build(&xml, &[], vec![]).expect("builds");
        let flags = image.objects[0].flags;
        assert!(flags.update, "UpdateFlag");
        assert!(!flags.write, "WriteFlag");
        assert!(flags.read, "ReadFlag");
        assert!(flags.transmit, "TransmitFlag");
        assert!(flags.communication, "CommunicationFlag");
    }

    #[test]
    fn a_parameter_reaching_into_the_group_object_table_is_refused() {
        // Shift AS-4400 by one octet so the table starts at 1. The 16-bit
        // union then starts at 0, before the table, and runs into it.
        let xml = PROGRAM
            .replace(
                r#"Size="16"><Data>AgAAB0BMAAdBbAeqAAu4AA==</Data>"#,
                r#"Size="17"><Data>VQIAAAdATAAHQWwHqgALuAA=</Data>"#,
            )
            .replace(
                r#"<ComObjectTable CodeSegment="AS-4400" Offset="0">"#,
                r#"<ComObjectTable CodeSegment="AS-4400" Offset="1">"#,
            )
            .replace(
                r#"CodeSegment="AS-4400" Offset="11""#,
                r#"CodeSegment="AS-4400" Offset="12""#,
            )
            .replace(
                r#"CodeSegment="AS-4400" Offset="12" BitOffset="0" /></Parameter>
  <Parameter Id="P-3""#,
                r#"CodeSegment="AS-4400" Offset="13" BitOffset="0" /></Parameter>
  <Parameter Id="P-3""#,
            )
            .replace(
                r#"<Memory CodeSegment="AS-4400" Offset="13" BitOffset="0" />
    <Parameter Id="UP-1""#,
                r#"<Memory CodeSegment="AS-4400" Offset="0" BitOffset="0" />
    <Parameter Id="UP-1""#,
            );
        let error = build(&xml, &[], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Unsupported { what } if what.contains("UP-1") && what.contains("group object table")),
            "{error}"
        );
    }

    #[test]
    fn a_parameter_right_behind_the_group_object_table_is_written() {
        // P-1 at octet 11 is the first octet after the table: allowed.
        let image = build(PROGRAM, &[], vec![]).expect("builds");
        assert_eq!(image.parameters.get("P-1_R-1"), Some(&1));
    }

    #[test]
    fn a_rename_does_not_hold_up_the_image_but_a_reference_below_one_does() {
        for kind in ["Rename", "ParameterBlockRename"] {
            let renamed = PROGRAM.replace(
                r#"<when test="1">"#,
                &format!(r#"<when test="1"><{kind} Id="PR-1" RefId="PB-1" Text="Other title" />"#),
            );
            build(&renamed, &[], vec![]).unwrap_or_else(|e| panic!("{kind}: {e}"));

            let hiding = PROGRAM.replace(
                r#"<when test="1">"#,
                &format!(
                    r#"<when test="1"><{kind} Id="PR-1" RefId="PB-1" Text="x"><ParameterRefRef RefId="P-2_R-2" /></{kind}>"#
                ),
            );
            match build(&hiding, &[], vec![]).unwrap_err() {
                ImageError::Evaluation(diagnostics) => assert!(
                    diagnostics
                        .iter()
                        .all(|d| matches!(d.diagnostic, Diagnostic::RefBelowSkippedNode { .. })),
                    "{kind}: {diagnostics:?}"
                ),
                other => panic!("{kind}: expected Evaluation, got {other}"),
            }
        }
    }

    #[test]
    fn any_other_diagnostic_refuses_the_whole_image() {
        let xml = PROGRAM.replace(
            r#"<when test="0">"#,
            r#"<when test="banana"><ParameterRefRef RefId="P-2_R-2" /></when><when test="0">"#,
        );
        let error = build(&xml, &[], vec![]).unwrap_err();
        match error {
            ImageError::Evaluation(diagnostics) => assert!(matches!(
                diagnostics[0].diagnostic,
                Diagnostic::UnparsableTest { .. }
            )),
            other => panic!("expected Evaluation, got {other}"),
        }
    }

    #[test]
    fn a_property_placement_is_refused_by_name() {
        let error = build(PROGRAM, &[("P-1_R-1", "0")], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Unsupported { what } if what.contains("P-3") && what.contains("Property")),
            "{error}"
        );
    }

    #[test]
    fn a_link_to_an_inactive_object_is_refused() {
        let error = build(PROGRAM, &[], vec![link(1, 0x1035, true)]).unwrap_err();
        assert!(
            matches!(error, ImageError::InactiveObject { object: 1 }),
            "{error}"
        );
    }

    #[test]
    fn a_priority_no_pdf_maps_is_refused() {
        let xml = PROGRAM.replace(r#"RefId="O-0" />"#, r#"RefId="O-0" Priority="High" />"#);
        let error = build(&xml, &[], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Unsupported { what } if what.contains("High")),
            "{error}"
        );
    }

    #[test]
    fn read_on_init_is_refused() {
        let xml = PROGRAM.replacen(
            r#"ReadOnInitFlag="Disabled""#,
            r#"ReadOnInitFlag="Enabled""#,
            1,
        );
        let error = build(&xml, &[], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Unsupported { what } if what.contains("ReadOnInit")),
            "{error}"
        );
    }

    #[test]
    fn a_change_to_another_masked_octet_is_refused() {
        // Octet 3 masked too: the first group address would change it.
        let xml = PROGRAM.replace("/wAA//////8=", "/wAAAP////8=");
        let error = build(&xml, &[], vec![link(0, 0x1035, true)]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Masked { segment, offset: 3 } if segment == "AS-4000"),
            "{error}"
        );
    }

    #[test]
    fn a_masked_octet_left_as_the_base_has_it_is_fine() {
        // Octet 7 masked, but nothing writes it.
        let xml = PROGRAM.replace("/wAA//////8=", "/wAA/////wA=");
        build(&xml, &[], vec![link(0, 0x1035, true)]).expect("builds");
    }

    #[test]
    fn a_table_larger_than_its_segment_is_refused() {
        let links = vec![
            link(0, 0x1035, true),
            link(0, 0x1036, false),
            link(0, 0x1037, false),
        ];
        let error = build(PROGRAM, &[], links).unwrap_err();
        assert!(
            matches!(
                error,
                ImageError::GroupTables(_) | ImageError::Unsupported { .. }
            ),
            "{error}"
        );
    }

    #[test]
    fn an_unknown_program_is_refused() {
        let (_dir, conn) = db(PROGRAM);
        let mut request = request(&[], vec![]);
        request.program_id = "M-0001_A-FFFF-01-0000".to_string();
        assert!(matches!(
            build_download_image(&conn, &request),
            Err(ImageError::UnknownProgram(_))
        ));
    }

    // ---- Signed numbers and text ------------------------------------------

    /// `PROGRAM` plus a 12-octet segment `AS-4600` (base `EEh` throughout)
    /// holding `S8`/`S16`, signed numbers at 0 and 1, and `T`, a 6-octet
    /// text at 3, all shown unconditionally. `extra_types`/`extra_params`/
    /// `extra_refs` add more; `options` becomes the program's `Options`.
    fn kinds_program_with(
        options: &str,
        text_default: &str,
        extra_types: &str,
        extra_params: &str,
        extra_refs: &str,
    ) -> String {
        PROGRAM
            .replace(
                r#"<AbsoluteSegment Id="AS-0700""#,
                r#"<AbsoluteSegment Id="AS-4600" Address="17920" Size="12"><Data>7u7u7u7u7u7u7u7u</Data></AbsoluteSegment>
  <AbsoluteSegment Id="AS-0700""#,
            )
            .replace(
                "</ParameterTypes>",
                &format!(
                    r#"<ParameterType Id="PT-S8" Name="S8"><TypeNumber SizeInBit="8" Type="signedInt" minInclusive="-100" maxInclusive="100" /></ParameterType>
  <ParameterType Id="PT-S16" Name="S16"><TypeNumber SizeInBit="16" Type="signedInt" minInclusive="-30000" maxInclusive="30000" /></ParameterType>
  <ParameterType Id="PT-T" Name="T"><TypeText SizeInBit="48" /></ParameterType>
  {extra_types}
</ParameterTypes>"#
                ),
            )
            .replace(
                "</Parameters>",
                &format!(
                    r#"<Parameter Id="P-S8" Name="S8" ParameterType="PT-S8" Value="2"><Memory CodeSegment="AS-4600" Offset="0" BitOffset="0" /></Parameter>
  <Parameter Id="P-S16" Name="S16" ParameterType="PT-S16" Value="300"><Memory CodeSegment="AS-4600" Offset="1" BitOffset="0" /></Parameter>
  <Parameter Id="P-T" Name="T" ParameterType="PT-T" Value="{text_default}"><Memory CodeSegment="AS-4600" Offset="3" BitOffset="0" /></Parameter>
  {extra_params}
</Parameters>"#
                ),
            )
            .replace(
                "</ParameterRefs>",
                &format!(
                    r#"<ParameterRef Id="P-S8_R-1" RefId="P-S8" />
  <ParameterRef Id="P-S16_R-1" RefId="P-S16" />
  <ParameterRef Id="P-T_R-1" RefId="P-T" />
  {extra_refs}
</ParameterRefs>
{options}"#
                ),
            )
            .replace(
                r#"<ParameterRefRef RefId="P-1_R-1" />"#,
                r#"<ParameterRefRef RefId="P-1_R-1" />
  <ParameterRefRef RefId="P-S8_R-1" /><ParameterRefRef RefId="P-S16_R-1" />
  <ParameterRefRef RefId="P-T_R-1" />"#,
            )
    }

    fn kinds_program(options: &str) -> String {
        kinds_program_with(options, "Grüß", "", "", "")
    }

    const LATIN_9: &str = r#"<Options TextParameterEncoding="iso-8859-15" />"#;

    /// A signed number at or above zero, high octet first; the text in the
    /// declared ISO-8859-15, zero-filled to its field.
    #[test]
    fn signed_numbers_and_text_are_encoded_into_the_image() {
        let image = build(&kinds_program(LATIN_9), &[], vec![]).expect("builds");
        assert_eq!(
            octets(&image, "AS-4600"),
            &[
                0x02, // S8 = 2
                0x01, 0x2C, // S16 = 300
                b'G', b'r', 0xFC, 0xDF, 0x00, 0x00, // T = "Grüß"
                0xEE, 0xEE, 0xEE, // untouched
            ][..]
        );
        assert_eq!(image.parameters.get("P-S16_R-1"), Some(&300));
        assert_eq!(image.parameters.get("P-T_R-1"), None, "not a number");
        assert_eq!(image.texts.get("P-T_R-1").map(String::as_str), Some("Grüß"));
    }

    #[test]
    fn a_chosen_signed_or_text_value_is_encoded() {
        let image = build(
            &kinds_program(LATIN_9),
            &[
                ("P-S8_R-1", "100"),
                ("P-S16_R-1", "30000"),
                ("P-T_R-1", "5 € ok"),
            ],
            vec![],
        )
        .expect("builds");
        assert_eq!(
            octets(&image, "AS-4600"),
            &[
                0x64, 0x75, 0x30, // 100, 30000
                b'5', b' ', 0xA4, b' ', b'o',
                b'k', // exactly fills, no terminator; € is A4h
                0xEE, 0xEE, 0xEE,
            ][..]
        );
    }

    #[test]
    fn values_a_signed_or_text_field_cannot_hold_are_refused() {
        let xml = kinds_program(LATIN_9);
        for (parameter_ref, value) in [
            ("P-S8_R-1", "101"),    // above maxInclusive
            ("P-S8_R-1", "-1"),     // below zero: see the test below
            ("P-S16_R-1", "1.5"),   // not an integer
            ("P-T_R-1", "abcdefg"), // longer than the field
            ("P-T_R-1", "Ω"),       // no ISO-8859-15 octet
            ("P-T_R-1", "½"),       // ISO-8859-1 has it, ISO-8859-15 does not
            ("P-T_R-1", "a\u{0}b"), // a NUL would end the text early
        ] {
            let error = build(&xml, &[(parameter_ref, value)], vec![]).unwrap_err();
            assert!(
                matches!(&error, ImageError::Value { parameter_ref: r, .. } if r == parameter_ref),
                "{parameter_ref} = {value:?}: {error}"
            );
        }
    }

    /// `[V]` 213 signed fields in the corpus's `070nh` base images hold
    /// their non-negative default high octet first, but none holds a
    /// negative one, and no PDF says how a signed parameter is stored. A
    /// value at or above zero has one pattern under every sign
    /// representation; a negative one is refused rather than guessed.
    #[test]
    fn a_negative_signed_value_is_refused_until_a_source_fixes_its_form() {
        let xml = kinds_program(LATIN_9);
        let error = build(&xml, &[("P-S16_R-1", "-300")], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Value { parameter_ref, cause } if parameter_ref == "P-S16_R-1" && cause.contains("negative")),
            "{error}"
        );
        let xml = xml.replace(
            r#"ParameterType="PT-S8" Value="2""#,
            r#"ParameterType="PT-S8" Value="-2""#,
        );
        let error = build(&xml, &[], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Value { parameter_ref, .. } if parameter_ref == "P-S8_R-1"),
            "a negative default as well: {error}"
        );
    }

    /// ISO-8859-1 and ISO-8859-15 are the two encodings the corpus's
    /// `070nh` programs declare. Without a declaration only a value whose
    /// octets are the same under every declared encoding (ASCII) is
    /// written; anything else is refused rather than guessed.
    #[test]
    fn a_text_value_is_encoded_as_declared_and_ascii_needs_no_declaration() {
        let latin_1 = kinds_program(r#"<Options TextParameterEncoding="iso-8859-1" />"#);
        build(&latin_1, &[("P-T_R-1", "½")], vec![]).expect("ISO-8859-1 has ½");
        let error = build(&latin_1, &[("P-T_R-1", "5 €")], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Value { .. }),
            "€ is not in ISO-8859-1: {error}"
        );

        let undeclared = kinds_program_with("", "Licht", "", "", "");
        let image = build(&undeclared, &[], vec![]).expect("ASCII needs no declaration");
        assert_eq!(&octets(&image, "AS-4600")[3..9], b"Licht\0");
        let error = build(&undeclared, &[("P-T_R-1", "Grüß")], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Value { cause, .. } if cause.contains("TextParameterEncoding")),
            "{error}"
        );

        let utf_8 = kinds_program_with(
            r#"<Options TextParameterEncoding="utf-8" />"#,
            "Licht",
            "",
            "",
            "",
        );
        let error = build(&utf_8, &[], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Unsupported { what } if what.contains("utf-8")),
            "{error}"
        );
    }

    /// `[V]` The corpus's `DPT 9` float fields match that encoding only at
    /// zero; the non-zero ones contradict it (RESEARCH §19.9). So a float
    /// is still refused by name, for its type, not its value's spelling.
    #[test]
    fn a_float_parameter_is_refused_for_its_type() {
        let xml = kinds_program_with(
            LATIN_9,
            "Grüß",
            r#"<ParameterType Id="PT-F9" Name="F9"><TypeFloat Encoding="DPT 9" /></ParameterType>"#,
            r#"<Parameter Id="P-F9" Name="F9" ParameterType="PT-F9" Value="5.000000000000000E+002"><Memory CodeSegment="AS-4600" Offset="10" BitOffset="0" /></Parameter>"#,
            r#"<ParameterRef Id="P-F9_R-1" RefId="P-F9" />"#,
        )
        .replace(
            r#"<ParameterRefRef RefId="P-T_R-1" />"#,
            r#"<ParameterRefRef RefId="P-T_R-1" /><ParameterRefRef RefId="P-F9_R-1" />"#,
        );
        let error = build(&xml, &[], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Unsupported { what } if what.contains("P-F9") && what.contains("Float")),
            "{error}"
        );
    }

    /// `parameter_image` writes high octet first and leaves it to its
    /// caller to refuse another declared order; this is that caller.
    #[test]
    fn a_program_declaring_another_byte_order_is_refused() {
        let xml = kinds_program(
            r#"<Options TextParameterEncoding="iso-8859-15" ParameterByteOrder="LittleEndian" />"#,
        );
        let error = build(&xml, &[], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Unsupported { what } if what.contains("ParameterByteOrder")),
            "{error}"
        );
        let big = kinds_program(
            r#"<Options TextParameterEncoding="iso-8859-15" ParameterByteOrder="BigEndian" />"#,
        );
        build(&big, &[], vec![]).expect("the order this crate writes");
    }

    #[test]
    fn the_images_match_load_program_code_segments() {
        let (_dir, conn) = db(PROGRAM);
        let code = load_program_code(&conn, ID)
            .expect("reads")
            .expect("exists");
        let image = build_download_image(&conn, &request(&[], vec![])).expect("builds");
        assert_eq!(image.code, code);
        for segment in &image.segments {
            let source = code.segment(&segment.id).expect("from the code");
            assert_eq!(segment.address, source.address);
            assert_eq!(segment.octets.len() as u32, source.size);
        }
    }
}
