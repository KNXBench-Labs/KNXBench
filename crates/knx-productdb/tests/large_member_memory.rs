//! PDB-10: installing the largest observed XML member stays within a documented memory bound.
//!
//! The member is as large as the largest one observed in the private corpus
//! (54,803,397 bytes, an MDT application program;
//! docs/PRODUCT_DATABASE_CORPUS.md).
//!
//! Its own test binary with one test, so the process high-water mark
//! (`VmHWM`, Linux) belongs to this install alone. The bound it asserts is
//! the one documented in docs/KNOWN_LIMITATIONS.md: install holds each
//! member whole (capped at 64 MiB by `MAX_MEMBER_SIZE`), so memory is
//! bounded by a small multiple of the largest member, not constant.

use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;

const LARGEST_OBSERVED_XML_MEMBER: usize = 54_803_397;

fn vm_hwm_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib * 1024)
}

/// A schema-valid program padded with `Parameter` rows to `target` bytes.
fn large_program(target: usize) -> Vec<u8> {
    let head = br#"<?xml version="1.0" encoding="utf-8"?><KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0083"><ApplicationPrograms><ApplicationProgram Id="M-0083_A-0001-01-0000" ApplicationNumber="1" ApplicationVersion="1" ProgramType="ApplicationProgram" MaskVersion="MV-07B0" Name="Large" DefaultLanguage="en-US" LoadProcedureStyle="MergedProcedure" PeiType="0" DynamicTableManagement="false" Linkable="false"><Static><ParameterTypes><ParameterType Id="M-0083_A-0001-01-0000_PT-N" Name="N"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255"/></ParameterType></ParameterTypes><Parameters>"#;
    let tail = br#"</Parameters></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
    let mut out = Vec::with_capacity(target + 256);
    out.extend_from_slice(head);
    let mut n = 0u64;
    while out.len() + tail.len() < target {
        write!(
            out,
            r#"<Parameter Id="M-0083_A-0001-01-0000_P-{n}" Name="P{n}" ParameterType="M-0083_A-0001-01-0000_PT-N" Text="Parameter {n} padding the member to the observed size" Value="0"/>"#
        )
        .unwrap();
        n += 1;
    }
    out.extend_from_slice(tail);
    out
}

#[test]
#[ignore = "heavy: builds and installs a 55 MB member; run with --ignored"]
fn installing_the_largest_observed_xml_member_stays_within_the_documented_bound() {
    let Some(before) = vm_hwm_bytes() else {
        eprintln!("VmHWM unavailable on this platform; nothing to measure");
        return;
    };
    let program = large_program(LARGEST_OBSERVED_XML_MEMBER);
    assert!(program.len() >= LARGEST_OBSERVED_XML_MEMBER);
    let master = br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0083" Name="Large"/></Manufacturers></MasterData></KNX>"#;
    let package = {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("knx_master.xml", options).unwrap();
        writer.write_all(master).unwrap();
        writer
            .start_file("M-0083/M-0083_A-0001-01-0000.xml", options)
            .unwrap();
        writer.write_all(&program).unwrap();
        writer.finish().unwrap().into_inner()
    };
    let member = program.len() as u64;
    drop(program);
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let report = knx_productdb::install_package(&conn, "large.knxprod", &package).unwrap();
    let after = vm_hwm_bytes().unwrap();
    assert!(report.facts.is_some());
    let peak = after.saturating_sub(before);
    eprintln!(
        "large-member install: member {member} B, package {} B, peak RSS growth {peak} B ({:.2}x member)",
        package.len(),
        peak as f64 / member as f64
    );
    // docs/KNOWN_LIMITATIONS.md states this bound; keep them in step.
    assert!(
        peak <= 8 * member,
        "peak RSS growth {peak} B exceeds 8x the {member} B member"
    );
}
