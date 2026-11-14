//! Exercises source-bound caption admission with independently authored product declarations.

use super::*;

// Pre-WP06-S1 call shapes: the project/20 walk every existing case was written for.
fn read_source(
    bytes: &[u8],
    program: &str,
    manufacturer: &str,
) -> Result<Vec<Candidate>, CaptionIssue> {
    super::read_source(bytes, program, manufacturer, PROJECT_20)
}
fn hardware_program_link(
    bytes: &[u8],
    hp: &str,
    hardware: &str,
    program: &str,
    manufacturer: &str,
) -> Result<(), CaptionIssue> {
    super::hardware_program_link(bytes, hp, hardware, program, manufacturer, PROJECT_20)
}

const AP: &str = "M-0001_A-0001-01-0000";
const HP: &str = "M-0001_H-1_HP-1";
const PRODUCT: &str = "M-0001_P-1";
const HARDWARE: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-1" Name="Fictional" SerialNumber="S" VersionNumber="1"><Products><Product Id="M-0001_P-1"/></Products><Hardware2Programs><Hardware2Program Id="M-0001_H-1_HP-1"><ApplicationProgramRef RefId="M-0001_A-0001-01-0000"/></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

fn source(dynamic: &str) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="{AP}" Name="Fictional" ApplicationVersion="1"><ModuleDefs><ModuleDef Id="{AP}_MD-1"><Dynamic><ParameterBlock Id="{AP}_PB-8" Text="Earlier module shadow" ShowInComObjectTree="true"/></Dynamic></ModuleDef></ModuleDefs><Dynamic>{dynamic}</Dynamic></ApplicationProgram><ApplicationProgram Id="M-0001_A-0002-01-0000"><Dynamic><ParameterBlock Id="M-0001_A-0002-01-0000_PB-8" Text="Other program" ShowInComObjectTree="true"/></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}
fn block(id: &str, text: &str) -> String {
    format!(r#"<ParameterBlock Id="{AP}_{id}" Text="{text}" ShowInComObjectTree="true"/>"#)
}
fn setup(dynamic: &str) -> (tempfile::TempDir, Connection, String) {
    let dir = tempfile::tempdir().unwrap();
    let conn = crate::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    crate::ingest_file(&conn, "M-0001/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    let xml = source(dynamic);
    crate::ingest_file(&conn, "M-0001/Application.xml", xml.as_bytes()).unwrap();
    (dir, conn, xml)
}
fn read(conn: &Connection) -> FolderCaptions {
    let before = conn.total_changes();
    let evidence = folder_captions(conn, PRODUCT, HP, 20).unwrap();
    assert_eq!(
        conn.total_changes(),
        before,
        "the reader must not repair evidence"
    );
    match evidence {
        CaptionEvidence::Declared(v) => v,
        CaptionEvidence::Unavailable(_) => BTreeMap::new(),
    }
}
fn key(nodes: &[(&str, &str)]) -> Vec<FolderPathNode> {
    nodes
        .iter()
        .map(|(kind, id)| FolderPathNode {
            kind: (*kind).into(),
            ref_id: (*id).into(),
        })
        .collect()
}

#[test]
fn exact_own_direct_dynamic_and_full_shown_ancestry_not_module_shadow_or_other_program() {
    let dynamic = format!(
        r#"<Channel Id="{AP}_CH-1"><ParameterBlock Id="{AP}_PB-1" Text="Hidden container"><choose ParamRefId="unresolved"><when test="1">{}</when></choose></ParameterBlock></Channel>{}<Repeat>{}</Repeat>"#,
        block("PB-8", "Equal &amp; &lt;b&gt;literal&lt;/b&gt;"),
        block("PB-9", "Equal &amp; &lt;b&gt;literal&lt;/b&gt;"),
        block("PB-10", "Repeated")
    );
    let (_dir, conn, xml) = setup(&dynamic);
    let captions = read(&conn);
    assert_eq!(captions.len(), 2);
    let a = &captions[&key(&[("Channel", "CH-1"), ("Folder", "PB-8")])];
    let b = &captions[&key(&[("Folder", "PB-9")])];
    assert_eq!(a.text, "Equal & <b>literal</b>");
    assert_eq!(a.text, b.text);
    assert_eq!(a.application_program_id, AP);
    assert_eq!(a.parameter_block_id, format!("{AP}_PB-8"));
    assert_eq!(a.source_sha256, crate::sha256_hex(xml.as_bytes()));
    assert_ne!(a.parameter_block_id, b.parameter_block_id);
    assert!(!captions.contains_key(&key(&[("Folder", "PB-8")])));
    assert_eq!(
        folder_captions(&conn, PRODUCT, HP, 23).unwrap(),
        CaptionEvidence::Declared(captions)
    );
    assert_eq!(
        folder_captions(&conn, PRODUCT, HP, 21).unwrap(),
        CaptionEvidence::Unavailable(CaptionIssue::UnsupportedSchema)
    );
}

#[test]
fn product_authority_requires_canonical_retained_owner_not_parser_carry_or_normalized_edge() {
    let xml = source(&block("PB-8", "Literal"));
    for hardware in [
        HARDWARE.replace("<Product Id=", "<Product xmlns=\"urn:foreign\" Id="),
        HARDWARE.replace("<Products>", "<Products xmlns=\"urn:foreign\">"),
        HARDWARE
            .replace("<Products>", "<Wrapper><Products>")
            .replace("</Products>", "</Products></Wrapper>"),
        HARDWARE
            .replace("<Products><Product Id=\"M-0001_P-1\"/></Products>", "")
            .replace(
                "</Hardware></Hardware>",
                "</Hardware><Products><Product Id=\"M-0001_P-1\"/></Products></Hardware>",
            ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let conn = crate::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        crate::ingest_file(&conn, "M-0001/Hardware.xml", hardware.as_bytes()).unwrap();
        crate::ingest_file(&conn, "M-0001/Application.xml", xml.as_bytes()).unwrap();
        let parent: String = conn
            .query_row(
                "SELECT hardware_id FROM product WHERE id=?1",
                [PRODUCT],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            parent, "M-0001_H-1",
            "calibrate the older ingester's carried/local-name owner"
        );
        let before = conn.total_changes();
        assert_eq!(
            folder_captions(&conn, PRODUCT, HP, 20).unwrap(),
            CaptionEvidence::Unavailable(CaptionIssue::SourceIdentity)
        );
        assert_eq!(conn.total_changes(), before);
    }
    let (_dir, conn, _) = setup(&block("PB-8", "Literal"));
    let second = HARDWARE
        .replace("M-0001_H-1", "M-0001_H-2")
        .replace(PRODUCT, "M-0001_P-2")
        .replace(AP, "M-0001_A-0002-01-0000");
    crate::ingest_file(&conn, "M-0001/SecondHardware.xml", second.as_bytes()).unwrap();
    assert!(
        matches!(folder_captions(&conn, "M-0001_P-2", "M-0001_H-2_HP-1", 20).unwrap(), CaptionEvidence::Declared(ref v) if !v.is_empty())
    );
    conn.execute(
        "UPDATE product SET hardware_id='M-0001_H-2' WHERE id=?1",
        [PRODUCT],
    )
    .unwrap();
    let before = conn.total_changes();
    assert_eq!(
        folder_captions(&conn, PRODUCT, "M-0001_H-2_HP-1", 20).unwrap(),
        CaptionEvidence::Unavailable(CaptionIssue::SourceIdentity)
    );
    assert_eq!(conn.total_changes(), before);
}

#[test]
fn distinct_product_and_h2p_winning_members_keep_their_own_authority() {
    let (_dir, conn, _) = setup(&block("PB-8", "Literal"));
    let expected = read(&conn);
    let second = HARDWARE.replace("</KNX>", "<!-- distinct retained source --></KNX>");
    crate::ingest_file(&conn, "M-0001/EquivalentHardware.xml", second.as_bytes()).unwrap();
    let sha = crate::sha256_hex(second.as_bytes());
    conn.execute(
        "UPDATE hardware2program SET source_sha256=?1 WHERE id=?2",
        [&sha, HP],
    )
    .unwrap();
    assert_eq!(read(&conn), expected);
    assert!(!expected.is_empty());
}

#[test]
fn strict_flags_attributes_text_alias_access_and_unsupported_ancestry_refuse() {
    let base = block("PB-8", "Literal");
    let cases = [
        base.replace("ShowInComObjectTree=\"true\"", ""),
        base.replace("=\"true\"", "=\"false\""),
        base.replace("=\"true\"", "=\"TRUE\""),
        base.replace("=\"true\"", "=\"yes\""),
        base.replace(
            "ShowInComObjectTree",
            "xmlns:q=\"urn:foreign\" q:ShowInComObjectTree",
        ),
        base.replace("Text=", "xmlns:q=\"urn:foreign\" q:Text="),
        base.replace("Id=", "xmlns:q=\"urn:foreign\" q:Id="),
        base.replace("Literal", ""),
        base.replace("Literal", "   "),
        base.replace("Literal", "{{0}}"),
        base.replace("Literal", "a }}"),
        base.replace("Literal", &"é".repeat(2049)),
        base.replace("Text=", "TextParameterRefId=\"x\" Text="),
        base.replace(
            "Text=",
            "xmlns:q=\"urn:foreign\" q:TextParameterRefId=\"x\" Text=",
        ),
        base.replace("Text=", "RefId=\"x\" Text="),
        base.replace("Text=", "Access=\"None\" Text="),
        base.replace("Text=", "Access=\"Unknown\" Text="),
        base.replace(AP, "wrong-prefix"),
        base.replace("<ParameterBlock ", "<ParameterBlock xmlns=\"urn:foreign\" "),
        format!("<Repeat>{base}</Repeat>"),
        format!("<Module>{base}</Module>"),
        format!("<Unsupported>{base}</Unsupported>"),
        format!("<Channel Id=\"{AP}_CH-1\" Access=\"None\">{base}</Channel>"),
        format!("<Channel Id=\"{AP}_CH-1\" RefId=\"alias\">{base}</Channel>"),
    ];
    for dynamic in cases {
        let (_dir, conn, _) = setup(&dynamic);
        assert!(read(&conn).is_empty(), "{dynamic}");
    }
    for access in ["Read", "ReadWrite"] {
        let (_dir, conn, _) = setup(&base.replace("Text=", &format!("Access=\"{access}\" Text=")));
        assert_eq!(read(&conn).len(), 1);
    }
    let (_dir, conn, _) = setup(&base.replace("=\"true\"", "=\"1\""));
    assert_eq!(read(&conn).len(), 1);
}

#[test]
fn duplicate_raw_ids_paths_and_shown_ancestors_withhold_descendants() {
    let b = block("PB-8", "Literal");
    for dynamic in [
        format!("{b}{b}"),
        format!(r#"<Channel Id="{AP}_CH-1">{b}</Channel><Channel Id="{AP}_CH-1"/>"#),
        format!(
            r#"<ParameterBlock Id="{AP}_PB-1" ShowInComObjectTree="true">{b}</ParameterBlock><ParameterBlock Id="{AP}_PB-1"/>"#
        ),
    ] {
        let (_dir, conn, _) = setup(&dynamic);
        assert!(read(&conn).is_empty());
    }
}

#[test]
fn root_renames_refuse_but_module_definition_rename_does_not_grant_authority() {
    for tag in ["Rename", "ParameterBlockRename"] {
        let (_dir, conn, _) = setup(&format!("{}<{tag}/>", block("PB-8", "Literal")));
        assert_eq!(
            folder_captions(&conn, PRODUCT, HP, 20).unwrap(),
            CaptionEvidence::Unavailable(CaptionIssue::UnsupportedXml)
        );
    }
}

#[test]
fn corrupt_source_ownership_normalized_rows_and_missing_resources_do_not_repair_or_caption() {
    for sql in [
        "UPDATE product SET hardware_id='other'",
        "UPDATE hardware2program SET application_program_ref='missing'",
        "UPDATE hardware2program SET application_program_ref='M-0001_A-0002-01-0000'",
        "UPDATE source_file SET bytes=CAST('corrupt' AS BLOB) WHERE source_path LIKE '%Hardware.xml'",
        "UPDATE source_file SET bytes=CAST('corrupt' AS BLOB) WHERE source_path LIKE '%Application.xml'",
        "UPDATE source_file SET len=len+1 WHERE source_path LIKE '%Application.xml'",
        "DELETE FROM source_file WHERE source_path LIKE '%Application.xml'",
        "DELETE FROM source_parse_evidence",
        "DELETE FROM source_identity_scan",
        "UPDATE source_identity_scan SET scanner=scanner+1",
        "UPDATE source_identity_scan SET status='unavailable',reason='synthetic'",
        "UPDATE source_identity SET digest=replace(hex(zeroblob(32)),'0','z')",
        "DELETE FROM dynamic_node WHERE module_def_id='' AND kind='ParameterBlock'",
        "UPDATE dynamic_node SET text='wrong' WHERE module_def_id='' AND kind='ParameterBlock'",
        "UPDATE dynamic_node SET text_parameter_ref_id='x' WHERE module_def_id='' AND kind='ParameterBlock'",
        "UPDATE dynamic_node SET ref_id='alias' WHERE module_def_id='' AND kind='ParameterBlock'",
        "UPDATE dynamic_node SET module_def_id='shadow' WHERE module_def_id='' AND kind='ParameterBlock'",
    ] {
        let (_dir,conn,_) = setup(&block("PB-8","Literal"));
        assert_eq!(read(&conn).len(),1,"calibrate {sql}");
        conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        conn.execute_batch(sql).unwrap();
        assert!(read(&conn).is_empty(),"{sql}");
    }
}

#[test]
fn divergent_or_unmeasured_alternative_program_is_not_the_current_source() {
    let (_dir, conn, xml) = setup(&block("PB-8", "Literal"));
    let other = xml.replace("Literal", "Divergent");
    crate::ingest_file(&conn, "M-0001/Other.xml", other.as_bytes()).unwrap();
    assert!(read(&conn).is_empty());
    conn.execute_batch("UPDATE source_identity_scan SET status='unavailable',reason='synthetic' WHERE source_sha256 IN (SELECT sha256 FROM source_file WHERE source_path LIKE '%Other.xml')").unwrap();
    assert!(read(&conn).is_empty());
}

#[test]
fn raw_parser_guards_canonical_owners_document_integrity_and_resource_limits() {
    let xml = source(&block("PB-8", "Literal"));
    for bad in [
        xml.replace("project/20", "project/23"),
        xml.replace("RefId=\"M-0001\"", "RefId=\"M-0002\""),
        format!("<!DOCTYPE KNX []>{xml}"),
        format!("{xml}{xml}"),
        xml.replace("</KNX>", ""),
        xml.replace("Literal", "&unknown;"),
        xml.replace("<Dynamic>", "<Dynamic xmlns=\"urn:foreign\">"),
        xml.replace("<ApplicationPrograms>", "<Wrapper><ApplicationPrograms>")
            .replace("</ApplicationPrograms>", "</ApplicationPrograms></Wrapper>"),
        source(&format!(
            "{}{}{}",
            "<choose>".repeat(MAX_DEPTH),
            block("PB-8", "Literal"),
            "</choose>".repeat(MAX_DEPTH)
        )),
        source(&"<ParameterBlock/>".repeat(MAX_EVENTS)),
        source(
            &(0..=MAX_IDENTITIES)
                .map(|i| block(&format!("PB-{i}"), "Literal"))
                .collect::<String>(),
        ),
    ] {
        assert!(read_source(bad.as_bytes(), AP, "M-0001").is_err());
    }
    let (_dir, conn, _) = setup(&block("PB-8", "Literal"));
    conn.execute("UPDATE source_file SET bytes=zeroblob(?1),len=?1 WHERE source_path LIKE '%Application.xml'",[MAX_SOURCE_BYTES as i64+1]).unwrap();
    assert_eq!(
        folder_captions(&conn, PRODUCT, HP, 20).unwrap(),
        CaptionEvidence::Unavailable(CaptionIssue::ResourceLimit)
    );
}

#[test]
fn duplicate_normalized_unscoped_rows_refuse_without_consuming_module_shadow() {
    let (_dir, conn, _) = setup(&block("PB-8", "Literal"));
    assert_eq!(read(&conn).len(), 1);
    let columns = conn
        .prepare("PRAGMA table_info(dynamic_node)")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let select = columns
        .iter()
        .map(|c| {
            if c == "node_id" {
                "node_id+1000000"
            } else if c == "position" {
                "position+1000000"
            } else {
                c.as_str()
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    conn.execute_batch(&format!("INSERT INTO dynamic_node SELECT {select} FROM dynamic_node WHERE module_def_id='' AND kind='ParameterBlock'")).unwrap();
    assert!(read(&conn).is_empty());
}

#[test]
fn retained_hardware_relationship_requires_exact_canonical_unqualified_owner_and_program() {
    assert!(hardware_program_link(HARDWARE.as_bytes(), HP, "M-0001_H-1", AP, "M-0001").is_ok());
    for xml in [
        HARDWARE.replace(AP, "M-0001_A-0002-01-0000"),
        HARDWARE.replace(
            "<ApplicationProgramRef ",
            "<ApplicationProgramRef xmlns=\"urn:foreign\" ",
        ),
        HARDWARE.replace(
            "<ApplicationProgramRef RefId=",
            "<ApplicationProgramRef xmlns:q=\"urn:foreign\" q:RefId=",
        ),
        HARDWARE.replace(
            "</Hardware2Program>",
            &format!("<ApplicationProgramRef RefId=\"{AP}\"/></Hardware2Program>"),
        ),
        HARDWARE.replace("RefId=\"M-0001\"", "RefId=\"M-0002\""),
        HARDWARE.replace("Id=\"M-0001_H-1\"", "Id=\"wrong\""),
        HARDWARE
            .replace("<Hardware2Programs>", "<Wrapper><Hardware2Programs>")
            .replace("</Hardware2Programs>", "</Hardware2Programs></Wrapper>"),
        format!("<!DOCTYPE KNX []>{HARDWARE}"),
    ] {
        assert!(hardware_program_link(xml.as_bytes(), HP, "M-0001_H-1", AP, "M-0001").is_err());
    }
}

#[test]
fn actual_database_failure_is_not_optional_absence() {
    let (_dir, conn, _) = setup(&block("PB-8", "Literal"));
    conn.execute_batch("DROP TABLE dynamic_node").unwrap();
    assert!(matches!(
        folder_captions(&conn, PRODUCT, HP, 20),
        Err(ProductDbError::Sqlite(_))
    ));
}

// Snapshot every row used by admission, plus deliberately unused diagnostics.
fn catalogue_snapshot(conn: &Connection) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
    [
        "product",
        "hardware",
        "hardware2program",
        "application_program",
        "source_file",
        "source_identity",
        "source_identity_scan",
        "source_parse_evidence",
        "package",
        "package_member",
        "package_conflict",
        "ingest_unknown",
        "dynamic_node",
    ]
    .into_iter()
    .map(|table| {
        let mut stmt = conn
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .unwrap();
        let columns = stmt.column_count();
        stmt.query_map([], |row| (0..columns).map(|i| row.get(i)).collect())
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    })
    .collect()
}

fn unchanged_read(conn: &Connection) -> CaptionEvidence {
    let before = catalogue_snapshot(conn);
    let changes = conn.total_changes();
    let result = folder_captions(conn, PRODUCT, HP, 20).unwrap();
    assert_eq!(conn.total_changes(), changes);
    assert_eq!(catalogue_snapshot(conn), before);
    result
}

fn alternative(conn: &Connection, xml: &str, index: usize) -> String {
    // A distinct real retained blob, with the same measured AP declaration.
    let bytes = format!("<!-- independent source {index} -->{xml}");
    crate::ingest_file(
        conn,
        &format!("M-0001/Alternative-{index}.xml"),
        bytes.as_bytes(),
    )
    .unwrap();
    crate::sha256_hex(bytes.as_bytes())
}

#[test]
fn identity_admission_all_equivalent_candidates_at_bound_and_bad_tail_never_approves() {
    assert_eq!(MAX_IDENTITY_ADMISSION_ROWS, 1024);
    let (_dir, conn, xml) = setup(&block("PB-8", "Literal"));
    let expected = unchanged_read(&conn);
    for index in 1..MAX_IDENTITY_ADMISSION_ROWS {
        alternative(&conn, &xml, index);
    }
    let count: i64 = conn.query_row("SELECT count(*) FROM source_identity WHERE table_name='application_program' AND logical_id=?1", [AP], |r| r.get(0)).unwrap();
    assert_eq!(count, 1024);
    assert_eq!(
        unchanged_read(&conn),
        expected,
        "all equivalent below the inclusive cap"
    );
    let tail = alternative(&conn, &xml, MAX_IDENTITY_ADMISSION_ROWS);
    assert_eq!(
        unchanged_read(&conn),
        CaptionEvidence::Unavailable(CaptionIssue::ResourceLimit),
        "even all-equivalent overflow refuses"
    );
    conn.execute("UPDATE source_identity SET digest=?1 WHERE source_sha256=?2 AND table_name='application_program' AND logical_id=?3", rusqlite::params!["1".repeat(64), tail, AP]).unwrap();
    assert_eq!(
        unchanged_read(&conn),
        CaptionEvidence::Unavailable(CaptionIssue::ResourceLimit),
        "divergent tail beyond the admitted prefix"
    );
    conn.execute(
        "UPDATE source_identity_scan SET status='unavailable',reason='tail' WHERE source_sha256=?1",
        [&tail],
    )
    .unwrap();
    assert_eq!(
        unchanged_read(&conn),
        CaptionEvidence::Unavailable(CaptionIssue::ResourceLimit)
    );
    eprintln!("IDENTITY candidates=1024 admitted; candidates=1025 divergent/unmeasured tail refused ResourceLimit");
}

// Native authorizer proves unused payloads are not even queried. Per connection,
// no shared atomics; restore the callback before the borrowed connection drops.
struct NoDiagnosticPayloads<'a>(&'a Connection);
impl<'a> NoDiagnosticPayloads<'a> {
    fn install(conn: &'a Connection) -> Self {
        unsafe extern "C" fn deny(
            _: *mut std::ffi::c_void,
            action: i32,
            table: *const std::ffi::c_char,
            column: *const std::ffi::c_char,
            _: *const std::ffi::c_char,
            _: *const std::ffi::c_char,
        ) -> i32 {
            if action != rusqlite::ffi::SQLITE_READ {
                return rusqlite::ffi::SQLITE_OK;
            }
            // SAFETY: SQLite's read-authorizer arguments are live C strings.
            let table = unsafe { std::ffi::CStr::from_ptr(table) }.to_bytes();
            let column = unsafe { std::ffi::CStr::from_ptr(column) }.to_bytes();
            if matches!(table, b"package" | b"package_member")
                || (table == b"source_file" && column == b"source_path")
            {
                rusqlite::ffi::SQLITE_DENY
            } else {
                rusqlite::ffi::SQLITE_OK
            }
        }
        // SAFETY: callback has no context and remains installed only for this guard.
        assert_eq!(
            unsafe {
                rusqlite::ffi::sqlite3_set_authorizer(
                    conn.handle(),
                    Some(deny),
                    std::ptr::null_mut(),
                )
            },
            rusqlite::ffi::SQLITE_OK
        );
        Self(conn)
    }
}
impl Drop for NoDiagnosticPayloads<'_> {
    fn drop(&mut self) {
        // SAFETY: connection outlives the guard; no statement is executing here.
        unsafe {
            rusqlite::ffi::sqlite3_set_authorizer(self.0.handle(), None, std::ptr::null_mut());
        }
    }
}

#[test]
fn identity_admission_large_package_memberships_do_not_read_diagnostic_payloads() {
    let (_dir, conn, xml) = setup(&block("PB-8", "Literal"));
    let expected = unchanged_read(&conn);
    let sha = crate::sha256_hex(xml.as_bytes());
    conn.execute_batch("BEGIN").unwrap();
    for index in 0..4096 {
        let package = format!("{index:064x}");
        conn.execute("INSERT INTO package(sha256,source_name,scheme,size,bytes,unknown_count) VALUES (?1,'fictional',20,0,x'',0)", [&package]).unwrap();
        conn.execute("INSERT INTO package_member(package_sha256,ordinal,path,role,source_sha256,size) VALUES (?1,0,?2,'application',?3,?4)", rusqlite::params![package, "unused".repeat(256), sha, xml.len() as i64]).unwrap();
    }
    conn.execute_batch("COMMIT").unwrap();
    conn.execute(
        "UPDATE source_file SET source_path=?1",
        ["unused diagnostic path".repeat(4096)],
    )
    .unwrap();
    let before = catalogue_snapshot(&conn);
    let changes = conn.total_changes();
    {
        let _guard = NoDiagnosticPayloads::install(&conn);
        assert!(
            crate::identity_candidates(&conn, IdentityKind::ApplicationProgram, AP).is_err(),
            "calibrate forbidden diagnostic API"
        );
        assert_eq!(folder_captions(&conn, PRODUCT, HP, 20).unwrap(), expected);
    }
    assert_eq!(conn.total_changes(), changes);
    assert_eq!(catalogue_snapshot(&conn), before);
    eprintln!("IDENTITY package_memberships=4096 admitted with native authorizer denying diagnostic payloads");
}

#[test]
fn identity_admission_caps_conflicts_before_holders_and_rejects_unmeasured_stale_malformed() {
    for table in ["package_conflict", "ingest_unknown"] {
        let (_dir, conn, xml) = setup(&block("PB-8", "Literal"));
        let sha = crate::sha256_hex(xml.as_bytes());
        let expected = unchanged_read(&conn);
        conn.execute("INSERT INTO package(sha256,source_name,scheme,size,bytes,unknown_count) VALUES (?1,'fictional',20,0,x'',0)", [&sha]).unwrap();
        for ordinal in 0..=MAX_IDENTITY_ADMISSION_ROWS {
            if table == "package_conflict" {
                conn.execute("INSERT INTO package_conflict(package_sha256,ordinal,table_name,logical_id,kept_sha256,other_sha256,occurrence) VALUES (?1,?2,'application_program',?3,?1,?1,1)", rusqlite::params![sha, ordinal as i64, AP]).unwrap();
            } else {
                conn.execute("INSERT INTO ingest_unknown(source_sha256,xpath,kind,name,occurrences,sample) VALUES (?1,'application_program','IdConflict',?2,?3,?1)", rusqlite::params![sha, AP, (ordinal+1) as i64]).unwrap();
            }
            if ordinal + 1 == MAX_IDENTITY_ADMISSION_ROWS {
                assert_eq!(unchanged_read(&conn), expected);
            }
        }
        assert_eq!(
            unchanged_read(&conn),
            CaptionEvidence::Unavailable(CaptionIssue::ResourceLimit),
            "{table}"
        );
    }
    for corruption in [
        "DELETE FROM source_identity_scan WHERE source_sha256=?1",
        "UPDATE source_identity_scan SET scanner=scanner+1 WHERE source_sha256=?1",
        "UPDATE source_identity_scan SET status='unavailable',reason=printf('%100000s','x') WHERE source_sha256=?1",
        "DELETE FROM source_parse_evidence WHERE sha256=?1",
        "UPDATE source_identity SET digest=replace(hex(zeroblob(32)),'0','z') WHERE source_sha256=?1",
        "UPDATE source_identity SET occurrence=4294967296 WHERE source_sha256=?1",
    ] {
        let (_dir, conn, xml) = setup(&block("PB-8", "Literal"));
        let sha = alternative(&conn, &xml, 1);
        assert!(matches!(unchanged_read(&conn), CaptionEvidence::Declared(v) if v.len()==1));
        conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        conn.execute(corruption, [&sha]).unwrap();
        assert_eq!(unchanged_read(&conn), CaptionEvidence::Unavailable(CaptionIssue::SourceIdentity), "{corruption}");
    }
}

#[test]
fn identity_admission_unmeasured_conflict_holder_without_candidate_cannot_approve() {
    for corrupt in [
        "UPDATE source_identity_scan SET status='unavailable',reason='synthetic unmeasured holder' WHERE source_sha256=?1",
        "UPDATE source_identity_scan SET scanner=scanner+1 WHERE source_sha256=?1",
        "DELETE FROM source_identity_scan WHERE source_sha256=?1",
    ] {
        let (_dir, conn, xml) = setup(&block("PB-8", "Literal"));
        let holder = alternative(&conn, &xml, 1);
        assert!(matches!(unchanged_read(&conn), CaptionEvidence::Declared(v) if v.len()==1));
        conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        conn.execute("DELETE FROM source_identity WHERE source_sha256=?1", [&holder]).unwrap();
        conn.execute(corrupt, [&holder]).unwrap();
        assert_eq!(unchanged_read(&conn), CaptionEvidence::Unavailable(CaptionIssue::SourceIdentity));
    }
}

#[test]
fn identity_admission_malformed_holder_hash_and_oversized_owner_metadata_refuse() {
    let (_dir, conn, xml) = setup(&block("PB-8", "Literal"));
    let sha = crate::sha256_hex(xml.as_bytes());
    conn.execute("INSERT INTO ingest_unknown(source_sha256,xpath,kind,name,occurrences,sample) VALUES (?1,'application_program','IdConflict',?2,1,?3)", rusqlite::params![sha, AP, "z".repeat(100_000)]).unwrap();
    assert_eq!(
        unchanged_read(&conn),
        CaptionEvidence::Unavailable(CaptionIssue::SourceIdentity)
    );
    conn.execute("DELETE FROM ingest_unknown WHERE kind='IdConflict'", [])
        .unwrap();
    conn.execute(
        "UPDATE application_program SET manufacturer_id=?1 WHERE id=?2",
        rusqlite::params!["x".repeat(MAX_ATTRIBUTE_BYTES + 1), AP],
    )
    .unwrap();
    assert_eq!(
        unchanged_read(&conn),
        CaptionEvidence::Unavailable(CaptionIssue::ResourceLimit)
    );
}

// Count actual SQLite instructions, not time or the number of returned rows.
struct VmSteps<'a> {
    conn: &'a Connection,
    steps: Box<usize>,
}
impl<'a> VmSteps<'a> {
    fn install(conn: &'a Connection) -> Self {
        unsafe extern "C" fn tick(context: *mut std::ffi::c_void) -> i32 {
            // SAFETY: boxed counter is pinned for the entire registered lifetime;
            // SQLite invokes the callback synchronously on this connection.
            unsafe {
                *context.cast::<usize>() += 1;
            }
            0
        }
        let mut counter = Self {
            conn,
            steps: Box::new(0),
        };
        // SAFETY: context points into the owned Box, unaffected by moving guard.
        unsafe {
            rusqlite::ffi::sqlite3_progress_handler(
                conn.handle(),
                1,
                Some(tick),
                (&mut *counter.steps as *mut usize).cast(),
            );
        }
        counter
    }
}
impl Drop for VmSteps<'_> {
    fn drop(&mut self) {
        // SAFETY: remove callback before its boxed context is freed.
        unsafe {
            rusqlite::ffi::sqlite3_progress_handler(
                self.conn.handle(),
                0,
                None,
                std::ptr::null_mut(),
            );
        }
    }
}

fn many_nodes(blocks: usize, unrelated: usize) -> String {
    format!(
        "{}{}",
        (0..blocks)
            .map(|i| block(&format!("PB-{i}"), "Literal"))
            .collect::<String>(),
        "<Separator/>".repeat(unrelated)
    )
}

#[test]
fn normalized_corroboration_sql_work_is_single_pass_independent_of_candidate_count() {
    let mut observations = Vec::new();
    for (blocks, unrelated) in [(1, 2048), (512, 2048), (512, 4096)] {
        let (_dir, conn, xml) = setup(&many_nodes(blocks, unrelated));
        assert!(xml.len() < MAX_SOURCE_BYTES);
        let parsed = read_source(xml.as_bytes(), AP, "M-0001").unwrap();
        assert_eq!(parsed.len(), blocks);
        let before = catalogue_snapshot(&conn);
        let changes = conn.total_changes();
        let guard = VmSteps::install(&conn);
        let all = corroborate(&conn, AP, &parsed).unwrap().unwrap();
        let all_steps = *guard.steps;
        drop(guard);
        assert_eq!(all, vec![true; blocks]);
        let guard = VmSteps::install(&conn);
        let one = corroborate(&conn, AP, &parsed[..1]).unwrap().unwrap();
        let one_steps = *guard.steps;
        drop(guard);
        assert_eq!(one, [true]);
        assert_eq!(
            all_steps, one_steps,
            "same SQL walk regardless of candidate count"
        );
        assert!(
            all_steps < 100 * (blocks + unrelated) + 1000,
            "linear SQL VM bound: {all_steps}"
        );
        assert!(
            matches!(folder_captions(&conn, PRODUCT, HP, 20).unwrap(), CaptionEvidence::Declared(v) if v.len()==blocks)
        );
        assert_eq!(conn.total_changes(), changes);
        assert_eq!(catalogue_snapshot(&conn), before);
        eprintln!("NORMALIZED blocks={blocks} unrelated={unrelated} all_vm_steps={all_steps} one_vm_steps={one_steps}");
        observations.push(all_steps);
    }
    assert!(
        observations[2] < observations[1] * 2,
        "doubling unrelated rows remains linear"
    );
}

#[test]
fn normalized_corroboration_row_boundary_refuses_even_unrelated_tail() {
    let (_dir, conn, xml) = setup(&block("PB-8", "Literal"));
    let expected = unchanged_read(&conn);
    let existing: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE program_id=?1 AND module_def_id=''",
            [AP],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(existing, 2, "the normalized Dynamic root is also visited");
    conn.execute("WITH RECURSIVE n(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<?2) INSERT INTO dynamic_node(program_id,module_def_id,node_id,parent_id,position,kind) SELECT ?1,'',i+1000,NULL,i+1000,'Separator' FROM n", rusqlite::params![AP, MAX_NORMALIZED_ROWS as i64-existing]).unwrap();
    assert_eq!(unchanged_read(&conn), expected, "inclusive row boundary");
    conn.execute("INSERT INTO dynamic_node(program_id,module_def_id,node_id,parent_id,position,kind) VALUES (?1,'',999999,NULL,999999,'Separator')", [AP]).unwrap();
    let parsed = read_source(xml.as_bytes(), AP, "M-0001").unwrap();
    let guard = VmSteps::install(&conn);
    assert_eq!(
        corroborate(&conn, AP, &parsed).unwrap(),
        Err(CaptionIssue::ResourceLimit)
    );
    let steps = *guard.steps;
    drop(guard);
    assert!(steps < 100 * (MAX_NORMALIZED_ROWS + 1) + 1000);
    assert_eq!(
        unchanged_read(&conn),
        CaptionEvidence::Unavailable(CaptionIssue::ResourceLimit)
    );
    eprintln!(
        "NORMALIZED row_cap={MAX_NORMALIZED_ROWS} over_rows={} vm_steps={steps}",
        MAX_NORMALIZED_ROWS + 1
    );
}

#[test]
fn normalized_corroboration_metadata_limits_do_not_approve_a_prefix() {
    for column in ["element_id", "text"] {
        let (_dir, conn, _) = setup(&block("PB-8", "Literal"));
        conn.execute(
            &format!(
                "UPDATE dynamic_node SET {column}=?1 WHERE program_id=?2 AND module_def_id=''"
            ),
            rusqlite::params!["x".repeat(MAX_ATTRIBUTE_BYTES + 1), AP],
        )
        .unwrap();
        assert_eq!(
            unchanged_read(&conn),
            CaptionEvidence::Unavailable(CaptionIssue::ResourceLimit)
        );
    }
    let (_dir, conn, _) = setup(&block("PB-8", "Literal"));
    // 4096-byte metadata fields individually fit; only the aggregate is over.
    conn.execute("WITH RECURSIVE n(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<=?2) INSERT INTO dynamic_node(program_id,module_def_id,node_id,parent_id,position,kind,element_id,text) SELECT ?1,'',i+1000,NULL,i+1000,'ParameterBlock',NULL,?3 FROM n", rusqlite::params![AP, (MAX_NORMALIZED_METADATA_BYTES/MAX_ATTRIBUTE_BYTES) as i64, "x".repeat(MAX_ATTRIBUTE_BYTES)]).unwrap();
    assert_eq!(
        unchanged_read(&conn),
        CaptionEvidence::Unavailable(CaptionIssue::ResourceLimit)
    );
}

#[test]
fn same_length_well_formed_raw_tamper_requires_hash_and_original_restore_admits() {
    for (original, changed, source_path) in [
        (
            source(&block("PB-8", "Literal")),
            source(&block("PB-8", "Changed")),
            "M-0001/Application.xml",
        ),
        (
            HARDWARE.to_owned(),
            HARDWARE.replace("Fictional", "Different"),
            "M-0001/Hardware.xml",
        ),
    ] {
        let (_dir, conn, _) = setup(&block("PB-8", "Literal"));
        let expected = unchanged_read(&conn);
        assert_eq!(original.len(), changed.len());
        assert_ne!(original, changed);
        crate::xml::validate_complete_document("synthetic tamper", changed.as_bytes()).unwrap();
        // AP non-caption text changes independently of corroboration so the
        // missing-hash mutant cannot hide behind normalized-text mismatch.
        let changed = if source_path.ends_with("Application.xml") {
            source(&block("PB-8", "Literal")).replace("Name=\"Fictional\"", "Name=\"Different\"")
        } else {
            changed
        };
        assert_eq!(original.len(), changed.len());
        assert_ne!(original, changed);
        crate::xml::validate_complete_document("synthetic same-length tamper", changed.as_bytes())
            .unwrap();
        conn.execute(
            "UPDATE source_file SET bytes=?1 WHERE source_path=?2",
            rusqlite::params![changed.as_bytes(), source_path],
        )
        .unwrap();
        let stored: (String, i64, i64) = conn
            .query_row(
                "SELECT sha256,len,length(bytes) FROM source_file WHERE source_path=?1",
                [source_path],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(stored.0, crate::sha256_hex(original.as_bytes()));
        assert_eq!(stored.1, stored.2);
        assert_eq!(
            unchanged_read(&conn),
            CaptionEvidence::Unavailable(CaptionIssue::SourceIdentity)
        );
        conn.execute(
            "UPDATE source_file SET bytes=?1 WHERE source_path=?2",
            rusqlite::params![original.as_bytes(), source_path],
        )
        .unwrap();
        assert_eq!(unchanged_read(&conn), expected);
    }
}

// --- WP06-S1: same-schema namespace admission and explicit-language captions.
// Fictional manufacturer/program, invented words and private language tags.

const PROJECT_23_URI: &str = "http://knx.org/xml/project/23";
const OTHER_PROGRAM: &str = r#"<ApplicationProgram Id="M-0001_A-0002-01-0000"><Dynamic><ParameterBlock Id="M-0001_A-0002-01-0000_PB-8" Text="Other program" ShowInComObjectTree="true"/></Dynamic></ApplicationProgram>"#;

fn in_namespace(xml: &str, namespace: &str) -> String {
    xml.replace("http://knx.org/xml/project/20", namespace)
}
/// `(language, block, Text)` rows grouped per `Language`, in first-seen order.
fn languages(rows: &[(&str, &str, &str)]) -> String {
    let mut order = Vec::<&str>::new();
    for (language, _, _) in rows {
        if !order.contains(language) {
            order.push(language);
        }
    }
    order
        .iter()
        .map(|language| {
            let elements: String = rows
                .iter()
                .filter(|(l, _, _)| l == language)
                .map(|(_, block, text)| format!(r#"<TranslationElement RefId="{AP}_{block}"><Translation AttributeName="Text" Text="{text}"/></TranslationElement>"#))
                .collect();
            format!(r#"<Language Identifier="{language}"><TranslationUnit RefId="{AP}">{elements}</TranslationUnit></Language>"#)
        })
        .collect()
}
/// One program per document, like a real application file, so the normalized
/// translation rows are stored under this program.
fn localized_source(dynamic: &str, languages: &str) -> String {
    source(dynamic).replace(OTHER_PROGRAM, "").replace(
        "</ApplicationPrograms>",
        &format!("</ApplicationPrograms><Languages>{languages}</Languages>"),
    )
}
fn setup_sources(hardware: &str, application: &str) -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = crate::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    crate::ingest_file(&conn, "M-0001/Hardware.xml", hardware.as_bytes()).unwrap();
    crate::ingest_file(&conn, "M-0001/Application.xml", application.as_bytes()).unwrap();
    (dir, conn)
}
fn localized(conn: &Connection, schema: u32, language: Option<&str>) -> CaptionEvidence {
    let before = catalogue_snapshot(conn);
    let changes = conn.total_changes();
    let evidence = localized_folder_captions(conn, PRODUCT, HP, schema, language).unwrap();
    assert_eq!(
        conn.total_changes(),
        changes,
        "the reader must not repair evidence"
    );
    assert_eq!(catalogue_snapshot(conn), before);
    evidence
}
fn only(evidence: CaptionEvidence) -> FolderCaption {
    let CaptionEvidence::Declared(captions) = evidence else {
        panic!("expected declared captions, got {evidence:?}");
    };
    assert_eq!(captions.len(), 1, "{captions:?}");
    captions.into_values().next().unwrap()
}
fn untranslated(reason: Untranslated) -> Option<CaptionLanguage> {
    Some(CaptionLanguage::Untranslated(reason))
}
fn translated(language: &str) -> Option<CaptionLanguage> {
    Some(CaptionLanguage::Translated(language.into()))
}

#[test]
fn schema23_project_admits_a_uniform_project23_product_source() {
    let application = in_namespace(&source(&block("PB-8", "Brindle shelf")), PROJECT_23_URI);
    let (_dir, conn) = setup_sources(&in_namespace(HARDWARE, PROJECT_23_URI), &application);
    let installed: i64 = conn
        .query_row(
            "SELECT count(*) FROM product p JOIN hardware2program hp ON hp.id=?2 WHERE p.id=?1 AND hp.application_program_ref=?3",
            [PRODUCT, HP, AP],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        installed, 1,
        "calibrate: the project/23 product is installed"
    );
    let caption = only(localized(&conn, 23, None));
    assert_eq!(caption.text, "Brindle shelf");
    assert_eq!(caption.application_program_id, AP);
    assert_eq!(caption.parameter_block_id, format!("{AP}_PB-8"));
    assert_eq!(
        caption.source_sha256,
        crate::sha256_hex(application.as_bytes())
    );
    assert_eq!(caption.language, None, "no language requested");
    assert_eq!(
        folder_captions(&conn, PRODUCT, HP, 23).unwrap(),
        localized(&conn, 23, None)
    );
    // Only the owning project's own schema admits project/23; schema 20 stays as today.
    assert_eq!(
        localized(&conn, 20, None),
        CaptionEvidence::Unavailable(CaptionIssue::SourceIdentity)
    );
    assert_eq!(
        localized(&conn, 21, Some("xx")),
        CaptionEvidence::Unavailable(CaptionIssue::UnsupportedSchema)
    );
}

#[test]
fn mixed_and_foreign_product_namespaces_stay_refused_with_todays_issues() {
    let application = source(&block("PB-8", "Brindle shelf"));
    for (hardware, application, expected) in [
        (
            HARDWARE.to_owned(),
            in_namespace(&application, PROJECT_23_URI),
            CaptionIssue::UnsupportedXml,
        ),
        (
            in_namespace(HARDWARE, PROJECT_23_URI),
            application.clone(),
            CaptionIssue::SourceIdentity,
        ),
        (
            in_namespace(HARDWARE, "http://knx.org/xml/project/21"),
            in_namespace(&application, "http://knx.org/xml/project/21"),
            CaptionIssue::SourceIdentity,
        ),
        (
            in_namespace(HARDWARE, "urn:fictional:catalogue"),
            in_namespace(&application, "urn:fictional:catalogue"),
            CaptionIssue::SourceIdentity,
        ),
    ] {
        let (_dir, conn) = setup_sources(&hardware, &application);
        for schema in [20, 23] {
            for language in [None, Some("xx-YY")] {
                assert_eq!(
                    localized(&conn, schema, language),
                    CaptionEvidence::Unavailable(expected),
                    "schema {schema} {language:?}: {hardware} / {application}"
                );
            }
        }
    }
    // A project/23 application whose inner Dynamic switches namespace is not uniform.
    let inner = in_namespace(&application, PROJECT_23_URI).replace(
        "<Dynamic>",
        "<Dynamic xmlns=\"http://knx.org/xml/project/20\">",
    );
    let (_dir, conn) = setup_sources(&in_namespace(HARDWARE, PROJECT_23_URI), &inner);
    assert_eq!(
        localized(&conn, 23, None),
        CaptionEvidence::Unavailable(CaptionIssue::UnsupportedXml)
    );
}

#[test]
fn explicit_language_selects_the_verified_translation_with_provenance() {
    let rows = [
        ("xx-YY", "PB-8", "Glimmerfach &amp; Tralvo"),
        ("zz-QQ", "PB-8", "Quorvel"),
    ];
    for namespace in ["http://knx.org/xml/project/20", PROJECT_23_URI] {
        let application = in_namespace(
            &localized_source(&block("PB-8", "Brindle shelf"), &languages(&rows)),
            namespace,
        );
        let (_dir, conn) = setup_sources(&in_namespace(HARDWARE, namespace), &application);
        let stored: String = conn
            .query_row(
                "SELECT text FROM translation WHERE scope='Program' AND scope_id=?1 AND language='xx-YY' AND ref_id=?2 AND attribute_name='Text'",
                [AP, &format!("{AP}_PB-8")],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            stored, "Glimmerfach & Tralvo",
            "calibrate the normalized row"
        );
        let default = only(localized(&conn, 23, None));
        let caption = only(localized(&conn, 23, Some("xx-YY")));
        assert_eq!(caption.text, "Glimmerfach & Tralvo");
        assert_eq!(caption.language, translated("xx-YY"));
        // Provenance and identity stay the exact installed declaration.
        assert_eq!(
            caption.application_program_id,
            default.application_program_id
        );
        assert_eq!(caption.parameter_block_id, default.parameter_block_id);
        assert_eq!(caption.source_sha256, default.source_sha256);
        assert_eq!(default.text, "Brindle shelf");
        assert_eq!(default.language, None);
        assert_eq!(only(localized(&conn, 23, Some("zz-QQ"))).text, "Quorvel");
    }
}

#[test]
fn language_selection_follows_best_matching_language_exactly() {
    for (rows, requested, text, language) in [
        // Prefix request: the lowest identifier of the matching variants.
        (
            vec![("xx-ZZ", "PB-8", "Zorvane"), ("xx-YY", "PB-8", "Yelmint")],
            "xx",
            "Yelmint",
            translated("xx-YY"),
        ),
        // An exact identifier wins over a prefix variant.
        (
            vec![("xx-YY", "PB-8", "Yelmint"), ("xx", "PB-8", "Plain xx")],
            "xx",
            "Plain xx",
            translated("xx"),
        ),
        // A longer request never falls back to a shorter stored identifier.
        (
            vec![("xx", "PB-8", "Plain xx")],
            "xx-YY",
            "Brindle shelf",
            untranslated(Untranslated::NoTranslation),
        ),
        // The separator is required: `xx` does not match `xxx-YY`.
        (
            vec![("xxx-YY", "PB-8", "Trixel")],
            "xx",
            "Brindle shelf",
            untranslated(Untranslated::NoTranslation),
        ),
    ] {
        let application = localized_source(&block("PB-8", "Brindle shelf"), &languages(&rows));
        let (_dir, conn) = setup_sources(HARDWARE, &application);
        let caption = only(localized(&conn, 20, Some(requested)));
        assert_eq!(
            (caption.text.as_str(), &caption.language),
            (text, &language),
            "{rows:?} {requested}"
        );
    }
}

#[test]
fn missing_or_unusable_translations_fall_back_to_the_default_text_with_a_reason() {
    for (rows, requested, reason) in [
        (vec![], "xx-YY", Untranslated::NoTranslation),
        (
            vec![("xx-YY", "PB-8", "Yelmint")],
            "zz",
            Untranslated::NoTranslation,
        ),
        (
            vec![("xx-YY", "PB-8", "Yelmint")],
            "",
            Untranslated::NoTranslation,
        ),
        (
            vec![("xx-YY", "PB-9", "Other block")],
            "xx-YY",
            Untranslated::NoTranslation,
        ),
        (
            vec![("xx-YY", "PB-8", "")],
            "xx-YY",
            Untranslated::UnusableTranslation,
        ),
        (
            vec![("xx-YY", "PB-8", "   ")],
            "xx-YY",
            Untranslated::UnusableTranslation,
        ),
        (
            vec![("xx-YY", "PB-8", "Kanal {{0}}")],
            "xx-YY",
            Untranslated::UnusableTranslation,
        ),
        (
            vec![("xx-YY", "PB-8", "a }}")],
            "xx-YY",
            Untranslated::UnusableTranslation,
        ),
        // A block translated twice in one language is ambiguous, even when equal.
        (
            vec![("xx-YY", "PB-8", "Yelmint"), ("xx-YY", "PB-8", "Yelmint")],
            "xx-YY",
            Untranslated::UnusableTranslation,
        ),
        // The selected variant is unusable: no fallback to another variant.
        (
            vec![("xx-AA", "PB-8", ""), ("xx-BB", "PB-8", "Usable")],
            "xx",
            Untranslated::UnusableTranslation,
        ),
    ] {
        let application = localized_source(&block("PB-8", "Brindle shelf"), &languages(&rows));
        let (_dir, conn) = setup_sources(HARDWARE, &application);
        let caption = only(localized(&conn, 20, Some(requested)));
        assert_eq!(caption.text, "Brindle shelf", "{rows:?} {requested}");
        assert_eq!(
            caption.language,
            untranslated(reason),
            "{rows:?} {requested}"
        );
    }
    // A translation owned by another program, manufacturer or attribute is not this block's.
    let rows = languages(&[("xx-YY", "PB-8", "Yelmint")]);
    for foreign in [
        rows.replace(
            &format!("TranslationUnit RefId=\"{AP}\""),
            "TranslationUnit RefId=\"M-0001_A-0009-01-0000\"",
        ),
        rows.replace("AttributeName=\"Text\"", "AttributeName=\"Name\""),
        rows.replace(
            "<TranslationUnit ",
            "<TranslationUnit xmlns=\"urn:fictional:foreign\" ",
        ),
        rows.replace(
            "Text=\"Yelmint\"",
            "xmlns:q=\"urn:fictional:foreign\" q:Text=\"Yelmint\"",
        ),
    ] {
        let application = localized_source(&block("PB-8", "Brindle shelf"), &foreign);
        let (_dir, conn) = setup_sources(HARDWARE, &application);
        let caption = only(localized(&conn, 20, Some("xx-YY")));
        assert_eq!(caption.text, "Brindle shelf", "{foreign}");
        assert_ne!(caption.language, translated("xx-YY"), "{foreign}");
    }
    // The 4096-byte attribute boundary is admitted; one byte more is today's
    // whole-source attribute limit, with or without a language.
    let boundary = "é".repeat(2048);
    let application = localized_source(
        &block("PB-8", "Brindle shelf"),
        &languages(&[("xx-YY", "PB-8", &boundary)]),
    );
    let (_dir, conn) = setup_sources(HARDWARE, &application);
    let caption = only(localized(&conn, 20, Some("xx")));
    assert_eq!(
        (caption.text, caption.language),
        (boundary, translated("xx-YY"))
    );
    let oversized = "é".repeat(2049);
    let application = localized_source(
        &block("PB-8", "Brindle shelf"),
        &languages(&[("xx-YY", "PB-8", &oversized)]),
    );
    let (_dir, conn) = setup_sources(HARDWARE, &application);
    for language in [None, Some("xx")] {
        assert_eq!(
            localized(&conn, 20, language),
            CaptionEvidence::Unavailable(CaptionIssue::ResourceLimit)
        );
    }
}

#[test]
fn translation_row_disagreeing_with_the_verified_bytes_is_not_shown() {
    let rows = languages(&[("xx-YY", "PB-8", "Yelmint")]);
    let application = localized_source(&block("PB-8", "Brindle shelf"), &rows);
    let update = "UPDATE translation SET text=?1 WHERE scope='Program' AND scope_id=?2 AND language='xx-YY' AND attribute_name='Text'";
    for (sql, value, expected) in [
        (
            update,
            Some("Tampered"),
            untranslated(Untranslated::TranslationDisagrees),
        ),
        (
            update,
            None,
            untranslated(Untranslated::TranslationDisagrees),
        ),
        (update, Some("Yelmint"), translated("xx-YY")),
        (
            "DELETE FROM translation WHERE scope='Program' AND scope_id=?2 AND ?1 IS NULL",
            None,
            translated("xx-YY"),
        ),
    ] {
        let (_dir, conn) = setup_sources(HARDWARE, &application);
        assert_eq!(
            only(localized(&conn, 20, Some("xx-YY"))).language,
            translated("xx-YY")
        );
        conn.execute(sql, rusqlite::params![value, AP]).unwrap();
        let caption = only(localized(&conn, 20, Some("xx-YY")));
        assert_eq!(caption.language, expected, "{sql} {value:?}");
        let text = if expected == translated("xx-YY") {
            "Yelmint"
        } else {
            "Brindle shelf"
        };
        assert_eq!(caption.text, text, "{sql} {value:?}");
    }
}
