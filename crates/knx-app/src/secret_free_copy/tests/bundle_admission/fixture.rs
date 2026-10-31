//! Builds importer-produced reports and native stores for bundle-admission tests.

use super::super::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub(super) enum Credential {
    None,
    FreeAccessBcuKey,
    Field(&'static str, &'static str),
    BusAccess(&'static str, &'static str),
}

#[derive(Clone, Copy)]
pub(super) enum Duplicate {
    None,
    Body(&'static str),
    SourcePath(&'static str),
    Xpath(&'static str),
}

#[derive(Clone, Copy)]
pub(super) enum Extra {
    None,
    NestedCredential,
    DiscardedCredential,
    WithheldCarriedCredential,
    WideUninspectable,
    UndecodableCredentialWithRawDuplicate,
    WhitespaceAttribute,
    WhitespaceElement,
    WhitespaceBusAccess,
    EmptyRecognizedValues,
}

pub(super) struct Produced {
    pub(super) source: PathBuf,
    pub(super) native: PathBuf,
    pub(super) report: Value,
    retained_bus_access: Option<RetainedBusAccess>,
    source_bytes: Vec<u8>,
    native_bytes: Vec<u8>,
    history_versions: usize,
}

struct RetainedBusAccess {
    source_path: String,
    xpath: String,
    parameter: String,
}

fn opaque_row(
    bytes: &[u8],
    kind: &str,
    name: &str,
    source_path: &str,
    xpath: &str,
) -> knx_store::StoredOpaqueEntry {
    knx_store::StoredOpaqueEntry {
        source_path: source_path.into(),
        xpath: xpath.into(),
        kind: kind.into(),
        name: name.into(),
        bytes: bytes.to_vec(),
        sha256: knx_store::project_history::image_hash(bytes),
    }
}

fn history_versions(path: &Path) -> usize {
    let store = knx_store::open_existing_read_only(path).unwrap();
    knx_store::project_history::list_versions(&store.conn)
        .unwrap()
        .len()
}

fn retained_bus_access(
    row: &knx_store::StoredOpaqueEntry,
    part: &str,
    value: &str,
) -> Option<RetainedBusAccess> {
    if row.kind != "RetainedElement"
        || row.name.rsplit(':').next() != Some("BusAccess")
        || !row
            .xpath
            .rsplit('/')
            .next()
            .is_some_and(|segment| segment.split('[').next().unwrap_or(segment) == "BusAccess")
    {
        return None;
    }
    let raw = std::str::from_utf8(&row.bytes).ok()?;
    let parameter = raw.split_once("Parameter=\"")?.1.split_once('"')?.0;
    let component = format!("{part}={value}");
    if !parameter
        .split(';')
        .any(|candidate| candidate == component.as_str())
    {
        return None;
    }
    Some(RetainedBusAccess {
        source_path: row.source_path.clone(),
        xpath: row.xpath.clone(),
        parameter: parameter.to_string(),
    })
}

pub(super) fn produce(
    dir: &Path,
    credential: Credential,
    duplicate: Duplicate,
    extra: Extra,
) -> Produced {
    let attribute = match credential {
        Credential::Field(name, value) => format!(r#" {name}="{value}""#),
        Credential::None | Credential::FreeAccessBcuKey | Credential::BusAccess(_, _) => {
            String::new()
        }
    };
    let witness = match duplicate {
        Duplicate::Body(value) => value,
        Duplicate::None | Duplicate::SourcePath(_) | Duplicate::Xpath(_) => "safe sample",
    };
    let mut xml = topology(false)
        .replace("/project/21", "/project/23")
        .replace(
            "<DeviceInstance ",
            &format!("<DeviceInstance{attribute} DiagnosticWitness=\"{witness}\" "),
        );
    if matches!(credential, Credential::None) {
        xml = xml.replace(r#" BCUKey="4294967295""#, "");
    }
    if let Credential::BusAccess(part, value) = credential {
        xml = xml.replace(
            "Target=203.0.113.250;Port=3671",
            &format!("Target=203.0.113.250;{part}={value};Port=3671"),
        );
    }
    let info = project_info(false).replace("/project/21", "/project/23");
    let source_bytes = knx_testsupport::zip_with_entries(&[
        ("P-0001.signature", b"synthetic signature"),
        ("P-0001/0.xml", xml.as_bytes()),
        ("P-0001/Project.xml", info.as_bytes()),
    ]);
    let source = dir.join("bundle-admission-source.knxproj");
    std::fs::write(&source, &source_bytes).unwrap();

    let staging = knx_store::open_and_migrate_in_memory().unwrap();
    let mut imported = crate::import_ets_project(&source, &staging).unwrap();
    let mut opaque = knx_store::load_opaque(&staging).unwrap();
    let refs = knx_store::load_manufacturer_refs(&staging).unwrap();

    if let Credential::Field(name, value) = credential {
        let unknown = imported
            .report
            .unknown
            .iter_mut()
            .find(|row| {
                row.name.rsplit(':').next() == Some(name) && row.sample.as_deref() == Some(value)
            })
            .expect("real report omitted the credential row");
        match duplicate {
            Duplicate::SourcePath(value) => unknown.source_path = format!("P-0001/{value}.xml"),
            Duplicate::Xpath(value) => unknown.xpath = format!("/KNX/{value}/DeviceInstance"),
            Duplicate::None | Duplicate::Body(_) => {}
        }

        let row = opaque
            .iter_mut()
            .find(|row| row.name.rsplit(':').next() == Some(name) && row.bytes == value.as_bytes())
            .expect("real producer omitted the credential row");
        match duplicate {
            Duplicate::SourcePath(value) => row.source_path = format!("P-0001/{value}.xml"),
            Duplicate::Xpath(value) => row.xpath = format!("/KNX/{value}/DeviceInstance"),
            Duplicate::None | Duplicate::Body(_) => {}
        }
    }

    match extra {
        Extra::NestedCredential => {
            let nested = knx_testsupport::zip_with_entries(&[
                ("credential.xml", br#"<a Password="123"/>"# as &[u8]),
                ("safe.bin", b"safe"),
            ]);
            opaque.push(opaque_row(
                &nested,
                "BundleAdmissionArchive",
                "archive",
                "nested.knxproj",
                "",
            ));
        }
        Extra::DiscardedCredential => opaque.push(opaque_row(
            br#"<a Password="123">"#,
            "RetainedElement",
            "a",
            "discarded.xml",
            "/a",
        )),
        Extra::WideUninspectable => {
            let mut wide = vec![0xff, 0xfe];
            wide.extend(r#"<a Password="123"/>"#.encode_utf16().flat_map(u16::to_le_bytes));
            opaque.push(opaque_row(
                &wide,
                "BundleAdmissionOpaque",
                "a",
                "wide.xml",
                "/a",
            ));
        }
        Extra::UndecodableCredentialWithRawDuplicate => {
            opaque.push(opaque_row(
                br#"<a Password="&undefined;"/>"#,
                "RetainedElement",
                "a",
                "synthetic-undecodable-credential.xml",
                "/a",
            ));
            opaque.push(opaque_row(
                b"&undefined;",
                "BundleAdmissionOpaque",
                "raw-duplicate",
                "synthetic-raw-duplicate.bin",
                "",
            ));
        }
        Extra::WhitespaceAttribute => opaque.push(opaque_row(
            b" ",
            "RetainedAttribute",
            "Password",
            "synthetic-whitespace-attribute.xml",
            "/KNX/Project/DeviceInstance",
        )),
        Extra::WhitespaceElement => opaque.push(opaque_row(
            b"<Password> </Password>",
            "RetainedElement",
            "Password",
            "synthetic-whitespace-element.xml",
            "/Password",
        )),
        Extra::WhitespaceBusAccess => opaque.push(opaque_row(
            b"Target=203.0.113.250;UserPassword= ;Port=3671",
            "RetainedAttribute",
            "Parameter",
            "synthetic-whitespace-bus-access.xml",
            "/KNX/Project/BusAccess",
        )),
        Extra::EmptyRecognizedValues => {
            opaque.push(opaque_row(
                b"",
                "RetainedAttribute",
                "Password",
                "synthetic-empty-attribute.xml",
                "/KNX/Project/DeviceInstance",
            ));
            opaque.push(opaque_row(
                b"<Password></Password>",
                "RetainedElement",
                "Password",
                "synthetic-empty-element.xml",
                "/Password",
            ));
            opaque.push(opaque_row(
                b"Target=203.0.113.250;UserPassword=;Port=3671",
                "RetainedAttribute",
                "Parameter",
                "synthetic-empty-bus-access.xml",
                "/KNX/Project/BusAccess",
            ));
        }
        Extra::None | Extra::WithheldCarriedCredential => {}
    }

    crate::import_evidence::capture_primary(&mut opaque, &imported.report).unwrap();
    let captured_report = serde_json::to_value(&imported.report).unwrap();
    assert!(captured_report["unknown"].as_array().is_some());

    let native = dir.join("bundle-admission-project.knxdb");
    let conn = knx_store::open_and_migrate(&native).unwrap();
    knx_store::save_project_with_passthrough(&conn, &imported.project, &opaque, &refs).unwrap();
    let mut saved = imported.project.clone();
    saved.info.name = "Bundle admission saved root".into();
    knx_store::save_project_with_passthrough(&conn, &saved, &opaque, &refs).unwrap();
    if matches!(extra, Extra::WithheldCarriedCredential) {
        knx_store::store_manufacturer_file(
            &conn,
            "M-TEST/bundle-admission.xml",
            br#"<a Password="123"/>"#,
        )
        .unwrap();
    }
    drop(conn);

    assert!(std::fs::read(&source).unwrap() == source_bytes);
    let reopened = knx_store::open_existing_read_only(&native).unwrap();
    knx_store::load_project(&reopened.conn).unwrap();
    let reopened_opaque = knx_store::load_opaque(&reopened.conn).unwrap();
    let historical = crate::import_evidence::read(&reopened_opaque)
        .unwrap()
        .expect("fixture must persist primary import evidence");
    assert_eq!(historical.report, captured_report);
    let retained_bus_access = match credential {
        Credential::BusAccess(part, value) => Some(
            reopened_opaque
                .iter()
                .find_map(|row| retained_bus_access(row, part, value))
                .expect("reopened native source omitted the retained BusAccess component"),
        ),
        Credential::None | Credential::FreeAccessBcuKey | Credential::Field(_, _) => None,
    };
    let report = historical.report;
    drop(reopened);
    let versions = history_versions(&native);
    assert!(versions > 0, "fixture must persist prior saved state");
    let native_bytes = std::fs::read(&native).unwrap();

    Produced {
        source,
        native,
        report,
        retained_bus_access,
        source_bytes,
        native_bytes,
        history_versions: versions,
    }
}

impl Produced {
    /// Adds a synthetic report row derived from the actual reopened retained element.
    pub(super) fn synthetic_supplementary_bus_access_report(&self) -> Value {
        let retained = self
            .retained_bus_access
            .as_ref()
            .expect("fixture has no retained BusAccess component");
        let mut report = self.report.clone();
        report["unknown"].as_array_mut().unwrap().push(json!({
            "source_path": retained.source_path.as_str(),
            "xpath": retained.xpath.as_str(),
            "kind": "Attribute",
            "name": "Parameter",
            "occurrences": 1,
            "sample": retained.parameter.as_str(),
            "sha256": knx_store::project_history::image_hash(retained.parameter.as_bytes()),
        }));
        report
    }

    pub(super) fn assert_sources_unchanged(&self) {
        assert!(std::fs::read(&self.source).unwrap() == self.source_bytes);
        assert!(std::fs::read(&self.native).unwrap() == self.native_bytes);
        assert_eq!(history_versions(&self.native), self.history_versions);
    }
}

pub(super) fn ordinary_report(report: &Value) -> SecretFreeCopy {
    match import_report_without_secrets(report) {
        Ok(copy) => copy,
        Err(_) => panic!("ordinary report helper did not reach bundle refusal calibration"),
    }
}

pub(super) fn ordinary_native(path: &Path) -> SecretFreeCopy {
    match native_project_without_secrets(path) {
        Ok(copy) => copy,
        Err(_) => panic!("ordinary native helper did not reach bundle refusal calibration"),
    }
}

pub(super) fn expected_parts(copy: SecretFreeCopy) -> (Vec<u8>, Vec<String>) {
    let mut notes = Vec::new();
    for redaction in &copy.redactions {
        notes.push(format!(
            "removed {} at {}",
            redaction.kind, redaction.location
        ));
    }
    for withheld in &copy.withheld {
        notes.push(format!("left out: {withheld}"));
    }
    (copy.bytes, notes)
}

pub(super) fn assert_bundle_unavailable(result: Result<BundleAdmittedCopy, BundleCopyUnavailable>) {
    assert!(result.is_err());
}

pub(super) fn admitted(
    result: Result<BundleAdmittedCopy, BundleCopyUnavailable>,
) -> (Vec<u8>, Vec<String>) {
    match result {
        Ok(copy) => copy.into_parts(),
        Err(_) => panic!("credential-free source was not admitted"),
    }
}
