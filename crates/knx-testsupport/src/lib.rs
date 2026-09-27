//! Owns the corpus fixture paths so no crate hard-codes the maintainer's export filenames.

use std::path::{Path, PathBuf};

/// The workspace root, resolved from *this crate's own* `CARGO_MANIFEST_DIR`
/// (`<root>/crates/knx-testsupport`) rather than the caller's. `env!` expands
/// where it is written, so this is always this crate's manifest directory no
/// matter who calls — which is the point: consumers live under both
/// `crates/<name>` and `apps/<name>`, and a version that walked up from the
/// caller's manifest would have to know which. Two parents up from
/// `crates/knx-testsupport` reaches the root; if this crate ever moves, this
/// is the one line that must move with it.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("knx-testsupport lives at <root>/crates/knx-testsupport")
        .to_path_buf()
}

fn overridable(env_var: &str, default_relative: &str) -> PathBuf {
    match std::env::var_os(env_var) {
        Some(value) => PathBuf::from(value),
        None => workspace_root().join(default_relative),
    }
}

/// The maintainer's real ETS4 export. Override with `KNXBENCH_REFERENCE_PROJECT`
/// to point at your own corpus — the golden tests assert counts measured
/// against the default fixture, so they will fail against a substitute; that
/// is expected, not a bug.
pub fn reference_ets4_path() -> PathBuf {
    overridable(
        "KNXBENCH_REFERENCE_PROJECT",
        "OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj",
    )
}

/// The same installation, re-exported from ETS 6.3.0. Override with
/// `KNXBENCH_REFERENCE_PROJECT_ETS6`; same golden-test caveat as
/// [`reference_ets4_path`].
pub fn reference_ets6_path() -> PathBuf {
    overridable(
        "KNXBENCH_REFERENCE_PROJECT_ETS6",
        "OriginalData/DemoProjects/Unser Zuhause ets 6.3.0 - 2026-09-02.knxproj",
    )
}

/// A second, independent schema-21 installation (KNX Association demo
/// project). Override with `KNXBENCH_REFERENCE_PROJECT_SCHEMA21`; same
/// golden-test caveat as [`reference_ets4_path`].
pub fn reference_kv_schema21_path() -> PathBuf {
    overridable(
        "KNXBENCH_REFERENCE_PROJECT_SCHEMA21",
        "OriginalData/DemoProjects/KV v2.5 - demo.knxproj",
    )
}

/// True when the gitignored `OriginalData/` fixture corpus is present
/// locally. It holds the maintainer's own real KNX installation and
/// manufacturer files — never committed, so CI (and any contributor
/// without a copy) has none of it. Every test that needs the corpus must
/// check this first and skip, not panic, or CI is permanently red.
///
/// All three projects, not just the ETS4 one: the three paths are
/// independently overridable, so checking one and handing out another is
/// how a guarded test still panics — override `KNXBENCH_REFERENCE_PROJECT`
/// alone and the ETS6 and schema-21 tests sail past the guard into an
/// `expect` on a file that was never there.
pub fn corpus_available() -> bool {
    reference_ets4_path().exists()
        && reference_ets6_path().exists()
        && reference_kv_schema21_path().exists()
}

/// A hand-written, structurally valid schema-11 `.knxproj` in memory: one
/// area, one line, one device, one group address, plus the `.signature`
/// entry [`knx_etsproj::Container::project_part`] needs to find the project
/// part at all. Six container entries in total, two of which the importer
/// regenerates rather than retains.
///
/// Exists so a test can exercise the *whole* import pipeline — container,
/// detection, parse, validate, map, infer, collect — without the gitignored
/// `OriginalData/` corpus, which CI and every contributor but the
/// maintainer lack. The real projects stay the golden-count fixtures; this
/// one is for tests about the pipeline's shape rather than its output, and
/// it is small enough that any count asserted against it can be read off
/// the source above.
pub fn minimal_knxproj_bytes() -> Vec<u8> {
    zip_with_entries(&[
        ("P-0001.signature", b"not a real signature"),
        ("P-0001/0.xml", MINIMAL_TOPOLOGY),
        ("P-0001/Project.xml", MINIMAL_PROJECT_INFO),
        ("knx_master.xml", MINIMAL_MASTER_DATA),
        ("M-0001/M-0001_A-1.xml", MINIMAL_MANUFACTURER_DATA),
        (
            "M-0001/Baggages/note.txt",
            b"carried through, never executed",
        ),
    ])
}

/// Writes `bytes` to a `.knxproj` file inside `dir` and returns its path —
/// the on-disk form of [`minimal_knxproj_bytes`], for the routes and CLI
/// paths that take a path rather than bytes.
pub fn write_minimal_knxproj(dir: &Path) -> PathBuf {
    let path = dir.join("minimal.knxproj");
    std::fs::write(&path, minimal_knxproj_bytes()).expect("writing a fixture into a temp dir");
    path
}

fn zip_with_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
    use std::io::{Cursor, Write};
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in entries {
        writer.start_file(*name, options).expect("zip entry");
        writer.write_all(bytes).expect("zip entry body");
    }
    writer.finish().expect("zip central directory").into_inner()
}

const MINIMAL_TOPOLOGY: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11" CreatedBy="ETS4" ToolVersion="ETS 4.1.8">
  <Project Id="P-0001">
    <Installations>
      <Installation InstallationId="0" Name="" DefaultLine="P-0001-0_L-2" CompletionStatus="Undefined">
        <Topology>
          <Area Id="P-0001-0_A-1" Name="A" Address="1" CompletionStatus="Undefined">
            <Line Id="P-0001-0_L-2" Name="L" Address="1" MediumTypeRefId="MT-0" CompletionStatus="Accepted">
              <DeviceInstance Id="P-0001-0_DI-1" Name="D" ProductRefId="M-0001_H-1_P-1"
                              Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1"
                              LastModified="2023-07-14T11:55:33" CompletionStatus="FinishedDesign"
                              IndividualAddressLoaded="1" ApplicationProgramLoaded="1"
                              ParametersLoaded="1" CommunicationPartLoaded="1"
                              MediumConfigLoaded="1" IsCommunicationObjectVisibilityCalculated="1"
                              Broken="0">
                <ComObjectInstanceRefs>
                  <ComObjectInstanceRef RefId="M-0001_A-1_O-0_R-1" DatapointType="" IsActive="1">
                    <Connectors><Send GroupAddressRefId="P-0001-0_GA-1" /></Connectors>
                  </ComObjectInstanceRef>
                </ComObjectInstanceRefs>
              </DeviceInstance>
            </Line>
          </Area>
        </Topology>
        <GroupAddresses>
          <GroupRanges>
            <GroupRange Id="P-0001-0_GR-1" Name="Licht" RangeStart="1" RangeEnd="255">
              <GroupRange Id="P-0001-0_GR-2" Name="An/Aus" RangeStart="1" RangeEnd="127">
                <GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" />
              </GroupRange>
            </GroupRange>
          </GroupRanges>
        </GroupAddresses>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

const MINIMAL_PROJECT_INFO: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <Project Id="P-0001">
    <ProjectInformation Name="Minimal" GroupAddressStyle="ThreeLevel" CompletionStatus="Undefined" />
  </Project>
</KNX>"#;

const MINIMAL_MASTER_DATA: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <MasterData />
</KNX>"#;

const MINIMAL_MANUFACTURER_DATA: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData />
</KNX>"#;

/// Recursively lists every regular file under `root`, name-sorted at each
/// directory level, depth-first. Symlinks are skipped, not followed.
///
/// This is dev-only test-fixture plumbing over the local (gitignored)
/// product corpus a contributor already controls — not code that parses
/// untrusted input — so it carries none of
/// `crates/knx-productdb/tests/corpus_support`'s hardening against a
/// hostile ZIP. It exists because the local corpus was reorganized into
/// per-manufacturer subdirectories (see `docs/KNOWN_LIMITATIONS.md`), so
/// tests that used to assume every fixture sat directly under
/// `OriginalData/ProductDatabases` can no longer just join a filename onto
/// the root.
pub fn walk_corpus_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_corpus_files_into(root, &mut files);
    files
}

fn walk_corpus_files_into(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = read_dir.filter_map(Result::ok).collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            walk_corpus_files_into(&path, files);
        } else if file_type.is_file() {
            files.push(path);
        }
    }
}

/// Recursively finds the one file under `root` whose file name (last path
/// component) matches `filename` exactly, wherever it sits in the tree.
///
/// Tests that name a fixture by its well-known filename (e.g.
/// `MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`) used to join it
/// directly onto `OriginalData/ProductDatabases`; the local corpus has
/// since grown per-manufacturer subdirectories, so that join now misses a
/// fixture that still exists, just one directory deeper. This resolves it
/// the same way `corpus_compatibility_matrix`'s own discovery already
/// does: by walking the tree instead of assuming a flat layout.
///
/// Returns `None` if no file with this name exists anywhere under `root`.
/// Panics if more than one does — naming a fixture by its filename only
/// makes sense while that name is unique; silently picking one of several
/// candidates would be a worse failure than refusing outright.
pub fn find_corpus_file(root: &Path, filename: &str) -> Option<PathBuf> {
    let mut matches: Vec<PathBuf> = walk_corpus_files(root)
        .into_iter()
        .filter(|path| path.file_name().and_then(|n| n.to_str()) == Some(filename))
        .collect();
    match matches.len() {
        0 => None,
        1 => matches.pop(),
        _ => panic!(
            "corpus fixture name {filename:?} is not unique under {}: {matches:?}",
            root.display()
        ),
    }
}
