//! Ingest of the committed ETS4 reference project's manufacturer data.
//!
//! Every number here was measured by running the ingest, never guessed. A
//! change to any of them is either a parser bug or a deliberate change
//! that must be explained in the commit that makes it.

use std::path::{Path, PathBuf};

fn reference_project_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

/// Reads the container directly with `zip`, so this crate's tests stay
/// independent of `knx-etsproj` exactly as its code is (ADR-0011).
fn manufacturer_files() -> Vec<(String, Vec<u8>)> {
    let file = std::fs::File::open(reference_project_path()).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut out = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let name = entry.name().to_string();
        if name.starts_with("M-") && !name.ends_with(".signature") {
            let mut bytes = Vec::new();
            std::io::copy(&mut entry, &mut bytes).unwrap();
            out.push((name, bytes));
        }
    }
    out
}

fn ingest_all() -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    for (path, bytes) in manufacturer_files() {
        knx_productdb::ingest_file(&conn, &path, &bytes).unwrap();
    }
    (dir, conn)
}

fn count(conn: &knx_productdb::Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap()
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_reference_projects_manufacturer_data_ingests_completely() {
    assert!(
        reference_project_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let (_dir, conn) = ingest_all();
    assert_eq!(count(&conn, "SELECT count(*) FROM manufacturer"), 4);
    assert_eq!(count(&conn, "SELECT count(*) FROM source_file"), 24);
    assert_eq!(count(&conn, "SELECT count(*) FROM application_program"), 12);
    assert_eq!(count(&conn, "SELECT count(*) FROM com_object"), 1250);
    assert_eq!(count(&conn, "SELECT count(*) FROM com_object_ref"), 5630);
    assert_eq!(count(&conn, "SELECT count(*) FROM parameter"), 11311);
    assert_eq!(
        count(&conn, "SELECT count(*) FROM parameter_type_enum"),
        3846
    );
    // Program 48057 + Catalog 109 + Hardware 24 = 48190 (measured; matches
    // the design's prediction exactly).
    assert_eq!(count(&conn, "SELECT count(*) FROM translation"), 48190);
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn every_blob_verifies_against_its_own_hash() {
    assert!(
        reference_project_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let (_dir, conn) = ingest_all();
    assert_eq!(knx_productdb::verify(&conn).unwrap(), vec![]);
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn a_second_ingest_of_the_same_files_stores_nothing_new() {
    assert!(
        reference_project_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let (_dir, conn) = ingest_all();
    let before = count(&conn, "SELECT count(*) FROM source_file");
    for (path, bytes) in manufacturer_files() {
        assert!(matches!(
            knx_productdb::ingest_file(&conn, &path, &bytes).unwrap(),
            knx_productdb::IngestOutcome::Skipped { .. }
        ));
    }
    assert_eq!(count(&conn, "SELECT count(*) FROM source_file"), before);
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_unknown_construct_table_is_a_short_list_not_a_flood() {
    assert!(
        reference_project_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    // Not zero — this is one manufacturer sample of four vendors, and an
    // unmodelled attribute is expected. What matters is that every one is
    // on record and that the result stays reviewable, which takes three
    // bounds on three different quantities, each guarding a different thing:
    //
    // * `distinct_constructs` bounds the length of the list a human reads —
    //   one entry per (`kind`, `xpath`, `name`).
    // * `rows` bounds the size of the table as the corpus grows. It is a
    //   corpus-size guard and nothing more: rows scale with
    //   files × programs × distinct xpaths, never with element instances,
    //   so de-modelling barely moves it. Measured: de-modelling the whole
    //   program body tops out at 1975 rows, under half of this bound.
    // * `occurrences` is the one that catches a parser regression rerouting
    //   the modelled tree through here, because that is the quantity such a
    //   regression inflates. Measured: dropping the `Parameter` arm alone
    //   takes it from 3078 to 91,468, and de-modelling the whole program
    //   body reaches 520,304.
    //
    // They differ because `ingest_unknown` keys on `source_sha256` as well:
    // one construct present in 12 program files is 12 rows, and
    // `occurrences` aggregates repeats only *within* a row, never across
    // files. `ApplicationProgram/@AdditionalAddressesCount` is exactly that
    // case here, 12 rows for one thing to review.
    //
    // Measured on this corpus 2026-09-14: 98 distinct constructs across 1013
    // rows from 24 source files (796 `Attribute`, 217 `Element`), summing to
    // 3078 occurrences. Either number moving is a deliberate change to
    // explain, not noise.
    let (_dir, conn) = ingest_all();
    let distinct_constructs = count(
        &conn,
        "SELECT count(*) FROM (SELECT DISTINCT kind, xpath, name FROM ingest_unknown)",
    );
    assert!(
        distinct_constructs < 200,
        "{distinct_constructs} distinct (kind, xpath, name) constructs to review, measured 98"
    );
    // Deliberately loose — ~4x headroom for a growing corpus, which at
    // today's construct profile is about 95 source files. For scale, this
    // corpus fills the eleven main parsed tables with 89,622 rows, ~22x
    // this bound (`manufacturer_files()` above only takes `M-*` members, so
    // `knx_master.xml` never reaches this fixture and three of those eleven
    // — `function_type`, `function_point`, `space_usage` — hold zero rows
    // here, T13 fix round 2); rerouting them here would not arrive
    // one-for-one, which is why this bound is not the regression guard.
    let rows = count(&conn, "SELECT count(*) FROM ingest_unknown");
    assert!(
        rows < 4000,
        "{rows} ingest_unknown rows, measured 1013 — that is a flood, not a list"
    );
    // The regression guard. `occurrences` counts things seen, not rows about
    // things, so it is the number that moves when the parser stops
    // recognising its own tree: 6.5x headroom over today, and every
    // de-modelling case measured above trips it by 5x or more.
    let occurrences = count(&conn, "SELECT sum(occurrences) FROM ingest_unknown");
    assert!(
        occurrences < 20_000,
        "{occurrences} unknown-construct occurrences, measured 3078 — the parser stopped recognising something large"
    );
}

/// `xknxproject`'s dump is read here as a committed *output file*, never a
/// dependency: it is GPL-2.0-only (RESEARCH §10, ADR-0002), and reading
/// JSON it already produced does not put its licence on this crate's own
/// dependency graph — `cargo deny check` covers `Cargo.lock`, not test
/// fixtures.
fn oracle_dump_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("project_dump.json")
}

/// `project_dump.json`'s `communication_objects` keys are
/// `"<individual_address>/<full_ref_id>"`, and `<full_ref_id>` is exactly
/// this crate's own `com_object_ref.id` (e.g.
/// `M-006A_A-0001-22-26C0-O0079_O-9_R-10015`) — the same string
/// `map.rs:660` sets as `ComObjectInstance.source.ets_id` on the project
/// side (RESEARCH, DATA_MODEL §4). Splits on the *first* `/`, since the
/// individual address itself never contains one.
fn ref_id_from_oracle_key(key: &str) -> Option<&str> {
    key.split_once('/').map(|(_, ref_id)| ref_id)
}

/// Oracle comparison, not a roundtrip: `xknxproject` resolves all the way
/// through the per-device instance layer (`0.xml`), which this crate never
/// sees at all — only `Program`/`ProgramRef` (spec §7). A device that
/// overrides text or datapoint type at instance level is expected to
/// disagree here, on top of RESEARCH §7.1's own known lossiness for
/// unlinked objects and this crate's own refusal to guess an ambiguous
/// multi-alternative `dpt_list` (Task 11). A high match rate over a
/// non-trivial sample is the right shape of assertion; 100% would either be
/// tautological (an empty sample) or wrong (it would mean instance-level
/// overrides do not exist in the reference project, which RESEARCH already
/// measured to be false).
#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn com_object_texts_and_datapoint_types_agree_with_the_oracle_at_a_high_rate() {
    assert!(
        reference_project_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    // The oracle is a second local-only artefact, and it does not live under
    // `OriginalData/`, so the guard above says nothing about it. Checking the
    // corpus and then unwrapping the dump panics on exactly one setup: a
    // worktree with the corpus linked in and the dump left behind at the
    // workspace root it was generated in.
    if !oracle_dump_path().exists() {
        eprintln!(
            "skip: project_dump.json not present at the workspace root \
             (gitignored, local-only; regenerate it with the xknxproject dump)"
        );
        return;
    }
    let (_dir, conn) = ingest_all();
    let raw = std::fs::read_to_string(oracle_dump_path()).unwrap();
    let oracle: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let com_objects = oracle["communication_objects"]
        .as_object()
        .expect("project_dump.json carries a communication_objects object");

    let mut text_compared = 0usize;
    let mut text_matched = 0usize;
    let mut dpt_compared = 0usize;
    let mut dpt_matched = 0usize;

    for (key, co) in com_objects {
        // RESEARCH §7.1: xknxproject is known lossy on objects with no
        // group address link, so only a linked object is a fair
        // comparison point.
        let has_link = co["group_address_links"]
            .as_array()
            .is_some_and(|v| !v.is_empty());
        if !has_link {
            continue;
        }
        let Some(ref_id) = ref_id_from_oracle_key(key) else {
            continue;
        };

        if let Some(oracle_text) = co["text"].as_str() {
            let row: Option<Option<String>> = conn
                .query_row(
                    "SELECT coalesce(cor.text, co.text)
                     FROM com_object_ref cor JOIN com_object co
                       ON co.program_id = cor.program_id AND co.id = cor.com_object_id
                     WHERE cor.id = ?1",
                    [ref_id],
                    |r| r.get(0),
                )
                .ok();
            if let Some(Some(our_text)) = row {
                text_compared += 1;
                if our_text == oracle_text {
                    text_matched += 1;
                }
            }
        }

        // Only a single-alternative oracle DPT is comparable at all: this
        // crate stores an ambiguous `dpt_list` verbatim and refuses to pick
        // one (Task 11's `AmbiguousDpt`), which is not a disagreement to
        // count, just a different, already-documented kind of gap.
        if let Some(dpts) = co["dpts"].as_array() {
            if let [one] = dpts.as_slice() {
                let main = one["main"].as_i64();
                let sub = one["sub"].as_i64();
                let expected = match (main, sub) {
                    (Some(m), Some(s)) => format!("DPST-{m}-{s}"),
                    (Some(m), None) => format!("DPT-{m}"),
                    _ => continue,
                };
                let row: Option<Option<String>> = conn
                    .query_row(
                        "SELECT coalesce(cor.dpt_list, co.dpt_list)
                         FROM com_object_ref cor JOIN com_object co
                           ON co.program_id = cor.program_id AND co.id = cor.com_object_id
                         WHERE cor.id = ?1",
                        [ref_id],
                        |r| r.get(0),
                    )
                    .ok();
                if let Some(Some(our_dpt)) = row {
                    if our_dpt.split_whitespace().count() == 1 {
                        dpt_compared += 1;
                        if our_dpt == expected {
                            dpt_matched += 1;
                        }
                    }
                }
            }
        }
    }

    assert!(
        text_compared > 50,
        "the oracle comparison matched too few com object texts: {text_compared}"
    );
    assert!(
        dpt_compared > 50,
        "the oracle comparison matched too few datapoint types: {dpt_compared}"
    );
    assert!(
        text_matched * 100 >= text_compared * 60,
        "text match rate too low: {text_matched}/{text_compared}"
    );
    assert!(
        dpt_matched * 100 >= dpt_compared * 80,
        "datapoint type match rate too low: {dpt_matched}/{dpt_compared}"
    );
}
