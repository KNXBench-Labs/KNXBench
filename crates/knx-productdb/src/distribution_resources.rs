//! Joins canonical baggage declarations to exact checked members, never native installation paths.

use super::{xml::Document, *};
use crate::baggage::{BaggagePayload, NestedArchive};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionResourceDeclaration {
    pub source: DistributionSourceRef,
    pub manufacturer_source: DistributionSourceRef,
    pub namespace: String,
    pub declared_manufacturer_id: Option<String>,
    pub id_presence: DistributionPresence,
    pub id: Option<String>,
    pub name: Option<String>,
    pub target_path: Option<String>,
    pub install_on_import: Option<String>,
    pub file_info: Vec<DistributionResourceFileInfo>,
    pub mapping_profile: String,
    pub resolution: String,
    pub expected_logical_path: Option<String>,
    pub member_candidates: Vec<u32>,
    pub media: Option<DistributionResourceMedia>,
    pub passive_presentation: DistributionPassivePresentation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionResourceFileInfo {
    pub source: DistributionSourceRef,
    pub attributes: Vec<DistributionDeclaredAttribute>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionResourceMedia {
    pub sha256: String,
    pub size: u64,
    pub content_class: String,
    pub extension_agrees: Option<bool>,
    pub archive_metadata: String,
    pub entries: Option<u64>,
    pub declared_expanded_size: Option<u64>,
    pub encrypted_entries: Option<u64>,
    pub archive_named_entries: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionPassivePresentation {
    pub state: String,
    pub reason: String,
    pub preview_member_ordinal: Option<u32>,
}
impl DistributionPassivePresentation {
    fn unavailable(reason: &str) -> Self {
        Self {
            state: "unavailable".into(),
            reason: reason.into(),
            preview_member_ordinal: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionResourcePass {
    pub association: String,
    pub native_package_baggage: String,
    pub help_rendering: String,
}

pub(super) fn collect(doc: &Document<'_>, report: &mut DistributionInspection) -> Result<()> {
    for (i, n) in doc.nodes.iter().enumerate() {
        if !doc.is_path(
            i,
            &[
                "KNX",
                "ManufacturerData",
                "Manufacturer",
                "Baggages",
                "Baggage",
            ],
        ) {
            continue;
        }
        let manufacturer = doc.manufacturer(i).expect("canonical baggage manufacturer");
        let id = n.attr("Id");
        let id_presence = match id.as_deref() {
            None => DistributionPresence::Absent,
            Some("") => DistributionPresence::PresentEmpty,
            Some(_) => DistributionPresence::PresentValue,
        };
        let mut file_info = Vec::new();
        for child in doc.children(i).filter(|&c| {
            doc.nodes[c].local == "FileInfo" && doc.nodes[c].ns.as_ref() == doc.namespace
        }) {
            let attributes = doc.nodes[child]
                .attrs
                .iter()
                .map(|a| DistributionDeclaredAttribute {
                    name: a.raw.clone(),
                    presence: if a.value.is_empty() {
                        DistributionPresence::PresentEmpty
                    } else {
                        DistributionPresence::PresentValue
                    },
                    value: Some(a.value.to_string()),
                })
                .collect();
            file_info.push(DistributionResourceFileInfo {
                source: doc.coordinate(child)?,
                attributes,
            });
        }
        doc.push(
            &mut report.resource_declarations,
            DistributionResourceDeclaration {
                source: doc.coordinate(i)?,
                manufacturer_source: doc.coordinate(manufacturer)?,
                namespace: doc.namespace.clone(),
                declared_manufacturer_id: doc.nodes[manufacturer].attr("RefId"),
                id_presence,
                id,
                name: n.attr("Name"),
                target_path: n.attr("TargetPath"),
                install_on_import: n.attr("InstallOnImport"),
                file_info,
                mapping_profile: "adjacent-xml-stem-baggages-v1".into(),
                resolution: if doc.member.is_some() {
                    "pending"
                } else {
                    "noArchiveEnvelope"
                }
                .into(),
                expected_logical_path: None,
                member_candidates: Vec::new(),
                media: None,
                passive_presentation: DistributionPassivePresentation::unavailable(
                    if doc.member.is_some() {
                        "pending"
                    } else {
                        "noArchiveEnvelope"
                    },
                ),
            },
        )?;
    }
    Ok(())
}

fn expected_path(
    xml: &str,
    d: &DistributionResourceDeclaration,
) -> std::result::Result<String, &'static str> {
    let stem = xml.strip_suffix(".xml").ok_or("mappingUnavailable")?;
    let name = d.name.as_deref().ok_or("unsafePath")?;
    let target = d
        .target_path
        .as_deref()
        .ok_or("unsafePath")?
        .replace('\\', "/");
    let safe = |s: &str| {
        !s.is_empty()
            && !matches!(s, "." | "..")
            && !s
                .chars()
                .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'))
    };
    if !safe(name) || (!target.is_empty() && !target.split('/').all(safe)) {
        return Err("unsafePath");
    }
    Ok(if target.is_empty() {
        format!("{stem}.baggages/{name}")
    } else {
        format!("{stem}.baggages/{target}/{name}")
    })
}

/// Resolve paths before reading any optional payload. Each selected ordinal is read only once.
pub(super) fn associate(report: &mut DistributionInspection) {
    let members = report
        .members
        .iter()
        .map(|m| (m.logical_path.as_str(), m.ordinal))
        .collect::<HashMap<_, _>>();
    for d in &mut report.resource_declarations {
        let Some(xml) = d
            .source
            .member_ordinal
            .and_then(|o| report.members.get(o as usize))
        else {
            continue;
        };
        match expected_path(&xml.logical_path, d) {
            Ok(path) => {
                if let Some(&ordinal) = members.get(path.as_str()) {
                    d.member_candidates.push(ordinal);
                    d.resolution = "resolved".into();
                } else {
                    d.resolution = "missingMember".into();
                }
                d.expected_logical_path = Some(path);
            }
            Err(reason) => d.resolution = reason.into(),
        }
        d.passive_presentation = DistributionPassivePresentation::unavailable(&d.resolution);
    }
}

pub(super) fn measure(path: String, sha: String, data: &[u8]) -> DistributionResourceMedia {
    let payload = BaggagePayload::measure(path, sha, data);
    let mut media = DistributionResourceMedia {
        sha256: payload.sha256.clone(),
        size: payload.size,
        content_class: payload.media_class.as_str().into(),
        extension_agrees: payload.extension_agrees(),
        archive_metadata: "notArchive".into(),
        entries: None,
        declared_expanded_size: None,
        encrypted_entries: None,
        archive_named_entries: None,
    };
    match payload.nested {
        NestedArchive::NotArchive => {}
        NestedArchive::Unreadable => media.archive_metadata = "archiveMetadataUnreadable".into(),
        NestedArchive::Read {
            entries,
            declared_expanded_size,
            encrypted_entries,
            archive_named_entries,
        } => {
            media.archive_metadata = "directoryMetadataOnly".into();
            media.entries = Some(entries);
            media.declared_expanded_size = Some(declared_expanded_size);
            media.encrypted_entries = Some(encrypted_entries);
            media.archive_named_entries = Some(archive_named_entries);
        }
    }
    media
}

pub(super) fn finish(report: &mut DistributionInspection) {
    report
        .limitations
        .retain(|l| *l != DistributionLimitation::BaggageAssociationNotPerformed);
    report
        .limitations
        .push(DistributionLimitation::NativePackageBaggageNotPerformed);
    report.resource_passes.push(DistributionResourcePass {
        association: if report.source_kind == DistributionSourceKind::DistributionZip {
            "performed"
        } else {
            "noArchiveEnvelope"
        }
        .into(),
        native_package_baggage: "notPerformed".into(),
        help_rendering: "notPerformed".into(),
    });
}
