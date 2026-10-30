//! Sentinel tests: no key or password of a synthetic project survives a secret-free copy.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::*;

mod bundle_admission;
mod review_archive;
mod review_budget;
mod review_connection_budget;
mod review_consumers;
mod review_integrity;
mod review_logical;
mod review_native;
mod review_report;
mod review_xml;

const BCU_KEY: &str = "3141592653";
const BACKBONE_KEY: &str = "SNTLbackboneKeyQUJDREVGR0g=";
const GROUP_KEY: &str = "SNTLgroupKeyMDEyMzQ1Njc4OQ==";
const TOOL_KEY: &str = "SNTLtoolKeyZmVkY2JhOTg3NjU=";
const LOADED_TOOL_KEY: &str = "SNTLloadedToolKeyQUFBQQ==";
const AUTH_CODE: &str = "SNTLdevAuthCode42";
const TUNNEL_PASSWORD: &str = "SNTLtunnelPw-7";
const FDSK: &str = "SNTLfdskAAAABBBBCCCCDDDDEEEE";
const PROJECT_PASSWORD: &str = "SNTLprojectPw-9";
const PRODUCT_KEY: &str = "SNTLproductAuthorizeKey";

const PLAIN_PRODUCT: &[u8] =
    br#"<KNX><ManufacturerData><Hardware Name="Plain"/></ManufacturerData></KNX>"#;

/// Every sentinel carries this prefix except the decimal BCU key.
const SENTINEL_PREFIX: &str = "SNTL";

const SENTINELS: &[&str] = &[
    BCU_KEY,
    BACKBONE_KEY,
    GROUP_KEY,
    TOOL_KEY,
    LOADED_TOOL_KEY,
    AUTH_CODE,
    TUNNEL_PASSWORD,
    FDSK,
    PROJECT_PASSWORD,
    PRODUCT_KEY,
];

fn topology(secrets: bool) -> String {
    let (installation, bus, security, group) = if secrets {
        (
            format!(r#" BCUKey="{BCU_KEY}" IPRoutingBackboneKey="{BACKBONE_KEY}""#),
            format!(r#"Target=203.0.113.250;UserPassword={TUNNEL_PASSWORD};Port=3671"#),
            format!(
                r#"<Security ToolKey="{TOOL_KEY}" LoadedToolKey="{LOADED_TOOL_KEY}"
                     DeviceAuthenticationCode="{AUTH_CODE}" SequenceNumber="7"/>"#
            ),
            format!(r#" Key="{GROUP_KEY}""#),
        )
    } else {
        (
            r#" BCUKey="4294967295""#.to_string(),
            "Target=203.0.113.250;Port=3671".to_string(),
            r#"<Security SequenceNumber="7"/>"#.to_string(),
            String::new(),
        )
    };
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21">
  <Project Id="P-0001"><Installations>
    <Installation InstallationId="0" Name="Main"{installation}>
      <Topology><Area Id="P-0001-0_A-1" Address="1">
        <Line Id="P-0001-0_L-2" Address="1">
          <Segment Id="P-0001-0_L-2_S-1" Number="0" MediumTypeRefId="MT-0">
            <BusAccess Name="Synthetic interface" Edi="synthetic" Parameter="{bus}"/>
            <DeviceInstance Id="P-0001-0_DI-1" Name="Kitchen actuator" Address="1"
                ProductRefId="M-TEST_H-1_P-1" Hardware2ProgramRefId="M-TEST_H-1_HP-1">
              {security}
            </DeviceInstance>
          </Segment>
        </Line>
      </Area></Topology>
      <GroupAddresses><GroupRanges>
        <GroupRange Id="P-0001-0_GR-1" RangeStart="1" RangeEnd="100" Name="Lights">
          <GroupAddress Id="P-0001-0_GA-1" Address="1" Name="Kitchen light"{group}/>
          <GroupAddress Id="P-0001-0_GA-2" Address="2" Name="Hall light"/>
        </GroupRange>
      </GroupRanges></GroupAddresses>
    </Installation>
  </Installations></Project>
</KNX>"#
    )
}

fn project_info(secrets: bool) -> String {
    let (password, certificates) = if secrets {
        (
            format!(r#" ProjectPassword="{PROJECT_PASSWORD}""#),
            format!(
                r#"<DeviceCertificates><DeviceCertificate SerialNumber="AAECAwQF" FDSK="{FDSK}"/></DeviceCertificates>"#
            ),
        )
    } else {
        (String::new(), String::new())
    };
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21"><Project Id="P-0001">
  <ProjectInformation Name="Secret witness" GroupAddressStyle="ThreeLevel"{password}>{certificates}</ProjectInformation>
</Project></KNX>"#
    )
}

fn knxproj(secrets: bool) -> Vec<u8> {
    let topology = topology(secrets);
    let info = project_info(secrets);
    knx_testsupport::zip_with_entries(&[
        ("P-0001.signature", b"synthetic signature"),
        ("P-0001/0.xml", topology.as_bytes()),
        ("P-0001/Project.xml", info.as_bytes()),
    ])
}

fn sha256(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

fn assert_no_sentinel(bytes: &[u8], what: &str) {
    for sentinel in SENTINELS.iter().chain([&SENTINEL_PREFIX]) {
        assert!(
            !contains(bytes, sentinel),
            "{what} still contains {sentinel}"
        );
    }
}

/// Imports the synthetic project the way the CLI does (import, evidence,
/// save with passthrough), saves an edit on top so history holds the
/// secret-bearing earlier state, and carries two product files.
fn native_project(dir: &Path, secrets: bool) -> PathBuf {
    let input = dir.join("source.knxproj");
    std::fs::write(&input, knxproj(secrets)).unwrap();
    let staged = knx_store::open_and_migrate_in_memory().unwrap();
    let imported = crate::import_ets_project(&input, &staged).unwrap();
    let mut opaque = knx_store::load_opaque(&staged).unwrap();
    crate::import_evidence::capture_primary(&mut opaque, &imported.report).unwrap();
    let refs = knx_store::load_manufacturer_refs(&staged).unwrap();
    let path = dir.join("project.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project_with_passthrough(&conn, &imported.project, &opaque, &refs).unwrap();
    if secrets {
        let mut edited = imported.project.clone();
        edited.info.name = "Secret witness, edited".into();
        knx_store::save_project_with_passthrough(&conn, &edited, &opaque, &refs).unwrap();
        let product = format!(
            r#"<KNX><ManufacturerData><LdCtrlAuthorize Key="{PRODUCT_KEY}"/></ManufacturerData></KNX>"#
        );
        knx_store::store_manufacturer_file(&conn, "M-TEST/M-TEST_A-1.xml", product.as_bytes())
            .unwrap();
    }
    knx_store::store_manufacturer_file(&conn, "M-TEST/Hardware.xml", PLAIN_PRODUCT).unwrap();
    path
}

fn kinds(copy: &SecretFreeCopy) -> BTreeSet<&str> {
    copy.redactions.iter().map(|r| r.kind.as_str()).collect()
}

fn reopened(bytes: &[u8], dir: &Path) -> knx_store::ReadOnlyStore {
    let path = dir.join("copy.knxdb");
    std::fs::write(&path, bytes).unwrap();
    knx_store::open_existing_read_only(&path).unwrap()
}

#[test]
fn secret_free_native_copy_drops_every_sentinel_and_still_opens() {
    let dir = tempfile::tempdir().unwrap();
    let source = native_project(dir.path(), true);
    let before = std::fs::read(&source).unwrap();
    let source_store = knx_store::open_existing_read_only(&source).unwrap();
    let source_project = knx_store::load_project(&source_store.conn).unwrap();
    let source_opaque = knx_store::load_opaque(&source_store.conn).unwrap();
    drop(source_store);
    // The fixture really carries what the copy must remove.
    for sentinel in [BCU_KEY, BACKBONE_KEY, GROUP_KEY, TOOL_KEY, TUNNEL_PASSWORD] {
        assert!(
            source_opaque
                .iter()
                .any(|row| contains(&row.bytes, sentinel)),
            "fixture lacks {sentinel}"
        );
    }

    let copy = native_project_without_secrets(&source).unwrap();

    assert_eq!(sha256(&std::fs::read(&source).unwrap()), sha256(&before));
    assert_no_sentinel(&copy.bytes, "raw copy");
    let store = reopened(&copy.bytes, dir.path());
    let project = knx_store::load_project(&store.conn).unwrap();
    assert_eq!(project, source_project, "modelled data must not change");
    let opaque = knx_store::load_opaque(&store.conn).unwrap();
    for row in &opaque {
        assert_no_sentinel(&row.bytes, &format!("row {}{}", row.source_path, row.xpath));
        assert_eq!(
            row.sha256,
            knx_store::project_history::image_hash(&row.bytes)
        );
    }
    for hash in knx_store::carried_manufacturer_hashes(&store.conn).unwrap() {
        let file = knx_store::load_manufacturer_file(&store.conn, &hash)
            .unwrap()
            .unwrap();
        assert_no_sentinel(&file.bytes, &file.source_path);
    }
    let carried: i64 = store
        .conn
        .query_row("SELECT count(*) FROM manufacturer_file", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(
        carried, 1,
        "the plain product file stays, the one naming a key goes"
    );
    let plain = knx_store::manufacturer_file_hash(PLAIN_PRODUCT);
    assert!(knx_store::load_manufacturer_file(&store.conn, &plain)
        .unwrap()
        .is_some());
    assert!(knx_store::project_history::load_editor(&store.conn)
        .unwrap()
        .is_none());
    assert!(knx_store::project_history::list_versions(&store.conn)
        .unwrap()
        .is_empty());
    // Evidence still admits and the bus access record still reads.
    assert!(crate::import_evidence::read(&opaque).unwrap().is_some());
    let access = crate::stored_bus_access::stored_bus_access(&project, &opaque, None);
    assert_eq!(access.records.len(), 1);

    let kinds = kinds(&copy);
    for kind in [
        "BCUKey",
        "IPRoutingBackboneKey",
        "Key",
        "ToolKey",
        "LoadedToolKey",
        "DeviceAuthenticationCode",
        "BusAccess/@Parameter:UserPassword",
    ] {
        assert!(
            kinds.contains(kind),
            "no redaction of kind {kind}: {kinds:?}"
        );
    }
    assert!(copy
        .withheld
        .iter()
        .any(|w| w.starts_with("project history:")));
    assert!(copy
        .withheld
        .iter()
        .any(|w| w.starts_with("carried product file M-TEST/M-TEST_A-1.xml:")));
    let note = serde_json::to_string(&copy).unwrap();
    let debug = format!("{copy:?}");
    assert_no_sentinel(note.as_bytes(), "serialized note");
    assert_no_sentinel(debug.as_bytes(), "Debug output");
    let keys: Vec<String> = serde_json::from_str::<serde_json::Value>(&note)
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        keys,
        ["redactions", "withheld"],
        "the payload is not in the note"
    );
}

#[test]
fn secret_free_native_copy_redacts_project_information_secrets_it_retains() {
    let dir = tempfile::tempdir().unwrap();
    let source = native_project(dir.path(), true);
    let copy = native_project_without_secrets(&source).unwrap();
    let kinds = kinds(&copy);
    assert!(kinds.contains("ProjectPassword"), "{kinds:?}");
    assert!(kinds.contains("FDSK"), "{kinds:?}");
}

#[test]
fn secret_free_native_copy_without_secrets_reports_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let source = native_project(dir.path(), false);
    let source_store = knx_store::open_existing_read_only(&source).unwrap();
    let project = knx_store::load_project(&source_store.conn).unwrap();
    let opaque = knx_store::load_opaque(&source_store.conn).unwrap();
    drop(source_store);

    let copy = native_project_without_secrets(&source).unwrap();

    assert!(copy.redactions.is_empty(), "{:?}", copy.redactions);
    assert!(copy.withheld.is_empty(), "{:?}", copy.withheld);
    let store = reopened(&copy.bytes, dir.path());
    assert_eq!(knx_store::load_project(&store.conn).unwrap(), project);
    assert_eq!(knx_store::load_opaque(&store.conn).unwrap(), opaque);
}

#[test]
fn secret_free_native_copy_refuses_a_file_that_is_not_a_project() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("not.knxdb");
    std::fs::write(&path, b"not a database").unwrap();
    assert!(native_project_without_secrets(&path).is_err());
    assert!(native_project_without_secrets(&dir.path().join("missing.knxdb")).is_err());
}

fn members(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|index| {
            let mut file = archive.by_index(index).unwrap();
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();
            (file.name().to_string(), content)
        })
        .collect()
}

fn utf16(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    bytes
}

#[test]
fn secret_free_import_copy_redacts_members_and_withholds_what_it_cannot_read() {
    let dir = tempfile::tempdir().unwrap();
    let topology = topology(true);
    let info = project_info(true);
    let nested = knx_testsupport::zip_with_entries(&[(
        "0.xml",
        format!(r#"<KNX><Installation BCUKey="{BCU_KEY}" Name="Nested"/></KNX>"#).as_bytes(),
    )]);
    let picture: &[u8] = b"\x89PNG\r\n\x1a\nsynthetic picture bytes";
    let wide = utf16(&format!(r#"<KNX Password="{PROJECT_PASSWORD}"/>"#));
    let broken = format!(r#"<KNX><Installation BCUKey="{BCU_KEY}"></KNX>"#);
    let doctype =
        format!(r#"<!DOCTYPE KNX [<!ENTITY k "{TOOL_KEY}">]><KNX><Security ToolKey="&k;"/></KNX>"#);
    let copied = format!(r#"see {AUTH_CODE} here"#);
    let source = knx_testsupport::zip_with_entries(&[
        ("P-0001.signature", b"synthetic signature"),
        ("P-0001/0.xml", topology.as_bytes()),
        ("P-0001/Project.xml", info.as_bytes()),
        ("M-TEST/Baggages/picture.png", picture),
        ("M-TEST/Baggages/plugin.zip", &nested),
        ("P-0001/wide.xml", &wide),
        ("P-0001/broken.xml", broken.as_bytes()),
        ("P-0001/doctype.xml", doctype.as_bytes()),
        ("P-0001/notes.txt", copied.as_bytes()),
    ]);
    let path = dir.path().join("source.knxproj");
    std::fs::write(&path, &source).unwrap();

    let copy = import_source_without_secrets(&path).unwrap();

    assert_eq!(std::fs::read(&path).unwrap(), source, "source untouched");
    assert_no_sentinel(&copy.bytes, "raw archive");
    let out = members(&copy.bytes);
    for (name, content) in &out {
        assert_no_sentinel(content, name);
        if xml::sniff(content) == xml::Content::Zip {
            for (inner, bytes) in members(content) {
                assert_no_sentinel(&bytes, &inner);
            }
        }
    }
    assert_eq!(out["P-0001.signature"], b"synthetic signature");
    assert_eq!(out["M-TEST/Baggages/picture.png"], picture);
    let redacted = String::from_utf8(out["P-0001/0.xml"].clone()).unwrap();
    assert!(redacted.contains(r#"Name="Kitchen actuator""#));
    assert!(redacted.contains(r#"Name="Kitchen light" Key="""#));
    assert!(redacted.contains("Parameter=\"Target=203.0.113.250;UserPassword=;Port=3671\""));
    // Untouched lines keep their exact bytes.
    let original_lines: BTreeSet<&str> = topology.lines().collect();
    let kept = redacted
        .lines()
        .filter(|line| original_lines.contains(line))
        .count();
    assert!(kept >= topology.lines().count() - 6);
    for withheld in [
        "P-0001/wide.xml",
        "P-0001/broken.xml",
        "P-0001/doctype.xml",
        "P-0001/notes.txt",
    ] {
        assert!(!out.contains_key(withheld), "{withheld} must be withheld");
        assert!(
            copy.withheld
                .iter()
                .any(|w| w.starts_with(&format!("{withheld}:"))),
            "{withheld} not listed: {:?}",
            copy.withheld
        );
    }
    let kinds = kinds(&copy);
    for kind in [
        "BCUKey",
        "IPRoutingBackboneKey",
        "Key",
        "ToolKey",
        "LoadedToolKey",
        "DeviceAuthenticationCode",
        "BusAccess/@Parameter:UserPassword",
        "ProjectPassword",
        "FDSK",
    ] {
        assert!(
            kinds.contains(kind),
            "no redaction of kind {kind}: {kinds:?}"
        );
    }
    assert!(copy
        .redactions
        .iter()
        .any(|r| r.location == "M-TEST/Baggages/plugin.zip!0.xml:/KNX/Installation/@BCUKey"));
    assert_no_sentinel(serde_json::to_string(&copy).unwrap().as_bytes(), "note");
    assert_no_sentinel(format!("{copy:?}").as_bytes(), "Debug output");
}

#[test]
fn secret_free_import_copy_withholds_a_password_protected_payload() {
    let dir = tempfile::tempdir().unwrap();
    let source = knx_testsupport::zipcrypto_minimal_knxproj_bytes();
    let path = dir.path().join("protected.knxproj");
    std::fs::write(&path, &source).unwrap();

    let copy = import_source_without_secrets(&path).unwrap();

    let before = members(&source);
    let after = members(&copy.bytes);
    assert!(!after.contains_key("P-0001.zip"));
    assert!(
        copy.withheld.iter().any(|w| w.starts_with("P-0001.zip:")),
        "{:?}",
        copy.withheld
    );
    for (name, content) in &before {
        if name != "P-0001.zip" && !name.ends_with('/') {
            assert_eq!(
                after.get(name),
                Some(content),
                "{name} must stay byte-identical"
            );
        }
    }
    let password = knx_testsupport::ZIPCRYPTO_MINIMAL_PASSWORD;
    assert!(!contains(&copy.bytes, password));
    assert!(!format!("{copy:?}").contains(password));
}

#[test]
fn secret_free_import_copy_without_secrets_is_the_source_itself() {
    let dir = tempfile::tempdir().unwrap();
    let path = knx_testsupport::write_minimal_knxproj(dir.path());
    let source = std::fs::read(&path).unwrap();
    let copy = import_source_without_secrets(&path).unwrap();
    assert!(copy.redactions.is_empty(), "{:?}", copy.redactions);
    assert!(copy.withheld.is_empty(), "{:?}", copy.withheld);
    assert_eq!(copy.bytes, source);
}

#[test]
fn secret_free_import_copy_refuses_a_file_that_is_not_an_archive() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plain.knxproj");
    std::fs::write(&path, format!("Password={PROJECT_PASSWORD}")).unwrap();
    let error = import_source_without_secrets(&path).unwrap_err();
    assert!(matches!(error, SecretFreeCopyError::Archive(_)));
    assert!(!error.to_string().contains(SENTINEL_PREFIX));
    assert!(!format!("{error:?}").contains(SENTINEL_PREFIX));
}

fn redact_text(xml: &str) -> (Option<String>, Findings) {
    let mut findings = Findings::default();
    let base = xml::Base {
        text: "m.xml:",
        root_in_base: false,
    };
    let out = xml::redact(xml.as_bytes(), &base, &mut findings)
        .unwrap()
        .map(|bytes| String::from_utf8(bytes).unwrap());
    (out, findings)
}

#[test]
fn secret_free_xml_keeps_every_untouched_byte_and_quote_style() {
    let input = "<?xml version=\"1.0\"?>\n<!-- c -->\n<KNX  a = 'x\"y'\tKey='SNTLk1234567' b=\"1\" >\r\n  <x/>\n</KNX>\n";
    let (out, findings) = redact_text(input);
    assert_eq!(
        out.unwrap(),
        "<?xml version=\"1.0\"?>\n<!-- c -->\n<KNX a='x\"y' Key=\"\" b=\"1\">\r\n  <x/>\n</KNX>\n"
    );
    assert_eq!(findings.redactions.len(), 1);
    assert!(findings.values.contains(b"SNTLk1234567".as_slice()));
}

#[test]
fn secret_free_xml_blanks_secret_element_text_split_by_references() {
    let (out, findings) =
        redact_text("<a><Password>SNTL&amp;pw<![CDATA[x]]></Password><b>keep</b></a>");
    assert_eq!(out.unwrap(), "<a><Password></Password><b>keep</b></a>");
    let rows: Vec<_> = findings.redactions.into_iter().collect();
    assert_eq!(
        rows,
        vec![(("m.xml:/a/Password".into(), "Password".into()), 1)]
    );
}

#[test]
fn secret_free_xml_keeps_the_free_access_sentinel_and_empty_values() {
    let input = r#"<Installation BCUKey="4294967295" ToolKey=""/>"#;
    let (out, findings) = redact_text(input);
    assert!(out.is_none());
    assert!(findings.redactions.is_empty());
}

#[test]
fn secret_free_xml_names_are_exact_and_case_sensitive() {
    let (out, _) = redact_text(r#"<a key="lower" RefKey="other" Keys="x" PasswordPolicy="y"/>"#);
    assert!(out.is_none());
    let (out, _) = redact_text(r#"<a xmlns:s="urn:s" s:Key="SNTLprefixed"/>"#);
    assert_eq!(out.unwrap(), r#"<a xmlns:s="urn:s" s:Key=""/>"#);
}

#[test]
fn secret_free_xml_refuses_what_it_cannot_inspect() {
    let base = xml::Base {
        text: "m.xml:",
        root_in_base: false,
    };
    for bad in ["<a><b></a>", "<a>", "<a k='1' k='2'/>", "<!DOCTYPE a><a/>"] {
        let mut findings = Findings::default();
        assert!(
            xml::redact(bad.as_bytes(), &base, &mut findings).is_err(),
            "{bad}"
        );
    }
}

#[test]
fn secret_free_connection_strings_lose_only_credential_values() {
    let mut findings = Findings::default();
    let redacted = xml::redact_connection_string(
        "Type=Tunnel;Host=203.0.113.250;UserPassword=SNTLa;DeviceAuthenticationCode=SNTLb;Port=3671",
        "x",
        &mut findings,
    );
    assert_eq!(
        redacted.as_deref(),
        Some("Type=Tunnel;Host=203.0.113.250;UserPassword=;DeviceAuthenticationCode=;Port=3671")
    );
    let mut findings = Findings::default();
    assert_eq!(
        xml::redact_connection_string("opaque password blob", "x", &mut findings).as_deref(),
        Some("")
    );
    assert!(xml::redact_connection_string("Host=a;Port=1", "x", &mut findings).is_none());
}

#[test]
fn secret_free_residual_search_finds_long_values_only() {
    let values: BTreeSet<Vec<u8>> = [b"SNTLabcdef".to_vec(), b"123456".to_vec()].into();
    let needles = Needles::new(&values);
    assert!(needles.any_in(b"xxSNTLabcdefxx"));
    assert!(needles.any_in(b"123456"));
    assert!(!needles.any_in(b"SNTLabcde 12345"));
    assert_eq!(needles.count_in(b"123456 SNTLabcdef 123456", usize::MAX), 2);
    let mut findings = Findings::default();
    findings.remember(b"12345");
    assert!(findings.values.is_empty(), "short values are not searched");
}

#[test]
fn secret_free_sniff_tells_xml_archives_wide_text_and_binary_apart() {
    assert_eq!(xml::sniff(b"\xEF\xBB\xBF \n<KNX/>"), xml::Content::Xml);
    assert_eq!(xml::sniff(b"PK\x03\x04rest"), xml::Content::Zip);
    assert_eq!(xml::sniff(&utf16("<KNX/>")), xml::Content::WideText);
    assert_eq!(xml::sniff(b"<\0K\0"), xml::Content::WideText);
    assert_eq!(xml::sniff(b"\0\0\x01\x02binary"), xml::Content::Other);
    assert_eq!(xml::sniff(b"plain text"), xml::Content::Other);
}
