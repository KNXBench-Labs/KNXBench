//! Complete raw and public NDJSON reports retain bounded publication evidence.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::OnceLock;

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::dto::{self, Artifact, ArtifactKind, FailureKind, Projection};
use super::persistence::{bounded_json, JobWorkspace};
use super::strict::Sha256 as WireSha256;
use super::{JobId, U64};

pub const ARTIFACT_BYTES: u64 = 67_108_864;
pub const ARTIFACT_RECORDS: u64 = 100_000;
pub const RECORD_BYTES: usize = 65_536;
pub const PAGE_BYTES: usize = 262_144;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: u32,
    kind: ArtifactKind,
    artifact_id: JobId,
    records: U64,
    raw_bytes: U64,
    public_bytes: U64,
    index_bytes: U64,
    raw_sha256: WireSha256,
    public_sha256: WireSha256,
    index_sha256: WireSha256,
    withheld_fields: U64,
    withheld_records: U64,
    withheld_occurrences: U64,
    complete: bool,
}

pub struct ArtifactWriter<'a> {
    workspace: &'a mut JobWorkspace,
    kind: ArtifactKind,
    id: JobId,
    raw: File,
    public: File,
    index: File,
    hashes: [Sha256; 3],
    lengths: [u64; 3],
    records: u64,
    withheld: [u64; 3],
}
impl<'a> ArtifactWriter<'a> {
    pub fn new(workspace: &'a mut JobWorkspace, kind: ArtifactKind) -> Result<Self, FailureKind> {
        let stem = stem(kind);
        let id = JobId::try_mint().map_err(|_| FailureKind::IoFailure)?;
        let raw = workspace
            .create_file(&format!("{stem}.raw.part"))
            .map_err(io)?;
        let public = workspace
            .create_file(&format!("{stem}.public.part"))
            .map_err(io)?;
        let index = workspace
            .create_file(&format!("{stem}.index.part"))
            .map_err(io)?;
        Ok(Self {
            workspace,
            kind,
            id,
            raw,
            public,
            index,
            hashes: std::array::from_fn(|_| Sha256::new()),
            lengths: [0; 3],
            records: 0,
            withheld: [0; 3],
        })
    }

    /// Borrowed serde adapters from the existing installer can emit every row
    /// without cloning a complete report. A single record is counted before its
    /// bounded scratch materialization; both mirrors/index/manifest are charged
    /// together before the first persistent write. No first-N truncation.
    pub fn append<T: Serialize>(&mut self, record: &T) -> Result<(), FailureKind> {
        if self.workspace.artifact_records >= ARTIFACT_RECORDS {
            return Err(FailureKind::ResultLimit);
        }
        let candidate = bounded_json(record, RECORD_BYTES - 1)?;
        let text = std::str::from_utf8(&candidate).map_err(|_| FailureKind::ReportMismatch)?;
        let value: Value = dto::from_str(text).map_err(|_| FailureKind::ReportMismatch)?;
        let kind = value["kind"].as_str().ok_or(FailureKind::ReportMismatch)?;
        let raw_schema = definition(&format!("Raw_{kind}"))?;
        validate(&value, raw_schema)?;
        if value["ordinal"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
            != Some(self.records)
        {
            return Err(FailureKind::ReportMismatch);
        }
        let summary = match self.kind {
            ArtifactKind::Preview => "previewSummary",
            ArtifactKind::Result => "resultSummary",
        };
        if (self.records == 0) != (kind == summary)
            || matches!(kind, "previewSummary" | "resultSummary") && kind != summary
        {
            return Err(FailureKind::ReportMismatch);
        }
        let fields = withheld_fields(kind);
        let mut withheld = Vec::new();
        for &field in fields {
            if let Some(text) = value[field].as_str() {
                withheld.push(Withheld {
                    field,
                    reason: "untrustedFreeText",
                    raw_utf8_bytes: U64(text.len() as u64),
                });
            }
        }
        let projected = Public {
            value: &value,
            schema: raw_schema,
            fields,
            withheld: &withheld,
        };
        let mut raw = bounded_json(
            &Ordered {
                value: &value,
                schema: raw_schema,
            },
            RECORD_BYTES - 1,
        )?;
        let mut public = bounded_json(&projected, RECORD_BYTES - 1)?;
        // Validate every structured public identity/path against the accepted
        // record schema BEFORE publication, never a heuristic string scrub.
        let public_value: Value =
            serde_json::from_slice(&public).map_err(|_| FailureKind::ReportMismatch)?;
        validate(&public_value, definition(&format!("Public_{kind}"))?)
            .map_err(|_| FailureKind::PublicProjectionRefused)?;
        validate_public_paths(&public_value)?;
        validate_semantics(&value)?;
        raw.push(b'\n');
        public.push(b'\n');
        let occurrences = if withheld.is_empty() {
            0
        } else {
            value
                .get("occurrences")
                .or_else(|| value.get("occurrence"))
                .and_then(Value::as_str)
                .map(|v| v.parse::<u64>())
                .transpose()
                .map_err(|_| FailureKind::ReportMismatch)?
                .unwrap_or(1)
        };
        let new_withheld = [
            add(self.withheld[0], withheld.len() as u64)?,
            add(self.withheld[1], u64::from(!withheld.is_empty()))?,
            add(self.withheld[2], occurrences)?,
        ];
        let new_lengths = [
            add(self.lengths[0], raw.len() as u64)?,
            add(self.lengths[1], public.len() as u64)?,
            add(self.lengths[2], 32)?,
        ];
        let manifest_size = bounded_json(
            &self.manifest(new_lengths, self.records + 1, new_withheld),
            16_384,
        )?
        .len() as u64;
        let row_charge = add(add(raw.len() as u64, public.len() as u64)?, 32)?;
        let prospective = add(
            add(self.workspace.artifact_bytes, row_charge)?,
            manifest_size,
        )?;
        if prospective > ARTIFACT_BYTES {
            return Err(FailureKind::ResultLimit);
        }
        let mut index = [0u8; 32];
        for (slot, n) in index.as_chunks_mut::<8>().0.iter_mut().zip([
            self.lengths[0],
            raw.len() as u64,
            self.lengths[1],
            public.len() as u64,
        ]) {
            slot.copy_from_slice(&n.to_le_bytes());
        }
        // Charge first. A failed/partial write keeps ownership and its charge.
        self.workspace.artifact_bytes = add(self.workspace.artifact_bytes, row_charge)?;
        self.workspace.artifact_records = add(self.workspace.artifact_records, 1)?;
        self.raw
            .write_all(&raw)
            .and_then(|_| self.public.write_all(&public))
            .and_then(|_| self.index.write_all(&index))
            .map_err(|_| FailureKind::IoFailure)?;
        for (hash, bytes) in
            self.hashes
                .iter_mut()
                .zip([raw.as_slice(), public.as_slice(), index.as_slice()])
        {
            hash.update(bytes);
        }
        self.lengths = new_lengths;
        self.withheld = new_withheld;
        self.records += 1;
        Ok(())
    }

    fn manifest(&self, lengths: [u64; 3], records: u64, withheld: [u64; 3]) -> Manifest {
        Manifest {
            schema_version: 1,
            kind: self.kind,
            artifact_id: self.id,
            records: U64(records),
            raw_bytes: U64(lengths[0]),
            public_bytes: U64(lengths[1]),
            index_bytes: U64(lengths[2]),
            raw_sha256: WireSha256::from_bytes(self.hashes[0].clone().finalize().into()),
            public_sha256: WireSha256::from_bytes(self.hashes[1].clone().finalize().into()),
            index_sha256: WireSha256::from_bytes(self.hashes[2].clone().finalize().into()),
            withheld_fields: U64(withheld[0]),
            withheld_records: U64(withheld[1]),
            withheld_occurrences: U64(withheld[2]),
            complete: true,
        }
    }

    pub fn finish(self) -> Result<Artifact, FailureKind> {
        if self.records == 0 {
            return Err(FailureKind::ReportMismatch);
        }
        let manifest = self.manifest(self.lengths, self.records, self.withheld);
        let bytes = bounded_json(&manifest, 16_384)?;
        if add(self.workspace.artifact_bytes, bytes.len() as u64)? > ARTIFACT_BYTES {
            return Err(FailureKind::ResultLimit);
        }
        self.raw
            .sync_all()
            .and_then(|_| self.public.sync_all())
            .and_then(|_| self.index.sync_all())
            .map_err(|_| FailureKind::IoFailure)?;
        let stem = stem(self.kind);
        for (i, suffix) in ["raw", "public", "index"].iter().enumerate() {
            let part = format!("{stem}.{suffix}.part");
            let mut file = self.workspace.open_artifact(&part).map_err(io)?;
            if file.metadata().map_err(|_| FailureKind::IoFailure)?.len() != self.lengths[i] {
                return Err(FailureKind::ReportMismatch);
            }
            let mut hash = Sha256::new();
            let mut buffer = [0u8; 65_536];
            loop {
                let n = file.read(&mut buffer).map_err(|_| FailureKind::IoFailure)?;
                if n == 0 {
                    break;
                }
                hash.update(&buffer[..n]);
            }
            if hash.finalize()[..] != self.hashes[i].clone().finalize()[..] {
                return Err(FailureKind::ReportMismatch);
            }
        }
        // Close application writers before making complete basenames visible.
        drop(self.raw);
        drop(self.public);
        drop(self.index);
        for suffix in ["raw", "public", "index"] {
            self.workspace
                .seal_file(
                    &format!("{stem}.{suffix}.part"),
                    &format!("{stem}.{suffix}"),
                )
                .map_err(io)?;
        }
        let part = format!("{stem}.manifest.part");
        let mut file = self.workspace.create_file(&part).map_err(io)?;
        self.workspace.artifact_bytes = add(self.workspace.artifact_bytes, bytes.len() as u64)?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| FailureKind::IoFailure)?;
        drop(file);
        self.workspace
            .seal_file(&part, &format!("{stem}.manifest"))
            .map_err(io)?;
        let descriptor = Artifact {
            artifact_id: self.id,
            kind: self.kind,
            public_sha256: manifest.public_sha256,
            public_bytes: manifest.public_bytes,
            records: manifest.records,
            withheld_fields: manifest.withheld_fields,
            withheld_records: manifest.withheld_records,
            withheld_occurrences: manifest.withheld_occurrences,
            projection: Projection::RedactedCompleteV1,
        };
        self.workspace.published[match self.kind {
            ArtifactKind::Preview => 0,
            ArtifactKind::Result => 1,
        }] = Some(descriptor.clone());
        Ok(descriptor)
    }
}

pub fn detail_page(
    workspace: &JobWorkspace,
    descriptor: &Artifact,
    offset: U64,
    limit: usize,
) -> Result<Vec<u8>, FailureKind> {
    if limit == 0 || limit > 200 || offset > descriptor.records {
        return Err(FailureKind::InvalidRequest);
    }
    let head = serde_json::json!({"schemaVersion":1,"handle": dto::Handle { server_incarnation: workspace.handle().incarnation, admission_serial: workspace.handle().serial, job_id: workspace.handle().job }, "artifactId":descriptor.artifact_id,"publicSha256":descriptor.public_sha256,"offset":offset,"totalRecords":descriptor.records});
    // Explicit schema field order, records appended as already validated compact JSON.
    let mut prefix = format!("{{\"schemaVersion\":1,\"handle\":{},\"artifactId\":\"{}\",\"publicSha256\":\"{}\",\"offset\":\"{}\",\"totalRecords\":\"{}\",\"records\":[", serde_json::to_string(&head["handle"]).map_err(|_| FailureKind::ReportMismatch)?, descriptor.artifact_id, descriptor.public_sha256, offset, descriptor.records).into_bytes();
    let stem = stem(descriptor.kind);
    // Manifest existence/identity is the publication boundary, not complete
    // basenames left by a crash between renames.
    let manifest = workspace
        .open_artifact(&format!("{stem}.manifest"))
        .map_err(io)?;
    if manifest
        .metadata()
        .map_err(|_| FailureKind::IoFailure)?
        .len()
        > 16_384
    {
        return Err(FailureKind::ReportMismatch);
    }
    let mut index = workspace
        .open_artifact(&format!("{stem}.index"))
        .map_err(io)?;
    let mut public = workspace
        .open_artifact(&format!("{stem}.public"))
        .map_err(io)?;
    if index.metadata().map_err(|_| FailureKind::IoFailure)?.len()
        != descriptor
            .records
            .get()
            .checked_mul(32)
            .ok_or(FailureKind::ReportMismatch)?
        || public.metadata().map_err(|_| FailureKind::IoFailure)?.len()
            != descriptor.public_bytes.get()
    {
        return Err(FailureKind::ReportMismatch);
    }
    let count = usize::try_from((descriptor.records.get() - offset.get()).min(limit as u64))
        .map_err(|_| FailureKind::InvalidRequest)?;
    index
        .seek(SeekFrom::Start(
            offset
                .get()
                .checked_mul(32)
                .ok_or(FailureKind::InvalidRequest)?,
        ))
        .map_err(|_| FailureKind::IoFailure)?;
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(count)
        .map_err(|_| FailureKind::ResultLimit)?;
    let mut selected_bytes = prefix.len();
    for _ in 0..count {
        let mut entry = [0u8; 32];
        index
            .read_exact(&mut entry)
            .map_err(|_| FailureKind::IoFailure)?;
        let start = u64::from_le_bytes(entry[16..24].try_into().unwrap());
        let len = u64::from_le_bytes(entry[24..32].try_into().unwrap());
        if len < 2
            || len > RECORD_BYTES as u64
            || start
                .checked_add(len)
                .is_none_or(|end| end > descriptor.public_bytes.get())
        {
            return Err(FailureKind::ReportMismatch);
        }
        // Longest possible nextOffset suffix fits in 48 bytes. Pick maximal
        // prefix by the EXACT candidate suffix, not a guessed fixed reserve.
        let next = offset.get() + entries.len() as u64 + 1;
        let suffix = page_suffix(next, descriptor.records.get());
        let candidate = selected_bytes
            .checked_add(len as usize - 1 + usize::from(!entries.is_empty()))
            .and_then(|n| n.checked_add(suffix.len()))
            .ok_or(FailureKind::ResultLimit)?;
        if candidate > PAGE_BYTES {
            break;
        }
        selected_bytes += len as usize - 1 + usize::from(!entries.is_empty());
        entries.push((start, len as usize));
    }
    if entries.is_empty() && offset < descriptor.records {
        return Err(FailureKind::ReportMismatch);
    }
    prefix
        .try_reserve_exact(selected_bytes + 48 - prefix.len())
        .map_err(|_| FailureKind::ResultLimit)?;
    for (i, (start, len)) in entries.iter().enumerate() {
        public
            .seek(SeekFrom::Start(*start))
            .map_err(|_| FailureKind::IoFailure)?;
        let mut scratch = vec![0u8; *len];
        public
            .read_exact(&mut scratch)
            .map_err(|_| FailureKind::IoFailure)?;
        if scratch.pop() != Some(b'\n') {
            return Err(FailureKind::ReportMismatch);
        }
        let row: Value =
            serde_json::from_slice(&scratch).map_err(|_| FailureKind::ReportMismatch)?;
        if row["ordinal"].as_str().and_then(|s| s.parse::<u64>().ok())
            != Some(offset.get() + i as u64)
        {
            return Err(FailureKind::ReportMismatch);
        }
        if i > 0 {
            prefix.push(b',');
        }
        prefix.extend_from_slice(&scratch);
    }
    prefix.extend_from_slice(
        page_suffix(
            offset.get() + entries.len() as u64,
            descriptor.records.get(),
        )
        .as_bytes(),
    );
    if prefix.len() > PAGE_BYTES {
        return Err(FailureKind::ResultLimit);
    }
    Ok(prefix)
}
fn page_suffix(next: u64, total: u64) -> String {
    if next < total {
        format!("],\"nextOffset\":\"{next}\"}}")
    } else {
        "],\"nextOffset\":null}".into()
    }
}
fn io(_: super::artifacts::ArtifactError) -> FailureKind {
    FailureKind::IoFailure
}
fn add(a: u64, b: u64) -> Result<u64, FailureKind> {
    a.checked_add(b).ok_or(FailureKind::ResultLimit)
}
fn stem(kind: ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Preview => "preview",
        ArtifactKind::Result => "result",
    }
}

fn schemas() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../docs/contracts/catalogue-jobs/wp043-artifact-records.schema.json"
        ))
        .expect("checked-in artifact schema")
    })
}
fn definition(name: &str) -> Result<&'static Value, FailureKind> {
    schemas()["$defs"]
        .get(name)
        .ok_or(FailureKind::ReportMismatch)
}
fn resolve(schema: &Value) -> &Value {
    if let Some(reference) = schema["$ref"].as_str() {
        &schemas()["$defs"][reference.rsplit('/').next().unwrap()]
    } else {
        schema
    }
}

/// Only the closed vocabulary used by the checked-in record schema. This is
/// deliberately not a new general JSON-Schema framework.
fn validate(value: &Value, schema: &Value) -> Result<(), FailureKind> {
    let schema = resolve(schema);
    let bad = FailureKind::ReportMismatch;
    if let Some(choices) = schema["anyOf"].as_array() {
        return if choices.iter().any(|s| validate(value, s).is_ok()) {
            Ok(())
        } else {
            Err(bad)
        };
    }
    if let Some(c) = schema.get("const") {
        if value != c {
            return Err(bad);
        }
    }
    if let Some(choices) = schema["enum"].as_array() {
        if !choices.contains(value) {
            return Err(bad);
        }
    }
    if let Some(t) = schema["type"].as_str() {
        let valid = match t {
            "null" => value.is_null(),
            "boolean" => value.is_boolean(),
            "string" => value.is_string(),
            "integer" => value.as_u64().is_some(),
            "object" => value.is_object(),
            "array" => value.is_array(),
            _ => false,
        };
        if !valid {
            return Err(bad);
        }
    }
    if let Some(s) = value.as_str() {
        if schema["x-maxUtf8Bytes"]
            .as_u64()
            .is_some_and(|n| s.len() as u64 > n)
        {
            return Err(bad);
        }
        if let Some(pattern) = schema["pattern"].as_str() {
            let okay = match pattern {
                "^(0|[1-9][0-9]{0,19})$" => s.parse::<U64>().is_ok(),
                "^[0-9a-f]{32}$" => s.parse::<JobId>().is_ok(),
                "^[0-9a-f]{64}$" => s.parse::<WireSha256>().is_ok(),
                "^[A-Za-z0-9_][A-Za-z0-9_.-]*$" => safe_id(s),
                _ => false,
            };
            if !okay {
                return Err(bad);
            }
        }
    }
    if let Some(n) = value.as_u64() {
        if schema["minimum"].as_u64().is_some_and(|min| n < min)
            || schema["maximum"].as_u64().is_some_and(|max| n > max)
        {
            return Err(bad);
        }
    }
    if let Some(map) = value.as_object() {
        let props = schema["properties"].as_object().ok_or(bad)?;
        if map.len() != props.len() {
            return Err(bad);
        }
        for (key, s) in props {
            validate(map.get(key).ok_or(bad)?, s)?;
        }
    }
    if let Some(array) = value.as_array() {
        if schema["maxItems"]
            .as_u64()
            .is_some_and(|n| array.len() as u64 > n)
        {
            return Err(bad);
        }
        for v in array {
            validate(v, &schema["items"])?;
        }
    }
    Ok(())
}
fn safe_id(s: &str) -> bool {
    let mut bytes = s.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
}
fn xml_name(s: &str) -> bool {
    let mut bytes = s.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-' | b':'))
}
// This is a generated declaration address, never an XPath evaluator. Preserve
// the producer's complete identity; only its terminal canonical ordinal is
// admitted, for the two declaration diagnostic kinds and five producer elements.
fn public_xml_path(path: &str, master_declaration: bool) -> bool {
    if path.len() > 8192 {
        return false;
    }
    let structural = if master_declaration {
        let Some((structural, ordinal)) = path
            .strip_suffix(']')
            .and_then(|p| p.rsplit_once("[@master-ordinal="))
        else {
            return false;
        };
        if ordinal.is_empty()
            || ordinal.starts_with('0')
            || !ordinal.bytes().all(|b| b.is_ascii_digit())
            || ordinal
                .parse::<usize>()
                .ok()
                .is_none_or(|n| n > knx_productdb::master_provenance::MAX_MASTER_DECLARATIONS)
            || !matches!(
                structural.rsplit('/').next(),
                Some(
                    "DatapointType"
                        | "DatapointSubtype"
                        | "FunctionType"
                        | "FunctionPoint"
                        | "SpaceUsage"
                )
            )
        {
            return false;
        }
        structural
    } else {
        path
    };
    structural == "/"
        || (structural.starts_with('/')
            && structural[1..]
                .split('/')
                .all(|part| part.len() <= 512 && xml_name(part)))
}
fn validate_public_paths(row: &Value) -> Result<(), FailureKind> {
    let bad = FailureKind::PublicProjectionRefused;
    let kind = row["kind"].as_str().ok_or(bad)?;
    for key in ["path", "archivePath", "indexPath", "memberPath"] {
        if let Some(path) = row.get(key).and_then(Value::as_str) {
            if path.is_empty()
                || path.len() > 8192
                || path.contains(['\\', ':'])
                || path.chars().any(super::dto::is_forbidden_control)
                || path
                    .split('/')
                    .any(|c| c.is_empty() || c == "." || c == "..")
            {
                return Err(bad);
            }
        }
    }
    let master_declaration = kind == "diagnostic"
        && matches!(
            row["diagnosticKind"].as_str(),
            Some("master-identity-collision" | "malformed-master-declaration")
        );
    if master_declaration && (row["occurrences"] != "1" || !row["xmlPath"].is_string()) {
        return Err(bad);
    }
    for key in ["xpath", "xmlPath"] {
        if let Some(path) = row.get(key).and_then(Value::as_str) {
            if !public_xml_path(path, master_declaration && key == "xmlPath") {
                return Err(bad);
            }
        }
    }
    if kind == "unknownConstruct"
        && row["name"]
            .as_str()
            .is_none_or(|n| n.len() > 512 || !xml_name(n))
    {
        return Err(bad);
    }
    if kind == "idConflict" {
        for key in ["id", "table"] {
            if row[key]
                .as_str()
                .is_none_or(|s| s.len() > 512 || !safe_id(s))
            {
                return Err(bad);
            }
        }
    }
    Ok(())
}
fn validate_semantics(row: &Value) -> Result<(), FailureKind> {
    let bad = FailureKind::ReportMismatch;
    match row["kind"].as_str().ok_or(bad)? {
        "baggageSummary" => {
            let available = row["status"] == "available";
            if available == row["declarations"].is_null() || available == row["payloads"].is_null()
            {
                return Err(bad);
            }
        }
        "baggageDeclaration" => {
            if (row["resolution"] == "resolved") == row["memberPath"].is_null() {
                return Err(bad);
            }
        }
        "baggagePayload" => {
            let read = row["nested"]["state"] == "read";
            for key in [
                "entries",
                "declaredExpandedSize",
                "encryptedEntries",
                "archiveNamedEntries",
            ] {
                if read == row["nested"][key].is_null() {
                    return Err(bad);
                }
            }
        }
        "packageReport" => {
            if (row["facts"] == "available") == row["unknownOccurrences"].is_null() {
                return Err(bad);
            }
        }
        "resultSummary" => {
            let bytes = row["cataloguePageBytes"]
                .as_str()
                .ok_or(bad)?
                .parse::<u64>()
                .map_err(|_| bad)?;
            if row["nextJobInputLimitExceeded"] != (bytes > 8_589_934_592) {
                return Err(bad);
            }
        }
        _ => (),
    }
    Ok(())
}

struct Ordered<'a> {
    value: &'a Value,
    schema: &'a Value,
}
impl Serialize for Ordered<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let schema = resolve(self.schema);
        if let Some(keys) = schema["required"].as_array() {
            let mut map = serializer.serialize_map(Some(keys.len()))?;
            for key in keys {
                let key = key.as_str().unwrap();
                map.serialize_entry(
                    key,
                    &Ordered {
                        value: &self.value[key],
                        schema: &schema["properties"][key],
                    },
                )?;
            }
            map.end()
        } else {
            self.value.serialize(serializer)
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Withheld<'a> {
    field: &'a str,
    reason: &'static str,
    raw_utf8_bytes: U64,
}
struct Public<'a> {
    value: &'a Value,
    schema: &'a Value,
    fields: &'static [&'static str],
    withheld: &'a [Withheld<'a>],
}
impl Serialize for Public<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let keys = self.schema["required"].as_array().unwrap();
        let mut map = serializer.serialize_map(Some(keys.len() + 1))?;
        for key in keys {
            let key = key.as_str().unwrap();
            if self.fields.contains(&key) {
                map.serialize_entry(key, &Option::<()>::None)?;
            } else {
                map.serialize_entry(
                    key,
                    &Ordered {
                        value: &self.value[key],
                        schema: &self.schema["properties"][key],
                    },
                )?;
            }
        }
        map.serialize_entry("withheld", self.withheld)?;
        map.end()
    }
}
fn withheld_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "program" => &["name"],
        "provenance" => &["alias"],
        "unknownConstruct" => &["sample"],
        "diagnostic" => &["detail"],
        "baggageDeclaration" => &[
            "declarationId",
            "name",
            "targetPath",
            "installOnImport",
            "timeInfo",
            "fileVersion",
            "detail",
        ],
        _ => &[],
    }
}

#[cfg(test)]
mod persistence_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue_jobs::{artifacts::ArtifactRoot, Handle, Incarnation};
    use std::sync::Arc;
    #[test]
    fn complete_artifact_preserves_raw_and_redacts_every_public_alias() {
        let tmp = tempfile::tempdir().unwrap();
        let root = Arc::new(ArtifactRoot::acquire(tmp.path()).unwrap());
        let incarnation = Incarnation::from_bytes([1; 16]);
        let dir = root.create_incarnation_dir(&incarnation).unwrap();
        let handle = Handle {
            incarnation,
            serial: U64(1),
            job: JobId::from_bytes([2; 16]),
        };
        let mut workspace = JobWorkspace::create(root, &dir, handle).unwrap();
        let mut writer = ArtifactWriter::new(&mut workspace, ArtifactKind::Result).unwrap();
        writer.append(&serde_json::json!({"ordinal":"0","kind":"resultSummary","packages":"1","cataloguePageBytes":"4096","nextJobInputLimitExceeded":false})).unwrap();
        writer.append(&serde_json::json!({"ordinal":"1","kind":"provenance","sourceKey":"0","sha256":"00".repeat(32),"alias":"private-origin.knxprod","packageKey":"0"})).unwrap();
        let descriptor = writer
            .finish()
            .expect("complete artifact must publish its manifest last");
        assert_eq!(descriptor.records, U64(2));
        assert_eq!(descriptor.withheld_fields, U64(1));
        let raw = std::fs::read_to_string(workspace.directory().join("result.raw")).unwrap();
        let public = std::fs::read_to_string(workspace.directory().join("result.public")).unwrap();
        assert!(raw.contains("private-origin.knxprod"));
        assert!(!public.contains("private-origin.knxprod"));
        assert!(public.contains("\"alias\":null"));
        assert!(workspace.directory().join("result.manifest").is_file());
        assert_eq!(
            std::fs::metadata(workspace.directory().join("result.index"))
                .unwrap()
                .len(),
            64
        );
        let page: Value =
            serde_json::from_slice(&detail_page(&workspace, &descriptor, U64(0), 200).unwrap())
                .unwrap();
        assert_eq!(page["records"].as_array().unwrap().len(), 2);
        assert!(page["nextOffset"].is_null());
    }
}
