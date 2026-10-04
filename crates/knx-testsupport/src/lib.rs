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
/// without a copy) has none of it. A test that needs the corpus is
/// `#[ignore = "requires the gitignored OriginalData/ corpus; run with
/// --ignored"]` and `assert!`s this first: without the corpus it is reported
/// *ignored* by default and fails by name under `--ignored`. Returning early
/// instead would count as a pass that tested nothing
/// (docs/KNOWN_LIMITATIONS.md §131; `cargo run -p xtask -- check-corpus-gates`).
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

/// The ZipCrypto password of [`zipcrypto_minimal_knxproj_bytes`]: a public
/// test-fixture value, not a secret. Tests also use it as a canary — it must
/// never show up in a report, log, error or saved store.
pub const ZIPCRYPTO_MINIMAL_PASSWORD: &str = "ar08-Hunter-Secret";

/// [`minimal_knxproj_bytes`]' project, with `P-0001/` moved into a nested
/// `P-0001.zip` that Info-ZIP `zip -P` protected with ZipCrypto (ETS4/ETS5
/// password protection). Generated outside this repository's code; the
/// exact commands are in `fixtures/README.md`.
pub fn zipcrypto_minimal_knxproj_bytes() -> Vec<u8> {
    include_bytes!("../fixtures/zipcrypto-minimal.knxproj").to_vec()
}

/// Writes [`zipcrypto_minimal_knxproj_bytes`] into `dir` and returns its path.
pub fn write_zipcrypto_minimal_knxproj(dir: &Path) -> PathBuf {
    let path = dir.join("protected.knxproj");
    std::fs::write(&path, zipcrypto_minimal_knxproj_bytes())
        .expect("writing a fixture into a temp dir");
    path
}

/// Builds a synthetic ZIP from explicit entries for cross-crate import tests.
/// No entries, signatures or XML schemas are inferred or validated here.
pub fn zip_with_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
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

/// Synthetic mapping witness, not a genuine ETS sample or XSD-validity claim.
/// Two lines, one space and two devices sharing an object RefId but carrying
/// different overrides/links. Arguments are literal fixture tokens, not an XML
/// escaping API. The second object is a well-formed continuation control.
pub fn mapping_boundary_knxproj_bytes(
    version: u32,
    installation_default_line: Option<&str>,
    space_default_line: Option<&str>,
    shared_object_ref: &str,
) -> Vec<u8> {
    assert!([11, 21, 23].contains(&version));
    let default_line_attribute = |value: Option<&str>| {
        value.map_or_else(String::new, |value| format!(r#" DefaultLine="{value}""#))
    };
    let mut devices = String::new();
    let other_ref = if version == 11 {
        "M-0001_A-1_O-19_R-2"
    } else {
        "O-19_R-2"
    };
    for (index, text, read) in [
        (1, "first device", "Enabled"),
        (2, "second device", "Disabled"),
    ] {
        let links = if version == 11 {
            format!(r#"<Connectors><Send GroupAddressRefId="P-0001-0_GA-{index}"/></Connectors>"#)
        } else {
            String::new()
        };
        let link_attribute = if version == 11 {
            String::new()
        } else {
            format!(r#" Links="GA-{index}""#)
        };
        let tree = match version {
            11 => String::new(),
            21 => format!(
                r#"<GroupObjectTree><Nodes><Node GroupObjectInstances="{shared_object_ref} {other_ref}"/></Nodes></GroupObjectTree>"#
            ),
            23 => format!(
                r#"<GroupObjectTree GroupObjectInstances="{shared_object_ref} {other_ref}"/>"#
            ),
            _ => unreachable!("fixture version checked above"),
        };
        devices.push_str(&format!(
            r#"<DeviceInstance Id="P-0001-0_DI-{index}" Name="{text}" Address="{index}"
                ProductRefId="M-0001_H-{index}_P-1" Hardware2ProgramRefId="M-0001_H-{index}_HP-1">
              <ComObjectInstanceRefs>
                <ComObjectInstanceRef RefId="{shared_object_ref}" Text="{text}" ReadFlag="{read}" IsActive="1"{link_attribute}>{links}</ComObjectInstanceRef>
                <ComObjectInstanceRef RefId="{other_ref}" IsActive="1"/>
              </ComObjectInstanceRefs>{tree}
            </DeviceInstance>"#
        ));
    }
    let (line_contents, medium, container, element) = if version == 11 {
        (
            devices,
            r#" MediumTypeRefId="MT-0""#,
            "Buildings",
            "BuildingPart",
        )
    } else {
        (
            format!(
                r#"<Segment Id="P-0001-0_L-2_S-1" Number="0" MediumTypeRefId="MT-0">{devices}</Segment>"#
            ),
            "",
            "Locations",
            "Space",
        )
    };
    let installation_default = default_line_attribute(installation_default_line);
    let space_default = default_line_attribute(space_default_line);
    let topology = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/{version}">
          <Project Id="P-0001"><Installations>
            <Installation InstallationId="0"{installation_default}>
              <Topology><Area Id="P-0001-0_A-1" Address="1">
                <Line Id="P-0001-0_L-2" Address="1"{medium}>{line_contents}</Line>
                <Line Id="P-0001-0_L-3" Address="2"{medium}/>
              </Area></Topology>
              <{container}><{element} Id="P-0001-0_BP-1" Name="Synthetic room" Type="Room"{space_default}>
                <DeviceInstanceRef RefId="P-0001-0_DI-1"/>
              </{element}></{container}>
              <GroupAddresses><GroupRanges>
                <GroupRange Id="P-0001-0_GR-1" RangeStart="1" RangeEnd="100" Name="Synthetic range">
                  <GroupAddress Id="P-0001-0_GA-1" Address="1" Name="First group"/>
                  <GroupAddress Id="P-0001-0_GA-2" Address="2" Name="Second group"/>
                </GroupRange>
              </GroupRanges></GroupAddresses>
            </Installation>
          </Installations></Project>
        </KNX>"#
    );
    let info = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/{version}"><Project Id="P-0001">
          <ProjectInformation Name="Synthetic mapping witness" GroupAddressStyle="ThreeLevel"/>
        </Project></KNX>"#
    );
    zip_with_entries(&[
        ("P-0001.signature", b"synthetic signature"),
        ("P-0001/0.xml", topology.as_bytes()),
        ("P-0001/Project.xml", info.as_bytes()),
        ("note.txt", b"opaque mapping witness"),
    ])
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
/// # Panics
///
/// On any `read_dir`, directory-entry or `file_type` error, including a
/// `root` that does not exist. A partially readable corpus is reported, not
/// measured as a smaller one.
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
    // A test that walks the corpus measures it, so an unreadable directory
    // must stop the test rather than quietly shrink what it measured
    // (docs/KNOWN_LIMITATIONS.md §131). Whether the corpus exists at all is
    // the caller's gate, asserted before the walk starts.
    let read_dir = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read corpus directory {}: {e}", dir.display()));
    let mut entries: Vec<_> = read_dir
        .map(|entry| {
            entry.unwrap_or_else(|e| panic!("cannot list corpus directory {}: {e}", dir.display()))
        })
        .collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let file_type = entry
            .file_type()
            .unwrap_or_else(|e| panic!("cannot stat corpus entry {}: {e}", entry.path().display()));
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
///
/// Inherits [`walk_corpus_files`]'s panics on unreadable directories.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, empty directory under the system temp dir, removed on drop.
    /// Hand-rolled so this crate keeps its single dependency.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("knx-testsupport-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn walk_lists_nested_files_name_sorted_depth_first() {
        let scratch = Scratch::new("walk");
        std::fs::create_dir_all(scratch.0.join("b")).unwrap();
        std::fs::write(scratch.0.join("b/z.knxprod"), b"").unwrap();
        std::fs::write(scratch.0.join("a.knxprod"), b"").unwrap();
        let names: Vec<_> = walk_corpus_files(&scratch.0)
            .into_iter()
            .map(|p| p.strip_prefix(&scratch.0).unwrap().to_path_buf())
            .collect();
        assert_eq!(
            names,
            [PathBuf::from("a.knxprod"), PathBuf::from("b/z.knxprod")]
        );
    }

    /// A root that cannot be read is an error, not an empty corpus: a test
    /// measuring the corpus would otherwise report on nothing (§131).
    #[test]
    #[should_panic(expected = "cannot read corpus directory")]
    fn walk_panics_on_a_missing_root_instead_of_measuring_nothing() {
        let scratch = Scratch::new("missing");
        walk_corpus_files(&scratch.0.join("not-there"));
    }
}
