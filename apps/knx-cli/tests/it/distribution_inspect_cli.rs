//! Exercises the real distribution-inspection CLI and proves source/profile nonmutation.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SOURCE:&[u8]=br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Catalog><CatalogItem Id="C" ProductRefId="P" Hardware2ProgramRefId="H2P"/></Catalog><ApplicationPrograms><ApplicationProgram Id="A" Name="Authored"><Static><ParameterTypes><ParameterType Id="T"><TypeNumber Type="unsignedInt" SizeInBit="8"/></ParameterType></ParameterTypes><Parameters><Parameter Id="control" ParameterType="T" Value="1"/></Parameters><ParameterRefs><ParameterRef Id="R" RefId="control"/></ParameterRefs></Static><Dynamic><choose ParamRefId="R"><when test="1"><ParameterRefRef RefId="R"/></when></choose></Dynamic></ApplicationProgram></ApplicationPrograms><Baggages><Baggage Id="B" Name="help.txt"/></Baggages><Hardware><Hardware Id="H"><Products><Product Id="P"/></Products><Hardware2Programs><Hardware2Program Id="H2P"><ApplicationProgramRef RefId="A"/></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["products", "inspect-distribution"])
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("TMPDIR", home.join("tmp"))
        .current_dir(home)
        .output()
        .unwrap()
}
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut todo = vec![root.to_path_buf()];
    while let Some(dir) = todo.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                todo.push(p);
            } else {
                files.insert(
                    p.strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(p).unwrap(),
                );
            }
        }
    }
    files
}
fn home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    for d in ["data/knx", "config/knx", "tmp"] {
        std::fs::create_dir_all(home.path().join(d)).unwrap();
    }
    std::fs::write(
        home.path().join("data/knx/products.sqlite"),
        b"DO NOT OPEN OR MIGRATE ME",
    )
    .unwrap();
    std::fs::write(home.path().join("config/knx/sentinel"), b"untouched").unwrap();
    home
}
#[test]
fn real_cli_emits_useful_json_without_source_sidecar_profile_or_authority_mutation() {
    let home = home();
    let source = home.path().join("authored.data");
    std::fs::write(&source, SOURCE).unwrap();
    let before = snapshot(home.path());
    let out = run(home.path(), &[source.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "real distribution inspection must succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["formatVersion"], 3);
    assert_eq!(r["sourceKind"], "manufacturerXml");
    assert_eq!(r["sections"].as_array().unwrap().len(), 4);
    assert_eq!(r["applications"][0]["activeParameterRefIds"][0], "R");
    assert_eq!(
        r["applications"][0]["parameterFallbacks"][0]["origin"],
        "parameter"
    );
    assert_eq!(r["complete"], false);
    assert!(r["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "writeAuthorityNotPerformed"));
    assert_eq!(out.stderr, b"");
    assert_eq!(snapshot(home.path()), before);
    let again = run(home.path(), &[source.to_str().unwrap()]);
    assert_eq!(again.stdout, out.stdout);
    assert_eq!(snapshot(home.path()), before);
}
#[test]
fn real_cli_windows_zip_keeps_raw_member_names_and_profile_untouched() {
    use std::io::Write;
    let home = home();
    let source = home.path().join("authored.zip");
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    writer
        .start_file(
            "data\\authored.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    writer.write_all(SOURCE).unwrap();
    writer
        .start_file("data\\inert.js", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"throw 'never execute';").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    std::fs::write(&source, &bytes).unwrap();
    let before = snapshot(home.path());
    let out = run(home.path(), &[source.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let r: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(r["sourceKind"], "distributionZip");
    assert_eq!(r["members"][0]["decodedName"], "data\\authored.xml");
    assert_eq!(r["members"][0]["logicalPath"], "data/authored.xml");
    assert_eq!(r["applications"][0]["source"][0], 0);
    assert_eq!(r["applications"][0]["activeParameterRefIds"][0], "R");
    assert_eq!(snapshot(home.path()), before);
}

#[test]
fn real_cli_format_three_resolves_compact_coordinates_and_matches_library_json() {
    let home = home();
    let source = home.path().join("format2.xml");
    std::fs::write(&source, SOURCE).unwrap();
    let before = snapshot(home.path());
    let output = run(home.path(), &[source.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: knx_productdb::distribution::DistributionInspection =
        serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report.format_version, 3);
    let expected =
        knx_productdb::distribution::inspect_distribution("format2.xml", SOURCE).unwrap();
    assert_eq!(report, expected);
    assert_eq!(report.serialized_size().unwrap() + 1, output.stdout.len());
    for identity in &report.identities {
        let coordinate = report.resolve_coordinate(identity.source).unwrap();
        assert!(coordinate.path.starts_with("/KNX[1]/"));
        assert!(SOURCE[coordinate.start_byte as usize..].starts_with(b"<"));
    }
    assert_eq!(snapshot(home.path()), before);
}

#[test]
fn real_cli_reports_dependency_evidence_exactly_like_the_library() {
    // Independently authored; the Script, handler and functions are inert test text.
    const DECLARATIONS: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/knx-productdb/tests/it/distribution_dependencies/declarations.xml"
    ));
    let home = home();
    let source = home.path().join("declarations.xml");
    std::fs::write(&source, DECLARATIONS).unwrap();
    let before = snapshot(home.path());
    let out = run(home.path(), &[source.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stderr, b"");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let rows = json["dependencyEvidence"].as_array().unwrap();
    assert_eq!(
        rows.iter().map(|r| r["kind"].clone()).collect::<Vec<_>>(),
        [
            "parameterCalculation",
            "parameterValidation",
            "script",
            "assign",
            "buttonHandler"
        ]
    );
    assert_eq!(json["formatVersion"], 3);
    assert!(rows
        .iter()
        .all(|r| r["disposition"] == "retainedNotExecuted"));
    let online = rows[4]["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "EventHandlerOnline")
        .unwrap();
    assert_eq!(online["presence"], "presentValue");
    assert_eq!(online["value"], "ConnectionOriented");
    assert_eq!(rows[3]["assignmentForm"], "sourceReference");
    assert_eq!(rows[3]["endpoints"][0]["referenceState"], "resolved");
    assert!(json["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "dependenciesNotExecuted"));
    assert!(!String::from_utf8_lossy(&out.stdout).contains("return 999"));
    let report: knx_productdb::distribution::DistributionInspection =
        serde_json::from_slice(&out.stdout).unwrap();
    let expected =
        knx_productdb::distribution::inspect_distribution("declarations.xml", DECLARATIONS)
            .unwrap();
    assert_eq!(report, expected);
    assert_eq!(snapshot(home.path()), before);
}

#[test]
fn argument_refusal_precedes_reads_and_never_accepts_db_or_output_authority() {
    let home = home();
    let before = snapshot(home.path());
    for args in [
        vec![],
        vec!["does-not-exist", "extra"],
        vec!["does-not-exist", "--product-db", "private"],
        vec!["--download"],
        vec!["--install"],
        vec!["does-not-exist", "--output", "report.json"],
    ] {
        let out = run(home.path(), &args);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        let text = String::from_utf8_lossy(&out.stderr);
        assert!(text.contains("usage:"), "{text}");
        assert!(
            !text.contains("No such file"),
            "must validate all args before reads: {text}"
        );
    }
    assert_eq!(snapshot(home.path()), before);
}
#[test]
fn malformed_and_oversized_regular_sources_return_errors_without_outputs_or_mutation() {
    let home = home();
    let bad = home.path().join("bad.xml");
    std::fs::write(&bad, b"<KNX>").unwrap();
    let huge = home.path().join("oversized.xml");
    let file = std::fs::File::create(&huge).unwrap();
    file.set_len(
        knx_productdb::distribution::DistributionLimits::default().max_input_bytes as u64 + 1,
    )
    .unwrap();
    drop(file);
    for p in [&bad, &huge] {
        let out = run(home.path(), &[p.to_str().unwrap()]);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(
            serde_json::from_slice::<serde_json::Value>(&out.stderr).is_ok(),
            "typed error JSON expected"
        );
    }
    assert_eq!(
        std::fs::metadata(&huge).unwrap().len(),
        knx_productdb::distribution::DistributionLimits::default().max_input_bytes as u64 + 1
    );
    assert_eq!(std::fs::read(&bad).unwrap(), b"<KNX>");
    assert!(!home
        .path()
        .join("data/knx/products.sqlite-journal")
        .exists());
}
