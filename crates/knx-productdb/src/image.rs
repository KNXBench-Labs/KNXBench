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
//! - **Relative source segments.** Retained in `ProgramCode`, but neither
//!   absolute image entry point omits or relocates them. They require a separate
//!   relative image builder, which this module does not implement.
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
//!   (docs/RESEARCH.md §19.1). `[A]` Union members are alternatives "not
//!   active at the same time" (KNX MT help); where the parameter tree
//!   activates two that share bits, the one later in the tree is written
//!   and the other is not — the device ETS programmed holds that one
//!   (RESEARCH §19.12). It is an inference (ADR-0086), recorded in
//!   [`DownloadImage::inferences`].
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
//!   two those programs declare, are understood. Without a declaration the
//!   text is written in Windows-1252: `[V]` ETS 6.4 on a Western European
//!   Windows wrote `ü ß Ä ö €` of an undeclared program as `FC DF C4 F6 80`
//!   (the euro sign rules out both ISO encodings); ASCII is unchanged, and
//!   a non-ASCII text names [`UNDECLARED_TEXT_WINDOWS_1252`]. Whether ETS
//!   follows the Windows code page elsewhere is not known.
//!   A `TypeFloat` is written in its declared `DPT 9` or `IEEE-754 Single`
//!   encoding by the memory (`070nh`) builder only, as `[V]` ETS 6.4 wrote
//!   it ([`float`], ADR-0169); the templates that disagree with `DPT 9` hold
//!   octets ETS overwrites. Every other type is refused by name. A
//!   `TypeNumber` bound that is declared but not an integer
//!   (empty, decimal, scientific, out of range) refuses the value by name; an
//!   absent bound constrains nothing (docs/contracts/parameter-number-bound-admission.md).
//! - **Byte order.** Numbers are written high octet first
//!   (`knx_core::commissioning::parameter_image`), as `BigEndian` and an
//!   absent declaration (its schema default) need. For a memory (`070nh`)
//!   program declaring `ParameterByteOrder="LittleEndian"`, a number whose
//!   field starts on an octet and is whole octets wide is written low octet
//!   first; one inside a single octet has no order; any other field crossing
//!   an octet boundary is refused; texts stay octet strings, first character
//!   first (KL-170, docs/research/parameter-byte-order.md, ADR-0159). Legacy
//!   layouts and any other declared order still refuse the image.
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

mod experimental;
pub mod float;
pub(crate) mod inactive_selection;
mod legacy_source;
mod object_flags;
pub mod property;
mod system1;

pub use object_flags::{
    resolve_effective_object_flags, resolve_effective_object_flags_with_refs, ActiveObjectFlags,
    EffectiveObjectFlags,
};

/// Validated GA/association reservations; the object span is not capacity padding.
pub(crate) fn system2_group_reservations(
    code: &ProgramCode,
) -> Result<Vec<(String, usize, usize)>, ImageError> {
    Ok(legacy_source::table_spans(code, ImageLayout::System2)?
        .into_iter()
        .skip(1)
        .collect())
}

pub use experimental::{
    build_experimental_system2_image, build_project_experimental_system2_image,
    ExperimentalParameterCoverage, ExperimentalParameterDisposition, ExperimentalSystem2Image,
    ExperimentalSystem2ImagePolicy, ExperimentalSystem2SourceBinding,
};

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use knx_core::commissioning::group_object_table::{
    write_group_object_table, ActiveObject, GroupObjectTableError, TransmissionPriority, ValueType,
};
use knx_core::commissioning::group_tables::{
    build_group_tables, GroupTableError, TableCapacity, TableLink,
};
use knx_core::commissioning::parameter_image::{
    reverse_octets, ParameterField, ParameterImage, ParameterImageError,
};
use knx_core::IndividualAddress;
use rusqlite::Connection;

use crate::code::{
    load_program_code, AbsoluteSegment, CodeError, MemoryPlacement, ParameterPlacement,
    ProgramCode, TablePlacement,
};
use crate::dynamic::evaluate::same_value;
use crate::dynamic::{
    evaluate, load_program_trees, resolve_values, Activation, AssignSource, Diagnostic,
    ScopedDiagnostic, ValueMap,
};
use crate::inference::Inference;
use crate::ProductDbError;

/// [`Inference::rule`] of the union rule: of two active members of one
/// union that share bits, the one later in the parameter tree is written.
pub const UNION_LATER_MEMBER: &str = "union-later-member";

/// [`Inference::rule`]: a program that declares no `TextParameterEncoding`
/// gets its non-ASCII text characters in Windows-1252, as ETS 6.4 wrote
/// them (docs/research/2026-11-07-ldctrlwriteprop-and-byte-order.md).
pub const UNDECLARED_TEXT_WINDOWS_1252: &str = "undeclared-text-windows-1252";

/// [`Inference::rule`]: `Priority="Alert"` is written as urgent, `10b`.
pub const ALERT_IS_URGENT: &str = "alert-is-urgent";

/// [`Inference::rule`]: an enabled `ReadOnInitFlag` has no bit on `070nh`.
pub const READ_ON_INIT_NOT_ON_070N: &str = "read-on-init-not-on-070n";

/// [`Inference::rule`]: a `TypeTime` value is written as a plain count of
/// its unit, like a number (ADR-0162).
pub const TIME_AS_UNIT_COUNT: &str = "time-as-unit-count";

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
    /// The priority literal each object instance states (a user edit or a
    /// captured Known declaration), by the instance's `ComObjectRef` id.
    /// An active ref's statement must equal the product's selected literal,
    /// or the build is refused ([`ImageError::PriorityDiffers`]); nothing
    /// here is ever encoded.
    pub priority_statements: BTreeMap<String, String>,
    /// Flags the project states for one object instance over the
    /// product's, by the instance's `ComObjectRef` id. Applied to the
    /// activated `ComObjectRef` with that id; an entry for a ref the
    /// parameters leave inactive has no octet to go to.
    pub flag_overrides: BTreeMap<String, FlagOverrides>,
}

/// The communication flags one object instance states over its product's
/// (`ComObjectInstanceRef/@ReadFlag` … `@CommunicationFlag`, or a user
/// edit). `None` keeps the product's flag.
///
/// `[V]` a device holds them: on the maintainer's house, every active
/// object whose instance carries such an attribute has it in its group
/// object table (RESEARCH §19.13).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlagOverrides {
    pub read: Option<bool>,
    pub write: Option<bool>,
    pub transmit: Option<bool>,
    pub update: Option<bool>,
    pub communication: Option<bool>,
    /// Has no bit on `070nh`; only decides whether its loss is disclosed.
    pub read_on_init: Option<bool>,
}

impl FlagOverrides {
    /// Whether no flag is overridden.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Writes the five stated flags; read-on-init is a separate request.
    fn apply(&self, flags: &mut EffectiveObjectFlags) {
        let set = |slot: &mut bool, stated: Option<bool>| {
            if let Some(value) = stated {
                *slot = value;
            }
        };
        set(&mut flags.read, self.read);
        set(&mut flags.write, self.write);
        set(&mut flags.transmit, self.transmit);
        set(&mut flags.update, self.update);
        set(&mut flags.communication, self.communication);
    }
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

/// The placed extent of one computed group table within its segment.
///
/// The source `Mask` marks the table's writable *capacity*, not its computed
/// content. Recording where the built table actually landed lets the plan
/// write the table instead of the template's full writable span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableWrite {
    /// The segment the table was placed into.
    pub segment_id: String,
    /// Byte offset of the table within the segment.
    pub start: usize,
    /// Exclusive end offset of the table within the segment.
    pub end: usize,
}

/// One actually written active parameter and its verified image bit placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterAttribution {
    pub reference: String,
    pub name: String,
    pub segment: String,
    pub field: ParameterField,
    pub value: AttributedValue,
}

/// Only encodings already admitted by the image writer, not guessed device semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributedValue {
    Number {
        value: u64,
        nonnegative_signed: bool,
        /// Written octet-reversed (`LittleEndian`, KL-170); decode likewise.
        low_octet_first: bool,
    },
    /// A `TypeFloat` value's bits in its encoding (ADR-0169).
    Float {
        bits: u64,
        encoding: float::FloatEncoding,
    },
    Text {
        value: String,
        encoding: Option<String>,
    },
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
    /// values; a float's value is its encoded bits (ADR-0169).
    pub parameters: BTreeMap<String, u64>,
    /// The text parameters written, by `ParameterRef` id, with their
    /// values.
    pub texts: BTreeMap<String, String>,
    /// The active group objects, by number.
    pub objects: Vec<ActiveObject>,
    /// The inferences the image rests on (ADR-0086), in the order applied.
    pub inferences: Vec<Inference>,
    /// Exact active parameter writes for read-only comparison; never persisted.
    pub parameter_fields: Vec<ParameterAttribution>,
    /// Where each computed group table was placed, so the plan writes the
    /// table content rather than the template's full writable capacity.
    pub table_writes: Vec<TableWrite>,
    /// The parameters placed in properties, one image per property
    /// ([`property::PropertyImage`]); empty for most programs.
    pub properties: Vec<property::PropertyImage>,
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
    /// A valid explicit application-source pin differs from the live winning row.
    /// A phase fact only: missing/invalid bytes and ownership still require admission.
    SourceBindingChanged,
    /// The database failed.
    Database(ProductDbError),
    /// The program's code could not be read.
    Code(CodeError),
    /// The database does not know the program.
    UnknownProgram(String),
    /// The `Dynamic` evaluation could not decide everything.
    Evaluation(Vec<ScopedDiagnostic>),
    /// The program declares a `ParameterByteOrder` this module does not
    /// write: anything but `BigEndian`, and `LittleEndian` outside a memory
    /// (`070nh`) layout (KL-170, ADR-0159). Its own variant so a client can
    /// explain it instead of quoting this text.
    UnsupportedByteOrder {
        /// The declared order, verbatim (e.g. `LittleEndian` for a legacy layout).
        order: String,
    },
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
    /// Two active parameters share bits of one segment and are not members
    /// of one union (those are resolved by [`UNION_LATER_MEMBER`]).
    Overlap(Box<ParameterOverlap>),
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
    /// The project states a priority for an active object that is not the
    /// product's selected literal; writing the product priority would
    /// silently program something else (docs/contracts/object-priority-edit.md).
    PriorityDiffers {
        /// The `ComObjectRef` id.
        ref_id: String,
        /// The project's stated token (user edit or captured declaration).
        project: String,
        /// The product's selected literal; `None` when it states none.
        product: Option<String>,
    },
    /// A change would hit an octet the segment's `Mask` marks.
    Masked {
        /// The segment.
        segment: String,
        /// The octet offset in it.
        offset: usize,
    },
}

/// Two active parameters that share bits ([`ImageError::Overlap`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterOverlap {
    /// The `ParameterRef` being written.
    pub parameter_ref: String,
    /// Where it lies.
    pub field: ParameterField,
    /// The `ParameterRef` written before, whose bits it shares.
    pub earlier_ref: String,
    /// Where that one lies.
    pub earlier_field: ParameterField,
    /// The segment.
    pub segment: String,
}

impl fmt::Display for ParameterOverlap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ParameterOverlap {
            parameter_ref,
            field,
            earlier_ref,
            earlier_field,
            segment,
        } = self;
        write!(
            f,
            "{parameter_ref} (offset {} bit {}) shares bits with {earlier_ref} \
             (offset {} bit {}) in {segment}; both are active",
            field.offset, field.bit_offset, earlier_field.offset, earlier_field.bit_offset
        )
    }
}

impl std::error::Error for ImageError {}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::SourceBindingChanged => write!(f, "not supported for a download: source ownership binding mismatch"),
            ImageError::Database(error) => write!(f, "{error}"),
            ImageError::Code(error) => write!(f, "{error}"),
            ImageError::UnknownProgram(id) => write!(f, "{id}: not in the product database"),
            ImageError::Evaluation(diagnostics) => write!(
                f,
                "the parameter tree could not be evaluated completely ({} diagnostics, first: {:?})",
                diagnostics.len(),
                diagnostics.first().map(|d| &d.diagnostic)
            ),
            ImageError::UnsupportedByteOrder { order } => write!(
                f,
                "not supported for a download: ParameterByteOrder {order:?}: only BigEndian, and \
                 LittleEndian in a memory (070nh) layout, are written"
            ),
            ImageError::Unsupported { what } => write!(f, "not supported for a download: {what}"),
            ImageError::Value {
                parameter_ref,
                cause,
            } => write!(f, "{parameter_ref}: {cause}"),
            ImageError::ConflictingValues { parameter } => {
                write!(f, "{parameter}: two active references hold different values")
            }
            ImageError::Overlap(overlap) => write!(f, "{overlap}"),
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
            ImageError::PriorityDiffers {
                ref_id,
                project,
                product,
            } => match product {
                Some(product) => write!(
                    f,
                    "{ref_id}: the project priority {project:?} differs from the product \
                     priority {product:?}; KNXBench does not program a changed priority"
                ),
                None => write!(
                    f,
                    "{ref_id}: the project priority {project:?} is not stated by the product; \
                     KNXBench does not program a changed priority"
                ),
            },
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
    build_image(conn, request, ImageLayout::Easy3)
}

/// Builds a source-bound System-2 image for offline inspection, never a wire plan.
///
/// Uses explicit legacy Type2/indexed-association formats and records the source
/// interpretation as an inference. Public download admission remains unchanged.
/// Uncorroborated placement, unsupported flags/encoding and overlapping tables
/// refuse rather than borrowing the hardware evidence of the Easy3 path.
pub fn build_system2_image(
    conn: &Connection,
    request: &ImageRequest,
) -> Result<DownloadImage, ImageError> {
    build_image(conn, request, ImageLayout::System2)
}

/// Builds a source-bound 0012/DefaultProcedure Type1 image for offline review.
///
/// Does not translate or execute the default procedure or grant device support.
pub fn build_system1_image(
    conn: &Connection,
    request: &ImageRequest,
) -> Result<DownloadImage, ImageError> {
    build_image(conn, request, ImageLayout::System1)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ImageLayout {
    Easy3,
    System1,
    System2,
}

impl ImageLayout {
    fn legacy(self) -> bool {
        self != Self::Easy3
    }

    fn family(self) -> &'static str {
        match self {
            Self::Easy3 => "Easy3",
            Self::System1 => "System1",
            Self::System2 => "System2",
        }
    }

    fn object_shape(self) -> (usize, usize) {
        match self {
            Self::Easy3 => (
                knx_core::commissioning::group_object_table::HEADER_OCTETS,
                knx_core::commissioning::group_object_table::DESCRIPTOR_OCTETS,
            ),
            Self::System1 | Self::System2 => (2, 3),
        }
    }
}

fn build_image(
    conn: &Connection,
    request: &ImageRequest,
    layout: ImageLayout,
) -> Result<DownloadImage, ImageError> {
    build_image_selected(conn, request, layout, None)
}

fn build_image_selected(
    conn: &Connection,
    request: &ImageRequest,
    layout: ImageLayout,
    experimental: Option<&experimental::Prepared>,
) -> Result<DownloadImage, ImageError> {
    let program_id = request.program_id.as_str();
    let code = load_program_code(conn, program_id)?
        .ok_or_else(|| ImageError::UnknownProgram(program_id.to_string()))?;
    // Retaining the original XML does not mean its transformations were applied.
    // All image families must refuse before returning a plausible partial image.
    if code.has_fixup_list {
        return Err(ImageError::Unsupported {
            what: format!(
                "{} FixupList requires a separate source interpretation",
                layout.family()
            ),
        });
    }
    if !code.relative_segments.is_empty() {
        return Err(ImageError::Unsupported {
            what: "relative source segments require a relative image builder".into(),
        });
    }
    // The strict source-image entries require this unresolved declaration to
    // be closed. Easy3 keeps its existing inference policy; changing its
    // established source/execution contract is a separate compatibility task.
    if layout != ImageLayout::Easy3 && experimental.is_none() && code.has_legacy_patch_always {
        return Err(ImageError::Unsupported {
            what: format!(
                "{} LegacyPatchAlways requires a separate source interpretation",
                layout.family()
            ),
        });
    }
    // Explicit inactive-parameter policies affect selection and possibly masks.
    // Keeping their source text is not the same as applying them; do not return
    // an active-only System-2 image while silently ignoring the declaration.
    if layout == ImageLayout::System2 && experimental.is_none() {
        if let Some(policy) = code.options.get("DownloadInvisibleParameters") {
            return Err(ImageError::Unsupported {
                what: format!(
                    "System2 DownloadInvisibleParameters={policy:?} requires a separate source interpretation"
                ),
            });
        }
    }
    let low_octet_first = match code.options.get("ParameterByteOrder").map(String::as_str) {
        None | Some("BigEndian") => false,
        Some("LittleEndian") if layout == ImageLayout::Easy3 => true,
        Some(order) => {
            return Err(ImageError::UnsupportedByteOrder {
                order: order.to_string(),
            })
        }
    };
    let protected_tables = if layout == ImageLayout::System2 {
        crate::system2::project_tool_metadata(&code).map_err(|error| ImageError::Unsupported {
            what: error.to_string(),
        })?;
        legacy_source::table_spans(&code, layout)?
    } else if layout == ImageLayout::System1 {
        system1::table_spans(&code)?
    } else {
        Vec::new()
    };
    let types = ParameterTypes::load(conn, program_id, &code)?;
    let types = if layout == ImageLayout::Easy3 && experimental.is_none() {
        types.admitting_floats()
    } else {
        types
    };

    // Every chosen value is checked before evaluation, so that a value the
    // type does not allow is reported as such, not as an undecided branch.
    for (parameter_ref, value) in &request.values {
        if let Some(policy) = experimental.filter(|p| p.retains_source_template()) {
            policy.validate_requested_value(&code, &types, parameter_ref, value)?;
        } else {
            types.value(parameter_ref, value)?;
        }
    }
    let (values, mut activation) = activate(conn, request, &types)?;
    if let Some(policy) = experimental {
        policy.extend_activation(&mut activation, &types)?;
        policy.validate_flag_overrides(request, &activation)?;
    }

    let experimental_base = experimental
        .map(|policy| policy.background(conn, &code, &types, &activation, &protected_tables))
        .transpose()?;
    let mut images: Vec<(&AbsoluteSegment, ParameterImage)> = code
        .segments
        .iter()
        .filter_map(|segment| {
            let data = experimental_base
                .as_ref()
                .and_then(|base| base.get(&segment.id))
                .cloned()
                .or_else(|| segment.data.clone())?;
            Some((segment, ParameterImage::new(data)))
        })
        .collect();

    // 1. Parameters.
    let group_objects = com_object_span_with_layout(&code, layout)?;
    let defaults = if layout.legacy() {
        Some(resolve_values(conn, program_id, &Default::default())?)
    } else {
        None
    };
    let (superseded, mut inferences) = if layout.legacy() {
        // No cross-family last-union-member inference: inferred placements need
        // source corroboration and overlapping alternatives still refuse.
        (BTreeSet::new(), Vec::new())
    } else {
        superseded_union_members(&activation.parameter_refs, &types, &code)
    };
    if layout == ImageLayout::System2 {
        inferences.push(Inference {
            rule: "system2-source-image",
            detail: "Offline source interpretation: Type2 descriptors, indexed association slots, big-endian fields and retained source masks; not device or executable-plan evidence".into(),
            reference: "docs/contracts/offline-download-system2-image.md",
        });
    } else if layout == ImageLayout::System1 {
        inferences.push(Inference {
            rule: "system1-source-image",
            detail: "Offline Type1 source interpretation: indexed associations, source pointers, corroborated fields and retained masks; not executable-plan or device evidence".into(),
            reference: "docs/contracts/offline-download-system1-image.md",
        });
    }
    let mut written: BTreeMap<String, (String, Encoded)> = BTreeMap::new();
    // Every field written so far, with its segment and `ParameterRef`, so
    // that an overlap can name both parties.
    let mut fields: Vec<(&str, ParameterField, &str)> = Vec::new();
    let mut parameters = BTreeMap::new();
    let mut texts = BTreeMap::new();
    let mut parameter_fields = Vec::new();
    let assigned = if layout == ImageLayout::Easy3 && experimental.is_none() {
        assigned_invisible_values(&code, &types, &activation, &values)?
    } else {
        Vec::new()
    };
    let refs = activation
        .parameter_refs
        .iter()
        .map(|active| active.ref_id.as_str());
    let assigned_refs = assigned.iter().map(|(target, _)| target.as_str());
    for parameter_ref in refs.chain(assigned_refs) {
        let parameter = types.parameter_of(parameter_ref)?;
        let Some(placement) = code.parameters.get(parameter) else {
            continue; // Lives nowhere in memory.
        };
        let assigned_value = assigned
            .iter()
            .find(|(target, _)| target == parameter_ref)
            .map(|(_, value)| value.as_str());
        let raw = assigned_value
            .or_else(|| values.get_unscoped(parameter_ref))
            .ok_or_else(|| ImageError::Value {
                parameter_ref: parameter_ref.to_string(),
                cause: "no value, and no default".to_string(),
            })?;
        // A placement the download cannot write is the reason, whatever the
        // value's type (census 3, G10).
        if let ParameterPlacement::Unmodelled { name, .. } = placement {
            return Err(unwritable_placement(parameter, name));
        }
        // Only under the policy ETS was observed with (ADR-0161).
        if matches!(placement, ParameterPlacement::Property(_))
            && code
                .options
                .get("DownloadInvisibleParameters")
                .map(String::as_str)
                != Some("Background")
        {
            return Err(unwritable_placement(parameter, "Property"));
        }
        let value = match experimental {
            Some(policy) => policy.value(&types, parameter_ref, raw)?,
            None => types.value(parameter_ref, raw)?,
        };
        if superseded.contains(parameter) {
            continue; // A later union member holds its bits.
        }
        match written.get(parameter) {
            Some((_, earlier)) if *earlier == value => continue,
            Some(_) => {
                return Err(ImageError::ConflictingValues {
                    parameter: parameter.to_string(),
                })
            }
            None => {}
        }
        // Written into the property's image after this loop (ADR-0161).
        if let ParameterPlacement::Property(_) = placement {
            if let Encoded::Number(number) = &value {
                parameters.insert(parameter_ref.to_string(), *number);
            }
            written.insert(parameter.to_string(), (parameter_ref.to_string(), value));
            continue;
        }
        if layout.legacy() && moves_past_a_mid_octet_union_start(placement) {
            // ADR-0168 rests on mask 0701h images; legacy layouts keep the refusal.
            return Err(ImageError::Unsupported {
                what: format!(
                    "{parameter}: a union member offset inside a union that starts mid-octet"
                ),
            });
        }
        let (segment_id, field) = field_of(parameter, placement, types.size(parameter_ref)?)?;
        if let Some(defaults) = &defaults {
            if experimental.is_none() {
                let raw_default = defaults.get_unscoped(parameter_ref).ok_or_else(|| {
                    ImageError::Unsupported {
                        what: format!(
                            "{parameter_ref}: {} source default unavailable",
                            layout.family()
                        ),
                    }
                })?;
                let default_value = types.value(parameter_ref, raw_default)?;
                legacy_source::validate_default(
                    &code,
                    layout,
                    segment_id,
                    field,
                    parameter_ref,
                    placement,
                    (&default_value, &value),
                )?;
            }
        }
        let field_start = field.offset as usize;
        let field_end =
            field_start + (u32::from(field.bit_offset) + field.size_in_bit).div_ceil(8) as usize;
        if protected_tables
            .iter()
            .any(|(id, start, end)| id == segment_id && field_start < *end && *start < field_end)
        {
            return Err(ImageError::Unsupported {
                what: format!(
                    "{parameter}: overlaps a reserved {} table region",
                    layout.family()
                ),
            });
        }
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
        let ordered = match &value {
            Encoded::Number(number) if low_octet_first && crosses_octet_boundary(field) => {
                reverse_octets(field, *number).map_err(|_| ImageError::Unsupported {
                    what: format!(
                        "{parameter_ref}: ParameterByteOrder \"LittleEndian\" field at bit \
                         offset {} with {} bits has no established octet order",
                        field.bit_offset, field.size_in_bit
                    ),
                })?
            }
            Encoded::Number(number) => *number,
            // Refused in a LittleEndian program (ParameterTypes::float_encoding).
            Encoded::Float { bits, .. } => *bits,
            Encoded::Text { .. } => 0,
        };
        match &value {
            Encoded::Number(_) | Encoded::Float { .. } => image.write(field, ordered),
            Encoded::Text { octets, .. } => image.write_octets(field, octets),
        }
        .map_err(|error| overlap_or(error, parameter_ref, segment_id, &fields))?;
        fields.push((segment_id, field, parameter_ref));
        let facts = types.get(parameter_ref)?;
        parameter_fields.push(ParameterAttribution {
            reference: parameter_ref.to_string(),
            name: facts
                .name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| parameter.to_string()),
            segment: segment_id.to_string(),
            field,
            value: match &value {
                Encoded::Number(number) => AttributedValue::Number {
                    value: *number,
                    nonnegative_signed: facts.number_type.as_deref() == Some("signedInt"),
                    low_octet_first: low_octet_first && crosses_octet_boundary(field),
                },
                Encoded::Float { bits, encoding } => AttributedValue::Float {
                    bits: *bits,
                    encoding: *encoding,
                },
                Encoded::Text { text, .. } => AttributedValue::Text {
                    value: text.clone(),
                    encoding: types.text_encoding.clone(),
                },
            },
        });
        match &value {
            Encoded::Number(number) | Encoded::Float { bits: number, .. } => {
                parameters.insert(parameter_ref.to_string(), *number)
            }
            Encoded::Text { text, .. } => {
                texts.insert(parameter_ref.to_string(), text.clone());
                None
            }
        };
        written.insert(parameter.to_string(), (parameter_ref.to_string(), value));
    }
    // Every time value written, by unit; invisible ones placed in a
    // property are added where their defaults are taken.
    let mut time_values: BTreeMap<String, usize> = BTreeMap::new();
    for (parameter_ref, _) in written.values() {
        if let Some(unit) = types.time_unit(parameter_ref)? {
            *time_values.entry(unit.to_string()).or_default() += 1;
        }
    }
    let properties = if code
        .parameters
        .values()
        .any(|placement| matches!(placement, ParameterPlacement::Property(_)))
    {
        if layout != ImageLayout::Easy3 || experimental.is_some() {
            return Err(ImageError::Unsupported {
                what: format!("{} parameters placed in properties", layout.family()),
            });
        }
        let defaults = resolve_values(conn, program_id, &Default::default())?;
        property::property_images(
            &code,
            &types,
            &written,
            &defaults,
            low_octet_first,
            &mut time_values,
            &mut inferences,
        )?
    } else {
        Vec::new()
    };
    if !time_values.is_empty() {
        let counts: Vec<String> = time_values
            .iter()
            .map(|(unit, count)| format!("{count} in {unit}"))
            .collect();
        inferences.push(Inference {
            rule: TIME_AS_UNIT_COUNT,
            detail: format!(
                "time parameters ({}) are written as the plain count of their unit, like a \
                 number, as ETS 6.4 wrote HundredMilliseconds",
                counts.join(", ")
            ),
            reference: "docs/research/2026-11-08-time-parameters-and-inline-property-writes.md",
        });
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
        if let Some(stated) = request.priority_statements.get(*ref_id) {
            if view.priority.as_deref() != Some(stated.as_str()) {
                return Err(ImageError::PriorityDiffers {
                    ref_id: (*ref_id).to_string(),
                    project: stated.clone(),
                    product: view.priority.clone(),
                });
            }
        }
        if layout.legacy() && !matches!(view.priority.as_deref(), None | Some("Low")) {
            return Err(ImageError::Unsupported {
                what: format!(
                    "{ref_id}: {} priority mapping is not established",
                    layout.family()
                ),
            });
        }
        let (object, requested_read_on_init) = active_object(
            ref_id,
            view,
            request.flag_overrides.get(*ref_id),
            &mut inferences,
        )?;
        if requested_read_on_init {
            if layout.legacy() {
                return Err(ImageError::Unsupported {
                    what: format!(
                        "{ref_id}: {} ReadOnInit encoding is not established",
                        layout.family()
                    ),
                });
            }
            inferences.push(Inference {
                rule: READ_ON_INIT_NOT_ON_070N,
                detail: format!(
                    "{ref_id}: ReadOnInit is enabled, but a 070nh group object has no \
                     read-on-init bit; the device will not read the value after a reset"
                ),
                reference: "RESEARCH §19.18",
            });
        }
        objects.push(object);
    }
    objects.sort_by_key(|object| object.number);
    match &group_objects {
        Some((segment_id, start, _)) => {
            let image = segment_mut(&mut segments, segment_id)?;
            let table = match layout {
                ImageLayout::Easy3 => write_group_object_table(&image.octets[*start..], &objects)
                    .map_err(ImageError::GroupObjects)?,
                ImageLayout::System1 | ImageLayout::System2 => {
                    knx_core::commissioning::legacy_object_table::rewrite(
                        if layout == ImageLayout::System1 {
                            knx_core::commissioning::legacy_object_table::LegacyObjectFormat::Type1
                        } else {
                            knx_core::commissioning::legacy_object_table::LegacyObjectFormat::Type2
                        },
                        &image.octets[*start..],
                        &objects,
                    )
                    .map_err(|error| ImageError::Unsupported {
                        what: error.to_string(),
                    })?
                }
            };
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
    let mut table_writes = Vec::new();
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
            let tables = match layout {
                ImageLayout::Easy3 => {
                    build_group_tables(request.individual_address, &request.links, capacity)
                        .map_err(ImageError::GroupTables)?
                }
                ImageLayout::System1 | ImageLayout::System2 => {
                    let (id, start, _) =
                        group_objects
                            .as_ref()
                            .ok_or_else(|| ImageError::Unsupported {
                                what: format!(
                                    "{} image requires a source object count",
                                    layout.family()
                                ),
                            })?;
                    let count = code
                        .segment(id)
                        .and_then(|s| s.data.as_ref())
                        .and_then(|data| data.get(*start))
                        .copied()
                        .ok_or_else(|| ImageError::Unsupported {
                            what: format!("{} object count missing", layout.family()),
                        })?;
                    knx_core::commissioning::legacy_group_tables::build(
                        request.individual_address,
                        count,
                        &request.links,
                        capacity,
                    )
                    .map_err(|error| ImageError::Unsupported {
                        what: error.to_string(),
                    })?
                }
            };
            table_writes.push(place(
                &mut segments,
                addresses,
                "AddressTable",
                &tables.address_table,
            )?);
            table_writes.push(place(
                &mut segments,
                associations,
                "AssociationTable",
                &tables.association_table,
            )?);
        }
        _ if !request.links.is_empty() => {
            return Err(ImageError::Unsupported {
                what: "group address links, but no AddressTable/AssociationTable placement"
                    .to_string(),
            })
        }
        _ => {}
    }

    if types.text_encoding.is_none() && texts.values().any(|text| !text.is_ascii()) {
        inferences.push(Inference {
            rule: UNDECLARED_TEXT_WINDOWS_1252,
            detail: "the program declares no TextParameterEncoding; non-ASCII text characters \
                     are written in Windows-1252, as ETS 6.4 on a Western European Windows did"
                .into(),
            reference: "docs/research/2026-11-07-ldctrlwriteprop-and-byte-order.md",
        });
    }
    check_masks(&code, &segments)?;
    Ok(DownloadImage {
        code,
        segments,
        parameters,
        texts,
        objects,
        inferences,
        parameter_fields,
        table_writes,
        properties,
    })
}

/// ADR-0171: the invisible parameters an active `Assign` sets, with the value
/// it sets, in a program whose `DownloadInvisibleParameters="Background"`.
/// ETS 6.4 wrote the assigned value into such a parameter's memory field,
/// where KNXBench wrote the template; ETS runs the assignment whatever the
/// project held before. Only plain `Memory` placements of unscoped
/// assignments are admitted, and only where writing the assigned value
/// cannot contradict the rest of the image; everything else refuses rather
/// than guesses:
/// - an unreadable active `Assign` (its target is unknown);
/// - assignments that disagree, or a source without a value;
/// - a target that steers the evaluation (the controller of a `choose` the
///   evaluation consulted, or the source of another active `Assign`) and
///   holds another value: KNXBench evaluated with the stored value, ETS with
///   the assigned one;
/// - a target whose parameter an active reference writes with another value.
fn assigned_invisible_values(
    code: &ProgramCode,
    types: &ParameterTypes,
    activation: &Activation,
    values: &ValueMap,
) -> Result<Vec<(String, String)>, ImageError> {
    if code
        .options
        .get("DownloadInvisibleParameters")
        .map(String::as_str)
        != Some("Background")
    {
        return Ok(Vec::new());
    }
    let refuse = |what: String| Err(ImageError::Unsupported { what });
    let active_refs: BTreeSet<&str> = activation
        .parameter_refs
        .iter()
        .map(|active| active.ref_id.as_str())
        .collect();
    let unscoped: Vec<_> = activation
        .assigned
        .iter()
        .filter(|active| active.scope.is_none())
        .collect();
    let sources: BTreeSet<&str> = unscoped
        .iter()
        .filter_map(|active| match &active.assignment.as_ref()?.source {
            AssignSource::Parameter(source) => Some(source.as_str()),
            AssignSource::Value(_) => None,
        })
        .collect();
    let mut out: Vec<(String, String)> = Vec::new();
    for active in unscoped {
        let Some(assignment) = &active.assignment else {
            return refuse("an active Assign could not be read".into());
        };
        let target = assignment.target.as_str();
        if active_refs.contains(target) {
            continue; // A visible target keeps its own value (ADR-0168 compares it).
        }
        let Ok(parameter) = types.parameter_of(target) else {
            return refuse(format!(
                "{target}: an active Assign targets an unknown parameter"
            ));
        };
        if !matches!(
            code.parameters.get(parameter),
            Some(ParameterPlacement::Memory(_))
        ) {
            continue; // Unions, properties and unplaced ones keep their handling.
        }
        let assigned = match &assignment.source {
            AssignSource::Parameter(source) => values.get_unscoped(source),
            AssignSource::Value(value) => Some(value.as_str()),
        };
        let Some(assigned) = assigned else {
            return refuse(format!("{target}: an active Assign's source has no value"));
        };
        let stored = values.get_unscoped(target);
        let steers = active.target_controls_choose || sources.contains(target);
        if steers && !stored.is_some_and(|stored| same_value(stored, assigned)) {
            return refuse(format!(
                "{target}: an active Assign sets {assigned:?}, the evaluation read {stored:?}"
            ));
        }
        for sibling in &activation.parameter_refs {
            let sibling = sibling.ref_id.as_str();
            if types.parameter_of(sibling).ok() == Some(parameter)
                && !values
                    .get_unscoped(sibling)
                    .is_some_and(|value| same_value(value, assigned))
            {
                return refuse(format!(
                    "{target}: an active Assign sets {assigned:?}, the visible {sibling} differs"
                ));
            }
        }
        match out.iter().find(|(known, _)| known == target) {
            Some((_, earlier)) if same_value(earlier, assigned) => {}
            Some((_, earlier)) => {
                return refuse(format!(
                    "{target}: active Assign elements set {earlier:?} and {assigned:?}"
                ))
            }
            None => out.push((target.to_string(), assigned.to_string())),
        }
    }
    Ok(out)
}

/// The parameters and objects `request`'s values activate, and every value
/// resolved: the evaluation half of the module documentation, shared with
/// [`crate::byte_order_probe`], which reads what an image would write.
///
/// A legal value no `when` covers activates nothing; every other evaluation
/// diagnostic, and every module instance, refuses.
pub(crate) fn activate(
    conn: &Connection,
    request: &ImageRequest,
    types: &ParameterTypes,
) -> Result<(ValueMap, Activation), ImageError> {
    activate_values(conn, &request.program_id, &request.values, types)
}

/// Shared checked activation, independent of image-only request fields.
fn activate_values(
    conn: &Connection,
    program_id: &str,
    choices: &BTreeMap<String, String>,
    types: &ParameterTypes,
) -> Result<(ValueMap, Activation), ImageError> {
    let supplied = choices.clone().into_iter().collect();
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
    Ok((values, activation))
}

/// `[A]` (ADR-0086, RESEARCH §19.12) The parameters not written because a
/// later member of their union holds the bits they share, and the
/// inference that says so for each.
///
/// Walks `active` (document order) backwards, so the member written is the
/// one later in the parameter tree; an earlier member of the same union
/// whose bits overlap a member already kept is superseded. A member whose
/// placement or size cannot be read is left to the write loop, which
/// refuses it by name.
pub(crate) fn superseded_union_members(
    active: &[crate::dynamic::ActiveRef],
    types: &ParameterTypes,
    code: &ProgramCode,
) -> (BTreeSet<String>, Vec<Inference>) {
    let mut kept: Vec<(&MemoryPlacement, ParameterField, &str, &str)> = Vec::new();
    let mut superseded = BTreeSet::new();
    let mut inferences = Vec::new();
    for active in active.iter().rev() {
        let parameter_ref = active.ref_id.as_str();
        let Ok(parameter) = types.parameter_of(parameter_ref) else {
            continue;
        };
        let Some(placement @ ParameterPlacement::UnionMember { union, .. }) =
            code.parameters.get(parameter)
        else {
            continue;
        };
        let Ok((_, field)) = types
            .size(parameter_ref)
            .and_then(|size| field_of(parameter, placement, size))
        else {
            continue;
        };
        if kept.iter().any(|(_, _, kept, _)| *kept == parameter) {
            continue; // Another reference of a member already kept.
        }
        let (start, end) = bit_span(field);
        let winner = kept.iter().find(|(kept_union, kept_field, _, _)| {
            let (s, e) = bit_span(*kept_field);
            *kept_union == union && s < end && start < e
        });
        match winner {
            Some((_, _, _, winner_ref)) => {
                if superseded.insert(parameter.to_string()) {
                    inferences.push(Inference {
                        rule: UNION_LATER_MEMBER,
                        detail: format!(
                            "{parameter_ref} is not written: {winner_ref}, later in the \
                             parameter tree, holds the union at {} offset {} bit {}",
                            union.code_segment, union.offset, union.bit_offset
                        ),
                        reference: "RESEARCH §19.12",
                    });
                }
            }
            None if !superseded.contains(parameter) => {
                kept.push((union, field, parameter, parameter_ref));
            }
            None => {}
        }
    }
    // Reported in document order, like everything else in the image.
    inferences.reverse();
    (superseded, inferences)
}

/// The bits a field covers, counted MSB-first from the segment start.
fn bit_span(field: ParameterField) -> (u64, u64) {
    let start = u64::from(field.offset) * 8 + u64::from(field.bit_offset);
    (start, start + u64::from(field.size_in_bit))
}

/// `error` as an [`ImageError`]: an overlap names the field written before
/// (from `fields`, the fields written so far). Every other refusal is
/// passed on as it is.
fn overlap_or(
    error: ParameterImageError,
    parameter_ref: &str,
    segment: &str,
    fields: &[(&str, ParameterField, &str)],
) -> ImageError {
    let ParameterImageError::Overlap { field, earlier } = error else {
        return ImageError::Parameter {
            parameter_ref: parameter_ref.to_string(),
            error,
        };
    };
    let Some((_, _, earlier_ref)) = fields
        .iter()
        .find(|(id, written, _)| *id == segment && *written == earlier)
    else {
        // Every write is recorded, so this is not reached; refuse the same
        // way all the same rather than panic.
        return ImageError::Parameter {
            parameter_ref: parameter_ref.to_string(),
            error,
        };
    };
    ImageError::Overlap(Box::new(ParameterOverlap {
        parameter_ref: parameter_ref.to_string(),
        field,
        earlier_ref: (*earlier_ref).to_string(),
        earlier_field: earlier,
        segment: segment.to_string(),
    }))
}

/// The refusal of a placement no memory image can hold (`Property` and the
/// other placements [`crate::code`] keeps as unmodelled), by name.
fn unwritable_placement(parameter: &str, name: &str) -> ImageError {
    ImageError::Unsupported {
        what: format!("{parameter}: placed by {name}"),
    }
}

/// Whether `placement` is a union member away from the start of a union that
/// itself starts inside an octet.
fn moves_past_a_mid_octet_union_start(placement: &ParameterPlacement) -> bool {
    matches!(
        placement,
        ParameterPlacement::UnionMember { union, offset, bit_offset }
            if union.bit_offset != 0 && (*offset != 0 || *bit_offset != 0)
    )
}

/// The segment and field a parameter's placement puts it at.
/// Whether `field` touches more than one octet, so that an octet order
/// decides where its bits go (KL-170).
fn crosses_octet_boundary(field: ParameterField) -> bool {
    u32::from(field.bit_offset) + field.size_in_bit > 8
}

pub(crate) fn field_of<'a>(
    parameter: &str,
    placement: &'a ParameterPlacement,
    size_in_bit: u32,
) -> Result<(&'a str, ParameterField), ImageError> {
    if let ParameterPlacement::UnionMember { union, offset, .. } = placement {
        if union.offset.checked_add(*offset).is_none() {
            return Err(ImageError::Unsupported {
                what: format!("{parameter}: union member offset exceeds 32-bit source placement"),
            });
        }
    }
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
        // (module documentation). A union starting inside an octet adds its
        // bit offset to the member's, carrying whole octets into the
        // offset: ETS 6.4 placed members at union bit 2 + member bits 3
        // and 4 that way (docs/research/2026-11-09-assigned-controllers-and-short-segments.md).
        ParameterPlacement::UnionMember {
            union,
            offset,
            bit_offset,
        } => {
            let bits = u32::from(union.bit_offset) + u32::from(*bit_offset);
            let octets = offset
                .checked_add(bits / 8)
                .filter(|octets| union.offset.checked_add(*octets).is_some())
                .ok_or_else(|| ImageError::Unsupported {
                    what: format!(
                        "{parameter}: union member offset exceeds 32-bit source placement"
                    ),
                })?;
            // `bits % 8 < 8`.
            let placed = field(union, octets, (bits % 8) as u8);
            if moves_past_a_mid_octet_union_start(placement) && crosses_octet_boundary(placed) {
                // Observed members all stayed inside one octet.
                return Err(ImageError::Unsupported {
                    what: format!(
                        "{parameter}: a mid-octet union member crossing an octet boundary"
                    ),
                });
            }
            Ok((union.code_segment.as_str(), placed))
        }
        // A property is not a segment: [`property::property_images`] places it.
        ParameterPlacement::Property(_) => Err(unwritable_placement(parameter, "Property")),
        ParameterPlacement::Unmodelled { name, .. } => Err(unwritable_placement(parameter, name)),
    }
}

/// `(segment, first octet, end)` of the group object table, from its
/// placement and its size octet.
fn com_object_span_with_layout(
    code: &ProgramCode,
    layout: ImageLayout,
) -> Result<Option<(String, usize, usize)>, ImageError> {
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
    let (header, descriptor) = layout.object_shape();
    let end = start + header + usize::from(size) * descriptor;
    if layout.legacy() && end > data.len() {
        return Err(ImageError::Unsupported {
            what: format!(
                "{} descriptors extend beyond their source segment",
                layout.family()
            ),
        });
    }
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
/// Returns the placed extent so the caller can record what is real content.
fn place(
    segments: &mut [SegmentImage],
    placement: &TablePlacement,
    table: &str,
    octets: &[u8],
) -> Result<TableWrite, ImageError> {
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
    Ok(TableWrite {
        segment_id: id.to_string(),
        start,
        end,
    })
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

/// One activated `ComObjectRef` as a group object table descriptor, and
/// whether read-on-init is requested. Inferences it applies
/// are pushed onto `inferences`.
fn active_object(
    ref_id: &str,
    view: &crate::query::ComObjectView,
    overrides: Option<&FlagOverrides>,
    inferences: &mut Vec<Inference>,
) -> Result<(ActiveObject, bool), ImageError> {
    let unsupported = |why: String| ImageError::Unsupported {
        what: format!("{ref_id}: {why}"),
    };
    let number = object_flags::object_number(ref_id, view)?;
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
        // `[A]` RESEARCH §19.18: the corpus's own base images.
        Some("Alert") => {
            inferences.push(Inference {
                rule: ALERT_IS_URGENT,
                detail: format!("{ref_id}: Priority Alert is written as urgent (10b)"),
                reference: "RESEARCH §19.18",
            });
            TransmissionPriority::Urgent
        }
        Some(other) => return Err(unsupported(format!("Priority {other:?}"))),
    };
    // Keep size/priority admission and inference order in the image path;
    // comparison resolves only flags through the same lower-level seam.
    let resolved = object_flags::resolve_flags(ref_id, view, overrides)?;
    Ok((
        ActiveObject {
            number,
            flags: resolved.flags.with_priority(priority),
            value_type,
        },
        resolved.requested_read_on_init,
    ))
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
pub(crate) enum Encoded {
    /// A number, written by width ([`ParameterImage::write`]).
    Number(u64),
    /// A `TypeFloat` value's bits, written by width like a number (ADR-0169).
    Float {
        bits: u64,
        encoding: float::FloatEncoding,
    },
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
    name: Option<String>,
}

/// The program's parameter refs and their types, loaded once.
pub(crate) struct ParameterTypes {
    refs: BTreeMap<String, RefType>,
    enums: BTreeMap<String, BTreeSet<String>>,
    /// `Options/@TextParameterEncoding`, if declared.
    text_encoding: Option<String>,
    /// `TypeTime/@Unit` by parameter type id ([`ProgramCode::time_units`]).
    time_units: BTreeMap<String, Option<String>>,
    /// `TypeFloat/@Encoding` by parameter type id
    /// ([`ProgramCode::float_encodings`]).
    float_encodings: BTreeMap<String, Option<String>>,
    /// The program declares `ParameterByteOrder="LittleEndian"`: no float
    /// was observed in one, so floats refuse (ADR-0169).
    little_endian: bool,
    /// Floats are written only by the memory (`070nh`) builder, the one
    /// layout ETS was observed in ([`ParameterTypes::admitting_floats`]).
    floats_admitted: bool,
}

/// The `TypeTime` units whose value is a plain count of the unit: the six
/// the local collection declares besides the packed ones
/// (docs/contracts/parameter-editor-admission.md). Such a value is encoded
/// like a `TypeNumber` (ADR-0162).
const COUNTED_TIME_UNITS: [&str; 6] = [
    "Hours",
    "Minutes",
    "Seconds",
    "HundredMilliseconds",
    "TenMilliseconds",
    "Milliseconds",
];

/// A text encoding this module writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextEncoding {
    /// ISO/IEC 8859-1: every character up to `U+00FF` is its own octet.
    Latin1,
    /// ISO/IEC 8859-15: ISO/IEC 8859-1 with eight positions replaced.
    Latin9,
    /// Windows-1252: ISO/IEC 8859-1 with 27 printable characters in
    /// `80h`–`9Fh`; `81h`, `8Dh`, `8Fh`, `90h`, `9Dh` and the other C1
    /// controls are refused. Only for a program that declares nothing.
    Windows1252,
}

impl TextEncoding {
    /// Windows-1252's printable assignments in `80h`–`9Fh` (the WHATWG
    /// Encoding Standard's `windows-1252` index, identical there).
    const WINDOWS_1252: [(char, u8); 27] = [
        ('€', 0x80),
        ('‚', 0x82),
        ('ƒ', 0x83),
        ('„', 0x84),
        ('…', 0x85),
        ('†', 0x86),
        ('‡', 0x87),
        ('ˆ', 0x88),
        ('‰', 0x89),
        ('Š', 0x8A),
        ('‹', 0x8B),
        ('Œ', 0x8C),
        ('Ž', 0x8E),
        ('‘', 0x91),
        ('’', 0x92),
        ('“', 0x93),
        ('”', 0x94),
        ('•', 0x95),
        ('–', 0x96),
        ('—', 0x97),
        ('˜', 0x98),
        ('™', 0x99),
        ('š', 0x9A),
        ('›', 0x9B),
        ('œ', 0x9C),
        ('ž', 0x9E),
        ('Ÿ', 0x9F),
    ];
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

    /// The character for `octet`, if the encoding assigns one.
    fn character(self, octet: u8) -> Option<char> {
        let table: &[(char, u8)] = match self {
            Self::Latin1 => &[],
            Self::Latin9 => &Self::LATIN_9,
            Self::Windows1252 => &Self::WINDOWS_1252,
        };
        if let Some((character, _)) = table.iter().find(|(_, byte)| *byte == octet) {
            return Some(*character);
        }
        let c1 = (0x80..=0x9F).contains(&octet);
        (!(c1 && self == Self::Windows1252)).then(|| char::from(octet))
    }

    /// The octet for `c`, if the encoding has one.
    fn octet(self, c: char) -> Option<u8> {
        // ISO/IEC 8859-15 reassigns A4h, A6h, A8h, B4h, B8h, BCh, BDh, BEh.
        let code = u32::from(c);
        match self {
            TextEncoding::Latin1 => u8::try_from(code).ok(),
            TextEncoding::Windows1252 => {
                if let Some((_, octet)) = Self::WINDOWS_1252.iter().find(|(known, _)| *known == c) {
                    return Some(*octet);
                }
                u8::try_from(code)
                    .ok()
                    .filter(|octet| !(0x80..=0x9F).contains(octet))
            }
            TextEncoding::Latin9 => {
                if let Some((_, octet)) = Self::LATIN_9.iter().find(|(known, _)| *known == c) {
                    return Some(*octet);
                }
                u8::try_from(code)
                    .ok()
                    .filter(|octet| !Self::LATIN_9.iter().any(|(_, taken)| taken == octet))
            }
        }
    }
}

/// Preflight cumulative padded Text widths without encoding defaults or values.
/// Includes inactive/duplicate references conservatively; this is an investigation
/// caller's allocation budget, not a new device compatibility/download limit.
pub fn text_parameter_buffers_fit(
    conn: &Connection,
    program_id: &str,
    code: &ProgramCode,
    max_octets: u64,
) -> Result<bool, ImageError> {
    let types = ParameterTypes::load(conn, program_id, code)?;
    let mut total = 0_u64;
    for (reference, kind) in &types.refs {
        if kind.kind.as_deref() != Some("Text") {
            continue;
        }
        let width = u64::from(types.size(reference)?) / 8;
        let Some(next) = total.checked_add(width).filter(|n| *n <= max_octets) else {
            return Ok(false);
        };
        total = next;
    }
    Ok(true)
}

/// Exact inverse of the admitted text writer; noncanonical padding stays unknown.
pub(crate) fn decode_parameter_text(encoding: Option<&str>, bytes: &[u8]) -> Option<String> {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    if bytes[end..].iter().any(|b| *b != 0) {
        return None;
    }
    let encoding = match encoding {
        Some(name) => TextEncoding::named(name)?,
        None => TextEncoding::Windows1252,
    };
    bytes[..end]
        .iter()
        .map(|b| encoding.character(*b))
        .collect()
}

/// A `TypeNumber` bound as the integer type this builder reads: `None` when the
/// attribute is absent, a refusal when it is declared but unreadable (empty, decimal,
/// scientific, negative for an unsigned type, out of range), so a bound never vanishes
/// quietly. Callers read the minimum first.
fn declared_bound<T: std::str::FromStr>(
    parameter_ref: &str,
    attribute: &str,
    number_type: &str,
    text: &Option<String>,
) -> Result<Option<T>, ImageError> {
    let Some(raw) = text.as_deref() else {
        return Ok(None);
    };
    raw.trim()
        .parse()
        .ok()
        .map(Some)
        .ok_or_else(|| ImageError::Value {
            parameter_ref: parameter_ref.to_string(),
            cause: format!(
                "{attribute} {raw:?} is not a {number_type} bound this builder reads (unsupported bound)"
            ),
        })
}

/// A `TypeFloat` bound as a number; an unreadable one refuses by name.
fn float_bound(
    parameter_ref: &str,
    attribute: &str,
    text: &Option<String>,
) -> Result<Option<f64>, ImageError> {
    let Some(raw) = text.as_deref() else {
        return Ok(None);
    };
    float::parse(raw).map(Some).map_err(|_| ImageError::Value {
        parameter_ref: parameter_ref.to_string(),
        cause: format!("{attribute} {raw:?} is not a float bound this builder reads"),
    })
}

impl ParameterTypes {
    pub(crate) fn load(
        conn: &Connection,
        program_id: &str,
        code: &ProgramCode,
    ) -> Result<Self, ImageError> {
        let mut stmt = conn.prepare(
            "SELECT pr.id, p.id, pt.kind, pt.size_in_bit, pt.base, pt.number_type,
                    pt.min_inclusive, pt.max_inclusive, pt.id, p.name
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
                        name: r.get(9)?,
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
            time_units: code.time_units.clone(),
            float_encodings: code.float_encodings.clone(),
            little_endian: code.options.get("ParameterByteOrder").map(String::as_str)
                == Some("LittleEndian"),
            floats_admitted: false,
        })
    }

    /// The same types, with `TypeFloat` values admitted (ADR-0169).
    pub(crate) fn admitting_floats(mut self) -> Self {
        self.floats_admitted = true;
        self
    }

    fn get(&self, parameter_ref: &str) -> Result<&RefType, ImageError> {
        self.refs
            .get(parameter_ref)
            .ok_or_else(|| ImageError::Value {
                parameter_ref: parameter_ref.to_string(),
                cause: "no such ParameterRef in this program".to_string(),
            })
    }

    /// Every `ParameterRef` of the program with the `Parameter` it names.
    pub(crate) fn refs(&self) -> impl Iterator<Item = (&str, &str)> {
        self.refs
            .iter()
            .map(|(parameter_ref, found)| (parameter_ref.as_str(), found.parameter.as_str()))
    }

    pub(crate) fn parameter_of(&self, parameter_ref: &str) -> Result<&str, ImageError> {
        Ok(self.get(parameter_ref)?.parameter.as_str())
    }

    /// The field width: 1 to 64 bits for a number, whole octets for a text.
    pub(crate) fn size(&self, parameter_ref: &str) -> Result<u32, ImageError> {
        if let Some(encoding) = self.float_encoding(parameter_ref)? {
            return Ok(encoding.size_in_bit());
        }
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
    pub(crate) fn value(&self, parameter_ref: &str, raw: &str) -> Result<Encoded, ImageError> {
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
                self.within_unsigned_bounds(parameter_ref, found, value)?;
                Ok(Encoded::Number(value))
            }
            // `[V]` ETS 6.4 wrote a `TypeTime` value in `HundredMilliseconds`
            // as the plain count, like a number in the declared byte order
            // (ADR-0162). A packed or undeclared unit stays refused.
            (Some("Time"), _, _) => {
                self.time_unit(parameter_ref)?;
                let value = unsigned()?;
                self.within_unsigned_bounds(parameter_ref, found, value)?;
                Ok(Encoded::Number(value))
            }
            (Some("Number"), _, Some("signedInt")) => {
                let value: i64 = raw
                    .trim()
                    .parse()
                    .map_err(|_| invalid(format!("{raw:?} is not an integer")))?;
                let min = declared_bound(parameter_ref, "minInclusive", "signedInt", &found.min);
                let max = declared_bound(parameter_ref, "maxInclusive", "signedInt", &found.max);
                let (min, max) = (min?, max?);
                if min.is_some_and(|min| value < min) || max.is_some_and(|max| value > max) {
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
            // `[V]` ETS 6.4 wrote both encodings as their bits, high octet
            // first, in a program without a declared byte order (ADR-0169).
            (Some("Float"), _, _) => {
                let encoding = self
                    .float_encoding(parameter_ref)?
                    .expect("a Float ref has an encoding or was refused");
                let value = float::parse(raw).map_err(&invalid)?;
                let min = float_bound(parameter_ref, "minInclusive", &found.min)?;
                let max = float_bound(parameter_ref, "maxInclusive", &found.max)?;
                if min.is_some_and(|min| value < min) || max.is_some_and(|max| value > max) {
                    return Err(invalid(format!(
                        "{raw} is outside {:?}..={:?}",
                        found.min, found.max
                    )));
                }
                Ok(Encoded::Float {
                    bits: encoding.encode(raw).map_err(invalid)?,
                    encoding,
                })
            }
            (kind, base, number_type) => Err(ImageError::Unsupported {
                what: format!(
                    "{}: parameter type {kind:?} (base {base:?}, number type {number_type:?})",
                    found.parameter
                ),
            }),
        }
    }

    /// A `TypeTime` ref's unit, if it is a plain count ([`COUNTED_TIME_UNITS`]);
    /// `None` for a ref of another kind; refused for any other unit.
    pub(crate) fn time_unit(&self, parameter_ref: &str) -> Result<Option<&str>, ImageError> {
        let found = self.get(parameter_ref)?;
        if found.kind.as_deref() != Some("Time") {
            return Ok(None);
        }
        let unit = found
            .parameter_type
            .as_ref()
            .and_then(|id| self.time_units.get(id))
            .and_then(Option::as_deref);
        match unit {
            Some(unit) if COUNTED_TIME_UNITS.contains(&unit) => Ok(Some(unit)),
            other => Err(ImageError::Unsupported {
                what: format!(
                    "{}: TypeTime unit {other:?} has no established encoding",
                    found.parameter
                ),
            }),
        }
    }

    /// A `TypeFloat` ref's encoding (ADR-0169); `None` for a ref of another
    /// kind; refused for an encoding not observed, or in a `LittleEndian`
    /// program.
    pub(crate) fn float_encoding(
        &self,
        parameter_ref: &str,
    ) -> Result<Option<float::FloatEncoding>, ImageError> {
        let found = self.get(parameter_ref)?;
        if found.kind.as_deref() != Some("Float") {
            return Ok(None);
        }
        let declared = found
            .parameter_type
            .as_ref()
            .and_then(|id| self.float_encodings.get(id))
            .and_then(Option::as_deref);
        let encoding = declared
            .and_then(float::FloatEncoding::named)
            .ok_or_else(|| ImageError::Unsupported {
                what: format!(
                    "{}: TypeFloat encoding {declared:?} has no established encoding",
                    found.parameter
                ),
            })?;
        if !self.floats_admitted {
            return Err(ImageError::Unsupported {
                what: format!(
                    "{}: a TypeFloat outside the memory (070nh) image builder has no \
                     established encoding",
                    found.parameter
                ),
            });
        }
        if self.little_endian {
            return Err(ImageError::Unsupported {
                what: format!(
                    "{}: a TypeFloat in a ParameterByteOrder \"LittleEndian\" program has no \
                     established octet order",
                    found.parameter
                ),
            });
        }
        Ok(Some(encoding))
    }

    /// Refuses `value` outside the ref's declared unsigned bounds.
    fn within_unsigned_bounds(
        &self,
        parameter_ref: &str,
        found: &RefType,
        value: u64,
    ) -> Result<(), ImageError> {
        let min = declared_bound(parameter_ref, "minInclusive", "unsignedInt", &found.min);
        let max = declared_bound(parameter_ref, "maxInclusive", "unsignedInt", &found.max);
        let (min, max): (Option<u64>, Option<u64>) = (min?, max?);
        if min.is_some_and(|min| value < min) || max.is_some_and(|max| value > max) {
            return Err(ImageError::Value {
                parameter_ref: parameter_ref.to_string(),
                cause: format!("{value} is outside {:?}..={:?}", found.min, found.max),
            });
        }
        Ok(())
    }

    /// A `TypeText` value in the program's declared encoding, zero-filled
    /// to its field.
    fn text(&self, parameter_ref: &str, found: &RefType, raw: &str) -> Result<Encoded, ImageError> {
        let invalid = |cause: String| ImageError::Value {
            parameter_ref: parameter_ref.to_string(),
            cause,
        };
        let encoding = match self.text_encoding.as_deref() {
            None => TextEncoding::Windows1252,
            Some(name) => TextEncoding::named(name).ok_or_else(|| ImageError::Unsupported {
                what: format!(
                    "{}: TextParameterEncoding {name:?} is not one this crate writes",
                    found.parameter
                ),
            })?,
        };
        let mut octets = Vec::with_capacity(raw.len());
        for c in raw.chars() {
            if c == '\0' {
                return Err(invalid(
                    "a NUL character would end the text early".to_string(),
                ));
            }
            let octet = encoding.octet(c).ok_or_else(|| {
                invalid(format!(
                    "{c:?} has no octet in {}",
                    self.text_encoding
                        .as_deref()
                        .unwrap_or("Windows-1252 (no TextParameterEncoding declared)")
                ))
            })?;
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

    mod number_bounds;

    /// ADR-0168: a union starting at bit 2 adds its bit offset to the
    /// member's, carrying into the next octet; the placements are those of
    /// the observed ETS 6.4 images. Legacy layouts keep the refusal.
    #[test]
    fn a_mid_octet_union_adds_its_bit_offset_to_the_members() {
        let union = |bit_offset| MemoryPlacement {
            code_segment: "AS-43FE".into(),
            offset: 460,
            bit_offset,
        };
        let member = |offset, bit_offset| ParameterPlacement::UnionMember {
            union: union(2),
            offset,
            bit_offset,
        };
        let at = |placement: &ParameterPlacement, size| {
            let (segment, field) = field_of("P", placement, size).unwrap();
            assert_eq!(segment, "AS-43FE");
            (field.offset, field.bit_offset, field.size_in_bit)
        };
        assert_eq!(at(&member(0, 0), 6), (460, 2, 6));
        assert_eq!(at(&member(0, 3), 3), (460, 5, 3));
        assert_eq!(at(&member(0, 4), 2), (460, 6, 2));
        assert_eq!(at(&member(0, 5), 1), (460, 7, 1));
        assert_eq!(at(&member(0, 6), 2), (461, 0, 2));
        assert_eq!(at(&member(1, 7), 1), (462, 1, 1));
        let aligned = ParameterPlacement::UnionMember {
            union: union(0),
            offset: 1,
            bit_offset: 3,
        };
        assert_eq!(at(&aligned, 1), (461, 3, 1));

        assert!(moves_past_a_mid_octet_union_start(&member(0, 4)));
        assert!(moves_past_a_mid_octet_union_start(&member(1, 0)));
        assert!(!moves_past_a_mid_octet_union_start(&member(0, 0)));
        assert!(!moves_past_a_mid_octet_union_start(&aligned));
        let far = ParameterPlacement::UnionMember {
            union: MemoryPlacement {
                offset: u32::MAX,
                ..union(2)
            },
            offset: 0,
            bit_offset: 6,
        };
        assert!(field_of("P", &far, 1).is_err());
        // Unobserved: a mid-octet member running into the next octet.
        assert!(field_of("P", &member(0, 4), 3).is_err());
        assert_eq!(at(&member(0, 0), 7), (460, 2, 7));
    }

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

    #[test]
    fn attribution_captures_only_actual_active_writes_with_exact_bits_and_source_names() {
        let (_dir, conn) = db(PROGRAM);
        let image = build_download_image(&conn, &request(&[], vec![])).unwrap();
        let fields = &image.parameter_fields;
        assert_eq!(
            fields.len(),
            3,
            "only Mode, Time and winning Wide are actually written"
        );
        let mode = fields.iter().find(|p| p.reference == "P-1_R-1").unwrap();
        assert_eq!(mode.name, "Mode");
        assert_eq!(mode.segment, "AS-4400");
        assert_eq!(
            mode.field,
            ParameterField {
                offset: 11,
                bit_offset: 0,
                size_in_bit: 2
            }
        );
        assert_eq!(
            mode.value,
            AttributedValue::Number {
                value: 1,
                nonnegative_signed: false,
                low_octet_first: false
            }
        );
        let narrow = build_download_image(&conn, &request(&[("P-1_R-1", "2")], vec![])).unwrap();
        assert_eq!(
            narrow.parameter_fields.len(),
            2,
            "duplicate aliases do not invent a second write"
        );
        let winner = narrow
            .parameter_fields
            .iter()
            .find(|p| p.reference == "UP-2_R-4")
            .unwrap();
        assert_eq!(winner.name, "Narrow");
        assert_eq!(
            winner.field,
            ParameterField {
                offset: 14,
                bit_offset: 5,
                size_in_bit: 2
            }
        );
    }

    fn db(xml: &str) -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().expect("tempdir");
        let conn = crate::open_and_migrate(&dir.path().join("products.sqlite")).expect("database");
        crate::ingest_file(&conn, "M-0001/A.xml", xml.as_bytes()).expect("ingests");
        (dir, conn)
    }

    #[test]
    fn uninterpreted_fixup_declarations_refuse_easy3_images_without_database_changes() {
        let (_dir, baseline) = db(PROGRAM);
        assert!(build_download_image(&baseline, &request(&[], vec![])).is_ok());
        for declaration in [
            "<FixupList />",
            r#"<FixupList><Fixup FunctionRef="MV-0701_ME-Unknown" CodeSegment="AS-4400"><Offset>12</Offset></Fixup></FixupList>"#,
        ] {
            let xml = PROGRAM.replace("</Static>", &format!("{declaration}</Static>"));
            let (_dir, conn) = db(&xml);
            assert!(
                load_program_code(&conn, ID)
                    .unwrap()
                    .unwrap()
                    .has_fixup_list
            );
            let before = conn.total_changes();
            let error = build_download_image(&conn, &request(&[], vec![]))
                .expect_err("an uninterpreted transformation must not yield a partial image");
            assert!(matches!(error, ImageError::Unsupported { ref what }
                if what == "Easy3 FixupList requires a separate source interpretation"));
            assert_eq!(conn.total_changes(), before);
        }
        let xml = PROGRAM.replace("</Static>", "<Extension><FixupList /></Extension></Static>");
        let (_dir, conn) = db(&xml);
        assert!(
            !load_program_code(&conn, ID)
                .unwrap()
                .unwrap()
                .has_fixup_list
        );
        assert!(build_download_image(&conn, &request(&[], vec![])).is_ok());
    }

    #[test]
    fn easy3_legacy_policy_is_unchanged_by_strict_source_image_work() {
        let (_dir, baseline) = db(PROGRAM);
        let original = build_download_image(&baseline, &request(&[], vec![])).unwrap();
        for value in ["", "0", "1", "false", "true"] {
            let inactive = format!("<Parameter Id=\"INACTIVE\" ParameterType=\"PT-N\" Value=\"2\" LegacyPatchAlways=\"{value}\"><Memory CodeSegment=\"AS-4400\" Offset=\"25\" BitOffset=\"0\" /></Parameter>");
            let xml = PROGRAM.replace("</Parameters>", &format!("{inactive}</Parameters>"));
            let (_dir, conn) = db(&xml);
            let before = conn.total_changes();
            let req = request(&[], vec![]);
            let code = crate::code::load_program_code(&conn, &req.program_id)
                .unwrap()
                .unwrap();
            assert!(code.has_legacy_patch_always);
            let image = build_download_image(&conn, &req)
                .expect("strict System1/System2 work must not redefine the existing Easy3 policy");
            assert_eq!(image.segments, original.segments);
            assert_eq!(conn.total_changes(), before);
        }
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
            flag_overrides: BTreeMap::new(),
            priority_statements: Default::default(),
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
    fn relative_sources_are_never_silently_omitted_from_absolute_images() {
        let relative = r#"<RelativeSegment Id="R" Offset="0" Size="1" LoadStateMachine="4"><Data>AQ==</Data></RelativeSegment>"#;
        let xml = PROGRAM.replace("<Code>", &format!("<Code>{relative}"));
        assert_ne!(
            xml, PROGRAM,
            "the mixed-source fixture must contain a relative segment"
        );
        let (_dir, conn) = db(&xml);
        let before = conn.total_changes();
        for build in [build_download_image, build_system2_image] {
            let error = build(&conn, &request(&[], vec![])).unwrap_err();
            assert!(
                matches!(&error, ImageError::Unsupported { what } if what == "relative source segments require a relative image builder"),
                "{error}"
            );
        }
        assert_eq!(conn.total_changes(), before);
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

    /// RESEARCH §19.12: `A-0019-13-B655` activates two members of one
    /// union, and the device ETS programmed holds the one later in the
    /// parameter tree. ADR-0086: that member is written, the earlier one is
    /// not, and the image says so.
    #[test]
    fn of_two_active_union_members_the_later_one_is_written_and_disclosed() {
        let xml = PROGRAM.replace(
            r#"<ParameterRefRef RefId="UP-1_R-3" />"#,
            r#"<ParameterRefRef RefId="UP-1_R-3" /><ParameterRefRef RefId="UP-2_R-4" />"#,
        );
        let image = build(&xml, &[], vec![]).expect("builds");
        let segment = octets(&image, "AS-4400");
        assert_eq!(
            &segment[13..15],
            &[0x0B, 0xBC],
            "UP-2 = 2 at octet 14 bits 5-6; UP-1's 255 is not written"
        );
        assert_eq!(image.parameters.get("UP-2_R-4"), Some(&2));
        assert_eq!(image.parameters.get("UP-1_R-3"), None);
        assert_eq!(
            image.inferences,
            vec![Inference {
                rule: UNION_LATER_MEMBER,
                detail: "UP-1_R-3 is not written: UP-2_R-4, later in the parameter tree, \
                         holds the union at AS-4400 offset 13 bit 0"
                    .to_string(),
                reference: "RESEARCH §19.12",
            }]
        );
    }

    #[test]
    fn product_defaults_need_no_inference() {
        let image = build(PROGRAM, &[], vec![]).expect("builds");
        assert!(image.inferences.is_empty());
    }

    #[test]
    fn two_overlapping_plain_parameters_are_refused_by_both_names() {
        let xml = PROGRAM.replace(
            r#"<Memory CodeSegment="AS-4400" Offset="12" BitOffset="0" />"#,
            r#"<Memory CodeSegment="AS-4400" Offset="11" BitOffset="0" />"#,
        );
        let error = build(&xml, &[], vec![]).unwrap_err();
        assert!(
            matches!(
                &error,
                ImageError::Overlap(overlap)
                    if overlap.parameter_ref == "P-2_R-2"
                        && overlap.earlier_ref == "P-1_R-1"
            ),
            "{error}"
        );
        assert_eq!(
            error.to_string(),
            "P-2_R-2 (offset 11 bit 0) shares bits with P-1_R-1 (offset 11 bit 0) in \
             AS-4400; both are active"
        );
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

    /// A hidden fixed value on a device property (census 3, G10: 70 programs
    /// place a 48-bit `BinaryValue` on `PID_HARDWARE_TYPE`) is refused for its
    /// placement, which is the real blocker, not for its value type.
    #[test]
    fn a_property_placement_is_named_before_the_value_type() {
        let program = PROGRAM
            .replace(
                r#"<Parameter Id="P-3" Name="Prop" ParameterType="PT-N" Value="1">"#,
                r#"<Parameter Id="P-3" Name="Prop" ParameterType="PT-B" Value="1">"#,
            )
            .replace(
                "</ParameterTypes>",
                r#"<ParameterType Id="PT-B" Name="B"><TypeRestriction Base="BinaryValue" SizeInBit="48">
    <Enumeration Id="PT-B_EN-1" Text="hw" Value="1" BinaryValue="AAEAAgAD" />
  </TypeRestriction></ParameterType>
</ParameterTypes>"#,
            );
        assert_ne!(program, PROGRAM);
        let error = build(&program, &[("P-1_R-1", "0")], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Unsupported { what }
                if what.contains("P-3") && what.contains("Property") && !what.contains("BinaryValue")),
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

    /// Object 0 (`O-0_R-1`, active with the defaults) with `overrides`.
    fn with_overrides(overrides: FlagOverrides) -> Result<DownloadImage, ImageError> {
        let (_dir, conn) = db(PROGRAM);
        let mut request = request(&[], vec![]);
        request
            .flag_overrides
            .insert("O-0_R-1".to_string(), overrides);
        build_download_image(&conn, &request)
    }

    #[test]
    fn an_instance_flag_override_replaces_the_products_flag() {
        // The product says read on, write off, update off (`O-0`); the
        // project's instance says the opposite for write and update, as on
        // the house's 1.1.20 object 0 (RESEARCH §19.13).
        let image = with_overrides(FlagOverrides {
            write: Some(true),
            update: Some(true),
            ..FlagOverrides::default()
        })
        .expect("builds");
        let flags = image.objects[0].flags;
        assert!(flags.write && flags.update, "overridden: {flags:?}");
        assert!(
            flags.read && flags.transmit && flags.communication,
            "kept: {flags:?}"
        );
        // The octet in the table, not only the descriptor: object 0's
        // config octet, update/transmit/write/read/communication and low
        // priority (segment selector 0 in the base image).
        assert_eq!(
            octets(&image, "AS-4400")
                [knx_core::commissioning::group_object_table::HEADER_OCTETS + 2],
            0b1101_1111
        );
    }

    #[test]
    fn an_override_can_also_switch_a_flag_off() {
        let image = with_overrides(FlagOverrides {
            read: Some(false),
            communication: Some(false),
            ..FlagOverrides::default()
        })
        .expect("builds");
        let flags = image.objects[0].flags;
        assert!(!flags.read && !flags.communication, "{flags:?}");
        assert!(flags.transmit, "{flags:?}");
    }

    #[test]
    fn an_override_on_an_object_the_parameters_leave_inactive_changes_nothing() {
        // Object 1 (`O-1_R-2`) is inactive with the defaults; its table
        // entry is the base octet with communication cleared either way.
        let (_dir, conn) = db(PROGRAM);
        let plain = build_download_image(&conn, &request(&[], vec![])).expect("builds");
        let mut request = request(&[], vec![]);
        request.flag_overrides.insert(
            "O-1_R-2".to_string(),
            FlagOverrides {
                read: Some(true),
                ..FlagOverrides::default()
            },
        );
        let overridden = build_download_image(&conn, &request).expect("builds");
        assert_eq!(overridden.segments, plain.segments);
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

    /// Builds `xml` (product `Priority` on `O-0` set to `product`, if any)
    /// with `statement` stated for `ref_id`, and the same build without it.
    fn with_statement(
        product: Option<&str>,
        ref_id: &str,
        statement: &str,
    ) -> (
        Result<DownloadImage, ImageError>,
        Result<DownloadImage, ImageError>,
    ) {
        let xml = match product {
            Some(p) => PROGRAM.replace(
                r#"RefId="O-0" />"#,
                &format!(r#"RefId="O-0" Priority="{p}" />"#),
            ),
            None => PROGRAM.to_string(),
        };
        let (_dir, conn) = db(&xml);
        let plain = build_download_image(&conn, &request(&[], vec![]));
        let mut stated = request(&[], vec![]);
        stated
            .priority_statements
            .insert(ref_id.to_string(), statement.to_string());
        (build_download_image(&conn, &stated), plain)
    }

    #[test]
    fn a_stated_priority_equal_to_the_product_literal_changes_no_octet() {
        for (product, statement) in [("Low", "Low"), ("Alert", "Alert")] {
            let (stated, plain) = with_statement(Some(product), "O-0_R-1", statement);
            let (stated, plain) = (stated.expect("admitted"), plain.expect("builds"));
            assert_eq!(stated.segments, plain.segments, "{product}");
            assert_eq!(stated.inferences, plain.inferences, "{product}");
        }
    }

    #[test]
    fn a_stated_priority_the_product_does_not_state_identically_is_refused() {
        for (product, statement) in [
            (None, "Low"),
            (Some("Low"), "Alert"),
            (Some("Alert"), "Low"),
            (Some("Low"), "High"),
        ] {
            let (stated, plain) = with_statement(product, "O-0_R-1", statement);
            plain.expect("the product alone builds");
            match stated {
                Err(ImageError::PriorityDiffers {
                    ref_id,
                    project,
                    product: found,
                }) => {
                    assert_eq!(ref_id, "O-0_R-1");
                    assert_eq!(project, statement);
                    assert_eq!(found.as_deref(), product);
                    let shown = ImageError::PriorityDiffers {
                        ref_id,
                        project,
                        product: found,
                    }
                    .to_string();
                    assert!(
                        shown.contains("does not program a changed priority"),
                        "{shown}"
                    );
                    assert!(shown.contains(&format!("{statement:?}")), "{shown}");
                }
                other => panic!("{product:?}/{statement}: {other:?}"),
            }
        }
    }

    #[test]
    fn an_equal_high_statement_still_meets_the_existing_high_refusal() {
        let (stated, _) = with_statement(Some("High"), "O-0_R-1", "High");
        assert!(
            matches!(&stated, Err(ImageError::Unsupported { what }) if what.contains("High")),
            "{stated:?}"
        );
    }

    #[test]
    fn a_statement_on_an_inactive_object_has_no_octet_to_guard() {
        let (stated, plain) = with_statement(None, "O-1_R-2", "Alert");
        assert_eq!(
            stated.expect("builds").segments,
            plain.expect("builds").segments
        );
    }

    /// RESEARCH §19.18: 284 of 285 `Alert` objects in the corpus's `070nh`
    /// base images carry `10b`, urgent. An inference (ADR-0086).
    #[test]
    fn alert_is_written_as_urgent_and_disclosed() {
        let xml = PROGRAM.replace(r#"RefId="O-0" />"#, r#"RefId="O-0" Priority="Alert" />"#);
        let image = build(&xml, &[], vec![]).expect("builds");
        // Object 0's config octet: T, R, C as before, priority bits 10b.
        assert_eq!(octets(&image, "AS-4400")[5], 0x4E);
        assert_eq!(
            image.inferences,
            vec![Inference {
                rule: ALERT_IS_URGENT,
                detail: "O-0_R-1: Priority Alert is written as urgent (10b)".into(),
                reference: "RESEARCH §19.18",
            }]
        );
    }

    /// `[D]` *Resources* §4.18.6.2.4.1.3, NOTE 85: "Value Read on
    /// Initialisation" is a System B feature; the `070nh` descriptor has no
    /// bit for it. The image is the same, and the loss is said.
    #[test]
    fn read_on_init_has_no_bit_on_070n_and_is_disclosed() {
        let xml = PROGRAM.replacen(
            r#"ReadOnInitFlag="Disabled""#,
            r#"ReadOnInitFlag="Enabled""#,
            1,
        );
        let image = build(&xml, &[], vec![]).expect("builds");
        let plain = build(PROGRAM, &[], vec![]).expect("builds");
        assert_eq!(image.segments, plain.segments);
        assert_eq!(
            image.inferences,
            vec![Inference {
                rule: READ_ON_INIT_NOT_ON_070N,
                detail: "O-0_R-1: ReadOnInit is enabled, but a 070nh group object has no \
                         read-on-init bit; the device will not read the value after a reset"
                    .into(),
                reference: "RESEARCH §19.18",
            }]
        );
    }

    /// The project's own `ReadOnInitFlag` wins over the product's, both
    /// ways.
    #[test]
    fn the_project_read_on_init_decides_the_disclosure() {
        let (_dir, conn) = db(PROGRAM);
        let mut request = request(&[], vec![]);
        request.flag_overrides.insert(
            "O-0_R-1".into(),
            FlagOverrides {
                read_on_init: Some(true),
                ..FlagOverrides::default()
            },
        );
        let image = build_download_image(&conn, &request).expect("builds");
        assert_eq!(image.inferences.len(), 1, "{:?}", image.inferences);

        let xml = PROGRAM.replacen(
            r#"ReadOnInitFlag="Disabled""#,
            r#"ReadOnInitFlag="Enabled""#,
            1,
        );
        let (_dir, conn) = db(&xml);
        request.flag_overrides.insert(
            "O-0_R-1".into(),
            FlagOverrides {
                read_on_init: Some(false),
                ..FlagOverrides::default()
            },
        );
        let image = build_download_image(&conn, &request).expect("builds");
        assert!(image.inferences.is_empty(), "{:?}", image.inferences);
    }

    #[test]
    fn an_unreadable_read_on_init_is_refused() {
        let xml = PROGRAM.replacen(
            r#"ReadOnInitFlag="Disabled""#,
            r#"ReadOnInitFlag="Sometimes""#,
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

    #[test]
    fn captured_parameter_pipeline_decodes_text_and_signed_positive_without_hiding_raw() {
        use knx_core::commissioning::device_backup::{BackupRegion, DeviceBackup};
        use knx_core::commissioning::load_state::MaskVersion;
        use knx_core::commissioning::memory_download::{MemoryDownloadPlan, MemoryDownloadStep};
        let image = build(&kinds_program(LATIN_9), &[], vec![]).unwrap();
        let plan = MemoryDownloadPlan {
            mask: MaskVersion(0x0701),
            manufacturer: 1,
            steps: image
                .segments
                .iter()
                .map(|s| MemoryDownloadStep::WriteMemory {
                    address: s.address as u16,
                    octets: s.octets.clone(),
                })
                .collect(),
        };
        let mut held = DeviceBackup {
            target: device(),
            mask: plan.mask,
            manufacturer: 1,
            load_states: vec![],
            regions: image
                .segments
                .iter()
                .map(|s| BackupRegion {
                    address: s.address as u16,
                    octets: s.octets.clone(),
                })
                .collect(),
            prior_application:
                knx_core::commissioning::device_backup::PriorApplication::NotRecorded,
            exemplar_serial: knx_core::commissioning::device_backup::ExemplarEvidence::NotRecorded,
        };
        let device_values = held
            .regions
            .iter_mut()
            .find(|r| r.address == 0x4600)
            .unwrap();
        device_values.octets[..9]
            .copy_from_slice(&[3, 0x01, 0x91, b'5', b' ', 0xA4, b' ', b'o', b'k']);
        let rows = crate::image_comparison::decode(&image, &held, &plan);
        let text = rows
            .iter()
            .find(|r| r.kind == "parameter" && r.label.contains("P-T_R-1"))
            .unwrap();
        assert_eq!(text.label, "T · P-T_R-1 · bit 0 / 48");
        assert_eq!(text.project.as_deref(), Some("Grüß"));
        assert_eq!(text.device.as_deref(), Some("5 € ok"));
        let signed = rows
            .iter()
            .find(|r| r.kind == "parameter" && r.label.contains("P-S16_R-1"))
            .unwrap();
        assert_eq!(signed.project.as_deref(), Some("300"));
        assert_eq!(signed.device.as_deref(), Some("401"));
        assert!(rows.iter().any(|r| r.kind == "raw"
            && r.device
                .as_deref()
                .is_some_and(|s| s.contains("35 20 A4 20 6F 6B"))));
        let device_values = held
            .regions
            .iter_mut()
            .find(|r| r.address == 0x4600)
            .unwrap();
        device_values.octets[1] = 0x80;
        device_values.octets[4] = 0;
        device_values.octets[5] = b'x';
        let rows = crate::image_comparison::decode(&image, &held, &plan);
        assert_eq!(
            rows.iter()
                .find(|r| r.label.contains("P-S16_R-1"))
                .unwrap()
                .device,
            None,
            "negative sign interpretation unverified"
        );
        let text = rows.iter().find(|r| r.label.contains("P-T_R-1")).unwrap();
        assert_eq!(
            text.device, None,
            "nonzero bytes after NUL cannot be silently trimmed"
        );
        assert!(text.reason.is_some());
    }

    fn kinds_program(options: &str) -> String {
        kinds_program_with(options, "Grüß", "", "", "")
    }

    const LATIN_9: &str = r#"<Options TextParameterEncoding="iso-8859-15" />"#;

    /// A signed number at or above zero, high octet first; the text in the
    /// declared ISO-8859-15, zero-filled to its field.
    #[test]
    fn comparison_text_inverse_respects_declared_encoding_and_canonical_padding() {
        assert_eq!(
            decode_parameter_text(None, b"abc\0\0").as_deref(),
            Some("abc")
        );
        assert_eq!(decode_parameter_text(None, &[0xA4]).as_deref(), Some("¤"));
        assert_eq!(
            decode_parameter_text(None, &[0x48, 0x80, 0xFC, 0x9F]).as_deref(),
            Some("H€üŸ")
        );
        assert_eq!(
            decode_parameter_text(None, &[0x81]),
            None,
            "unassigned in Windows-1252"
        );
        assert_eq!(
            decode_parameter_text(Some("iso-8859-1"), &[0x81]).as_deref(),
            Some("\u{81}"),
            "ISO-8859-1 keeps its C1 controls"
        );
        assert_eq!(decode_parameter_text(Some("utf-8"), b"abc"), None);
        assert_eq!(
            decode_parameter_text(Some("ISO-8859-1"), &[0xA4, 0xBC, 0]).as_deref(),
            Some("¤¼")
        );
        let bytes = [0xA4, 0xA6, 0xA8, 0xB4, 0xB8, 0xBC, 0xBD, 0xBE];
        assert_eq!(
            decode_parameter_text(Some("iso-8859-15"), &bytes).as_deref(),
            Some("€ŠšŽžŒœŸ")
        );
        assert_eq!(decode_parameter_text(Some("iso-8859-15"), b"a\0x"), None);
        assert_eq!(decode_parameter_text(None, b""), Some(String::new()));
    }

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
    /// `070nh` programs declare. Without a declaration the text is written
    /// in Windows-1252, as ETS 6.4 wrote it, and says so when it is not
    /// ASCII; a character Windows-1252 lacks is refused.
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
        assert!(!image
            .inferences
            .iter()
            .any(|inference| inference.rule == UNDECLARED_TEXT_WINDOWS_1252));
        // `[V]` The observed ETS octets, euro sign included.
        let image = build(&undeclared, &[("P-T_R-1", "€ßÄö")], vec![]).unwrap();
        assert_eq!(
            &octets(&image, "AS-4600")[3..8],
            [0x80, 0xDF, 0xC4, 0xF6, 0x00]
        );
        assert!(image
            .inferences
            .iter()
            .any(|inference| inference.rule == UNDECLARED_TEXT_WINDOWS_1252));
        let error = build(&undeclared, &[("P-T_R-1", "Ω")], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Value { cause, .. } if cause.contains("Windows-1252")),
            "{error}"
        );
        let error = build(&undeclared, &[("P-T_R-1", "\u{81}")], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Value { .. }),
            "an unassigned position: {error}"
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

    /// ADR-0169: a `TypeFloat` value is written in its declared encoding,
    /// high octet first (`[V]` ETS 6.4 on a memory program); an encoding not
    /// observed, an undeclared one, a `LittleEndian` program and a value out
    /// of bounds or unparsable are refused. Property placements:
    /// `a_float_placed_in_a_property_is_refused`.
    #[test]
    fn a_float_parameter_is_written_in_its_encoding() {
        // The fixture's 12-octet segment holds a 48-bit text at 3; a single
        // needs 4 octets, so its variant narrows the text to one octet.
        let placed = |options: &str, encoding: &str, bounds: &str, offset: u32| {
            kinds_program_with(
                options,
                if offset == 10 { "Grüß" } else { "G" },
                &format!(
                    r#"<ParameterType Id="PT-F" Name="F"><TypeFloat {encoding} {bounds} /></ParameterType>"#
                ),
                &format!(
                    r#"<Parameter Id="P-F" Name="F" ParameterType="PT-F" Value="1.000000000000000E+000"><Memory CodeSegment="AS-4600" Offset="{offset}" BitOffset="0" /></Parameter>"#
                ),
                r#"<ParameterRef Id="P-F_R-1" RefId="P-F" />"#,
            )
            .replace(
                r#"<ParameterRefRef RefId="P-T_R-1" />"#,
                r#"<ParameterRefRef RefId="P-T_R-1" /><ParameterRefRef RefId="P-F_R-1" />"#,
            )
            .replace(
                r#"<TypeText SizeInBit="48" />"#,
                if offset == 10 {
                    r#"<TypeText SizeInBit="48" />"#
                } else {
                    r#"<TypeText SizeInBit="8" />"#
                },
            )
        };
        let program =
            |options: &str, encoding: &str, bounds: &str| placed(options, encoding, bounds, 10);
        let dpt9 = r#"Encoding="DPT 9""#;
        let single = r#"Encoding="IEEE-754 Single""#;
        let bounds = r#"minInclusive="-50" maxInclusive="600""#;
        // The product default, in its C# "E15" spelling: 100 hundredths.
        let image = build(&program(LATIN_9, dpt9, bounds), &[], vec![]).expect("DPT 9 builds");
        assert_eq!(&octets(&image, "AS-4600")[10..12], &[0x00, 0x64]);
        let image = build(
            &program(LATIN_9, dpt9, bounds),
            &[("P-F_R-1", "-12.5")],
            vec![],
        )
        .expect("a negative DPT 9 builds");
        assert_eq!(&octets(&image, "AS-4600")[10..12], &[0x83, 0x1E]);
        let attribution = image
            .parameter_fields
            .iter()
            .find(|field| field.reference == "P-F_R-1")
            .expect("attributed");
        assert_eq!(
            attribution.value,
            AttributedValue::Float {
                bits: 0x831E,
                encoding: float::FloatEncoding::Dpt9
            }
        );
        assert_eq!(attribution.field.size_in_bit, 16);
        let image = build(
            &placed(LATIN_9, single, "", 4),
            &[("P-F_R-1", "1.3"), ("P-T_R-1", "G")],
            vec![],
        )
        .expect("IEEE-754 Single builds");
        assert_eq!(&octets(&image, "AS-4600")[4..8], &[0x3F, 0xA6, 0x66, 0x66]);

        let refused = |xml: &str, values: &[(&str, &str)], needle: &str| {
            let error = build(xml, values, vec![]).unwrap_err();
            assert!(error.to_string().contains(needle), "{needle}: {error}");
        };
        refused(
            &program(LATIN_9, r#"Encoding="IEEE-754 Double""#, ""),
            &[],
            "TypeFloat encoding Some(\"IEEE-754 Double\")",
        );
        refused(&program(LATIN_9, "", ""), &[], "TypeFloat encoding None");
        refused(
            &program(r#"<Options ParameterByteOrder="LittleEndian" />"#, dpt9, ""),
            &[],
            "LittleEndian",
        );
        refused(
            &program(LATIN_9, dpt9, bounds),
            &[("P-F_R-1", "600.5")],
            "outside",
        );
        refused(
            &program(LATIN_9, dpt9, bounds),
            &[("P-F_R-1", "-50.5")],
            "outside",
        );
        refused(
            &program(LATIN_9, dpt9, bounds),
            &[("P-F_R-1", "5,0")],
            "not a decimal",
        );
        refused(
            &program(LATIN_9, dpt9, bounds),
            &[("P-F_R-1", "NaN")],
            "not a decimal",
        );
        refused(
            &program(LATIN_9, dpt9, r#"minInclusive="low""#),
            &[],
            "not a float bound",
        );
    }

    /// ADR-0162: a `TypeTime` value in a plain unit is the count of that
    /// unit, written like a number in the declared byte order (`[V]` ETS 6.4:
    /// 37230 in `HundredMilliseconds` as `6E 91` under `LittleEndian`); a
    /// packed, undeclared or unknown unit and a value out of bounds are refused.
    #[test]
    fn a_time_parameter_is_a_count_of_its_unit_in_the_declared_byte_order() {
        let program = |options: &str, unit: &str| {
            kinds_program_with(
                options,
                "Grüß",
                &format!(
                    r#"<ParameterType Id="PT-TT" Name="TT"><TypeTime SizeInBit="16" {unit} minInclusive="0" maxInclusive="63000" UIHint="Duration_hhmmss" /></ParameterType>"#
                ),
                r#"<Parameter Id="P-TT" Name="TT" ParameterType="PT-TT" Value="0"><Memory CodeSegment="AS-4600" Offset="9" BitOffset="0" /></Parameter>"#,
                r#"<ParameterRef Id="P-TT_R-1" RefId="P-TT" />"#,
            )
            .replace(
                r#"<ParameterRefRef RefId="P-T_R-1" />"#,
                r#"<ParameterRefRef RefId="P-T_R-1" /><ParameterRefRef RefId="P-TT_R-1" />"#,
            )
        };
        let hundred = r#"Unit="HundredMilliseconds""#;
        let little =
            r#"<Options TextParameterEncoding="iso-8859-15" ParameterByteOrder="LittleEndian" />"#;
        let image = build(&program(little, hundred), &[("P-TT_R-1", "37230")], vec![])
            .expect("a counted unit builds");
        assert_eq!(&octets(&image, "AS-4600")[9..11], &[0x6E, 0x91]);
        assert_eq!(image.parameters.get("P-TT_R-1"), Some(&37230));
        let time = image
            .inferences
            .iter()
            .find(|inference| inference.rule == TIME_AS_UNIT_COUNT)
            .expect("the time rule is named");
        assert!(
            time.detail.contains("1 in HundredMilliseconds"),
            "{}",
            time.detail
        );
        let big = build(&program(LATIN_9, hundred), &[("P-TT_R-1", "6010")], vec![]).unwrap();
        assert_eq!(&octets(&big, "AS-4600")[9..11], &[0x17, 0x7A]);

        for unit in [
            r#"Unit="PackedSecondsAndMilliseconds""#,
            r#"Unit="Fortnights""#,
            "",
        ] {
            let error = build(&program(little, unit), &[], vec![]).unwrap_err();
            assert!(
                matches!(&error, ImageError::Unsupported { what }
                    if what.starts_with("P-TT: TypeTime unit")),
                "{unit}: {error}"
            );
        }
        let error = build(&program(little, hundred), &[("P-TT_R-1", "63001")], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Value { cause, .. } if cause.contains("outside")),
            "{error}"
        );
        // A program without a time parameter names no time rule.
        let plain = build(&kinds_program(LATIN_9), &[], vec![]).unwrap();
        assert!(!plain
            .inferences
            .iter()
            .any(|inference| inference.rule == TIME_AS_UNIT_COUNT));
    }

    /// WP-08: a `TypeDate` value has a schema-documented spelling, but no
    /// source fixes how a declared `Encoding="DPT 11"` is laid out in a
    /// product's memory, so a placed date is refused by name, for its type.
    #[test]
    fn a_date_parameter_is_refused_for_its_type() {
        let xml = kinds_program_with(
            LATIN_9,
            "Grüß",
            r#"<ParameterType Id="PT-D" Name="D"><TypeDate Encoding="DPT 11" /></ParameterType>"#,
            r#"<Parameter Id="P-D" Name="D" ParameterType="PT-D" Value="2022-09-15"><Memory CodeSegment="AS-4600" Offset="9" BitOffset="0" /></Parameter>"#,
            r#"<ParameterRef Id="P-D_R-1" RefId="P-D" />"#,
        )
        .replace(
            r#"<ParameterRefRef RefId="P-T_R-1" />"#,
            r#"<ParameterRefRef RefId="P-T_R-1" /><ParameterRefRef RefId="P-D_R-1" />"#,
        );
        let error = build(&xml, &[], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Unsupported { what } if what.contains("P-D") && what.contains("\"Date\"")),
            "{error}"
        );
    }

    fn with_options(xml: &str, options: &str) -> String {
        xml.replace("<Code>", &format!("{options}<Code>"))
    }

    /// ADR-0161: under `DownloadInvisibleParameters="Background"` every parameter
    /// placed in a property is imaged, a visible one with its value, an invisible
    /// one (`P-4`, in no `Dynamic` branch) with its default, numbers low octet
    /// first; the octet no parameter reaches is not owned.
    #[test]
    fn property_parameters_are_imaged_with_values_defaults_and_an_ownership_mask() {
        let program = with_options(
            PROGRAM,
            r#"<Options ParameterByteOrder="LittleEndian" DownloadInvisibleParameters="Background" />"#,
        )
        .replace(
            "<Union SizeInBit=\"16\">",
            r#"<Parameter Id="P-4" Name="Hidden" ParameterType="PT-W" Value="255"><Property ObjectIndex="1" PropertyId="2" Offset="2" BitOffset="0" /></Parameter>
  <Union SizeInBit="16">"#,
        )
        .replace(
            r#"<ParameterRef Id="P-3_R-6" RefId="P-3" />"#,
            r#"<ParameterRef Id="P-3_R-6" RefId="P-3" /><ParameterRef Id="P-4_R-7" RefId="P-4" />"#,
        );
        let property = |image: &DownloadImage| {
            let [property] = image.properties.as_slice() else {
                panic!("{:?}", image.properties);
            };
            assert_eq!((property.object_index, property.property_id), (1, 2));
            assert_eq!(property.owned, vec![0xFF, 0x00, 0xFF, 0xFF]);
            property.octets.clone()
        };
        let visible = build(&program, &[("P-1_R-1", "0"), ("P-3_R-6", "5")], vec![]).unwrap();
        assert_eq!(property(&visible), vec![0x05, 0x00, 0xFF, 0x00]);
        assert_eq!(visible.parameters.get("P-3_R-6"), Some(&5));
        let rules: Vec<_> = visible.inferences.iter().map(|i| i.rule).collect();
        assert!(
            rules.contains(&property::PROPERTY_PARAMETERS_MERGED),
            "{rules:?}"
        );
        assert!(rules.contains(&property::INVISIBLE_PROPERTY_PARAMETERS_DEFAULT));
        // P-3 invisible: its default, 1, whatever the project holds for it.
        let hidden = build(&program, &[("P-1_R-1", "1"), ("P-3_R-6", "5")], vec![]).unwrap();
        assert_eq!(property(&hidden), vec![0x01, 0x00, 0xFF, 0x00]);

        // Two refs of an invisible parameter that disagree on its default.
        let disagreeing = program.replace(
            r#"<ParameterRef Id="P-4_R-7" RefId="P-4" />"#,
            r#"<ParameterRef Id="P-4_R-7" RefId="P-4" /><ParameterRef Id="P-4_R-8" RefId="P-4" Value="2" />"#,
        );
        let error = build(&disagreeing, &[("P-1_R-1", "0")], vec![]).unwrap_err();
        assert!(error.to_string().contains("different defaults"), "{error}");
        // Without the observed policy an invisible one is still not written.
        let unknown = PROGRAM.replace(
            "<Code>",
            r#"<Options DownloadInvisibleParameters="DefaultValue" /><Code>"#,
        );
        assert!(build(&unknown, &[], vec![]).unwrap().properties.is_empty());
    }

    /// ADR-0171: under `Background`, an invisible memory parameter an active
    /// `Assign` sets is written with the assigned value, whatever the project
    /// held; disagreeing assignments refuse; an `Assign` in an inactive branch,
    /// or without that policy, writes nothing; a visible target keeps its own
    /// value.
    #[test]
    fn an_invisible_parameter_an_active_assign_sets_is_written() {
        let assign = |test: &str| {
            with_options(PROGRAM, r#"<Options DownloadInvisibleParameters="Background" />"#)
                .replace(
                    r#"<when test="0"><ParameterRefRef RefId="P-3_R-6" /></when>"#,
                    &format!(
                        r#"<when test="0"><ParameterRefRef RefId="P-3_R-6" /></when><when test="{test}"><Assign TargetParamRefRef="P-2_R-2" Value="42" /></when>"#
                    ),
                )
        };
        let program = assign("2");
        assert!(
            program.contains(r#"<when test="2"><Assign TargetParamRefRef="P-2_R-2" Value="42" />"#)
        );
        let time = |image: &DownloadImage| octets(image, "AS-4400")[12];
        // P-1 = 2 hides P-2; the active Assign set it and the project holds 42.
        let image = build(&program, &[("P-1_R-1", "2"), ("P-2_R-2", "42")], vec![]).unwrap();
        assert_eq!(time(&image), 42);
        assert!(image
            .parameter_fields
            .iter()
            .any(|f| f.reference == "P-2_R-2"));
        // The project still holds the default: ETS runs the assignment anyway.
        let image = build(&program, &[("P-1_R-1", "2")], vec![]).unwrap();
        assert_eq!(time(&image), 42);
        // Two active assignments that disagree: refused, not picked.
        let twice = program.replace(
            r#"<Assign TargetParamRefRef="P-2_R-2" Value="42" />"#,
            r#"<Assign TargetParamRefRef="P-2_R-2" Value="42" /><Assign TargetParamRefRef="P-2_R-2" Value="43" />"#,
        );
        assert_ne!(twice, program);
        let error = build(&twice, &[("P-1_R-1", "2")], vec![]).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("P-2_R-2: active Assign elements set"),
            "{error}"
        );
        // A value the type does not allow is refused as such.
        let illegal = program.replace(r#"Value="42" />"#, r#"Value="101" />"#);
        assert!(build(&illegal, &[("P-1_R-1", "2")], vec![]).is_err());
        // The Assign sits in a branch the value does not select.
        let inactive = assign("3");
        let image = build(&inactive, &[("P-1_R-1", "2"), ("P-2_R-2", "42")], vec![]).unwrap();
        assert_eq!(time(&image), 0, "template");
        // Without the observed policy nothing is written.
        let plain = program.replace(
            r#"<Options DownloadInvisibleParameters="Background" />"#,
            "",
        );
        assert_ne!(plain, program);
        let image = build(&plain, &[("P-1_R-1", "2"), ("P-2_R-2", "42")], vec![]).unwrap();
        assert_eq!(time(&image), 0, "template");
        // A visible target is written with its own value as before.
        let image = build(&program, &[("P-1_R-1", "1"), ("P-2_R-2", "7")], vec![]).unwrap();
        assert_eq!(time(&image), 7);
    }

    /// ADR-0171's boundaries, each with an active `Assign` (P-1 = 2): a
    /// visible target keeps its own value; a visible sibling reference that
    /// differs, a target that steers a consulted `choose` or another `Assign`
    /// with another stored value, and an unreadable `Assign` refuse; a `choose`
    /// the evaluation never reached steers nothing; union members and property
    /// placements are not written by an `Assign`.
    #[test]
    fn assigned_invisible_parameters_never_contradict_the_image() {
        let base = with_options(PROGRAM, r#"<Options DownloadInvisibleParameters="Background" />"#)
            .replace(
                r#"<Parameter Id="P-3" Name="Prop""#,
                r#"<Parameter Id="P-4" Name="Steer" ParameterType="PT-N" Value="50"><Memory CodeSegment="AS-4400" Offset="15" BitOffset="0" /></Parameter>
  <Parameter Id="P-3" Name="Prop""#,
            )
            .replace(
                r#"<ParameterRef Id="P-3_R-6" RefId="P-3" />"#,
                r#"<ParameterRef Id="P-3_R-6" RefId="P-3" /><ParameterRef Id="P-4_R-8" RefId="P-4" /><ParameterRef Id="P-2_R-9" RefId="P-2" />"#,
            );
        assert!(base.contains(r#"<ParameterRef Id="P-4_R-8" RefId="P-4" />"#));
        let with = |inside: &str| {
            let xml = base.replace(
                r#"<when test="2"><ParameterRefRef RefId="UP-2_R-4" />"#,
                &format!(r#"<when test="2">{inside}<ParameterRefRef RefId="UP-2_R-4" />"#),
            );
            assert_ne!(xml, base, "{inside}");
            xml
        };
        let selected = [("P-1_R-1", "2")];
        let plain = build(&base, &selected, vec![]).unwrap();

        // A visible target keeps its own value.
        let visible = with(
            r#"<ParameterRefRef RefId="P-2_R-2" /><Assign TargetParamRefRef="P-2_R-2" Value="42" />"#,
        );
        let image = build(&visible, &[("P-1_R-1", "2"), ("P-2_R-2", "7")], vec![]).unwrap();
        assert_eq!(octets(&image, "AS-4400")[12], 7);

        // A visible sibling reference of the same parameter that differs.
        let sibling = with(
            r#"<ParameterRefRef RefId="P-2_R-2" /><Assign TargetParamRefRef="P-2_R-9" Value="42" />"#,
        );
        let error = build(&sibling, &[("P-1_R-1", "2"), ("P-2_R-2", "7")], vec![]).unwrap_err();
        assert!(
            error.to_string().contains("the visible P-2_R-2 differs"),
            "{error}"
        );

        // A union member and a property placement are not written by an Assign.
        let union = with(r#"<Assign TargetParamRefRef="UP-1_R-3" Value="0" />"#);
        let image = build(&union, &selected, vec![]).unwrap();
        assert_eq!(octets(&image, "AS-4400"), octets(&plain, "AS-4400"));
        let property = with(r#"<Assign TargetParamRefRef="P-3_R-6" Value="5" />"#);
        let image = build(&property, &selected, vec![]).unwrap();
        assert_eq!(image.properties, plain.properties);

        // A target that steers a `choose`: the evaluation read the stored value.
        let steering = with(
            r#"<Assign TargetParamRefRef="P-4_R-8" Value="60" /><choose ParamRefId="P-4_R-8"><when test="60"><ComObjectRefRef RefId="O-0_R-1" /></when></choose>"#,
        );
        let error = build(&steering, &selected, vec![]).unwrap_err();
        assert!(
            error.to_string().contains("P-4_R-8: an active Assign sets"),
            "{error}"
        );
        let image = build(&steering, &[("P-1_R-1", "2"), ("P-4_R-8", "60")], vec![]).unwrap();
        assert_eq!(octets(&image, "AS-4400")[15], 60);
        // The same `choose` in a branch the evaluation never reached steers nothing.
        let unreached = with(r#"<Assign TargetParamRefRef="P-4_R-8" Value="60" />"#).replace(
            r#"<when test="0"><ParameterRefRef RefId="P-3_R-6" /></when>"#,
            r#"<when test="0"><ParameterRefRef RefId="P-3_R-6" /><choose ParamRefId="P-4_R-8"><when test="60"><ComObjectRefRef RefId="O-0_R-1" /></when></choose></when>"#,
        );
        assert!(unreached.contains(r#"<choose ParamRefId="P-4_R-8">"#));
        let image = build(&unreached, &selected, vec![]).unwrap();
        assert_eq!(octets(&image, "AS-4400")[15], 60);

        // A chain: the source of another active Assign holds another value.
        let chain = with(
            r#"<Assign TargetParamRefRef="P-4_R-8" Value="60" /><Assign SourceParamRefRef="P-4_R-8" TargetParamRefRef="P-2_R-2" />"#,
        );
        let error = build(&chain, &selected, vec![]).unwrap_err();
        assert!(
            error.to_string().contains("P-4_R-8: an active Assign sets"),
            "{error}"
        );
        let image = build(&chain, &[("P-1_R-1", "2"), ("P-4_R-8", "60")], vec![]).unwrap();
        assert_eq!(
            octets(&image, "AS-4400")[12],
            60,
            "P-2 from the chained source"
        );
        assert_eq!(octets(&image, "AS-4400")[15], 60);

        // An unreadable Assign: its target is unknown.
        let unreadable =
            with(r#"<Assign TargetParamRefRef="P-2_R-2" SourceParamRefRef="P-4_R-8" Value="1" />"#);
        let error = build(&unreadable, &selected, vec![]).unwrap_err();
        assert!(error.to_string().contains("could not be read"), "{error}");
    }

    /// An Assign-only module can reach a global reference without activating scoped refs.
    #[test]
    fn a_scoped_assign_cannot_write_a_global_memory_parameter() {
        let program = with_options(
            PROGRAM,
            r#"<Options DownloadInvisibleParameters="Background" />"#,
        )
        .replace(
            "</Static>",
            r#"</Static><ModuleDefs><ModuleDef Id="MD-1" Name="assign-only"><Dynamic><Assign TargetParamRefRef="P-2_R-2" Value="42" /></Dynamic></ModuleDef></ModuleDefs>"#,
        )
        .replace(
            r#"<when test="2"><ParameterRefRef RefId="UP-2_R-4" />"#,
            r#"<when test="2"><Module Id="MD-1_M-1" RefId="MD-1" /><ParameterRefRef RefId="UP-2_R-4" />"#,
        );
        assert!(program.contains(r#"<Module Id="MD-1_M-1" RefId="MD-1" />"#));
        assert!(program.contains(r#"<ParameterRef Id="P-2_R-2" RefId="P-2" />"#));
        let image = build(&program, &[("P-1_R-1", "2")], vec![]).unwrap();
        assert_eq!(
            octets(&image, "AS-4400")[12],
            0,
            "a scoped Assign must not replace the global parameter's template byte"
        );
        assert!(!image.parameters.contains_key("P-2_R-2"));

        // Prove admission reaches the scope guard rather than a scoped-ref refusal.
        let (_dir, conn) = db(&program);
        let values = resolve_values(
            &conn,
            ID,
            &[("P-1_R-1".to_string(), "2".to_string())]
                .into_iter()
                .collect(),
        )
        .unwrap();
        let activation = evaluate(&load_program_trees(&conn, ID).unwrap(), &values);
        assert!(
            activation.diagnostics.is_empty(),
            "{:?}",
            activation.diagnostics
        );
        assert_eq!(activation.assigned.len(), 1);
        assert!(activation.assigned[0].scope.is_some());
        assert_eq!(
            activation.assigned[0].assignment.as_ref().unwrap().target,
            "P-2_R-2"
        );
        assert!(activation.parameter_refs.iter().all(|r| r.scope.is_none()));
        assert!(activation.com_object_refs.iter().all(|r| r.scope.is_none()));
    }

    /// Unknown targets and absent sources refuse even without any choose-controller candidates.
    #[test]
    fn an_assign_with_an_unknown_target_or_missing_source_is_refused() {
        let base = with_options(
            PROGRAM,
            r#"<Options DownloadInvisibleParameters="Background" />"#,
        );
        let start = base.find("<Dynamic>").unwrap();
        let end = base.find("</Dynamic>").unwrap() + "</Dynamic>".len();
        for (assignment, reason) in [
            (
                r#"<Assign TargetParamRefRef="P-999_R-999" Value="42" />"#,
                "an active Assign targets an unknown parameter",
            ),
            (
                r#"<Assign SourceParamRefRef="P-999_R-999" TargetParamRefRef="P-2_R-2" />"#,
                "an active Assign's source has no value",
            ),
        ] {
            let program = format!(
                "{}<Dynamic>{assignment}</Dynamic>{}",
                &base[..start],
                &base[end..]
            );
            assert!(program.contains(assignment));
            assert!(!program.contains("<choose"));
            let error = build(&program, &[], vec![]).unwrap_err();
            assert!(error.to_string().contains(reason), "{error}");
        }
    }

    /// An earlier pass's consultation must not refuse the final, consistent image.
    #[test]
    fn an_assign_only_checks_controllers_consulted_in_the_final_pass() {
        let program = with_options(
            PROGRAM,
            r#"<Options DownloadInvisibleParameters="Background" />"#,
        )
        .replace(
            r#"<Parameter Id="P-3" Name="Prop""#,
            r#"<Parameter Id="P-4" Name="Steer" ParameterType="PT-N" Value="50"><Memory CodeSegment="AS-4400" Offset="15" BitOffset="0" /></Parameter><Parameter Id="P-3" Name="Prop""#,
        )
        .replace(
            r#"<ParameterRef Id="P-3_R-6" RefId="P-3" />"#,
            r#"<ParameterRef Id="P-3_R-6" RefId="P-3" /><ParameterRef Id="P-4_R-8" RefId="P-4" />"#,
        )
        .replace(
            r#"<when test="2"><ParameterRefRef RefId="UP-2_R-4" />"#,
            r#"<when test="2"><Assign TargetParamRefRef="P-4_R-8" Value="60" /><choose ParamRefId="P-2_R-2"><when test="50"><choose ParamRefId="P-4_R-8"><when test="50"/><when test="60"><ComObjectRefRef RefId="O-0_R-1" /></when></choose></when></choose><ParameterRefRef RefId="UP-2_R-4" />"#,
        );
        assert!(program.contains(r#"<Parameter Id="P-4" Name="Steer""#));
        assert!(program.contains(r#"<ParameterRef Id="P-4_R-8" RefId="P-4" />"#));
        assert!(program.contains(r#"<choose ParamRefId="P-4_R-8">"#));
        // P-2 is placed but not active, so the final pass marks its choose inactive;
        // the nested P-4 choose was consulted earlier but no longer steers this image.
        let image = build(&program, &[("P-1_R-1", "2")], vec![])
            .expect("an earlier-pass consultation must not refuse the final-pass image");
        assert_eq!(octets(&image, "AS-4400")[15], 60);
        assert!(image.objects.iter().all(|object| object.number != 0));
    }

    /// ADR-0169: no float placed in a property was observed, so it refuses,
    /// visible or written as an invisible default.
    #[test]
    fn a_float_placed_in_a_property_is_refused() {
        let program = with_options(
            PROGRAM,
            r#"<Options DownloadInvisibleParameters="Background" />"#,
        )
        .replace(
            "</ParameterTypes>",
            r#"<ParameterType Id="PT-F" Name="F"><TypeFloat Encoding="DPT 9" /></ParameterType></ParameterTypes>"#,
        )
        .replace(
            r#"<Parameter Id="P-3" Name="Prop" ParameterType="PT-N" Value="1">"#,
            r#"<Parameter Id="P-3" Name="Prop" ParameterType="PT-F" Value="1.5">"#,
        );
        assert!(program.contains(r#"ParameterType="PT-F" Value="1.5""#));
        for values in [
            &[("P-1_R-1", "0"), ("P-3_R-6", "2.5")][..],
            &[("P-1_R-1", "1")],
        ] {
            let error = build(&program, values, vec![]).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("P-3: a float placed in a property"),
                "{values:?}: {error}"
            );
        }
    }

    /// KL-170: a `LittleEndian` memory program writes whole-octet numbers low
    /// octet first, as an ETS download image of such a program holds them;
    /// one-octet values and texts are the same either way.
    #[test]
    fn a_little_endian_program_writes_whole_octet_numbers_low_octet_first() {
        let order = |order: &str| {
            kinds_program(&format!(
                r#"<Options TextParameterEncoding="iso-8859-15" ParameterByteOrder="{order}" />"#
            ))
        };
        let little = build(&order("LittleEndian"), &[], vec![]).expect("little endian");
        let big = build(&order("BigEndian"), &[], vec![]).expect("big endian");
        // P-S8 = 2 at octet 0, P-S16 = 300 at octets 1-2, the text from octet 3.
        assert_eq!(&octets(&big, "AS-4600")[..3], &[0x02, 0x01, 0x2C]);
        assert_eq!(&octets(&little, "AS-4600")[..3], &[0x02, 0x2C, 0x01]);
        assert_eq!(
            octets(&little, "AS-4600")[3..],
            octets(&big, "AS-4600")[3..]
        );
        // The logical values are the same; only their attribution decodes reversed.
        assert_eq!(little.parameters, big.parameters);
        let attributed = |image: &DownloadImage, reference: &str| {
            image
                .parameter_fields
                .iter()
                .find(|field| field.reference == reference)
                .map(|field| field.value.clone())
        };
        assert_eq!(
            attributed(&little, "P-S16_R-1"),
            Some(AttributedValue::Number {
                value: 300,
                nonnegative_signed: true,
                low_octet_first: true
            })
        );
        assert!(matches!(
            attributed(&little, "P-S8_R-1"),
            Some(AttributedValue::Number {
                low_octet_first: false,
                ..
            })
        ));
        // The 16-bit union member at AS-4400 octet 13, default 255.
        let little = build(
            &with_options(PROGRAM, r#"<Options ParameterByteOrder="LittleEndian" />"#),
            &[],
            vec![],
        )
        .expect("little endian");
        let big = build(PROGRAM, &[], vec![]).expect("undeclared is big endian");
        assert_eq!(&octets(&big, "AS-4400")[13..15], &[0x00, 0xFF]);
        assert_eq!(&octets(&little, "AS-4400")[13..15], &[0xFF, 0x00]);
    }

    /// A field crossing an octet boundary without being whole octets has no
    /// observed little-endian placement, so it refuses the image by name.
    #[test]
    fn a_little_endian_field_across_an_octet_boundary_is_refused() {
        // Mode 2 activates the 2-bit union member, moved to straddle octets 14-15.
        let xml = with_options(PROGRAM, r#"<Options ParameterByteOrder="LittleEndian" />"#)
            .replace(
                r#"Offset="1" BitOffset="5" Value="2""#,
                r#"Offset="1" BitOffset="7" Value="2""#,
            );
        let error = build(&xml, &[("P-1_R-1", "2")], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::Unsupported { what }
                if what.starts_with("UP-2_R-4: ParameterByteOrder \"LittleEndian\" field at bit offset 7")),
            "{error}"
        );
        let big = PROGRAM.replace(
            r#"Offset="1" BitOffset="5" Value="2""#,
            r#"Offset="1" BitOffset="7" Value="2""#,
        );
        build(&big, &[("P-1_R-1", "2")], vec![]).expect("big endian has a placement");
    }

    /// Any other declared order, and a little-endian legacy layout, keep the
    /// typed refusal (issue #10); its text names what is written.
    #[test]
    fn another_declared_byte_order_is_refused() {
        let xml = kinds_program(r#"<Options ParameterByteOrder="MiddleEndian" />"#);
        let error = build(&xml, &[], vec![]).unwrap_err();
        assert!(
            matches!(&error, ImageError::UnsupportedByteOrder { order } if order == "MiddleEndian"),
            "{error}"
        );
        assert_eq!(
            error.to_string(),
            "not supported for a download: ParameterByteOrder \"MiddleEndian\": only BigEndian, \
             and LittleEndian in a memory (070nh) layout, are written"
        );
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
