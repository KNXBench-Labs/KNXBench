//! Regression tests for consented community evidence.
use std::fs;
use std::io::Write;
use std::process::Command;
fn knx(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn analysis_above_32_mib_is_admitted_and_reports_xml_scan_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("large.knxprod");
    let mut archive = zip::ZipWriter::new(fs::File::create(&input).unwrap());
    archive
        .start_file(
            "large.xml",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
    let chunk = [b' '; 64 * 1024];
    for _ in 0..33 * 16 {
        archive.write_all(&chunk).unwrap();
    }
    archive.finish().unwrap();
    assert!(input.metadata().unwrap().len() > 32 * 1024 * 1024);
    let output = knx(&["contribution", "analyze", input.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "partial");
    assert_eq!(report["members"].as_array().unwrap().len(), 1);
    assert_eq!(report["members"][0]["sampleAllowed"], false);
    assert_eq!(report["originalAllowed"], false);
    assert!(report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["name"] == "typed-import" && c["status"] == "not-examined"));
}

#[test]
fn sparse_input_above_1024_mib_is_refused_without_reading_or_export() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("oversize.knxprod");
    fs::File::create(&input)
        .unwrap()
        .set_len(knx_app::contribution::MAX_INPUT_BYTES as u64 + 1)
        .unwrap();
    let output = knx(&["contribution", "analyze", input.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("input size limit exceeded"));
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 1);
}

#[test]
fn real_cli_analyzes_offline_and_exports_without_overwriting() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.knxproj");
    fs::write(&input, knx_testsupport::minimal_knxproj_bytes()).unwrap();
    let result = knx(&["contribution", "analyze", input.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["kind"], "project");
    assert!(report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["name"] == "hardware" && c["status"] == "not-examined"));
    let opts = dir.path().join("options.json");
    fs::write(
        &opts,
        r#"{"audience":"public","sampleIds":[],"consent":true}"#,
    )
    .unwrap();
    let out = dir.path().join("evidence.zip");
    let args = [
        "contribution",
        "export",
        input.to_str().unwrap(),
        out.to_str().unwrap(),
        "--options",
        opts.to_str().unwrap(),
    ];
    let result = knx(&args);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let original = fs::read(&out).unwrap();
    assert!(original.starts_with(b"PK"));
    let result = knx(&args);
    assert!(!result.status.success());
    assert_eq!(fs::read(&out).unwrap(), original);
}
#[test]
fn real_cli_reports_ap1_sequences_but_reduced_preview_omits_their_source_data() {
    let dir = tempfile::tempdir().unwrap();
    let bytes=knx_testsupport::zip_with_entries(&[
        ("knx_master.xml", br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Synthetic"/></Manufacturers><MaskVersions><MaskVersion Id="MV-07B0"><HawkConfigurationData><Procedures><Procedure ProcedureType="Load" ProcedureSubType="ap1"><LdCtrlConnect/><LdCtrlMerge MergeId="2"/><LdCtrlMerge MergeId="4"/><LdCtrlRestart/></Procedure></Procedures></HawkConfigurationData></MaskVersion></MaskVersions></MasterData></KNX>"#),
        ("M-0001/A.xml", br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-1" MaskVersion="MV-07B0" LoadProcedureStyle="MergedProcedure"><Static><LoadProcedures><LoadProcedure MergeId="2"><LdCtrlRelSegment LsmIdx="4" Size="16"/></LoadProcedure><LoadProcedure MergeId="4"><LdCtrlWriteRelMem ObjIdx="4" Size="16"/></LoadProcedure></LoadProcedures></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#),
    ]);
    let input = dir.path().join("source.knxprod");
    fs::write(&input, bytes).unwrap();
    let result = knx(&["contribution", "analyze", input.to_str().unwrap()]);
    assert!(result.status.success());
    let local: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(local["procedureResolutions"][0]["status"], "expanded");
    assert_eq!(local["procedureResolutions"][0]["executable"], false);
    let sha = local["procedureResolutions"][0]["sources"][0]["sha256"]
        .as_str()
        .unwrap();
    let options = dir.path().join("options.json");
    fs::write(
        &options,
        r#"{"audience":"public","sampleIds":[],"consent":false}"#,
    )
    .unwrap();
    let result = knx(&[
        "contribution",
        "preview",
        input.to_str().unwrap(),
        "--options",
        options.to_str().unwrap(),
    ]);
    assert!(result.status.success());
    let preview: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let text = preview["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["path"] == "findings.json")
        .unwrap()["text"]
        .as_str()
        .unwrap();
    assert!(!text.contains("procedureResolutions"));
    assert!(!text.contains(sha));
    assert!(!text.contains("M-0001_A-1"));
    assert!(text.contains("offline-procedure-resolution"));
}
#[test]
fn real_cli_refuses_bad_inputs_before_creating_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.knxprod");
    fs::write(&input, b"not a ZIP").unwrap();
    let opts = dir.path().join("options.json");
    fs::write(&opts, r#"{"audience":"public","consent":true}"#).unwrap();
    let out = dir.path().join("evidence.zip");
    assert!(!knx(&[
        "contribution",
        "export",
        input.to_str().unwrap(),
        out.to_str().unwrap(),
        "--options",
        opts.to_str().unwrap()
    ])
    .status
    .success());
    assert!(!out.exists());
}
