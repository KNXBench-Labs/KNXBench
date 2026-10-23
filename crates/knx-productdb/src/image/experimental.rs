//! Source-bound strong hypotheses remain separate from ordinary image admission.

use super::*;
use rusqlite::OptionalExtension;

mod source;

/// Caller-owned source identity; a package binding is checked when supplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentalSystem2SourceBinding {
    pub program_id: String,
    pub schema_namespace: String,
    pub source_sha256: String,
    pub package_sha256: Option<String>,
}

/// Every rule is mandatory. There is intentionally no `Default` implementation.
/// These versioned rule names opt into hypotheses, not confirmed device support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentalSystem2ImagePolicy {
    pub revision: u32,
    pub binding: ExperimentalSystem2SourceBinding,
    pub numeric_order: String,
    pub bit_union_offsets: String,
    pub legacy_patch_always: String,
    pub invisible_parameters: String,
    /// Bounded BinaryValue restriction mapping, required for source completeness.
    pub binary_restrictions: String,
    /// Explicit cross-version reference activation interpretation.
    pub reference_activation: String,
}

/// Experimental image and exhaustive source-parameter dispositions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentalSystem2Image {
    pub image: DownloadImage,
    /// Every source parameter, including virtual and inactive alternatives.
    pub coverage: Vec<ExperimentalParameterCoverage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperimentalParameterDisposition {
    Virtual,
    /// No inactive default was written: the source segment remains the base.
    RetainedSourceTemplate,
    BackgroundDefault,
    LegacyPatchDefaultReference,
    DefaultUnionBase,
    InactiveUnionAlternative,
    ActiveOverlay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentalParameterCoverage {
    pub parameter_id: String,
    pub references: Vec<String>,
    pub source_default: String,
    pub segment: Option<String>,
    pub field: Option<ParameterField>,
    pub legacy_patch_always: bool,
    pub default_union_parameter: bool,
    pub active_reference: Option<String>,
    pub disposition: ExperimentalParameterDisposition,
}

/// Build a complete default-configuration candidate with caller-pinned hypotheses.
///
/// This delegates to the production evaluator, resolver and image/table codecs.
/// Project value, link and flag overrides are intentionally unsupported. Every
/// source parameter receives a coverage disposition; unsupported mandatory data
/// refuses instead of returning a partial image. The ordinary image entry points
/// and live admission are unchanged, and no hardware equivalence is asserted.
pub fn build_experimental_system2_image(
    conn: &Connection,
    request: &ImageRequest,
    policy: &ExperimentalSystem2ImagePolicy,
) -> Result<ExperimentalSystem2Image, ImageError> {
    let prepared = Prepared::load(conn, request, policy, false)?;
    let mut image =
        super::build_image_selected(conn, request, ImageLayout::System2, Some(&prepared))?;
    image.inferences.push(Inference {
        rule: "experimental-system2-numeric-order",
        detail: "STRONG HYPOTHESIS: numeric fields use high octet first, including absent ParameterByteOrder (schema mirror default); source template equality is not required. Not independently validated on hardware.".into(),
        reference: "KNX Project Schema v10 description v1.2; third-party schema20 mirror; caller-pinned source",
    });
    image.inferences.push(Inference {
        rule: "experimental-system2-legacy-patch-always",
        detail: "STRONG HYPOTHESIS: schema10 inactive-write and reference-value rules are applied to the caller-pinned schema20 source; disagreeing inactive references refuse, active reference wins. No schema20 normative or device equivalence claimed.".into(),
        reference: "KNX Project Schema v10 description v1.2 pp19-21; caller-pinned source",
    });
    image.inferences.push(Inference {
        rule: "experimental-system2-background-union-overlay",
        detail: "STRONG HYPOTHESIS: absent/DefaultValue/Background initializes ordinary parameter defaults and one declared DefaultUnionParameter per exact union, then overlays active refs. Schema mirrors name these controls but do not define this precedence; it remains a candidate algorithm, not confirmed support.".into(),
        reference: "KNX Project Schema v10 description v1.2 p20; third-party schema20 mirror; caller-pinned source",
    });
    image.inferences.push(Inference {
        rule: "experimental-system2-binary-restriction",
        detail: "STRONG HYPOTHESIS: a one-octet BinaryValue restriction writes its selected enumeration payload, not its decimal selector; malformed/ambiguous/missing or wider payloads refuse. No device confirmation.".into(),
        reference: "third-party schema20 mirror TypeRestriction/Enumeration; caller-pinned source",
    });
    image.inferences.push(Inference {
        rule: "experimental-system2-bit-union-offsets",
        detail: "STRONG HYPOTHESIS: schema10 union-relative octet offsets and schema23 MSB0 bit positions apply to this schema20 source; only byte-aligned unions and in-bounds members are admitted. Non-union overlap and overlapping active alternatives refuse.".into(),
        reference: "KNX Project Schema v10 description v1.2 pp19-20; Project Schema23 1.1.3.17 pp29-30",
    });
    image.inferences.push(Inference {
        rule: "experimental-system2-reference-activation",
        detail: "STRONG HYPOTHESIS: apply schema10's no-dynamic-occurrence reference activation to this schema20 source. The production evaluator still selects conditional refs; multiple active refs for one parameter refuse even if their values agree.".into(),
        reference: "KNX Project Schema v10 description v1.2 pp20-21; caller-pinned schema20 source",
    });
    let coverage = prepared.coverage(conn, &image)?;
    Ok(ExperimentalSystem2Image { image, coverage })
}

/// Build a project-configured source-preserving candidate under revision 2.
/// This is an explicit hypothesis, not ordinary image or hardware admission.
pub fn build_project_experimental_system2_image(
    conn: &Connection,
    request: &ImageRequest,
    policy: &ExperimentalSystem2ImagePolicy,
) -> Result<ExperimentalSystem2Image, ImageError> {
    let prepared = Prepared::load(conn, request, policy, true)?;
    let mut image =
        super::build_image_selected(conn, request, ImageLayout::System2, Some(&prepared))?;
    image.inferences.push(Inference {
        rule: "experimental-system2-source-template-project-overlay",
        detail: "STRONG HYPOTHESIS revision 2: retain source template, validate inactive alternatives without writing ordinary or union defaults; write inactive explicit LegacyPatchAlways reference consensus and overlay active project references. Numeric, binary, union-offset and reference-activation rules are caller-selected hypotheses, not normative precedence or hardware support.".into(),
        reference: "caller-pinned schema20 or schema23 source; explicit revision2 policy",
    });
    let coverage = prepared.coverage(conn, &image)?;
    Ok(ExperimentalSystem2Image { image, coverage })
}

pub(super) struct Prepared {
    source_template: bool,
    parameters: BTreeMap<String, ParameterPolicy>,
    binary_values: BTreeMap<(String, u64), u8>,
    source_refs: BTreeMap<String, String>,
    dynamic_refs: BTreeSet<String>,
}

struct ParameterPolicy {
    legacy_patch: bool,
    default_union: bool,
}

fn boolean(attributes: &crate::xml::Attrs, key: &str) -> Result<bool, ImageError> {
    match attributes.get(key) {
        None | Some("false" | "0") => Ok(false),
        Some("true" | "1") => Ok(true),
        Some(value) => Err(unsupported(format!(
            "{key} has unrecognized boolean {value:?}"
        ))),
    }
}

struct SourcePolicies {
    parameters: BTreeMap<String, ParameterPolicy>,
    references: BTreeMap<String, String>,
    dynamic_refs: BTreeSet<String>,
}

fn parameter_policies(bytes: &[u8], program: &str) -> Result<SourcePolicies, ImageError> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_reader(bytes);
    let mut path = Vec::<String>::new();
    let mut selected = None;
    let mut result = BTreeMap::new();
    let mut references = BTreeMap::new();
    let mut dynamic_refs = BTreeSet::new();
    let mut controls = BTreeSet::new();
    let mut identities = BTreeSet::new();
    let mut matches = 0;
    let mut nodes = Vec::<usize>::new();
    let mut ordinal = 0;
    let mut singleton_children = BTreeSet::new();
    loop {
        let event = reader
            .read_event()
            .map_err(|e| unsupported(e.to_string()))?;
        match event {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let name = e.name().as_ref().to_string();
                let attrs = crate::xml::attrs(e, "experimental source")?;
                if name == "ApplicationProgram" && attrs.get("Id") == Some(program) {
                    matches += 1;
                    selected = Some(path.len());
                }
                ordinal += 1;
                if ordinal > 250_000 || path.len() > 64 {
                    return Err(unsupported("experimental source scan budget exceeded"));
                }
                if selected.is_some() {
                    source::validate(&name, &path, &attrs)?;
                    if let Some(id) = attrs.get("Id") {
                        if !identities.insert(id.to_string()) {
                            return Err(unsupported(format!("duplicate source Id {id}")));
                        }
                    }
                    if matches!(
                        name.as_str(),
                        "Static"
                            | "Dynamic"
                            | "Options"
                            | "Code"
                            | "ParameterTypes"
                            | "Parameters"
                            | "ParameterRefs"
                            | "ComObjectTable"
                            | "ComObjectRefs"
                            | "AddressTable"
                            | "AssociationTable"
                            | "Memory"
                            | "Data"
                            | "Mask"
                            | "TypeNumber"
                            | "TypeRestriction"
                            | "TypeNone"
                    ) && !singleton_children.insert((nodes.last().copied(), name.clone()))
                    {
                        return Err(unsupported(format!("duplicate source declaration {name}")));
                    }
                }
                if selected.is_some() && name == "ParameterRef" {
                    let id = attrs
                        .get("Id")
                        .ok_or_else(|| unsupported("ParameterRef has no Id"))?;
                    let parameter = attrs
                        .get("RefId")
                        .ok_or_else(|| unsupported("ParameterRef has no RefId"))?;
                    references.insert(id.into(), parameter.into());
                }
                if selected.is_some() && path.iter().any(|p| p == "Dynamic") {
                    if name == "ParameterRefRef" {
                        dynamic_refs.insert(
                            attrs
                                .get("RefId")
                                .ok_or_else(|| unsupported("ParameterRefRef has no RefId"))?
                                .to_string(),
                        );
                    }
                    if let Some(reference) = attrs.get("ParamRefId") {
                        controls.insert(reference.to_string());
                    }
                }
                if selected.is_some()
                    && name == "Parameter"
                    && matches!(
                        path.last().map(String::as_str),
                        Some("Parameters" | "Union")
                    )
                {
                    let id = attrs
                        .get("Id")
                        .ok_or_else(|| unsupported("Parameter has no Id"))?;
                    let value = ParameterPolicy {
                        legacy_patch: boolean(&attrs, "LegacyPatchAlways")?,
                        default_union: boolean(&attrs, "DefaultUnionParameter")?,
                    };
                    if result.insert(id.into(), value).is_some() {
                        return Err(unsupported(format!("duplicate parameter {id}")));
                    }
                }
                if matches!(event, Event::Start(_)) {
                    path.push(name);
                    nodes.push(ordinal);
                }
            }
            Event::End(_) => {
                path.pop();
                nodes.pop();
                if selected == Some(path.len()) {
                    selected = None;
                }
            }
            Event::Text(text)
                if selected.is_some() && !path.iter().any(|p| p == "LoadProcedures") =>
            {
                if !matches!(path.last().map(String::as_str), Some("Data" | "Mask"))
                    && !text.as_ref().trim().is_empty()
                {
                    return Err(unsupported("unrecognized source text"));
                }
            }
            Event::CData(_)
                if selected.is_some() && !path.iter().any(|p| p == "LoadProcedures") =>
            {
                if !matches!(path.last().map(String::as_str), Some("Data" | "Mask")) {
                    return Err(unsupported("unrecognized source text (CDATA)"));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if matches != 1 {
        return Err(unsupported(
            "source does not uniquely own the requested application",
        ));
    }
    if references.values().any(|p| !result.contains_key(p))
        || dynamic_refs
            .iter()
            .chain(&controls)
            .any(|r| !references.contains_key(r))
    {
        return Err(unsupported("dangling source parameter reference"));
    }
    Ok(SourcePolicies {
        parameters: result,
        references,
        dynamic_refs,
    })
}

fn unsupported(what: impl Into<String>) -> ImageError {
    ImageError::Unsupported {
        what: format!("experimental System2: {}", what.into()),
    }
}

impl Prepared {
    pub(super) fn retains_source_template(&self) -> bool {
        self.source_template
    }

    pub(super) fn validate_requested_value(
        &self,
        code: &ProgramCode,
        types: &ParameterTypes,
        reference: &str,
        raw: &str,
    ) -> Result<(), ImageError> {
        let value = self.value(types, reference, raw)?;
        let parameter = types.parameter_of(reference)?;
        if let Some(placement) = code.parameters.get(parameter) {
            let (segment, field) = field_of(parameter, placement, types.size(reference)?)?;
            let source = code
                .segment(segment)
                .ok_or_else(|| unsupported("unknown requested segment"))?;
            let mut probe = ParameterImage::new(
                source
                    .data
                    .clone()
                    .ok_or_else(|| unsupported("requested field has no source data"))?,
            );
            match value {
                Encoded::Number(n) => probe.write(field, n),
                Encoded::Text { octets, .. } => probe.write_octets(field, &octets),
            }
            .map_err(|error| ImageError::Parameter {
                parameter_ref: reference.into(),
                error,
            })?;
            check_masks(
                code,
                &[SegmentImage {
                    id: segment.into(),
                    address: source.address,
                    octets: probe.into_octets(),
                    mask: source.mask.clone(),
                }],
            )?;
        }
        Ok(())
    }

    pub(super) fn validate_flag_overrides(
        &self,
        request: &ImageRequest,
        activation: &Activation,
    ) -> Result<(), ImageError> {
        if self.source_template {
            for (reference, flags) in &request.flag_overrides {
                if !flags.is_empty()
                    && activation
                        .com_object_refs
                        .iter()
                        .filter(|r| r.scope.is_none() && r.ref_id == *reference)
                        .count()
                        != 1
                {
                    return Err(unsupported(format!(
                        "{reference}: flag override requires one exact active unscoped reference"
                    )));
                }
            }
        }
        Ok(())
    }

    pub(super) fn extend_activation(
        &self,
        activation: &mut Activation,
        types: &ParameterTypes,
    ) -> Result<(), ImageError> {
        if types.refs.len() != self.source_refs.len()
            || self
                .source_refs
                .iter()
                .any(|(id, parameter)| types.refs.get(id).is_none_or(|r| r.parameter != *parameter))
        {
            return Err(unsupported(
                "source/normalized reference ownership mismatch",
            ));
        }
        for reference in self
            .source_refs
            .keys()
            .filter(|id| !self.dynamic_refs.contains(*id))
        {
            activation.parameter_refs.push(crate::dynamic::ActiveRef {
                scope: None,
                ref_id: reference.clone(),
                channel: None,
                block_access: None,
            });
        }
        let mut active_parameters = BTreeSet::new();
        for active in &activation.parameter_refs {
            let parameter = types.parameter_of(&active.ref_id)?;
            if !active_parameters.insert(parameter) {
                return Err(unsupported(format!(
                    "{parameter}: multiple active references"
                )));
            }
        }
        Ok(())
    }

    fn coverage(
        &self,
        conn: &Connection,
        image: &DownloadImage,
    ) -> Result<Vec<ExperimentalParameterCoverage>, ImageError> {
        let types = ParameterTypes::load(conn, &image.code.program_id, &image.code)?;
        let mut stmt =
            conn.prepare("SELECT id, value FROM parameter WHERE program_id = ?1 ORDER BY id")?;
        let defaults = stmt
            .query_map([&image.code.program_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
            })?
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        if defaults.keys().ne(self.parameters.keys()) {
            return Err(unsupported("normalized/source parameter coverage mismatch"));
        }
        let mut result = Vec::new();
        for (parameter, policy) in &self.parameters {
            let references: Vec<String> = types
                .refs
                .iter()
                .filter(|(_, facts)| facts.parameter == *parameter)
                .map(|(id, _)| id.clone())
                .collect();
            let active_reference = image
                .parameter_fields
                .iter()
                .find(|f| references.contains(&f.reference))
                .map(|f| f.reference.clone());
            let (segment, field, disposition) = match image.code.parameters.get(parameter) {
                None => (None, None, ExperimentalParameterDisposition::Virtual),
                Some(placement) => {
                    let reference = references.first().ok_or_else(|| {
                        unsupported(format!("{parameter}: no reference for type validation"))
                    })?;
                    let (segment, field) = field_of(parameter, placement, types.size(reference)?)?;
                    let disposition = if active_reference.is_some() {
                        ExperimentalParameterDisposition::ActiveOverlay
                    } else if self.source_template {
                        if policy.legacy_patch {
                            ExperimentalParameterDisposition::LegacyPatchDefaultReference
                        } else {
                            ExperimentalParameterDisposition::RetainedSourceTemplate
                        }
                    } else if matches!(placement, ParameterPlacement::UnionMember { .. }) {
                        if policy.default_union {
                            ExperimentalParameterDisposition::DefaultUnionBase
                        } else {
                            ExperimentalParameterDisposition::InactiveUnionAlternative
                        }
                    } else if policy.legacy_patch {
                        ExperimentalParameterDisposition::LegacyPatchDefaultReference
                    } else {
                        ExperimentalParameterDisposition::BackgroundDefault
                    };
                    (Some(segment.into()), Some(field), disposition)
                }
            };
            result.push(ExperimentalParameterCoverage {
                parameter_id: parameter.clone(),
                references,
                source_default: defaults
                    .get(parameter)
                    .and_then(Clone::clone)
                    .ok_or_else(|| unsupported(format!("{parameter}: no source default")))?,
                segment,
                field,
                legacy_patch_always: policy.legacy_patch,
                default_union_parameter: policy.default_union,
                active_reference,
                disposition,
            });
        }
        Ok(result)
    }

    pub(super) fn value(
        &self,
        types: &ParameterTypes,
        reference: &str,
        raw: &str,
    ) -> Result<Encoded, ImageError> {
        let facts = types.get(reference)?;
        if facts.kind.as_deref() == Some("Restriction")
            && facts.base.as_deref() == Some("BinaryValue")
        {
            if types.size(reference)? != 8 {
                return Err(unsupported(format!(
                    "{reference}: BinaryValue only supports exactly one octet"
                )));
            }
            let selected: u64 = raw.parse().map_err(|_| {
                unsupported(format!("{reference}: nonnumeric binary enumeration value"))
            })?;
            let type_id = facts
                .parameter_type
                .as_ref()
                .ok_or_else(|| unsupported("missing BinaryValue type"))?;
            let value = self
                .binary_values
                .get(&(type_id.clone(), selected))
                .ok_or_else(|| {
                    unsupported(format!("{reference}: unresolved BinaryValue enumeration"))
                })?;
            return Ok(Encoded::Number(u64::from(*value)));
        }
        types.value(reference, raw)
    }

    pub(super) fn background(
        &self,
        conn: &Connection,
        code: &ProgramCode,
        types: &ParameterTypes,
        activation: &Activation,
        tables: &[(String, usize, usize)],
    ) -> Result<BTreeMap<String, Vec<u8>>, ImageError> {
        if !matches!(
            code.options
                .get("DownloadInvisibleParameters")
                .map(String::as_str),
            None | Some("Background" | "DefaultValue")
        ) {
            return Err(unsupported(
                "unrecognized DownloadInvisibleParameters value",
            ));
        }
        let mut images: BTreeMap<String, ParameterImage> = code
            .segments
            .iter()
            .map(|s| {
                s.data
                    .clone()
                    .map(|d| (s.id.clone(), ParameterImage::new(d)))
                    .ok_or_else(|| unsupported(format!("{}: segment ships no Data", s.id)))
            })
            .collect::<Result<_, _>>()?;
        let mut stmt = conn.prepare(
            "SELECT id, union_id, union_size_in_bit FROM parameter WHERE program_id = ?1",
        )?;
        let union_metadata = stmt
            .query_map([&code.program_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    (r.get::<_, Option<i64>>(1)?, r.get::<_, Option<i64>>(2)?),
                ))
            })?
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let active_parameters = activation
            .parameter_refs
            .iter()
            .map(|r| types.parameter_of(&r.ref_id))
            .collect::<Result<BTreeSet<_>, _>>()?;
        let defaults = resolve_values(conn, &code.program_id, &Default::default())?;
        let mut union_defaults = BTreeMap::<i64, usize>::new();
        let mut all_fields = Vec::<(String, ParameterField, Option<i64>, String)>::new();
        for (parameter, placement) in &code.parameters {
            let reference = types
                .refs
                .iter()
                .find(|(_, r)| r.parameter == *parameter)
                .map(|(id, _)| id.as_str())
                .ok_or_else(|| {
                    unsupported(format!("{parameter}: no reference for type validation"))
                })?;
            let raw: Option<String> = conn.query_row(
                "SELECT value FROM parameter WHERE program_id = ?1 AND id = ?2",
                [&code.program_id, parameter],
                |r| r.get(0),
            )?;
            let raw = raw.ok_or_else(|| unsupported(format!("{parameter}: no source default")))?;
            let mut value = self.value(types, reference, &raw)?;
            let policy = self
                .parameters
                .get(parameter)
                .ok_or_else(|| unsupported(format!("{parameter}: no retained source policy")))?;
            if policy.legacy_patch && !active_parameters.contains(parameter.as_str()) {
                let mut agreed = None;
                for (reference, facts) in &types.refs {
                    if facts.parameter != *parameter {
                        continue;
                    }
                    let raw = defaults
                        .get_unscoped(reference)
                        .ok_or_else(|| unsupported(format!("{reference}: no default")))?;
                    let candidate = self.value(types, reference, raw)?;
                    if agreed.as_ref().is_some_and(|earlier| *earlier != candidate) {
                        return Err(unsupported(format!(
                            "{parameter}: inactive LegacyPatchAlways references disagree"
                        )));
                    }
                    agreed = Some(candidate);
                }
                if let Some(agreed) = agreed {
                    value = agreed;
                }
            }
            let (segment, field) = field_of(parameter, placement, types.size(reference)?)?;
            let (union_id, union_size) = union_metadata
                .get(parameter)
                .copied()
                .ok_or_else(|| unsupported("missing normalized parameter"))?;
            if let ParameterPlacement::UnionMember { union, .. } = placement {
                if union.bit_offset != 0 {
                    return Err(unsupported(format!(
                        "{parameter}: policy requires a byte-aligned union"
                    )));
                }
                let id = union_id
                    .ok_or_else(|| unsupported(format!("{parameter}: no exact union identity")))?;
                let size = union_size
                    .and_then(|n| u64::try_from(n).ok())
                    .filter(|n| *n > 0)
                    .ok_or_else(|| unsupported("invalid union size"))?;
                let union_start = u64::from(union.offset) * 8 + u64::from(union.bit_offset);
                let (start, end) = bit_span(field);
                if start < union_start || end > union_start + size {
                    return Err(unsupported(format!(
                        "{parameter}: field extends outside its exact union"
                    )));
                }
                let count = union_defaults.entry(id).or_default();
                if policy.default_union {
                    *count += 1;
                }
            } else if union_id.is_some() || policy.default_union {
                return Err(unsupported(format!(
                    "{parameter}: contradictory union policy"
                )));
            }
            let (first, stop) = bit_span(field);
            for (id, earlier, earlier_union, earlier_parameter) in &all_fields {
                let (a, b) = bit_span(*earlier);
                if id == segment
                    && first < b
                    && a < stop
                    && !(union_id.is_some() && union_id == *earlier_union)
                {
                    return Err(unsupported(format!(
                        "{parameter}: overlaps {earlier_parameter} outside its exact union"
                    )));
                }
            }
            all_fields.push((segment.into(), field, union_id, parameter.clone()));
            let start = field.offset as usize;
            let end =
                start + (u32::from(field.bit_offset) + field.size_in_bit).div_ceil(8) as usize;
            if tables
                .iter()
                .any(|(id, a, b)| id == segment && start < *b && *a < end)
            {
                return Err(unsupported(format!(
                    "{parameter}: overlaps reserved table region"
                )));
            }
            let image = images
                .get_mut(segment)
                .ok_or_else(|| unsupported(format!("{parameter}: segment ships no Data")))?;
            // Validate every alternative, even when not selected for the base.
            let mut probe = ParameterImage::new(image.octets().to_vec());
            match &value {
                Encoded::Number(n) => probe.write(field, *n),
                Encoded::Text { octets, .. } => probe.write_octets(field, octets),
            }
            .map_err(|error| ImageError::Parameter {
                parameter_ref: reference.into(),
                error,
            })?;
            if self.source_template {
                let source = code
                    .segment(segment)
                    .ok_or_else(|| unsupported("unknown segment"))?;
                check_masks(
                    code,
                    &[SegmentImage {
                        id: segment.into(),
                        address: source.address,
                        octets: probe.into_octets(),
                        mask: source.mask.clone(),
                    }],
                )?;
            }
            if self.source_template {
                if !policy.legacy_patch || active_parameters.contains(parameter.as_str()) {
                    continue;
                }
            } else if union_id.is_some() && !policy.default_union {
                continue;
            }
            match value {
                Encoded::Number(n) => image.write(field, n),
                Encoded::Text { octets, .. } => image.write_octets(field, &octets),
            }
            .map_err(|error| ImageError::Parameter {
                parameter_ref: reference.into(),
                error,
            })?;
        }
        if union_defaults.values().any(|count| *count != 1) {
            return Err(unsupported("each exact union requires one default member"));
        }
        Ok(images
            .into_iter()
            .map(|(id, image)| (id, image.into_octets()))
            .collect())
    }

    fn load(
        conn: &Connection,
        request: &ImageRequest,
        policy: &ExperimentalSystem2ImagePolicy,
        source_template: bool,
    ) -> Result<Self, ImageError> {
        for (name, actual, expected) in [
            (
                "reference_activation",
                policy.reference_activation.as_str(),
                "schema10-no-dynamic-reference-is-active-v1",
            ),
            (
                "binary_restrictions",
                policy.binary_restrictions.as_str(),
                "single-octet-enumeration-binary-payload-v1",
            ),
            (
                "numeric_order",
                policy.numeric_order.as_str(),
                "high-octet-first-explicit-or-schema-default-v1",
            ),
            (
                "bit_union_offsets",
                &policy.bit_union_offsets,
                "msb0-byte-aligned-union-relative-v1",
            ),
            (
                "legacy_patch_always",
                &policy.legacy_patch_always,
                "inactive-default-ref-consensus-active-wins-v1",
            ),
            (
                "invisible_parameters",
                &policy.invisible_parameters,
                if source_template {
                    "source-template-inactive-legacy-patch-active-overlay-v1"
                } else {
                    "background-parameter-default-union-base-active-overlay-v1"
                },
            ),
        ] {
            if actual != expected {
                return Err(unsupported(format!(
                    "unrecognized {name} policy {actual:?}"
                )));
            }
        }
        if policy.revision != if source_template { 2 } else { 1 } {
            return Err(unsupported("unrecognized policy revision"));
        }
        if request.program_id != policy.binding.program_id {
            return Err(unsupported("application binding mismatch"));
        }
        if !source_template
            && (!request.values.is_empty()
                || !request.links.is_empty()
                || !request.flag_overrides.is_empty())
        {
            return Err(unsupported("only complete default configuration without project overrides or links is supported"));
        }
        let sha: Option<String> = conn
            .query_row(
                "SELECT source_sha256 FROM application_program WHERE id = ?1",
                [&request.program_id],
                |r| r.get(0),
            )
            .optional()?;
        let sha = sha.ok_or_else(|| ImageError::UnknownProgram(request.program_id.clone()))?;
        let valid_hash = |s: &str| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if !valid_hash(&policy.binding.source_sha256) || sha != policy.binding.source_sha256 {
            return Err(unsupported("source ownership binding mismatch"));
        }
        let bytes = crate::load_source_file(conn, &sha)?
            .ok_or_else(|| unsupported("source bytes unavailable"))?;
        if crate::sha256_hex(&bytes) != sha {
            return Err(unsupported("source content digest mismatch"));
        }
        if let Some(package) = &policy.binding.package_sha256 {
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM package_member WHERE package_sha256 = ?1 AND source_sha256 = ?2", [package, &sha], |r| r.get(0))?;
            if !valid_hash(package) || count != 1 {
                return Err(unsupported("package source ownership mismatch"));
            }
        }
        let mut reader = quick_xml::Reader::from_reader(bytes.as_slice());
        loop {
            match reader
                .read_event()
                .map_err(|e| unsupported(e.to_string()))?
            {
                quick_xml::events::Event::Start(element)
                | quick_xml::events::Event::Empty(element) => {
                    if element.name().as_ref() != "KNX" {
                        return Err(unsupported("source root is not KNX"));
                    }
                    let attributes = crate::xml::attrs(&element, "experimental source")?;
                    if attributes.get("xmlns") != Some(policy.binding.schema_namespace.as_str())
                        || !(policy.binding.schema_namespace == "http://knx.org/xml/project/20"
                            || (source_template
                                && policy.binding.schema_namespace
                                    == "http://knx.org/xml/project/23"))
                    {
                        return Err(unsupported("schema binding mismatch"));
                    }
                    break;
                }
                quick_xml::events::Event::Eof => return Err(unsupported("source has no root")),
                _ => {}
            }
        }
        use base64::Engine as _;
        let mut binary_values = BTreeMap::new();
        let mut stmt = conn.prepare("SELECT e.parameter_type_id, e.value, e.binary_value FROM parameter_type_enum e JOIN parameter_type t ON t.program_id = e.program_id AND t.id = e.parameter_type_id WHERE e.program_id = ?1 AND t.base = 'BinaryValue'")?;
        let rows = stmt.query_map([&request.program_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })?;
        for row in rows {
            let (type_id, value, binary) = row?;
            let selected = value
                .and_then(|v| v.parse::<u64>().ok())
                .ok_or_else(|| unsupported("invalid BinaryValue enumeration selector"))?;
            let binary =
                binary.ok_or_else(|| unsupported("BinaryValue enumeration has no payload"))?;
            let payload = base64::engine::general_purpose::STANDARD
                .decode(binary)
                .map_err(|_| unsupported("malformed BinaryValue payload"))?;
            if payload.len() != 1 {
                return Err(unsupported("BinaryValue payload must be exactly one octet"));
            }
            if binary_values
                .insert((type_id, selected), payload[0])
                .is_some()
            {
                return Err(unsupported("duplicate BinaryValue selector"));
            }
        }
        let source = parameter_policies(&bytes, &request.program_id)?;
        Ok(Self {
            source_template,
            parameters: source.parameters,
            binary_values,
            source_refs: source.references,
            dynamic_refs: source.dynamic_refs,
        })
    }
}
