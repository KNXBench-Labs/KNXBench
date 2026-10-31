//! Inspects immutable manufacturer distributions without installing or granting write authority.
//!
//! Declared identities and source fallbacks are not registration or initialization evidence.

#[path = "distribution_bounds.rs"]
mod bounds;
#[path = "distribution_links.rs"]
mod links;
#[path = "distribution_source.rs"]
mod provenance;
#[path = "distribution_readers.rs"]
mod readers;
#[path = "distribution_xml.rs"]
mod xml;
#[path = "distribution_zip.rs"]
mod zip;
pub use provenance::{DistributionLocation, DistributionSourceName, DistributionSourceRef};

use serde::{Deserialize, Serialize};

/// A separate source-inspection report, never an installed product-package result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionInspection {
    pub format_version: u32,
    pub source_kind: DistributionSourceKind,
    pub source: DistributionSource,
    pub documents: Vec<DistributionDocument>,
    pub sections: Vec<DistributionSection>,
    pub catalog_items: Vec<DistributionCatalogItem>,
    pub hardware: Vec<DistributionHardware>,
    pub applications: Vec<DistributionApplication>,
    pub limitations: Vec<DistributionLimitation>,
    pub reader_passes: Vec<DistributionReaderPass>,
    pub unknowns: Vec<DistributionUnknown>,
    pub references: Vec<DistributionReference>,
    pub identities: Vec<DistributionIdentity>,
    pub diagnostics: Vec<DistributionDiagnostic>,
    pub members: Vec<DistributionMember>,
    pub complete: bool,
    pub declared_metadata: Vec<DistributionDeclaredMetadata>,
    pub preserved_content: Vec<DistributionContent>,
}

/// Values asserted by the source, never verified registration, version or producer identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionDeclaredMetadata {
    pub attribute: String,
    pub value: String,
    pub source: DistributionSourceRef,
}

/// Non-whitespace character data remains hash-bound to exact original bytes, never executed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionContent {
    pub source: DistributionSourceRef,
    pub sha256: String,
    pub kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionSourceKind {
    ManufacturerXml,
    DistributionZip,
}

/// Identity of the exact caller-supplied bytes, not a derived parser wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionSource {
    pub name: String,
    pub sha256: String,
    pub size: u64,
}

/// Namespace recognition is not XSD, master-data or manufacturer-authenticity validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionDocument {
    pub member_ordinal: Option<u32>,
    pub namespace: String,
    pub sha256: String,
    pub size: u64,
    pub names: Vec<DistributionSourceName>,
    pub locations: Vec<DistributionLocation>,
}

/// Byte offsets are half-open ranges in the immutable original XML member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionCoordinate {
    pub member_ordinal: Option<u32>,
    pub element_ordinal: u64,
    pub start_byte: u64,
    pub end_byte: u64,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionSectionKind {
    Catalog,
    ApplicationPrograms,
    Baggages,
    Hardware,
}

/// A state describes only the named pass, never global compatibility or authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionPassState {
    Parsed,
    Preserved,
    PartiallyEvaluated,
    NotAttempted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionSection {
    pub kind: DistributionSectionKind,
    pub state: DistributionPassState,
    pub declared_manufacturer_id: Option<String>,
    pub source: DistributionSourceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionCatalogItem {
    pub id: Option<String>,
    pub name: Option<String>,
    pub product_ref_id: Option<String>,
    pub hardware2program_ref_id: Option<String>,
    pub source: DistributionSourceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionHardware {
    pub id: Option<String>,
    pub name: Option<String>,
    pub product_ids: Vec<String>,
    pub program_links: Vec<DistributionProgramLink>,
    pub source: DistributionSourceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionProgramLink {
    pub id: Option<String>,
    pub application_program_ref_ids: Vec<String>,
    pub source: DistributionSourceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionApplication {
    pub id: Option<String>,
    pub name: Option<String>,
    pub declared_manufacturer_id: Option<String>,
    pub source: DistributionSourceRef,
    pub parameter_fallbacks: Vec<DistributionParameterFallback>,
    pub dynamic_state: DistributionPassState,
    pub active_parameter_ref_ids: Vec<String>,
    pub active_parameter_refs: Vec<DistributionActiveRef>,
}

/// Evaluated-subset identity is app/view-bound; scope node IDs are not original XML ordinals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionActiveRef {
    pub ref_id: String,
    pub module_scope: Vec<DistributionModuleScope>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionModuleScope {
    pub module_node: i64,
    pub module_id: Option<String>,
    pub module_def_id: String,
}

/// An uncomputed source fallback, not an initialized or manufacturer-validated value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionParameterFallback {
    pub parameter_ref_id: String,
    pub parameter_id: String,
    pub value: Option<String>,
    pub origin: DistributionFallbackOrigin,
    pub source: DistributionSourceRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionFallbackOrigin {
    ParameterRef,
    Parameter,
    /// The target Parameter resolved uniquely and declares no Value.
    Absent,
    /// No Parameter with the target identity exists in the original scope.
    DanglingParameter,
    /// Several Parameters share the target identity; none is chosen.
    AmbiguousParameter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionLimitation {
    MissingMaster,
    MasterNotInspected,
    XsdNotPerformed,
    AuthenticityNotVerified,
    WriteAuthorityNotPerformed,
    InitializationNotPerformed,
    MasterDependenciesNotResolved,
    TranslationsNotPerformed,
    BaggageAssociationNotPerformed,
    RegistrationNotVerified,
}

/// Identities and references are lookup keys, not payload; longer values are refused.
const MAX_IDENTITY_BYTES: usize = 4096;
/// A caller-supplied source label is echoed into the report and bounded like an identity.
const MAX_SOURCE_NAME_BYTES: usize = 4096;
/// Evaluator diagnostics are Debug text; longer details are cut with an explicit marker.
const MAX_DIAGNOSTIC_DETAIL_BYTES: usize = 4096;

/// Largest admitted element count of one XML member (largest measured OpenKNX member: 488,283).
const MAX_ELEMENTS: usize = 500_000;

/// Resource ceilings are inclusive. They may be lowered, never raised beyond the default profile.
#[derive(Debug, Clone, Copy)]
pub struct DistributionLimits {
    pub max_input_bytes: usize,
    pub max_member_bytes: u64,
    pub max_expanded_bytes: u64,
    pub max_elements: usize,
    pub max_depth: usize,
    pub max_attributes_per_element: usize,
    pub max_text_bytes: usize,
    pub max_references: usize,
    pub max_output_records: usize,
    pub max_derived_bytes: usize,
    pub max_scopes: usize,
    /// Exact compact JSON byte ceiling, independent of XML retention and Dynamic work.
    pub max_report_bytes: usize,
    /// Retained XML capacities and owned payloads; excludes input bytes and allocator overhead.
    pub max_xml_working_bytes: usize,
    /// Conservative Dynamic scalar-copy/work admission, independent of report serialization.
    pub max_dynamic_work_bytes: usize,
}
impl DistributionLimits {
    fn bounded(self) -> Self {
        let standard = Self::default();
        Self {
            max_input_bytes: self.max_input_bytes.min(standard.max_input_bytes),
            max_member_bytes: self.max_member_bytes.min(standard.max_member_bytes),
            max_expanded_bytes: self.max_expanded_bytes.min(standard.max_expanded_bytes),
            max_elements: self.max_elements.min(standard.max_elements),
            max_depth: self.max_depth.min(standard.max_depth),
            max_attributes_per_element: self
                .max_attributes_per_element
                .min(standard.max_attributes_per_element),
            max_text_bytes: self.max_text_bytes.min(standard.max_text_bytes),
            max_references: self.max_references.min(standard.max_references),
            max_output_records: self.max_output_records.min(standard.max_output_records),
            max_derived_bytes: self.max_derived_bytes.min(standard.max_derived_bytes),
            max_scopes: self.max_scopes.min(standard.max_scopes),
            max_report_bytes: self.max_report_bytes.min(standard.max_report_bytes),
            max_xml_working_bytes: self
                .max_xml_working_bytes
                .min(standard.max_xml_working_bytes),
            max_dynamic_work_bytes: self
                .max_dynamic_work_bytes
                .min(standard.max_dynamic_work_bytes),
        }
    }
}

impl Default for DistributionLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 256 * 1024 * 1024,
            max_member_bytes: 128 * 1024 * 1024,
            max_expanded_bytes: 512 * 1024 * 1024,
            max_elements: MAX_ELEMENTS,
            max_depth: 128,
            max_attributes_per_element: 128,
            max_text_bytes: 64 * 1024 * 1024,
            // Row ceilings are structural defence in depth scaled to the element ceiling; the
            // exact incremental serialized-byte ceiling below is the memory guard for the report.
            max_references: 4 * MAX_ELEMENTS,
            max_output_records: 8 * MAX_ELEMENTS,
            max_derived_bytes: 128 * 1024 * 1024,
            max_scopes: 4096,
            max_report_bytes: 256 * 1024 * 1024,
            max_xml_working_bytes: 256 * 1024 * 1024,
            max_dynamic_work_bytes: 256 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionReaderPass {
    pub reader: String,
    pub state: DistributionPassState,
    pub source: DistributionSourceRef,
    pub original_sha256: String,
    pub derived_sha256: String,
    pub derived_size: u64,
    pub transformation: String,
    pub diagnostics: Vec<String>,
}

/// Each original occurrence appears once, independently of the number of readers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionUnknown {
    pub kind: String,
    pub expanded_name: String,
    pub raw_name: String,
    pub source: DistributionSourceRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionReferenceState {
    Resolved,
    Dangling,
    Ambiguous,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionReference {
    pub attribute: String,
    pub declared_target: String,
    pub source: DistributionSourceRef,
    pub state: DistributionReferenceState,
    pub candidates: Vec<DistributionSourceRef>,
    pub target_kind: Option<String>,
    pub manufacturer_source: DistributionSourceRef,
    pub resolution_basis: DistributionResolutionBasis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionResolutionBasis {
    OriginalScope,
    /// A ModuleDef reference that only the enclosing application's scope declares.
    EnclosingApplicationScope,
    DeclaredManufacturerAcrossMembers,
    ExternalUnperformed,
}

/// Source declarations remain occurrence-distinct, even when a typed reader would deduplicate IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionIdentity {
    pub kind: String,
    pub declared_id: String,
    pub source: DistributionSourceRef,
    pub manufacturer_source: DistributionSourceRef,
    pub application_scope: Option<DistributionSourceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionDiagnostic {
    pub code: String,
    pub source: Option<DistributionSourceRef>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionMember {
    pub ordinal: u32,
    pub raw_name: Vec<u8>,
    pub decoded_name: String,
    pub logical_path: String,
    pub compressed_size: u64,
    pub size: u64,
    pub sha256: String,
    pub state: DistributionPassState,
    pub error: Option<DistributionInspectionError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", content = "detail", rename_all = "camelCase")]
pub enum DistributionInspectionError {
    MalformedXml(String),
    Namespace(String),
    UnsupportedSchema(String),
    UnrecognizedSource,
    DtdForbidden,
    EncodingConflict,
    UnsupportedEncoding(String),
    ResourceLimit(String),
    Archive(String),
    Reader(String),
}
impl std::fmt::Display for DistributionInspectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DistributionInspectionError {}

type Result<T> = std::result::Result<T, DistributionInspectionError>;

pub(super) fn limit(name: &str) -> DistributionInspectionError {
    DistributionInspectionError::ResourceLimit(name.into())
}

impl DistributionInspection {
    fn empty(name: &str, bytes: &[u8], kind: DistributionSourceKind) -> Self {
        Self {
            format_version: 2,
            source_kind: kind,
            source: DistributionSource {
                name: name.into(),
                sha256: crate::sha256_hex(bytes),
                size: bytes.len() as u64,
            },
            documents: Vec::new(),
            sections: Vec::new(),
            catalog_items: Vec::new(),
            hardware: Vec::new(),
            applications: Vec::new(),
            reader_passes: Vec::new(),
            unknowns: Vec::new(),
            references: Vec::new(),
            identities: Vec::new(),
            diagnostics: Vec::new(),
            members: Vec::new(),
            // This is deliberately not an all-pass/package-completeness assertion.
            complete: false,
            declared_metadata: Vec::new(),
            preserved_content: Vec::new(),

            limitations: vec![
                DistributionLimitation::MissingMaster,
                DistributionLimitation::XsdNotPerformed,
                DistributionLimitation::AuthenticityNotVerified,
                DistributionLimitation::WriteAuthorityNotPerformed,
                DistributionLimitation::InitializationNotPerformed,
                DistributionLimitation::MasterDependenciesNotResolved,
                DistributionLimitation::TranslationsNotPerformed,
                DistributionLimitation::BaggageAssociationNotPerformed,
                DistributionLimitation::RegistrationNotVerified,
            ],
        }
    }
}

/// Inspects only caller-supplied bytes. Never opens an installed or default product database.
pub fn inspect_distribution(source_name: &str, bytes: &[u8]) -> Result<DistributionInspection> {
    inspect_distribution_with_limits(source_name, bytes, DistributionLimits::default())
}

pub fn inspect_distribution_with_limits(
    source_name: &str,
    bytes: &[u8],
    limits: DistributionLimits,
) -> Result<DistributionInspection> {
    let limits = limits.bounded();
    if bytes.len() > limits.max_input_bytes || source_name.len() > MAX_SOURCE_NAME_BYTES {
        return Err(limit("input bytes/name"));
    }

    let mut report =
        DistributionInspection::empty(source_name, bytes, DistributionSourceKind::ManufacturerXml);
    if bytes.starts_with(b"PK") {
        zip::inspect(bytes, &limits, &mut report)?;
    } else {
        xml::inspect(bytes, None, &limits, &mut report)?;
    }
    readers::output_limit(&report, &limits)?;
    provenance::json_size(&report, limits.max_report_bytes)?;
    Ok(report)
}
