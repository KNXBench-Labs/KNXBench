//! Bounded decoding, validation and offline search of a catalogue index document.

use std::io::Read;

use serde::Deserialize;

use super::error::OnlineError;
use super::source::validate_relative;

/// The only index schema this build understands; any other is refused whole.
pub const SCHEMA: u32 = 1;
/// Largest index body accepted on the wire (about thirteen times the size seen in practice).
pub const MAX_COMPRESSED_BYTES: usize = 64 * 1024 * 1024;
/// Largest decoded index document (about eight times the size seen in practice).
pub const MAX_DECODED_BYTES: usize = 256 * 1024 * 1024;
/// Most index entries kept; a larger index is refused rather than truncated.
pub const MAX_ENTRIES: usize = 500_000;
/// Per-file cap: the same ceiling the product-package admission applies.
pub const MAX_FILE_BYTES: u64 = knx_core::limits::MAX_RETAINED_BYTES;
/// The most search hits one request returns.
pub const MAX_PAGE: usize = 100;
/// The file types the catalogue offers and the API lets through.
pub const DOWNLOADABLE_TYPES: [&str; 6] = ["knxprod", "vd1", "vd2", "vd3", "vd4", "vd5"];
/// Longest accepted display string of an entry.
const MAX_TEXT_BYTES: usize = 1024;
const MAX_LIST: usize = 256;
const MAX_ID_BYTES: usize = 64;
const MAX_FILE_NAME_BYTES: usize = crate::catalogue_jobs::dto::MAX_SOURCE_NAME_BYTES;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawIndex {
    version: String,
    generated_at: String,
    count: u64,
    base_path: String,
    download_requires_token: bool,
    schema: u32,
    entries: Vec<RawEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEntry {
    id: String,
    manufacturer: String,
    manufacturer_id: String,
    name: String,
    #[serde(default)]
    order_numbers: Vec<String>,
    file_name: String,
    file_type: String,
    size: u64,
    sha256: String,
    updated: String,
    #[serde(default)]
    programs: Vec<RawProgram>,
    #[serde(rename = "download_url")]
    download_url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawProgram {
    #[serde(default)]
    display_key: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    application_number: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    mask_version: String,
    #[serde(default)]
    product_names: Vec<String>,
}

/// One application program an entry carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub display_key: String,
    pub name: String,
    pub application_number: String,
    pub version: String,
    pub mask_version: String,
    pub product_names: Vec<String>,
}

/// One downloadable catalogue file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub manufacturer: String,
    pub manufacturer_id: String,
    pub name: String,
    pub order_numbers: Vec<String>,
    pub file_name: String,
    pub file_type: String,
    pub size: u64,
    pub sha256: String,
    pub updated: String,
    pub programs: Vec<Program>,
    pub download_url: String,
}

impl Entry {
    /// Whether KNXBench's existing import path accepts this file type.
    pub fn importable(&self) -> bool {
        matches!(self.file_type.as_str(), "knxprod" | "vd3" | "vd4" | "vd5")
    }
}

/// A validated index: the entries plus what the server said about the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub version: String,
    pub generated_at: String,
    /// Entries refused individually (bad id, digest, size, file name, address or a duplicate).
    pub skipped: u64,
    pub entries: Vec<Entry>,
}

/// An in-memory index with a lowercase search text per entry.
#[derive(Debug)]
pub struct Snapshot {
    pub parsed: Parsed,
    haystacks: Vec<Box<str>>,
}

/// One page of search results.
#[derive(Debug)]
pub struct Page<'a> {
    pub total: usize,
    pub entries: Vec<&'a Entry>,
}

impl Snapshot {
    pub fn new(parsed: Parsed) -> Self {
        let haystacks = parsed.entries.iter().map(haystack).collect();
        Self { parsed, haystacks }
    }

    pub fn find(&self, id: &str) -> Option<&Entry> {
        self.parsed.entries.iter().find(|e| e.id == id)
    }

    /// Every whitespace-separated word of `query` must occur somewhere in an entry (case-insensitive).
    pub fn search(
        &self,
        query: &str,
        file_type: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> Page<'_> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        let limit = limit.clamp(1, MAX_PAGE);
        let mut total = 0usize;
        let mut entries = Vec::new();
        for (entry, text) in self.parsed.entries.iter().zip(&self.haystacks) {
            if file_type.is_some_and(|t| entry.file_type != t) {
                continue;
            }
            if !words.iter().all(|w| text.contains(w.as_str())) {
                continue;
            }
            if total >= offset && entries.len() < limit {
                entries.push(entry);
            }
            total += 1;
        }
        Page { total, entries }
    }
}

fn haystack(entry: &Entry) -> Box<str> {
    let mut text = String::new();
    for part in [
        &entry.manufacturer,
        &entry.manufacturer_id,
        &entry.name,
        &entry.file_name,
    ] {
        text.push_str(part);
        text.push('\n');
    }
    for order in &entry.order_numbers {
        text.push_str(order);
        text.push('\n');
    }
    for program in &entry.programs {
        for part in [
            &program.display_key,
            &program.name,
            &program.application_number,
            &program.version,
            &program.mask_version,
        ] {
            text.push_str(part);
            text.push('\n');
        }
        for product in &program.product_names {
            text.push_str(product);
            text.push('\n');
        }
    }
    text.to_lowercase().into_boxed_str()
}

/// Decompresses a gzip body, refusing anything that decodes past `limit`.
pub fn gunzip_bounded(compressed: &[u8], limit: usize) -> Result<Vec<u8>, OnlineError> {
    let mut decoded = Vec::new();
    let mut reader = flate2::read::MultiGzDecoder::new(compressed).take(limit as u64 + 1);
    reader
        .read_to_end(&mut decoded)
        .map_err(|_| OnlineError::Integrity("the index is not valid gzip data"))?;
    if decoded.len() > limit {
        return Err(OnlineError::Integrity("the index is larger than the limit"));
    }
    Ok(decoded)
}

/// Parses and validates a decoded index document. Whole-document defects refuse the document;
/// individually invalid entries are skipped and counted, never silently dropped.
pub fn parse(decoded: &[u8]) -> Result<Parsed, OnlineError> {
    let raw: RawIndex = serde_json::from_slice(decoded)
        .map_err(|_| OnlineError::Integrity("the index is not the expected JSON document"))?;
    if raw.schema != SCHEMA {
        return Err(OnlineError::Integrity("unsupported index schema"));
    }
    if !raw.base_path.is_empty() {
        return Err(OnlineError::Integrity("unsupported index base path"));
    }
    if !raw.download_requires_token {
        return Err(OnlineError::Integrity("unsupported index download policy"));
    }
    if raw.entries.len() > MAX_ENTRIES || raw.count != raw.entries.len() as u64 {
        return Err(OnlineError::Integrity(
            "the index entry count is inconsistent",
        ));
    }
    if raw.version.is_empty() || raw.version.len() > MAX_TEXT_BYTES || raw.generated_at.len() > 64 {
        return Err(OnlineError::Integrity("the index header is invalid"));
    }
    let mut seen = std::collections::HashSet::with_capacity(raw.entries.len());
    let mut skipped = 0u64;
    let mut entries = Vec::with_capacity(raw.entries.len());
    for entry in raw.entries {
        match admit(entry) {
            Some(entry) if seen.insert(entry.id.clone()) => entries.push(entry),
            _ => skipped += 1,
        }
    }
    Ok(Parsed {
        version: raw.version,
        generated_at: raw.generated_at,
        skipped,
        entries,
    })
}

fn admit(raw: RawEntry) -> Option<Entry> {
    let id_ok = !raw.id.is_empty()
        && raw.id.len() <= MAX_ID_BYTES
        && raw
            .id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    let sha_ok = raw.sha256.len() == 64
        && raw
            .sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    let file_name_ok = !raw.file_name.is_empty()
        && raw.file_name.len() <= MAX_FILE_NAME_BYTES
        && raw.file_name != "."
        && raw.file_name != ".."
        && !raw.file_name.contains(['/', '\\', ':', '\0'])
        && !raw.file_name.chars().any(char::is_control)
        && raw.file_name.trim() == raw.file_name
        && raw
            .file_name
            .rsplit_once('.')
            .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case(&raw.file_type));
    let text_ok = [
        &raw.manufacturer,
        &raw.manufacturer_id,
        &raw.name,
        &raw.updated,
    ]
    .iter()
    .all(|s| s.len() <= MAX_TEXT_BYTES)
        && raw.order_numbers.len() <= MAX_LIST
        && raw.order_numbers.iter().all(|s| s.len() <= MAX_TEXT_BYTES)
        && raw.programs.len() <= MAX_LIST
        && raw.programs.iter().all(|p| {
            [
                &p.display_key,
                &p.name,
                &p.application_number,
                &p.version,
                &p.mask_version,
            ]
            .iter()
            .all(|s| s.len() <= MAX_TEXT_BYTES)
                && p.product_names.len() <= MAX_LIST
                && p.product_names.iter().all(|s| s.len() <= MAX_TEXT_BYTES)
        });
    if !(id_ok
        && sha_ok
        && file_name_ok
        && text_ok
        && (1..=MAX_FILE_BYTES).contains(&raw.size)
        && DOWNLOADABLE_TYPES.contains(&raw.file_type.as_str())
        && validate_relative(&raw.download_url).is_ok())
    {
        return None;
    }
    Some(Entry {
        id: raw.id,
        manufacturer: raw.manufacturer,
        manufacturer_id: raw.manufacturer_id,
        name: raw.name,
        order_numbers: raw.order_numbers,
        file_name: raw.file_name,
        file_type: raw.file_type,
        size: raw.size,
        sha256: raw.sha256,
        updated: raw.updated,
        programs: raw
            .programs
            .into_iter()
            .map(|p| Program {
                display_key: p.display_key,
                name: p.name,
                application_number: p.application_number,
                version: p.version,
                mask_version: p.mask_version,
                product_names: p.product_names,
            })
            .collect(),
        download_url: raw.download_url,
    })
}

#[cfg(test)]
pub(crate) mod fixtures {
    use serde_json::{json, Value};

    pub const SHA_A: &str = "5ffb15167106eca6fd06e9731e418e0486bedb0a4d293a2fba91e084ae45a707";

    pub fn entry(id: &str, file_type: &str, name: &str) -> Value {
        json!({
            "id": id,
            "manufacturer": "Contoso Lighting",
            "manufacturerId": "0042",
            "name": name,
            "orderNumbers": ["CL 100-1", "CL 100-2"],
            "fileName": format!("{id}.{file_type}"),
            "fileType": file_type,
            "size": 4,
            "sha256": SHA_A,
            "updated": "2026-01-01T00:00:00+00:00",
            "programs": [{
                "key": "M-0042_A-0001-01-0000",
                "displayKey": "0042 0001 01 [0000]",
                "name": "Dimmer 4-fold",
                "manufacturer": "Contoso Lighting",
                "manufacturerId": "0042",
                "applicationNumber": "0001",
                "version": "01",
                "maskVersion": "0000",
                "orderNumbers": ["CL 100-1"],
                "productNames": ["Dimmer actuator", "Dim 4x"]
            }],
            "download_url": format!("/api/download?path=Hersteller%2F0042_Contoso%2F{id}.{file_type}")
        })
    }

    pub fn document(entries: Vec<Value>) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "version": "abc123",
            "generatedAt": "2026-01-01T00:00:00+00:00",
            "count": entries.len(),
            "basePath": "",
            "downloadRequiresToken": true,
            "schema": 1,
            "entries": entries,
        }))
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;
    use serde_json::json;

    #[test]
    fn a_valid_document_parses_and_is_searchable_across_fields() {
        let doc = document(vec![
            entry("a1", "knxprod", "Dimmer Actuator"),
            entry("b2", "vd5", "Blind Controller"),
        ]);
        let parsed = parse(&doc).unwrap();
        assert_eq!((parsed.entries.len(), parsed.skipped), (2, 0));
        let snapshot = Snapshot::new(parsed);
        assert_eq!(snapshot.search("", None, 0, 10).total, 2);
        // A product name, an order number, the manufacturer and the application number all hit.
        for q in ["dim 4x", "cl 100-2", "CONTOSO", "0001 blind"] {
            assert!(snapshot.search(q, None, 0, 10).total >= 1, "{q}");
        }
        assert_eq!(snapshot.search("blind", None, 0, 10).entries[0].id, "b2");
        assert_eq!(snapshot.search("", Some("vd5"), 0, 10).total, 1);
        assert_eq!(snapshot.search("nothing-like-this", None, 0, 10).total, 0);
        let page = snapshot.search("contoso", None, 1, 1);
        assert_eq!(
            (page.total, page.entries.len(), page.entries[0].id.as_str()),
            (2, 1, "b2")
        );
        assert!(snapshot.find("a1").unwrap().importable());
    }

    #[test]
    fn vd1_and_vd2_are_listed_but_not_importable() {
        let doc = document(vec![entry("x1", "vd1", "Old"), entry("x2", "vd2", "Older")]);
        let snapshot = Snapshot::new(parse(&doc).unwrap());
        assert!(snapshot.parsed.entries.iter().all(|e| !e.importable()));
        assert_eq!(snapshot.parsed.entries.len(), 2);
    }

    #[test]
    fn mismatched_file_extensions_are_not_advertised_as_an_importable_type() {
        let mut invalid = entry("a1", "knxprod", "Mismatched");
        invalid["fileName"] = json!("not-a-package.vd2");
        let mut valid = entry("b2", "knxprod", "Uppercase extension");
        valid["fileName"] = json!("valid.KNXPROD");
        let parsed = parse(&document(vec![invalid, valid])).unwrap();
        assert_eq!(parsed.skipped, 1);
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].file_name, "valid.KNXPROD");
        assert!(parsed.entries[0].importable());
    }

    #[test]
    fn whole_document_defects_refuse_the_index() {
        let mut ok: serde_json::Value =
            serde_json::from_slice(&document(vec![entry("a1", "knxprod", "A")])).unwrap();
        let cases: Vec<(&str, serde_json::Value)> = vec![
            ("schema", json!(2)),
            ("count", json!(7)),
            ("basePath", json!("/x")),
            ("downloadRequiresToken", json!(false)),
        ];
        for (key, value) in cases {
            let original = ok[key].clone();
            ok[key] = value;
            assert!(parse(&serde_json::to_vec(&ok).unwrap()).is_err(), "{key}");
            ok[key] = original;
        }
        assert!(parse(b"{").is_err());
        assert!(parse(b"[]").is_err());
        assert!(parse(&serde_json::to_vec(&ok).unwrap()).is_ok());
    }

    #[test]
    fn invalid_and_duplicate_entries_are_skipped_and_counted_not_lost_silently() {
        let mut bad_sha = entry("b1", "knxprod", "B");
        bad_sha["sha256"] = json!("XYZ");
        let mut bad_url = entry("b2", "knxprod", "B");
        bad_url["download_url"] = json!("https://evil.example/x");
        let mut bad_type = entry("b3", "zip", "B");
        bad_type["fileType"] = json!("zip");
        let mut zero = entry("b4", "knxprod", "B");
        zero["size"] = json!(0);
        let mut huge = entry("b5", "knxprod", "B");
        huge["size"] = json!(MAX_FILE_BYTES + 1);
        let mut slash = entry("b6", "knxprod", "B");
        slash["fileName"] = json!("../x.knxprod");
        let mut bad_id = entry("b7", "knxprod", "B");
        bad_id["id"] = json!("../b7");
        let doc = document(vec![
            entry("a1", "knxprod", "A"),
            entry("a1", "knxprod", "A again"),
            bad_sha,
            bad_url,
            bad_type,
            zero,
            huge,
            slash,
            bad_id,
        ]);
        let parsed = parse(&doc).unwrap();
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.skipped, 8);
        assert_eq!(parsed.entries[0].name, "A");
    }

    #[test]
    fn gunzip_refuses_garbage_and_oversize_output() {
        use flate2::{write::GzEncoder, Compression};
        use std::io::Write;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&vec![b'a'; 10_000]).unwrap();
        let gz = encoder.finish().unwrap();
        assert_eq!(gunzip_bounded(&gz, 10_000).unwrap().len(), 10_000);
        assert!(gunzip_bounded(&gz, 9_999).is_err());
        assert!(gunzip_bounded(b"not gzip", 100).is_err());
        let mut with_trailing_garbage = gz.clone();
        with_trailing_garbage.extend_from_slice(b"not another gzip member");
        assert!(gunzip_bounded(&with_trailing_garbage, 20_000).is_err());
        let mut concatenated = gz.clone();
        concatenated.extend_from_slice(&gz);
        assert_eq!(gunzip_bounded(&concatenated, 20_000).unwrap().len(), 20_000);
        assert!(gunzip_bounded(&concatenated, 19_999).is_err());
    }
}
