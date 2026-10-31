//! Checks native residuals in decoded archives, carried files and SQLite logical values.

use super::*;

fn deflated(entries: &[(&str, &[u8])]) -> Vec<u8> {
    use std::io::Write;
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        zip.start_file(
            *name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn add_row(conn: &knx_store::Connection, kind: &str, name: &str, bytes: &[u8]) {
    let (source_path, xpath) = if kind == "SelectiveImportArchive" {
        (
            format!(
                "imports/{}/source.knxproj",
                knx_store::project_history::image_hash(bytes)
            ),
            String::new(),
        )
    } else {
        ("witness".into(), "/a".into())
    };
    conn.execute("INSERT INTO opaque_entry(source_path,xpath,kind,name,bytes,sha256) VALUES(?1,?2,?3,?4,?5,?6)",
        (&source_path,&xpath,kind,name,bytes,knx_store::project_history::image_hash(bytes))).unwrap();
}

#[test]
fn secret_free_review_r1_native_decodes_nested_residuals() {
    native_archive(false);
}
#[test]
fn secret_free_review_r1_native_enforces_late_discovered_secret() {
    native_archive(true);
}

fn native_archive(late: bool) {
    let dir = tempfile::tempdir().unwrap();
    let path = native_project(dir.path(), false);
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let nested = deflated(&[
        ("secret.xml", br#"<a Password="NestedPasswordWitness"/>"#),
        ("notes.txt", b"NestedPasswordWitness"),
        ("safe.txt", b"keep this"),
    ]);
    assert!(
        !contains(&nested, "NestedPasswordWitness"),
        "compressed fixture must hide physical literal"
    );
    if !late {
        add_row(&conn, "SelectiveImportArchive", "", &nested);
    }
    let early = deflated(&[
        ("notes.txt", b"LatePasswordWitness"),
        ("safe.txt", b"keep this"),
    ]);
    if late {
        add_row(&conn, "SelectiveImportArchive", "", &early);
        add_row(
            &conn,
            "RetainedAttribute",
            "Password",
            b"LatePasswordWitness",
        );
    }
    drop(conn);
    let before = std::fs::read(&path).unwrap();
    let copy = native_project_without_secrets(&path).unwrap();
    let out = reopened(&copy.bytes, dir.path());
    let rows = knx_store::load_opaque(&out.conn).unwrap();
    for row in rows
        .iter()
        .filter(|r| xml::sniff(&r.bytes) == xml::Content::Zip)
    {
        let entries = members(&row.bytes);
        assert_eq!(entries["safe.txt"], b"keep this");
        assert!(!entries.values().any(|v| contains(v,"NestedPasswordWitness") || contains(v,"LatePasswordWitness")), "decoded native archive leaked known secret");
        assert!(!entries.contains_key("notes.txt"));
    }
    assert!(rows.iter().any(|r| r.kind == "SelectiveImportArchive"));
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn secret_free_review_r1_carried_zlib_residual_is_checked_decoded() {
    let dir = tempfile::tempdir().unwrap();
    let path = native_project(dir.path(), false);
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    add_row(
        &conn,
        "RetainedAttribute",
        "Password",
        b"CarriedPasswordWitness",
    );
    let bytes = br#"<KNX><Hardware Name="CarriedPasswordWitness"/></KNX>"#;
    knx_store::store_manufacturer_file(&conn, "M-TEST/witness.xml", bytes).unwrap();
    let hash = knx_store::manufacturer_file_hash(bytes);
    let mut refs = knx_store::load_manufacturer_refs(&conn).unwrap();
    refs.push(knx_store::ManufacturerRef {
        source_path: "M-TEST/witness.xml".into(),
        sha256: hash.clone(),
        len: bytes.len() as i64,
        kind: "Hardware".into(),
    });
    knx_store::save_project_with_passthrough(
        &conn,
        &knx_store::load_project(&conn).unwrap(),
        &knx_store::load_opaque(&conn).unwrap(),
        &refs,
    )
    .unwrap();
    // Storage must genuinely hide the literal; the normal reader restores it.
    let compressed: Vec<u8> = conn
        .query_row(
            "SELECT bytes FROM manufacturer_file WHERE sha256=?1",
            [&hash],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!contains(&compressed, "CarriedPasswordWitness"));
    assert_eq!(
        knx_store::load_manufacturer_file(&conn, &hash)
            .unwrap()
            .unwrap()
            .bytes,
        bytes
    );
    drop(conn);
    let copy = native_project_without_secrets(&path).unwrap();
    let out = reopened(&copy.bytes, dir.path());
    assert!(
        knx_store::load_manufacturer_file(&out.conn, &hash)
            .unwrap()
            .is_none(),
        "decoded carried file leaked known secret"
    );
    assert_eq!(
        knx_store::load_manufacturer_refs(&out.conn).unwrap(),
        refs,
        "withholding does not forge a replacement product identity"
    );
    assert!(copy
        .withheld
        .iter()
        .any(|reason| reason.contains("carried product file")));
    let safe = knx_store::manufacturer_file_hash(PLAIN_PRODUCT);
    assert_eq!(
        knx_store::load_manufacturer_file(&out.conn, &safe)
            .unwrap()
            .unwrap()
            .bytes,
        PLAIN_PRODUCT
    );
}

#[test]
fn secret_free_review_r1_model_note_crosses_final_sqlite_overflow_boundary() {
    use knx_core::note::{ProjectNote, ProjectNoteTarget};
    let dir = tempfile::tempdir().unwrap();
    let path = native_project(dir.path(), false);
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let mut project = knx_store::load_project(&conn).unwrap();
    let id = project.ids.next_project_note_id().unwrap();
    project.notes.push(
        ProjectNote::new(id, ProjectNoteTarget::Project, "Note".into(), "body".into()).unwrap(),
    );
    let rows = knx_store::load_opaque(&conn).unwrap();
    knx_store::save_project_with_passthrough(&conn, &project, &rows, &[]).unwrap();
    add_row(
        &conn,
        "RetainedAttribute",
        "Password",
        b"OverflowPasswordWitness",
    );
    let page_size = conn
        .query_row("PRAGMA page_size", [], |r| r.get::<_, u32>(0))
        .unwrap() as usize;
    // Calibrate the compact final layout with exactly the same redacted key row.
    conn.execute(
        "UPDATE opaque_entry SET bytes=x'',sha256=?1 WHERE name='Password'",
        [knx_store::project_history::image_hash(b"")],
    )
    .unwrap();
    conn.execute_batch("DELETE FROM project_history_stack;DELETE FROM project_history_version;DELETE FROM project_history_state;DELETE FROM project_history_context;").unwrap();
    let mut aligned = None;
    for pad in 0..page_size {
        let body = format!(
            "{}OverflowPasswordWitness{}",
            "a".repeat(5000 + pad),
            "b".repeat(10000 - pad)
        );
        conn.execute("UPDATE project_note SET body=?1", [&body])
            .unwrap();
        let image = dir.path().join("calibrated.knxdb");
        conn.execute("VACUUM INTO ?1", [image.to_str().unwrap()])
            .unwrap();
        let raw = std::fs::read(&image).unwrap();
        let compact = knx_store::open_existing_read_only(&image).unwrap();
        let logical = knx_store::load_project(&compact.conn).unwrap();
        assert_eq!(logical.notes[0].body(), body);
        drop(compact);
        std::fs::remove_file(image).unwrap();
        if !contains(&raw, "OverflowPasswordWitness") {
            // Prove an actual overflow-page link interrupts the live TEXT value,
            // not merely that a physical string search failed for another reason.
            let secret = b"OverflowPasswordWitness";
            let linked_split = (1..secret.len()).any(|split| {
                raw.windows(split).enumerate().any(|(offset, part)| {
                    if part != &secret[..split] || (offset + split) % page_size != 0 {
                        return false;
                    }
                    let page_start = offset / page_size * page_size;
                    let next =
                        u32::from_be_bytes(raw[page_start..page_start + 4].try_into().unwrap())
                            as usize;
                    if next == 0 {
                        return false;
                    }
                    let suffix = (next - 1) * page_size + 4;
                    raw.get(suffix..suffix + secret.len() - split) == Some(&secret[split..])
                })
            });
            assert!(
                linked_split,
                "fixture did not identify a real overflow-page link"
            );
            aligned = Some(body);
            break;
        }
    }
    assert!(
        aligned.is_some(),
        "fixture did not cross actual compact SQLite payload/overflow split"
    );
    conn.execute(
        "UPDATE opaque_entry SET bytes=?1,sha256=?2 WHERE name='Password'",
        (
            b"OverflowPasswordWitness".as_slice(),
            knx_store::project_history::image_hash(b"OverflowPasswordWitness"),
        ),
    )
    .unwrap();
    drop(conn);
    let before = std::fs::read(&path).unwrap();
    let result = native_project_without_secrets(&path);
    if let Ok(copy) = &result {
        let out = reopened(&copy.bytes, dir.path());
        let model = knx_store::load_project(&out.conn).unwrap();
        assert!(
            !model.notes[0].body().contains("OverflowPasswordWitness"),
            "logical note secret survived final overflow boundary"
        );
    }
    assert!(
        matches!(result, Err(SecretFreeCopyError::ResidualSecret { .. })),
        "model semantics must not be silently changed"
    );
    assert_eq!(std::fs::read(path).unwrap(), before);
}
