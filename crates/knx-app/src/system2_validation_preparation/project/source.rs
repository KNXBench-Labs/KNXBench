//! Bounded winning-source closure, not historical archive membership or authenticity.
use super::*;
use knx_productdb::identity::{owned_source_identity, IdentityKind};
use quick_xml::{events::Event, name::ResolveResult, NsReader};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceMemberRole {
    Hardware2programSource,
    ApplicationProgramSource,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceOwnerDescriptor {
    pub kind: String,
    pub logical_id: String,
    pub occurrence: u32,
    pub element_digest: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceMemberDescriptor {
    pub role: SourceMemberRole,
    pub source_path: String,
    pub schema_namespace: String,
    pub source_sha256: String,
    pub byte_len: u64,
    pub owner: SourceOwnerDescriptor,
}
/// Canonical fixed-role descriptor. Hash excludes profile and raw bytes, preventing cycles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceClosureDescriptor {
    pub closure_revision: u16,
    pub program_ref: String,
    pub program_id: String,
    pub members: [SourceMemberDescriptor; 2],
}
impl SourceClosureDescriptor {
    pub fn sha256(&self) -> io::Result<String> {
        #[derive(Serialize)]
        struct Identity<'a> {
            domain: &'static str,
            descriptor: &'a SourceClosureDescriptor,
        }
        Ok(digest(
            &serde_json::to_vec(&Identity {
                domain: "knxbench-project-product-source-closure-v1",
                descriptor: self,
            })
            .map_err(io::Error::other)?,
        ))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ProjectProductInput {
    #[serde(rename = "standalone-package-v1")]
    StandalonePackage {
        source_member: String,
        #[serde(with = "super::retained_bytes::bytes")]
        source_bytes: Vec<u8>,
        #[serde(with = "super::retained_bytes::bytes")]
        package_bytes: Vec<u8>,
        package_sha256: String,
    },
    #[serde(rename = "retained-source-members-v1")]
    RetainedSourceMembers {
        descriptor: Box<SourceClosureDescriptor>,
        #[serde(with = "super::retained_bytes::members")]
        member_bytes: [Vec<u8>; 2],
        closure_sha256: String,
    },
}
fn path_ok(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 1024
        && !path.contains(['\\', ':'])
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
}
// XML1.0 Fifth Edition §2.3 NameStartChar/NameChar, excluding colon for
// this deliberately unprefixed element/attribute boundary (NCName). Keep the
// ranges aligned with the existing ETS instructions admission helper.
fn xml_name_start(c: char) -> bool {
    matches!(c, 'A'..='Z' | '_' | 'a'..='z' | '\u{c0}'..='\u{d6}' | '\u{d8}'..='\u{f6}'
        | '\u{f8}'..='\u{2ff}' | '\u{370}'..='\u{37d}' | '\u{37f}'..='\u{1fff}'
        | '\u{200c}'..='\u{200d}' | '\u{2070}'..='\u{218f}' | '\u{2c00}'..='\u{2fef}'
        | '\u{3001}'..='\u{d7ff}' | '\u{f900}'..='\u{fdcf}' | '\u{fdf0}'..='\u{fffd}'
        | '\u{10000}'..='\u{effff}')
}
fn xml_ncname(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(xml_name_start)
        && chars.all(|c| {
            xml_name_start(c)
                || matches!(c, '-' | '.' | '0'..='9' | '\u{b7}' | '\u{300}'..='\u{36f}' | '\u{203f}'..='\u{2040}')
        })
}

// Attribute lookups reachable from retained Hardware/ApplicationProgram originals:
// hardware/program ingest, identity dispatch, dynamic ingest, code and experimental
// source readers. This is a lexical superset of their context-sensitive lookups,
// not an XSD. Unrelated prefixes (including Unicode) remain declarations. A
// colliding declaration is safe only when the ordinary spelling is also present:
// ProductDB's unqualified value wins independently of declaration order/value.
fn xml_semantic_attribute(element: &str, name: &str, path: &[String]) -> bool {
    if (name == "Id" && path.iter().any(|part| part == "ApplicationProgram"))
        || (name == "ParamRefId" && path.iter().any(|part| part == "Dynamic"))
    {
        return true;
    }
    let keys = match element {
        "Manufacturer" | "ApplicationProgramRef" => "RefId",
        "Hardware" => {
            "Id Name SerialNumber VersionNumber BusCurrent HasIndividualAddress HasApplicationProgram IsAccessory IsCoupler IsPowerSupply IsIPEnabled IsPowerLineRepeater OriginalManufacturer"
        }
        "Product" => "Id Text OrderNumber IsRailMounted WidthInMillimeter DefaultLanguage Hash",
        "Hardware2Program" => "Id MediumTypes Hash",
        "RegistrationInfo" => "RegistrationNumber RegistrationStatus RegistrationSignature",
        "ApplicationProgram" => {
            "Id Name ApplicationNumber ApplicationVersion ProgramType MaskVersion PeiType LoadProcedureStyle DefaultLanguage Hash Linkable OriginalManufacturer DynamicTableManagement"
        }
        "ParameterType" => "Id Name",
        "TypeRestriction" => "SizeInBit Base",
        "TypeNumber" => "SizeInBit Type minInclusive maxInclusive",
        "TypeText" => "SizeInBit",
        "TypeFloat" => "minInclusive maxInclusive",
        "TypeTime" => "SizeInBit minInclusive maxInclusive",
        "Enumeration" => "Id Value Text DisplayOrder BinaryValue Name Icon PictureAlignment",
        "Union" => "SizeInBit",
        "Parameter" => {
            "Id Name Text ParameterType Access Value Suffix SuffixText Offset BitOffset LegacyPatchAlways DefaultUnionParameter"
        }
        "ParameterRef" => "Id RefId DisplayOrder Tag Text Value Access SuffixText",
        "Memory" => "CodeSegment Offset BitOffset",
        "Property" => "ObjectIndex ObjectType PropertyId Offset BitOffset Occurrence",
        "Language" => "Identifier",
        "TranslationElement" => "RefId",
        "Translation" => "AttributeName Text",
        "ComObject" => {
            "Id Number Name Text FunctionText VisibleDescription ObjectSize Priority DatapointType ReadFlag WriteFlag TransmitFlag UpdateFlag CommunicationFlag ReadOnInitFlag"
        }
        "ComObjectRef" => {
            "Id RefId Tag Text FunctionText VisibleDescription ObjectSize Priority DatapointType ReadFlag WriteFlag TransmitFlag UpdateFlag CommunicationFlag ReadOnInitFlag TextParameterRefId"
        }
        "ModuleDef" => "Id",
        "Argument" => "Id Name Type Allocates",
        "choose" => "ParamRefId",
        "when" => "Id test default",
        "Channel" => "Id RefId Text Name Number TextParameterRefId",
        "ParameterBlock" => "Id RefId Text Name TextParameterRefId Access",
        "ParameterRefRef" | "ComObjectRefRef" => "RefId",
        "ParameterSeparator" => "Id Text UIHint TextParameterRefId Access",
        "Module" => "Id RefId",
        "NumericArg" | "TextArg" => "Id RefId Value",
        "Assign" => "TargetParamRefRef Value SourceParamRefRef",
        "ParameterCalculation" => "Id",
        "AbsoluteSegment" => "Id Address Size",
        "RelativeSegment" => "Id Size Offset LoadStateMachine",
        "AddressTable" | "AssociationTable" | "ComObjectTable" => "CodeSegment Offset MaxEntries",
        "LoadProcedure" => "MergeId",
        "LdCtrlCompareProp" => "ObjIdx PropId InlineData",
        "LdCtrlUnload" | "LdCtrlLoad" | "LdCtrlLoadCompleted" => "LsmIdx",
        "LdCtrlAbsSegment" => "LsmIdx SegType Address Size Access MemType SegFlags",
        "LdCtrlTaskSegment" => "LsmIdx Address",
        "LdCtrlTaskCtrl1" => "LsmIdx Address Count",
        "LdCtrlTaskCtrl2" => "LsmIdx Callback Address Seg0 Seg1",
        _ => "",
    };
    keys.split_ascii_whitespace().any(|key| key == name)
}

/// Full-document namespace/structure boundary before any local-name ingest pass.
pub(super) fn validate_xml(bytes: &[u8], namespace: &str) -> io::Result<()> {
    if bytes.len() > LIMIT
        || !matches!(
            namespace,
            "http://knx.org/xml/project/20" | "http://knx.org/xml/project/23"
        )
    {
        return Err(invalid("retained source size or namespace unsupported"));
    }
    let document = std::str::from_utf8(bytes).map_err(io::Error::other)?;
    let legal = |c: char| matches!(c as u32, 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF);
    if !document.chars().all(legal) {
        return Err(invalid("retained source illegal XML character"));
    }
    let mut reader = NsReader::from_reader(bytes);
    reader.config_mut().check_comments = true;
    let mut seen_event = false;
    let mut depth = 0usize;
    let mut events = 0usize;
    let mut roots = 0usize;
    let mut path = Vec::new();
    loop {
        let (ns, event) = reader.read_resolved_event().map_err(io::Error::other)?;
        let is_empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                events += 1;
                if events > 250_000 || depth >= 64 {
                    return Err(invalid("retained source XML event/depth limit"));
                }
                if !matches!(ns, ResolveResult::Bound(ref n) if n.as_ref() == namespace)
                    || e.name().as_ref().contains(':')
                {
                    return Err(invalid(
                        "retained source element namespace differs or prefixed spelling",
                    ));
                }
                if !xml_ncname(e.name().as_ref()) {
                    return Err(invalid("retained source invalid XML name"));
                }
                if depth == 0 {
                    roots += 1;
                    if roots != 1 || e.name().as_ref() != "KNX" {
                        return Err(invalid("retained source requires one literal KNX root"));
                    }
                }
                let mut ordinary = std::collections::HashSet::new();
                let mut declarations = Vec::new();
                for attr in e.attributes() {
                    let attr = attr.map_err(io::Error::other)?;
                    let value = attr
                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .map_err(io::Error::other)?;
                    if !value.chars().all(legal) {
                        return Err(invalid("retained source illegal XML attribute character"));
                    }
                    // The legacy ingest reader falls back to local attribute names.
                    // Refuse qualified metadata here rather than let a foreign alias
                    // acquire the meaning of an unqualified KNX field (e.g. RefId).
                    // Namespace declarations are checked for semantic aliases below.
                    if attr.key.as_ref().contains(':') && !attr.key.as_ref().starts_with("xmlns:") {
                        return Err(invalid("retained source qualified attribute unsupported"));
                    }
                    let name = attr.key.as_ref();
                    if !xml_ncname(name.strip_prefix("xmlns:").unwrap_or(name)) {
                        return Err(invalid("retained source invalid XML name"));
                    }
                    if let Some(prefix) = name.strip_prefix("xmlns:") {
                        declarations.push(prefix.to_owned());
                    } else {
                        ordinary.insert(name.to_owned());
                    }
                }
                if declarations.iter().any(|name| {
                    xml_semantic_attribute(e.name().as_ref(), name, &path)
                        && !ordinary.contains(name)
                }) {
                    return Err(invalid(
                        "retained source namespace declaration alias unsupported",
                    ));
                }
                if !is_empty {
                    path.push(e.name().as_ref().to_owned());
                    depth += 1;
                }
            }
            Event::End(_) => {
                events += 1;
                if events > 250_000 {
                    return Err(invalid("retained source XML event/depth limit"));
                }
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| invalid("retained source unexpected end"))?;
                path.pop();
            }
            Event::Text(text) => {
                if text.as_ref().contains("]]>") {
                    return Err(invalid("retained source forbidden text delimiter"));
                }
                if depth == 0
                    && !text
                        .as_ref()
                        .bytes()
                        .all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
                {
                    return Err(invalid("retained source text outside root"));
                }
            }
            Event::DocType(_) => return Err(invalid("retained source DTD forbidden")),
            Event::CData(_) if depth == 0 => {
                return Err(invalid("retained source content outside root"));
            }
            Event::GeneralRef(reference) => {
                if depth == 0 {
                    return Err(invalid("retained source content outside root"));
                }
                match reference.resolve_char_ref().map_err(io::Error::other)? {
                    Some(c) if legal(c) => {}
                    None if quick_xml::escape::resolve_xml_entity(reference.as_ref()).is_some() => {
                    }
                    _ => return Err(invalid("retained source invalid or undeclared XML entity")),
                }
            }
            Event::Decl(declaration) => {
                if seen_event {
                    return Err(invalid("retained source repeated or misplaced declaration"));
                }
                if declaration.version().map_err(io::Error::other)?.as_ref() != "1.0" {
                    return Err(invalid("retained source requires XML1.0"));
                }
                let attributes =
                    quick_xml::events::BytesStart::from_content(declaration.as_ref(), 3);
                let mut previous = 0;
                for attribute in attributes.attributes() {
                    let attribute = attribute.map_err(io::Error::other)?;
                    let rank = match (attribute.key.as_ref(), attribute.value.as_ref()) {
                        ("version", "1.0") => 1,
                        ("encoding", value) if value.eq_ignore_ascii_case("UTF-8") => 2,
                        ("standalone", "yes" | "no") => 3,
                        _ => return Err(invalid("retained source unsupported XML declaration")),
                    };
                    if rank <= previous {
                        return Err(invalid("retained source XML declaration attribute order"));
                    }
                    previous = rank;
                }
            }
            Event::Eof => break,
            _ => {}
        }
        seen_event = true;
    }
    if roots != 1 || depth != 0 {
        return Err(invalid("retained source incomplete document"));
    }
    Ok(())
}

fn chain(
    conn: &knx_productdb::Connection,
    project: &Project,
    device: DeviceId,
) -> io::Result<(String, String, String, String)> {
    let instance = project
        .devices
        .get(device)
        .ok_or_else(|| invalid("unknown project device"))?;
    if instance.program_ref.is_empty() || instance.program_ref.len() > 1024 {
        return Err(invalid("explicit project program_ref required"));
    }
    let (ap, hp_source): (String, String) = conn
        .query_row(
            "SELECT application_program_ref, source_sha256 FROM hardware2program WHERE id=?1",
            [&instance.program_ref],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(io::Error::other)?;
    let ap_source: String = conn
        .query_row(
            "SELECT source_sha256 FROM application_program WHERE id=?1",
            [&ap],
            |r| r.get(0),
        )
        .map_err(io::Error::other)?;
    Ok((instance.program_ref.clone(), ap, hp_source, ap_source))
}
/// Read only and pinned to one database read snapshot. No archive membership is asserted.
pub fn describe_project_source_closure(
    conn: &knx_productdb::Connection,
    project: &Project,
    device: DeviceId,
    schema_namespace: &str,
) -> io::Result<SourceClosureDescriptor> {
    let tx = conn.unchecked_transaction().map_err(io::Error::other)?;
    let (descriptor, _) = collect(&tx, project, device, schema_namespace)?;
    Ok(descriptor)
}
pub(super) fn collect(
    conn: &knx_productdb::Connection,
    project: &Project,
    device: DeviceId,
    namespace: &str,
) -> io::Result<(SourceClosureDescriptor, [Vec<u8>; 2])> {
    let (program_ref, program_id, hp_source, ap_source) = chain(conn, project, device)?;
    if hp_source == ap_source {
        return Err(invalid(
            "winning source roles require two distinct originals",
        ));
    }
    let request = knx_productdb::image_request::image_request_for_device(conn, project, device)
        .map_err(io::Error::other)?;
    if request.program_id != program_id {
        return Err(invalid("winning source resolution differs"));
    }
    let mut descriptors = Vec::new();
    let mut total = 0usize;
    for (role, kind, logical_id, sha) in [
        (
            SourceMemberRole::Hardware2programSource,
            IdentityKind::Hardware2Program,
            &program_ref,
            &hp_source,
        ),
        (
            SourceMemberRole::ApplicationProgramSource,
            IdentityKind::ApplicationProgram,
            &program_id,
            &ap_source,
        ),
    ] {
        let owner = owned_source_identity(conn, kind, logical_id, sha).map_err(io::Error::other)?;
        let (path, length): (String, i64) = conn
            .query_row(
                "SELECT source_path, length(bytes) FROM source_file WHERE sha256=?1",
                [sha],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(io::Error::other)?;
        if !path_ok(&path) || length < 0 {
            return Err(invalid("unsafe retained source path or length"));
        }
        total = total
            .checked_add(usize::try_from(length).map_err(io::Error::other)?)
            .filter(|n| *n <= LIMIT)
            .ok_or_else(|| invalid("retained source aggregate exceeds bundle budget"))?;
        descriptors.push(SourceMemberDescriptor {
            role,
            source_path: path,
            schema_namespace: namespace.into(),
            source_sha256: sha.clone(),
            byte_len: length as u64,
            owner: SourceOwnerDescriptor {
                kind: kind.as_table().into(),
                logical_id: logical_id.clone(),
                occurrence: owner.occurrence,
                element_digest: owner.digest,
            },
        });
    }
    let descriptor = SourceClosureDescriptor {
        closure_revision: 1,
        program_ref,
        program_id,
        members: descriptors
            .try_into()
            .map_err(|_| invalid("source cardinality"))?,
    };
    if descriptor.members[0].source_path == descriptor.members[1].source_path {
        return Err(invalid("retained source paths must be distinct"));
    }
    let bytes = [
        retained_blob(conn, "source_file", &hp_source)?,
        retained_blob(conn, "source_file", &ap_source)?,
    ];
    validate_members(&descriptor, &bytes, namespace)?;
    Ok((descriptor, bytes))
}
fn validate_members(
    descriptor: &SourceClosureDescriptor,
    bytes: &[Vec<u8>; 2],
    namespace: &str,
) -> io::Result<()> {
    if descriptor.closure_revision != 1
        || descriptor.program_ref.is_empty()
        || descriptor.program_id.is_empty()
        || descriptor.members[0].source_sha256 == descriptor.members[1].source_sha256
        || descriptor.members[0].source_path == descriptor.members[1].source_path
    {
        return Err(invalid("invalid retained source closure inventory"));
    }
    let mut total = 0usize;
    for (index, (member, bytes)) in descriptor.members.iter().zip(bytes).enumerate() {
        let (role, kind, id) = if index == 0 {
            (
                SourceMemberRole::Hardware2programSource,
                IdentityKind::Hardware2Program,
                &descriptor.program_ref,
            )
        } else {
            (
                SourceMemberRole::ApplicationProgramSource,
                IdentityKind::ApplicationProgram,
                &descriptor.program_id,
            )
        };
        if member.role != role
            || member.owner.kind != kind.as_table()
            || &member.owner.logical_id != id
            || member.owner.occurrence != 1
            || !profile::hash(&member.owner.element_digest)
            || !path_ok(&member.source_path)
            || member.schema_namespace != namespace
            || member.byte_len != bytes.len() as u64
            || member.source_sha256 != digest(bytes)
        {
            return Err(invalid("retained source member descriptor differs"));
        }
        total = total
            .checked_add(bytes.len())
            .filter(|n| *n <= LIMIT)
            .ok_or_else(|| invalid("retained source aggregate exceeds bundle budget"))?;
        validate_xml(bytes, namespace)?;
    }
    Ok(())
}
#[derive(Serialize)]
#[serde(tag = "kind")]
pub(super) enum ProductInputIdentity<'a> {
    #[serde(rename = "standalone-package-v1")]
    StandalonePackage {
        source_member: &'a str,
        source_len: usize,
        package_len: usize,
        package_sha256: &'a str,
    },
    #[serde(rename = "retained-source-members-v1")]
    RetainedSourceMembers {
        descriptor: &'a SourceClosureDescriptor,
        closure_sha256: &'a str,
    },
}
impl ProjectProductInput {
    pub(super) fn identity(&self) -> ProductInputIdentity<'_> {
        match self {
            Self::StandalonePackage {
                source_member,
                source_bytes,
                package_bytes,
                package_sha256,
            } => ProductInputIdentity::StandalonePackage {
                source_member,
                source_len: source_bytes.len(),
                package_len: package_bytes.len(),
                package_sha256,
            },
            Self::RetainedSourceMembers {
                descriptor,
                closure_sha256,
                ..
            } => ProductInputIdentity::RetainedSourceMembers {
                descriptor,
                closure_sha256,
            },
        }
    }
    pub(super) fn matches(&self, profile: &ProjectValidationProfile) -> io::Result<()> {
        match (self, &profile.product_input) {
            (
                Self::StandalonePackage {
                    source_bytes,
                    package_bytes,
                    package_sha256,
                    ..
                },
                ProjectProductSelector::StandalonePackage {
                    package_sha256: pin,
                },
            ) if package_sha256 == pin
                && digest(package_bytes) == *pin
                && digest(source_bytes) == profile.source_sha256 =>
            {
                Ok(())
            }
            (
                Self::RetainedSourceMembers {
                    descriptor,
                    member_bytes,
                    closure_sha256,
                },
                ProjectProductSelector::RetainedSourceMembers {
                    closure_sha256: pin,
                },
            ) if closure_sha256 == pin
                && descriptor.sha256()? == *pin
                && descriptor.program_id == profile.program_id
                && descriptor.members[1].source_sha256 == profile.source_sha256 =>
            {
                validate_members(descriptor, member_bytes, &profile.schema_namespace)
            }
            _ => Err(invalid("explicit product input variant or pin differs")),
        }
    }
    pub(super) fn install(
        &self,
        conn: &knx_productdb::Connection,
        project: &Project,
        device: DeviceId,
        profile: &ProjectValidationProfile,
    ) -> io::Result<()> {
        self.matches(profile)?;
        match self {
            Self::StandalonePackage {
                source_member,
                source_bytes,
                package_bytes,
                ..
            } => {
                validate_xml(source_bytes, &profile.schema_namespace)?;
                let mut archive = zip::ZipArchive::new(io::Cursor::new(package_bytes))
                    .map_err(io::Error::other)?;
                let mut expanded = 0u64;
                for index in 0..archive.len() {
                    expanded = expanded
                        .checked_add(archive.by_index(index).map_err(io::Error::other)?.size())
                        .filter(|n| *n <= LIMIT as u64)
                        .ok_or_else(|| {
                            invalid("retained package expansion exceeds replay budget")
                        })?;
                }
                let member = archive.by_name(source_member).map_err(io::Error::other)?;
                if member.size() != source_bytes.len() as u64 {
                    return Err(invalid("retained package/source membership differs"));
                }
                let mut actual = Vec::new();
                member
                    .take(source_bytes.len() as u64 + 1)
                    .read_to_end(&mut actual)?;
                if actual != *source_bytes {
                    return Err(invalid("retained package/source bytes differ"));
                }
                knx_productdb::install_package_with_limits(
                    conn,
                    "retained.knxprod",
                    package_bytes,
                    knx_productdb::PackageLimits {
                        max_member_size: LIMIT as u64,
                        max_expanded_size: LIMIT as u64,
                    },
                )
                .map_err(io::Error::other)?;
            }
            Self::RetainedSourceMembers {
                descriptor,
                member_bytes,
                ..
            } => {
                for (index, (member, bytes)) in
                    descriptor.members.iter().zip(member_bytes).enumerate()
                {
                    let expected = if index == 0 {
                        knx_productdb::FileKind::Hardware
                    } else {
                        knx_productdb::FileKind::ApplicationProgram
                    };
                    match knx_productdb::ingest_file(conn, &member.source_path, bytes)
                        .map_err(io::Error::other)?
                    {
                        knx_productdb::IngestOutcome::Ingested { sha256, kind, .. }
                            if sha256 == member.source_sha256 && kind == expected => {}
                        _ => {
                            return Err(invalid(
                                "fresh retained source ingest kind/hash differs or skipped",
                            ))
                        }
                    }
                }
                let (actual, _) = collect(conn, project, device, &profile.schema_namespace)?;
                if actual != **descriptor {
                    return Err(invalid("retained source owners/bridge did not reproduce"));
                }
            }
        }
        Ok(())
    }
}
