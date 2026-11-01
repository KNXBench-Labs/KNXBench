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
fn explicit_html_is_offline_passive_and_json_remains_the_default() {
    use std::io::Write;
    let home = home();
    let xml = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-1"><ApplicationPrograms><ApplicationProgram Id="A" Name="&lt;img src=x onerror=alert(1)&gt; &lt;script&gt;TEXT&lt;/script&gt;" ContextHelpFile="H"><Static><Script><![CDATA[VENDOR_SCRIPT_MUST_NOT_APPEAR]]></Script></Static></ApplicationProgram></ApplicationPrograms><Baggages><Baggage Id="P" Name="pixel.png" TargetPath=""/><Baggage Id="B" Name="broken.png" TargetPath=""/><Baggage Id="H" Name="help.zip" TargetPath=""/></Baggages></Manufacturer></ManufacturerData></KNX>"#;
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        // Standard PNG CRC, implemented here so this synthetic CLI fixture needs no new dependency.
        let mut crc = 0xffff_ffffu32;
        for b in kind.iter().chain(data) {
            crc ^= u32::from(*b);
            for _ in 0..8 {
                crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb8_8320 } else { 0 };
            }
        }
        out.extend_from_slice(&(!crc).to_be_bytes());
    }
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut png, b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0]);
    let pixel = [0u8, 255, 0, 0, 255];
    let (mut a, mut b) = (1u32, 0u32);
    for p in pixel {
        a = (a + u32::from(p)) % 65521;
        b = (b + a) % 65521;
    }
    let mut z = vec![0x78, 0x01, 0x01, 0x05, 0x00, 0xfa, 0xff];
    z.extend_from_slice(&pixel);
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    chunk(&mut png, b"IDAT", &z);
    chunk(&mut png, b"IEND", &[]);
    let mut help = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    help.start_file("help.html", zip::write::SimpleFileOptions::default())
        .unwrap();
    help.write_all(b"<script>HELP_MUST_NOT_APPEAR</script>")
        .unwrap();
    let help = help.finish().unwrap().into_inner();
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("product.xml", xml.as_slice()),
        ("product.baggages/pixel.png", png.as_slice()),
        (
            "product.baggages/broken.png",
            b"\x89PNG\r\n\x1a\n".as_slice(),
        ),
        ("product.baggages/help.zip", help.as_slice()),
    ] {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    let source = home.path().join("hostile.zip");
    std::fs::write(&source, zip.finish().unwrap().into_inner()).unwrap();
    let before = snapshot(home.path());
    let default = run(home.path(), &[source.to_str().unwrap()]);
    assert!(
        default.status.success(),
        "{}",
        String::from_utf8_lossy(&default.stderr)
    );
    let json = run(
        home.path(),
        &[source.to_str().unwrap(), "--output-format", "json"],
    );
    assert!(
        json.status.success(),
        "{}",
        String::from_utf8_lossy(&json.stderr)
    );
    assert_eq!(json.stdout, default.stdout);
    let html = run(
        home.path(),
        &[source.to_str().unwrap(), "--output-format", "html"],
    );
    assert!(
        html.status.success(),
        "{}",
        String::from_utf8_lossy(&html.stderr)
    );
    assert!(html.stderr.is_empty());
    let text = String::from_utf8(html.stdout).unwrap();
    assert!(text.starts_with("<!doctype html>"));
    assert!(
        text.contains("data-resource-preview"),
        "must display a decoded PNG, not metadata alone"
    );
    assert!(text.contains("src=\"data:image/png;base64,"));
    assert!(text.contains("data-resource-fallback"));
    assert!(text.contains("pngDecodeFailed"));
    assert!(text.contains("helpArchiveAvailable"));
    assert!(text.contains("&lt;img src=x onerror=alert(1)&gt;"));
    for forbidden in [
        "<script",
        "<img src=x",
        "VENDOR_SCRIPT_MUST_NOT_APPEAR",
        "HELP_MUST_NOT_APPEAR",
        "<iframe",
        "<object",
        "<form",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
    for directive in [
        "script-src 'none'",
        "connect-src 'none'",
        "base-uri 'none'",
        "form-action 'none'",
        "default-src 'none'",
    ] {
        assert!(text.contains(directive), "{directive}");
    }
    assert_eq!(snapshot(home.path()), before);
}

#[test]
fn html_sampling_is_visible_and_does_not_remove_json_facts() {
    let home = home();
    let name = "n".repeat(400);
    let bags = (0..205)
        .map(|i| format!(r#"<Baggage Id="B{i}" Name="x" TargetPath=""/>"#))
        .collect::<String>();
    let xml = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M"><ApplicationPrograms><ApplicationProgram Id="A" Name="{name}"/></ApplicationPrograms><Baggages>{bags}</Baggages></Manufacturer></ManufacturerData></KNX>"#
    );
    let source = home.path().join("sample.xml");
    std::fs::write(&source, xml).unwrap();
    let before = snapshot(home.path());
    let json = run(home.path(), &[source.to_str().unwrap()]);
    assert!(json.status.success());
    let j: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(j["resourceDeclarations"].as_array().unwrap().len(), 205);
    assert_eq!(j["applications"][0]["name"], name);
    let html = run(
        home.path(),
        &["--output-format", "html", source.to_str().unwrap()],
    );
    assert!(html.status.success());
    let text = String::from_utf8(html.stdout).unwrap();
    assert!(text.contains("200 of 205 rows shown"));
    assert_eq!(text.matches("<article ").count(), 200);
    assert!(text.contains("[text sampled]"));
    assert_eq!(snapshot(home.path()), before);
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
    assert_eq!(r["formatVersion"], 5);
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
fn real_cli_format_five_resolves_compact_coordinates_and_matches_library_json() {
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
    assert_eq!(report.format_version, 5);
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
    assert_eq!(json["formatVersion"], 5);
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
fn real_cli_reports_source_candidates_exactly_like_the_library() {
    // Independently authored; byte-identical to the readiness packet fixtures.
    const VALUES: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/knx-productdb/tests/it/distribution_candidates/values.xml"
    ));
    const UNION: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/knx-productdb/tests/it/distribution_candidates/union-alternatives.xml"
    ));
    let home = home();
    for (name, bytes) in [("values.xml", VALUES), ("union.xml", UNION)] {
        let source = home.path().join(name);
        std::fs::write(&source, bytes).unwrap();
    }
    let before = snapshot(home.path());
    let values = run(
        home.path(),
        &[home.path().join("values.xml").to_str().unwrap()],
    );
    let union = run(
        home.path(),
        &[home.path().join("union.xml").to_str().unwrap()],
    );
    for out in [&values, &union] {
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(out.stderr, b"");
    }
    let json: serde_json::Value = serde_json::from_slice(&values.stdout).unwrap();
    assert_eq!(json["formatVersion"], 5);
    let rows = json["sourceCandidates"].as_array().unwrap();
    assert_eq!(
        rows.iter()
            .map(|r| (r["kind"].clone(), r["valueCheck"]["comparison"].clone()))
            .collect::<Vec<_>>(),
        [
            ("parameterValue", "valueAbsent"),
            ("parameterValue", "valueEmpty"),
            ("parameterValue", "below"),
            ("parameterValue", "above"),
            ("parameterRefValue", "valueEmpty"),
        ]
        .map(|(k, c)| (serde_json::Value::from(k), serde_json::Value::from(c))),
        "the default lists findings; values inside their bounds are counted, not listed"
    );
    let totals = json["sourceCandidateTotals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["kind"].as_str().unwrap().to_string(),
                t["valueComparison"].as_str().unwrap().to_string(),
                t["occurrences"].as_u64().unwrap(),
                t["rows"].as_u64().unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let total = |kind: &str, comparison: &str| {
        totals
            .iter()
            .find(|t| t.0 == kind && t.1 == comparison)
            .map(|t| (t.2, t.3))
    };
    assert_eq!(total("parameterValue", "within"), Some((3, 0)));
    assert_eq!(total("parameterRefValue", "within"), Some((1, 0)));
    assert_eq!(total("parameterValue", "below"), Some((1, 1)));
    assert_eq!(
        totals.iter().map(|t| t.2).sum::<u64>(),
        9,
        "every checked occurrence is counted"
    );
    for row in rows {
        assert_eq!(row["severity"], "sourceCandidate");
        assert_eq!(row["initialization"], "notPerformed");
        assert_eq!(row["activity"]["state"], "unavailable");
    }
    let union_json: serde_json::Value = serde_json::from_slice(&union.stdout).unwrap();
    let members = union_json["sourceCandidates"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "unionMemberInterval")
        .map(|r| {
            (
                r["intervalCheck"]["comparison"].clone(),
                r["activity"]["state"].clone(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        members,
        [
            ("outsideExtent", "unavailable"),
            ("offsetAbsent", "unavailable"),
        ]
        .map(|(c, a)| (serde_json::Value::from(c), serde_json::Value::from(a)))
    );
    for (out, name, bytes) in [
        (&values, "values.xml", VALUES),
        (&union, "union.xml", UNION),
    ] {
        let report: knx_productdb::distribution::DistributionInspection =
            serde_json::from_slice(&out.stdout).unwrap();
        let expected = knx_productdb::distribution::inspect_distribution(name, bytes).unwrap();
        assert_eq!(report, expected);
    }
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
        vec!["does-not-exist", "--output-format", "svg"],
        vec!["does-not-exist", "--output-format"],
        vec![
            "does-not-exist",
            "--output-format",
            "html",
            "--output-format",
            "json",
        ],
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
