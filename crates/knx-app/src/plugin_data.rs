//! Data-only adoption of plug-in bytes between two projects of one lineage.
//!
//! Moves manufacturer plug-in bytes from a project archive into a native project
//! (ADR-0025). ETS keeps a manufacturer plug-in editor's configuration in two places that
//! KNXBench stores but never interprets: the device's `BinaryData/<guid>.dat`
//! blob and the `ExtraData/<n>.*` table whose number sits in one of the
//! device's parameter values. Adopting means replacing the destination's bytes
//! for those entries with the source's bytes, verbatim. Nothing is decoded,
//! converted, merged or executed; there is no plug-in host here.
//!
//! Because the bytes are opaque, "is it safe to swap them?" can only be asked
//! of what the model *does* know. The plan therefore refuses unless both
//! devices have the same product/application references, the same plug-in
//! channel references and the same group-address links. Anything else would be
//! a guess about what the plug-in data embeds.
//!
//! The bytes a replacement supersedes stay in the project as retained opaque
//! entries, so nothing is discarded even if saved versions are pruned. The
//! opaque store is not part of the command undo stack: the rollback for an
//! adoption is the saved version the project history records on save.

use std::collections::BTreeMap;

use knx_core::{DeviceId, DeviceInstance, Direction, GroupAddressId, Project};
use knx_store::project_history::NativeSnapshot;
use knx_store::StoredOpaqueEntry;
use serde::{Deserialize, Serialize};

use crate::selective_import::Source;

/// Kind of the retained bytes a replacement superseded.
pub const SUPERSEDED_KIND: &str = "KnxBenchPluginDataSuperseded";
/// Kind of the per-adoption provenance record.
pub const ADOPTION_KIND: &str = "KnxBenchPluginDataAdoption";
const NATIVE_DIR: &str = "knxbench-native/plugin-data";
const RECORD_FORMAT: &str = "knxbench-plugin-data-adoption";
const RECORD_VERSION: u32 = 1;
/// A numeric parameter value that names an `ExtraData` file is short in every
/// observed project; anything longer is not treated as a file number.
const MAX_EXTRA_DATA_NUMBER_DIGITS: usize = 20;
const PRODUCT_REFERENCE: &str = "product reference";
const APPLICATION_REFERENCE: &str = "application program reference";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub source_device: u32,
    pub target_device: u32,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Channel {
    BinaryData,
    ExtraData,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Replacement {
    pub channel: Channel,
    /// Archive member path of the destination entry whose bytes are replaced.
    pub path: String,
    pub bytes_before: usize,
    pub bytes_after: usize,
    pub sha256_before: String,
    pub sha256_after: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSummary {
    pub id: u32,
    pub name: String,
    pub ets_id: String,
    pub address: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub source_hash: String,
    pub request: Request,
    pub source_device: DeviceSummary,
    pub target_device: DeviceSummary,
    /// Entries whose bytes already match and therefore stay untouched.
    pub unchanged_paths: Vec<String>,
    pub replacements: Vec<Replacement>,
    pub notes: Vec<String>,
}

/// Why a plan cannot be built. Each variant names the one fact that failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    UnknownDevice {
        side: &'static str,
        id: u32,
    },
    NoProjectDirectory {
        side: &'static str,
        device: String,
    },
    ProductDiffers {
        what: &'static str,
        source: String,
        target: String,
    },
    NoPluginData,
    BinaryDataReferencesDiffer,
    ExtraDataAssociationDiffers,
    GroupLinksDiffer,
    DanglingGroupLink {
        side: &'static str,
    },
    EntryMissing {
        side: &'static str,
        path: String,
    },
    EntryAmbiguous {
        side: &'static str,
        path: String,
    },
    AlreadyIdentical,
    Stale,
    Unreopenable(String),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownDevice { side, id } => write!(f, "{side} device {id} does not exist"),
            Self::NoProjectDirectory { side, device } => write!(
                f,
                "{side} device '{device}' has no project directory in its source path; its plug-in entries cannot be located"
            ),
            Self::ProductDiffers { what, source, target } => write!(
                f,
                "the devices differ in {what} ({source} vs {target}); plug-in bytes of another product or application version are not adopted"
            ),
            Self::NoPluginData => write!(
                f,
                "neither device carries BinaryData references or ExtraData-numbered parameters; there is nothing to adopt"
            ),
            Self::BinaryDataReferencesDiffer => write!(
                f,
                "the devices reference different BinaryData ids or names; remapping blobs would be a guess about the plug-in format"
            ),
            Self::ExtraDataAssociationDiffers => write!(
                f,
                "the devices associate different ExtraData files with their parameters; remapping file numbers would be a guess about the plug-in format"
            ),
            Self::GroupLinksDiffer => write!(
                f,
                "the devices link their communication objects to different group addresses; plug-in tables may embed those links, so the bytes are not adopted"
            ),
            Self::DanglingGroupLink { side } => write!(
                f,
                "a {side} communication object links a group address that does not exist"
            ),
            Self::EntryMissing { side, path } => {
                write!(f, "the {side} project retains no entry '{path}'")
            }
            Self::EntryAmbiguous { side, path } => {
                write!(f, "the {side} project retains several entries '{path}'")
            }
            Self::AlreadyIdentical => write!(
                f,
                "the destination already holds exactly these plug-in bytes; nothing to adopt"
            ),
            Self::Stale => write!(f, "plug-in adoption preview is stale; preview again"),
            Self::Unreopenable(reason) => {
                write!(f, "the adopted project would not reopen exactly: {reason}")
            }
        }
    }
}

impl std::error::Error for Refusal {}

/// A refusal as data: a stable `code` per class plus the facts its text
/// names, so an adapter can word it in any language. `message` is the
/// English text of [`Refusal`]'s `Display`, unchanged.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RefusalDetail {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl Refusal {
    /// Stable camelCase name of the refusal class; never changes with the text.
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownDevice { .. } => "unknownDevice",
            Self::NoProjectDirectory { .. } => "noProjectDirectory",
            Self::ProductDiffers { what, .. } if *what == PRODUCT_REFERENCE => {
                "productReferenceDiffers"
            }
            Self::ProductDiffers { .. } => "applicationReferenceDiffers",
            Self::NoPluginData => "noPluginData",
            Self::BinaryDataReferencesDiffer => "binaryDataReferencesDiffer",
            Self::ExtraDataAssociationDiffers => "extraDataAssociationDiffers",
            Self::GroupLinksDiffer => "groupLinksDiffer",
            Self::DanglingGroupLink { .. } => "danglingGroupLink",
            Self::EntryMissing { .. } => "entryMissing",
            Self::EntryAmbiguous { .. } => "entryAmbiguous",
            Self::AlreadyIdentical => "alreadyIdentical",
            Self::Stale => "stale",
            Self::Unreopenable(_) => "unreopenable",
        }
    }

    pub fn detail(&self) -> RefusalDetail {
        let mut detail = RefusalDetail {
            code: self.code(),
            message: self.to_string(),
            ..RefusalDetail::default()
        };
        match self {
            Self::UnknownDevice { side, id } => {
                detail.side = Some(side);
                detail.id = Some(*id);
            }
            Self::NoProjectDirectory { side, device } => {
                detail.side = Some(side);
                detail.device = Some(device.clone());
            }
            Self::ProductDiffers { source, target, .. } => {
                detail.source_value = Some(source.clone());
                detail.target_value = Some(target.clone());
            }
            Self::DanglingGroupLink { side } => detail.side = Some(side),
            Self::EntryMissing { side, path } | Self::EntryAmbiguous { side, path } => {
                detail.side = Some(side);
                detail.path = Some(path.clone());
            }
            Self::Unreopenable(reason) => detail.reason = Some(reason.clone()),
            Self::NoPluginData
            | Self::BinaryDataReferencesDiffer
            | Self::ExtraDataAssociationDiffers
            | Self::GroupLinksDiffer
            | Self::AlreadyIdentical
            | Self::Stale => {}
        }
        detail
    }
}

/// What `plan` needs of a source: a project and where its archive entries
/// live. Kept separate from `Source` so the rules are testable without an
/// archive.
pub struct SourceView<'a> {
    pub project: &'a Project,
    pub opaque: &'a [StoredOpaqueEntry],
    /// Prepended to an archive member path to find its entry in `opaque`.
    pub entry_prefix: String,
    pub source_hash: &'a str,
}

impl Source {
    pub fn plugin_data_view(&self) -> SourceView<'_> {
        // Selective-import entries are stored as `{prefix}/{member}`.
        SourceView {
            project: &self.project,
            opaque: &self.opaque,
            entry_prefix: format!("{}/", self.prefix),
            source_hash: &self.source_hash,
        }
    }
}

pub struct Plan {
    pub preview: Preview,
    before: NativeSnapshot,
    after: NativeSnapshot,
}

impl std::fmt::Debug for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Plan")
            .field("source_hash", &self.preview.source_hash)
            .field("replacements", &self.preview.replacements.len())
            .finish_non_exhaustive()
    }
}

impl Plan {
    /// Binds every effect, the source, the request and the caller scope.
    pub fn confirmation_token(&self, binding: &str) -> Result<String, String> {
        let payload = serde_json::json!({
            "version": 1,
            "binding": binding,
            "source": self.preview.source_hash,
            "request": self.preview.request,
            "before": self.before.semantic_hash().map_err(|e| e.to_string())?,
            "after": self.after.semantic_hash().map_err(|e| e.to_string())?,
        });
        Ok(knx_etsproj::opaque::sha256_hex(
            &serde_json::to_vec(&payload).map_err(|e| e.to_string())?,
        ))
    }
}

/// One candidate pair found by [`candidates`].
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub source_device: DeviceSummary,
    pub target_device: DeviceSummary,
    /// `None` when a plan can be built, otherwise the refusal text.
    pub refusal: Option<String>,
    /// The same refusal as code and facts, for adapters that word it.
    pub refusal_detail: Option<RefusalDetail>,
    pub replacements: usize,
}

/// Pairs devices that carry the same ETS device id in both projects and says,
/// for each pair, whether adoption is possible. Read-only.
pub fn candidates(target: &NativeSnapshot, source: &Source) -> Vec<Candidate> {
    candidates_from(target, &source.plugin_data_view())
}

pub fn candidates_from(target: &NativeSnapshot, source: &SourceView<'_>) -> Vec<Candidate> {
    let mut found = Vec::new();
    for source_device in source.project.devices.iter() {
        for target_device in target.project.devices.iter() {
            if source_device.source.ets_id != target_device.source.ets_id {
                continue;
            }
            let request = Request {
                source_device: source_device.id.0,
                target_device: target_device.id.0,
            };
            let (refusal, replacements) = match plan_from(target, source, request) {
                Ok(plan) => (None, plan.preview.replacements.len()),
                Err(Refusal::NoPluginData) => continue,
                Err(refusal) => (Some(refusal), 0),
            };
            found.push(Candidate {
                source_device: summary(source_device),
                target_device: summary(target_device),
                refusal: refusal.as_ref().map(Refusal::to_string),
                refusal_detail: refusal.as_ref().map(Refusal::detail),
                replacements,
            });
        }
    }
    found
}

pub fn plan(target: &NativeSnapshot, source: &Source, request: Request) -> Result<Plan, Refusal> {
    plan_from(target, &source.plugin_data_view(), request)
}

pub fn plan_from(
    target: &NativeSnapshot,
    source: &SourceView<'_>,
    request: Request,
) -> Result<Plan, Refusal> {
    let source_device = source
        .project
        .devices
        .get(DeviceId(request.source_device))
        .ok_or(Refusal::UnknownDevice {
            side: "source",
            id: request.source_device,
        })?;
    let target_device = target
        .project
        .devices
        .get(DeviceId(request.target_device))
        .ok_or(Refusal::UnknownDevice {
            side: "destination",
            id: request.target_device,
        })?;

    if source_device.product_ref != target_device.product_ref {
        return Err(Refusal::ProductDiffers {
            what: PRODUCT_REFERENCE,
            source: source_device.product_ref.clone(),
            target: target_device.product_ref.clone(),
        });
    }
    if source_device.program_ref != target_device.program_ref {
        return Err(Refusal::ProductDiffers {
            what: APPLICATION_REFERENCE,
            source: source_device.program_ref.clone(),
            target: target_device.program_ref.clone(),
        });
    }
    let source_dir = project_dir(source_device, "source")?;
    let target_dir = project_dir(target_device, "destination")?;

    let source_extra = extra_data_association(
        source.project,
        source_device,
        source.opaque,
        &source.entry_prefix,
        source_dir,
    )?;
    let target_extra = extra_data_association(
        &target.project,
        target_device,
        &target.opaque,
        "",
        target_dir,
    )?;
    if source_device.binary_data.is_empty()
        && target_device.binary_data.is_empty()
        && source_extra.is_empty()
        && target_extra.is_empty()
    {
        return Err(Refusal::NoPluginData);
    }
    if source_device.binary_data != target_device.binary_data {
        return Err(Refusal::BinaryDataReferencesDiffer);
    }
    if source_extra != target_extra {
        return Err(Refusal::ExtraDataAssociationDiffers);
    }
    if group_links(source.project, source_device, "source")?
        != group_links(&target.project, target_device, "destination")?
    {
        return Err(Refusal::GroupLinksDiffer);
    }

    // (channel, archive path in the destination project, same path in source)
    let mut members: Vec<(Channel, String, String)> = Vec::new();
    for blob in &target_device.binary_data {
        members.push((
            Channel::BinaryData,
            format!("{target_dir}/BinaryData/{}.dat", blob.id),
            format!("{source_dir}/BinaryData/{}.dat", blob.id),
        ));
    }
    for (_, _, file) in &target_extra {
        members.push((
            Channel::ExtraData,
            format!("{target_dir}/ExtraData/{file}"),
            format!("{source_dir}/ExtraData/{file}"),
        ));
    }

    let mut replacements = Vec::new();
    let mut unchanged_paths = Vec::new();
    let mut new_bytes: Vec<(String, Vec<u8>)> = Vec::new();
    for (channel, target_path, source_path) in members {
        let wanted = format!("{}{source_path}", source.entry_prefix);
        let new = unique(source.opaque, &wanted, &source_path, "source")?;
        let old = unique(&target.opaque, &target_path, &target_path, "destination")?;
        if new.bytes == old.bytes {
            unchanged_paths.push(target_path);
            continue;
        }
        replacements.push(Replacement {
            channel,
            path: target_path.clone(),
            bytes_before: old.bytes.len(),
            bytes_after: new.bytes.len(),
            sha256_before: knx_etsproj::opaque::sha256_hex(&old.bytes),
            sha256_after: knx_etsproj::opaque::sha256_hex(&new.bytes),
        });
        new_bytes.push((target_path, new.bytes.clone()));
    }
    if replacements.is_empty() {
        return Err(Refusal::AlreadyIdentical);
    }

    let preview = Preview {
        source_hash: source.source_hash.to_string(),
        request,
        source_device: summary(source_device),
        target_device: summary(target_device),
        unchanged_paths,
        replacements,
        notes: notes(source_device, target_device),
    };
    let after = apply_to_copy(target, &preview, new_bytes)?;
    Ok(Plan {
        preview,
        before: target.clone(),
        after,
    })
}

/// Replaces the planned entries on a copy, retains what they supersede and
/// appends the provenance record. Everything else is carried verbatim.
fn apply_to_copy(
    base: &NativeSnapshot,
    preview: &Preview,
    new_bytes: Vec<(String, Vec<u8>)>,
) -> Result<NativeSnapshot, Refusal> {
    let mut after = base.clone();
    let mut retained = Vec::new();
    for (path, bytes) in new_bytes {
        let entry = after
            .opaque
            .iter_mut()
            .find(|e| e.xpath.is_empty() && e.source_path == path)
            .ok_or_else(|| Refusal::EntryMissing {
                side: "destination",
                path: path.clone(),
            })?;
        let previous = std::mem::replace(&mut entry.bytes, bytes.clone());
        entry.sha256 = knx_etsproj::opaque::sha256_hex(&bytes);
        retained.push(StoredOpaqueEntry {
            source_path: format!(
                "{NATIVE_DIR}/superseded/{}/{path}",
                knx_etsproj::opaque::sha256_hex(&previous)
            ),
            xpath: String::new(),
            kind: SUPERSEDED_KIND.into(),
            name: path,
            sha256: knx_etsproj::opaque::sha256_hex(&previous),
            bytes: previous,
        });
    }
    for entry in retained {
        if !after.opaque.contains(&entry) {
            after.opaque.push(entry);
        }
    }
    let record = serde_json::to_vec_pretty(&serde_json::json!({
        "format": RECORD_FORMAT,
        "version": RECORD_VERSION,
        "sourceArchiveSha256": preview.source_hash,
        "sourceDevice": preview.source_device,
        "targetDevice": preview.target_device,
        "replacements": preview.replacements,
        "interpretation": "none: bytes were copied verbatim; their meaning, ETS acceptance and device behaviour are not verified",
    }))
    .map_err(|e| Refusal::Unreopenable(e.to_string()))?;
    let digest = knx_etsproj::opaque::sha256_hex(&record);
    after.opaque.push(StoredOpaqueEntry {
        source_path: format!("{NATIVE_DIR}/adoption-{digest}.json"),
        xpath: String::new(),
        kind: ADOPTION_KIND.into(),
        name: "adoption-record.json".into(),
        sha256: digest,
        bytes: record,
    });
    after
        .encode()
        .map_err(|e| Refusal::Unreopenable(e.to_string()))?;
    Ok(after)
}

/// Replaces the destination only if it is still exactly what was previewed.
/// The command undo stack is untouched: the opaque store is outside it.
pub fn apply(target: &mut NativeSnapshot, plan: Plan) -> Result<Preview, Refusal> {
    if *target != plan.before {
        return Err(Refusal::Stale);
    }
    *target = plan.after;
    Ok(plan.preview)
}

fn summary(device: &DeviceInstance) -> DeviceSummary {
    DeviceSummary {
        id: device.id.0,
        name: device.name.clone(),
        ets_id: device.source.ets_id.clone(),
        address: device.address.map(|a| a.to_string()),
    }
}

fn notes(source: &DeviceInstance, target: &DeviceInstance) -> Vec<String> {
    let mut notes = vec![
        "Bytes are copied verbatim and never decoded; their meaning and ETS acceptance are not verified.".to_string(),
        "Superseded bytes stay in the project as retained entries; roll back through the saved version recorded on save.".to_string(),
        "KNXBench writes no .knxproj (ADR-0028): these bytes can be read back from the project, not re-imported into ETS by KNXBench.".to_string(),
    ];
    if source.address != target.address {
        notes.push(format!(
            "The devices carry different individual addresses ({} vs {}); the plug-in bytes may embed the source address.",
            source.address.map_or("none".into(), |a| a.to_string()),
            target.address.map_or("none".into(), |a| a.to_string()),
        ));
    }
    notes
}

/// The folder of the device's source XML: `P-0512` for `P-0512/0.xml`, and
/// `imports/<hash>/P-0512` for a device that arrived through selective import.
/// Its `BinaryData/` and `ExtraData/` folders sit beside the XML.
fn project_dir<'a>(device: &'a DeviceInstance, side: &'static str) -> Result<&'a str, Refusal> {
    device
        .source
        .path
        .rsplit_once('/')
        .map(|(dir, _)| dir)
        .filter(|dir| !dir.is_empty())
        .ok_or_else(|| Refusal::NoProjectDirectory {
            side,
            device: device.name.clone(),
        })
}

fn unique<'a>(
    entries: &'a [StoredOpaqueEntry],
    path: &str,
    member: &str,
    side: &'static str,
) -> Result<&'a StoredOpaqueEntry, Refusal> {
    let mut found = entries
        .iter()
        .filter(|e| e.xpath.is_empty() && e.source_path == path);
    let first = found.next().ok_or_else(|| Refusal::EntryMissing {
        side,
        path: member.to_string(),
    })?;
    if found.next().is_some() {
        return Err(Refusal::EntryAmbiguous {
            side,
            path: member.to_string(),
        });
    }
    Ok(first)
}

/// (parameter reference id, value, `ExtraData` file name) for each parameter
/// of `device` whose value is a number naming a retained `ExtraData` file.
/// That the number selects the file is observed, not documented ETS behaviour
/// (docs/research); a coincidental match is harmless because both sides must
/// agree on it and a file that does not differ is not touched.
type ExtraAssociation = Vec<(String, String, String)>;

fn extra_data_association(
    project: &Project,
    device: &DeviceInstance,
    opaque: &[StoredOpaqueEntry],
    entry_prefix: &str,
    dir: &str,
) -> Result<ExtraAssociation, Refusal> {
    let folder = format!("{entry_prefix}{dir}/ExtraData/");
    let files: Vec<&str> = opaque
        .iter()
        .filter(|e| e.xpath.is_empty())
        .filter_map(|e| e.source_path.strip_prefix(&folder))
        .filter(|name| !name.contains('/'))
        .collect();
    let mut found = ExtraAssociation::new();
    for parameter in project
        .installations
        .iter()
        .flat_map(|i| i.parameters.iter())
        .filter(|p| p.device == device.id)
    {
        let value = parameter.raw.trim();
        if value.is_empty()
            || value.len() > MAX_EXTRA_DATA_NUMBER_DIGITS
            || !value.bytes().all(|b| b.is_ascii_digit())
        {
            continue;
        }
        let matching: Vec<&&str> = files
            .iter()
            .filter(|name| name.rsplit_once('.').map_or(**name, |(stem, _)| stem) == value)
            .collect();
        match matching.as_slice() {
            [] => {}
            [name] => found.push((
                parameter.source.ets_id.clone(),
                value.to_string(),
                (**name).to_string(),
            )),
            _ => {
                return Err(Refusal::EntryAmbiguous {
                    side: "project",
                    path: format!("{folder}{value}.*"),
                })
            }
        }
    }
    found.sort();
    found.dedup();
    Ok(found)
}

/// (communication object number, group address, direction) of every link of
/// the device, sorted, so two projects compare by address and not by internal id.
fn group_links(
    project: &Project,
    device: &DeviceInstance,
    side: &'static str,
) -> Result<Vec<(u16, u16, u8)>, Refusal> {
    let addresses: BTreeMap<GroupAddressId, u16> = project
        .installations
        .iter()
        .flat_map(|i| i.group_addresses.iter())
        .map(|g| (g.id, g.address.raw()))
        .collect();
    let mut links = Vec::new();
    for id in &device.com_objects {
        let Some(object) = project.devices.com_object(*id) else {
            continue;
        };
        for link in &object.links {
            let address = addresses
                .get(&link.ga)
                .ok_or(Refusal::DanglingGroupLink { side })?;
            links.push((
                object.number,
                *address,
                match link.direction {
                    Direction::Send => 0,
                    Direction::Receive => 1,
                },
            ));
        }
    }
    links.sort_unstable();
    Ok(links)
}
