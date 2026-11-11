//! Verifies literal metadata completeness, safe relations and retained-master name fallback.
use super::*;

fn db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = crate::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    conn.execute_batch(include_str!("../../tests/fixtures/product_details.sql"))
        .unwrap();
    (dir, conn)
}

fn master(conn: &Connection, filename: &str, names: &str) {
    let xml = format!("<KNX><MasterData><Manufacturers>{names}</Manufacturers></MasterData></KNX>");
    crate::ingest_file(conn, filename, xml.as_bytes()).unwrap();
}

#[test]
fn names_resolve_existing_null_and_id_placeholders_without_changing_real_metadata() {
    let (_dir, conn) = db();
    master(
        &conn,
        "knx_master.xml",
        "<Manufacturer Id='M-0083' Name='Fictional Devices'/>",
    );
    for placeholder in [None, Some(""), Some("M-0083"), Some("0083")] {
        assert_eq!(
            manufacturer_name(&conn, "M-0083", placeholder)
                .unwrap()
                .as_deref(),
            Some("Fictional Devices")
        );
    }
    assert_eq!(
        manufacturer_name(&conn, "M-0083", Some("Installed Name"))
            .unwrap()
            .as_deref(),
        Some("Installed Name")
    );
    assert_eq!(manufacturer_name(&conn, "M-FFFF", None).unwrap(), None);
    assert_eq!(manufacturer_name(&conn, "M-83", None).unwrap(), None);
    let row = crate::query::device_product(&conn, "P-rich", "HP-rich", None)
        .unwrap()
        .unwrap();
    assert_eq!(row.manufacturer_name.as_deref(), Some("Fictional Devices"));
    let unchanged: String = conn
        .query_row("SELECT name FROM manufacturer WHERE id='M-0083'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(unchanged, "M-0083");
}

#[test]
fn renamed_sole_generic_master_is_discovered() {
    let (_dir, conn) = db();
    master(
        &conn,
        "renamed.registry",
        "<Manufacturer Id='M-0083' Name='One'/>",
    );
    assert_eq!(
        manufacturer_name(&conn, "M-0083", None).unwrap().as_deref(),
        Some("One")
    );
}

#[test]
fn canonical_master_cannot_hide_a_renamed_generic_conflicting_master() {
    for filename in ["knx_master-v2.xml", "second/knx_master.xml"] {
        let (_dir, conn) = db();
        master(
            &conn,
            "knx_master.xml",
            "<Manufacturer Id='M-0083' Name='One'/>",
        );
        master(&conn, filename, "<Manufacturer Id='M-0083' Name='Two'/>");
        assert_eq!(
            manufacturer_name(&conn, "M-0083", None).unwrap(),
            None,
            "{filename}"
        );
        assert_eq!(
            manufacturer_name(&conn, "M-0083", Some("Real"))
                .unwrap()
                .as_deref(),
            Some("Real")
        );
    }
}

fn retain(conn: &Connection, bytes: &[u8]) {
    conn.execute(
        "INSERT INTO source_file(sha256,source_path,len,bytes) VALUES(?1,'arbitrary-name',?2,?3)",
        rusqlite::params![crate::sha256_hex(bytes), bytes.len() as i64, bytes],
    )
    .unwrap();
}

// Observe actual VM instructions, not returned rows or wall-clock timing.
struct VmSteps<'a> {
    conn: &'a Connection,
    steps: Box<usize>,
}
impl<'a> VmSteps<'a> {
    fn install(conn: &'a Connection) -> Self {
        unsafe extern "C" fn tick(context: *mut std::ffi::c_void) -> i32 {
            // SAFETY: synchronous callback; boxed counter lives until unregister in Drop.
            unsafe {
                *context.cast::<usize>() += 1;
            }
            0
        }
        let mut guard = Self {
            conn,
            steps: Box::new(0),
        };
        // SAFETY: moving this guard does not move the boxed callback context.
        unsafe {
            rusqlite::ffi::sqlite3_progress_handler(
                conn.handle(),
                1,
                Some(tick),
                (&mut *guard.steps as *mut usize).cast(),
            );
        }
        guard
    }
}
impl Drop for VmSteps<'_> {
    fn drop(&mut self) {
        // SAFETY: unregister before freeing the counter.
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

#[test]
fn discovery_sql_work_is_linear_and_shared_for_all_manufacturers_in_one_read() {
    let mut measured = Vec::new();
    for n in [100, 1000] {
        let (_dir, conn) = db();
        master(
            &conn,
            "registry.data",
            "<Manufacturer Id='M-0083' Name='One'/><Manufacturer Id='M-0084' Name='Four'/>",
        );
        for i in 0..n {
            let bytes = format!("<KNX><Baggage Id='{i}'/></KNX>");
            retain(&conn, bytes.as_bytes());
            conn.execute("INSERT INTO package_member(package_sha256,ordinal,path,role,source_sha256,size) VALUES('fixture-package',?1,?2,'Hardware',?3,?4)",
                rusqlite::params![i + 2, format!("other-{i}.xml"), crate::sha256_hex(bytes.as_bytes()), bytes.len() as i64]).unwrap();
        }
        let observer = VmSteps::install(&conn);
        let mut cache = ManufacturerNameCache::default();
        assert_eq!(cache.resolve(&conn, "M-0083", None).as_deref(), Some("One"));
        let first = *observer.steps;
        assert_eq!(
            cache.resolve(&conn, "M-0084", None).as_deref(),
            Some("Four")
        );
        for id in 0x100..0x200 {
            assert_eq!(cache.resolve(&conn, &format!("M-{id:04X}"), None), None);
        }
        assert_eq!(
            *observer.steps, first,
            "other manufacturers must reuse the complete index without SQL"
        );
        assert_eq!(cache.refusal(), None);
        assert!(
            first < 100_000,
            "bounded linear SQL, n={n}, actual steps={first}"
        );
        measured.push(first);
    }
    assert!(
        measured[1] <= measured[0] * 12 + 1000,
        "not quadratic: {measured:?}"
    );
    eprintln!("retained-name SQL VM steps (100,1000 unrelated source/member pairs): {measured:?}");
}

#[test]
fn name_cache_lives_only_for_its_read_and_never_overrides_a_real_name() {
    let (_dir, conn) = db();
    master(&conn, "first", "<Manufacturer Id='M-0083' Name='One'/>");
    let mut cache = ManufacturerNameCache::default();
    let row = crate::query::device_product_cached(&conn, "P-rich", "HP-rich", None, &mut cache)
        .unwrap()
        .unwrap();
    assert_eq!(row.manufacturer_name.as_deref(), Some("One"));
    // The owner must discard the old read cache. A new read observes new conflicting evidence.
    master(&conn, "second", "<Manufacturer Id='M-0083' Name='Two'/>");
    assert_eq!(
        crate::query::device_product_cached(
            &conn,
            "P-rich",
            "HP-rich",
            None,
            &mut ManufacturerNameCache::default()
        )
        .unwrap()
        .unwrap()
        .manufacturer_name,
        None
    );
    assert_eq!(
        cache.resolve(&conn, "M-0083", Some("Installed")).as_deref(),
        Some("Installed")
    );
}

#[test]
fn source_count_boundary_refuses_all_names_without_suppressing_product_facts() {
    assert_eq!(MAX_NAME_SOURCES, 4096);
    let (_dir, conn) = db();
    master(&conn, "first", "<Manufacturer Id='M-0083' Name='One'/>");
    for i in 0..4094 {
        retain(&conn, format!("<KNX Id='{i}'/>").as_bytes());
    }
    let mut admitted = ManufacturerNameCache::default();
    assert_eq!(
        admitted.resolve(&conn, "M-0083", None).as_deref(),
        Some("One")
    );
    assert_eq!(admitted.refusal(), None);
    master(
        &conn,
        "unexamined-conflict",
        "<Manufacturer Id='M-0083' Name='Two'/>",
    );
    let observer = VmSteps::install(&conn);
    let mut refused = ManufacturerNameCache::default();
    let row = crate::query::device_product_cached(&conn, "P-rich", "HP-rich", None, &mut refused)
        .unwrap()
        .unwrap();
    assert_eq!(row.manufacturer_name, None);
    assert_eq!(refused.refusal(), Some(NameDiscoveryRefusal::SourceCount));
    assert_eq!(row.product_text.as_deref(), Some("Fictional actuator"));
    assert!(!device_product_details(&conn, "P-rich", "HP-rich")
        .unwrap()
        .is_empty());
    assert!(
        *observer.steps < 200_000,
        "row refusal must not scan source x membership: {}",
        observer.steps
    );
    assert_eq!(
        refused
            .resolve(&conn, "M-0083", Some("Installed"))
            .as_deref(),
        Some("Installed")
    );
}

fn padded_xml(size: usize, marker: &str) -> Vec<u8> {
    let mut xml =
        format!("<KNX><MasterData><Manufacturers>{marker}</Manufacturers></MasterData><!--");
    xml.extend(std::iter::repeat_n(
        ' ',
        size - xml.len() - "--></KNX>".len(),
    ));
    xml.push_str("--></KNX>");
    assert_eq!(xml.len(), size);
    xml.into_bytes()
}

#[test]
fn per_source_and_aggregate_byte_boundaries_never_approve_partial_uniqueness() {
    assert_eq!(MAX_NAME_SOURCE_BYTES, 8_388_608);
    assert_eq!(MAX_NAME_TOTAL_BYTES, 33_554_432);
    let (_dir, conn) = db();
    let declaration = "<Manufacturer Id='M-0083' Name='One'/>";
    retain(&conn, &padded_xml(8_388_608, declaration));
    assert_eq!(
        manufacturer_name(&conn, "M-0083", None).unwrap().as_deref(),
        Some("One")
    );
    retain(
        &conn,
        &padded_xml(8_388_609, "<Manufacturer Id='M-0083' Name='Two'/>"),
    );
    let mut refused = ManufacturerNameCache::default();
    assert_eq!(refused.resolve(&conn, "M-0083", None), None);
    assert_eq!(refused.refusal(), Some(NameDiscoveryRefusal::SourceBytes));
    conn.execute(
        "DELETE FROM source_file WHERE source_path='arbitrary-name'",
        [],
    )
    .unwrap();
    // The fixture owns 6 bytes. Four individually admitted sources fill the aggregate exactly.
    for (size, marker) in [
        (8_388_602, declaration),
        (8_388_608, "<!--a-->"),
        (8_388_608, "<!--b-->"),
        (8_388_608, "<!--c-->"),
    ] {
        retain(&conn, &padded_xml(size, marker));
    }
    assert_eq!(
        manufacturer_name(&conn, "M-0083", None).unwrap().as_deref(),
        Some("One")
    );
    retain(&conn, b"<KNX/>"); // next reachable valid source exceeds the aggregate by six bytes
    let mut refused = ManufacturerNameCache::default();
    assert_eq!(refused.resolve(&conn, "M-0083", None), None);
    assert_eq!(refused.refusal(), Some(NameDiscoveryRefusal::SourceBytes));
    assert_eq!(
        refused.resolve(&conn, "M-0083", Some("Real")).as_deref(),
        Some("Real")
    );
}

#[test]
fn aggregate_xml_work_boundary_refuses_without_losing_normalized_product_values() {
    assert_eq!(MAX_NAME_XML_EVENTS, 262_144);
    assert_eq!(MAX_NAME_XML_DEPTH, 128);
    assert_eq!(MAX_NAME_XML_ATTRIBUTES, 256);
    let (_dir, conn) = db();
    master(&conn, "first", "<Manufacturer Id='M-0083' Name='One'/>");
    // Root start/end/EOF + 262130 empty events, plus 8 master events: 262141 <= 262144.
    retain(
        &conn,
        format!("<KNX>{}</KNX>", "<X/>".repeat(262_130)).as_bytes(),
    );
    let mut admitted = ManufacturerNameCache::default();
    assert_eq!(
        admitted.resolve(&conn, "M-0083", None).as_deref(),
        Some("One")
    );
    retain(&conn, b"<KNX><!--next--></KNX>");
    let mut refused = ManufacturerNameCache::default();
    let row = crate::query::device_product_cached(&conn, "P-rich", "HP-rich", None, &mut refused)
        .unwrap()
        .unwrap();
    assert_eq!(row.manufacturer_name, None);
    assert_eq!(refused.refusal(), Some(NameDiscoveryRefusal::XmlWork));
    assert_eq!(row.order_number.as_deref(), Some("ORDER-004"));
}

// A prepare-time projection oracle fails SELECT * even when the payload is skipped in Rust.
struct DenyPayloads<'a>(&'a Connection);
impl<'a> DenyPayloads<'a> {
    fn install(conn: &'a Connection) -> Self {
        unsafe extern "C" fn deny(
            _: *mut std::ffi::c_void,
            action: i32,
            table: *const std::ffi::c_char,
            column: *const std::ffi::c_char,
            _: *const std::ffi::c_char,
            _: *const std::ffi::c_char,
        ) -> i32 {
            if action == rusqlite::ffi::SQLITE_READ && !table.is_null() && !column.is_null() {
                // SAFETY: SQLite provides valid NUL-terminated callback strings.
                let (table, column) = unsafe {
                    (
                        std::ffi::CStr::from_ptr(table).to_bytes(),
                        std::ffi::CStr::from_ptr(column).to_bytes(),
                    )
                };
                if matches!(table, b"package" | b"source_file") && column == b"bytes" {
                    return rusqlite::ffi::SQLITE_DENY;
                }
            }
            rusqlite::ffi::SQLITE_OK
        }
        // SAFETY: callback has no borrowed context; guard unregisters before connection drop.
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
impl Drop for DenyPayloads<'_> {
    fn drop(&mut self) {
        // SAFETY: unregister while the borrowed connection is still alive.
        unsafe {
            rusqlite::ffi::sqlite3_set_authorizer(self.0.handle(), None, std::ptr::null_mut());
        }
    }
}

#[test]
fn xml_shape_boundaries_refuse_even_if_another_master_already_has_a_name() {
    for (admitted, refused) in [
        (
            format!("{}{}", "<X>".repeat(128), "</X>".repeat(128)),
            format!("{}{}", "<X>".repeat(129), "</X>".repeat(129)),
        ),
        (
            format!(
                "<KNX {}/>",
                (0..256).map(|i| format!("a{i}='' ")).collect::<String>()
            ),
            format!(
                "<KNX {}/>",
                (0..257).map(|i| format!("a{i}='' ")).collect::<String>()
            ),
        ),
    ] {
        let (_dir, conn) = db();
        master(&conn, "first", "<Manufacturer Id='M-0083' Name='One'/>");
        retain(&conn, admitted.as_bytes());
        assert_eq!(
            manufacturer_name(&conn, "M-0083", None).unwrap().as_deref(),
            Some("One")
        );
        retain(&conn, refused.as_bytes());
        let mut cache = ManufacturerNameCache::default();
        assert_eq!(cache.resolve(&conn, "M-0083", None), None);
        assert_eq!(cache.refusal(), Some(NameDiscoveryRefusal::XmlWork));
    }
}

#[test]
fn scalar_details_never_project_package_or_source_payloads() {
    let (_dir, conn) = db();
    let expected = device_product_details(&conn, "P-rich", "HP-rich").unwrap();
    conn.execute("UPDATE package SET bytes=zeroblob(33554432)", [])
        .unwrap();
    conn.execute("UPDATE source_file SET bytes=zeroblob(33554432)", [])
        .unwrap();
    let _guard = DenyPayloads::install(&conn);
    assert!(
        conn.prepare("SELECT bytes FROM package").is_err(),
        "oracle must deny binary access"
    );
    assert!(conn.prepare("SELECT bytes FROM source_file").is_err());
    assert_eq!(
        device_product_details(&conn, "P-rich", "HP-rich").unwrap(),
        expected
    );
}

#[test]
fn unreadable_optional_discovery_keeps_product_values_and_installed_names() {
    let (_dir, conn) = db();
    let _guard = DenyPayloads::install(&conn);
    let mut cache = ManufacturerNameCache::default();
    let row = crate::query::device_product_cached(&conn, "P-rich", "HP-rich", None, &mut cache)
        .unwrap()
        .unwrap();
    assert_eq!(row.manufacturer_name, None);
    assert_eq!(cache.refusal(), Some(NameDiscoveryRefusal::Unreadable));
    assert_eq!(row.product_text.as_deref(), Some("Fictional actuator"));
    assert_eq!(
        cache.resolve(&conn, "M-0083", Some("Real")).as_deref(),
        Some("Real")
    );
}

#[test]
fn duplicate_names_agree_but_conflicting_or_malformed_master_evidence_does_not_guess() {
    let (_dir, conn) = db();
    master(
        &conn,
        "one/knx_master.xml",
        "<Manufacturer Id='M-0083' Name='One'/><Manufacturer Id='M-0083' Name='One'/>",
    );
    assert_eq!(
        manufacturer_name(&conn, "M-0083", None).unwrap().as_deref(),
        Some("One")
    );
    master(
        &conn,
        "two/knx_master.xml",
        "<Manufacturer Id='M-0083' Name='Two'/>",
    );
    assert_eq!(manufacturer_name(&conn, "M-0083", None).unwrap(), None);
    assert_eq!(
        manufacturer_name(&conn, "M-0083", Some("Real"))
            .unwrap()
            .as_deref(),
        Some("Real")
    );
    conn.execute(
        "UPDATE source_file SET bytes=X'3C' WHERE source_path LIKE '%knx_master.xml'",
        [],
    )
    .unwrap();
    assert_eq!(manufacturer_name(&conn, "M-0083", None).unwrap(), None);
}

#[test]
fn well_formed_but_changed_master_bytes_are_not_name_evidence() {
    let (_dir, conn) = db();
    master(
        &conn,
        "knx_master.xml",
        "<Manufacturer Id='M-0083' Name='One'/>",
    );
    conn.execute("UPDATE source_file SET bytes=CAST(replace(CAST(bytes AS TEXT),'One','Two') AS BLOB) WHERE source_path='knx_master.xml'", []).unwrap();
    assert_eq!(manufacturer_name(&conn, "M-0083", None).unwrap(), None);
    assert_eq!(
        manufacturer_name(&conn, "M-0083", Some("Real Name"))
            .unwrap()
            .as_deref(),
        Some("Real Name")
    );
}

#[test]
fn all_normalized_columns_and_every_placement_are_preserved_without_binary_or_local_paths() {
    let (_dir, conn) = db();
    let details = device_product_details(&conn, "P-rich", "HP-rich").unwrap();
    for (kind, table) in [
        ("product", "product"),
        ("hardware", "hardware"),
        ("programLink", "hardware2program"),
        ("application", "application_program"),
        ("manufacturer", "manufacturer"),
    ] {
        let section = details.iter().find(|s| s.kind == kind).unwrap();
        let stmt = conn.prepare(&format!("SELECT * FROM {table}")).unwrap();
        let keys: Vec<_> = stmt
            .column_names()
            .into_iter()
            .filter(|key| *key != "write_authority_recorded")
            .map(str::to_owned)
            .collect();
        assert_eq!(
            section
                .fields
                .iter()
                .map(|f| f.key.clone())
                .collect::<Vec<_>>(),
            keys,
            "{table} metadata must not silently shrink"
        );
    }
    assert_eq!(
        details
            .iter()
            .filter(|s| s.kind == "catalogItem")
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>(),
        vec!["CI-a", "CI-b"]
    );
    assert_eq!(
        details
            .iter()
            .filter(|s| s.kind == "catalogSection")
            .count(),
        2
    );
    let source = details.iter().find(|s| s.kind == "source").unwrap();
    assert!(source
        .fields
        .iter()
        .any(|f| f.key == "source_name" && f.value == "fixture.xml"));
    assert!(details.iter().any(|s| s.kind == "additionalAttributes"
        && s.fields
            .iter()
            .any(|f| f.value == "FutureHardwareAttribute")));
    assert!(details
        .iter()
        .flat_map(|s| &s.fields)
        .all(|f| f.key != "bytes" && !f.value.contains("/home/alice")));
    let package = details.iter().find(|s| s.kind == "package").unwrap();
    assert_eq!(
        package
            .fields
            .iter()
            .map(|f| (f.key.as_str(), f.value.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("sha256", "fixture-package"),
            ("source_name", "fixture.knxprod"),
            ("scheme", "11"),
            ("size", "3")
        ]
    );
    assert!(source
        .fields
        .iter()
        .any(|f| f.key == "source_name" && f.value == "vendor.zip!/fixture.knxprod"));
    assert!(device_product_details(&conn, "missing", "HP-rich")
        .unwrap()
        .is_empty());
}

#[test]
fn missing_optional_values_cycles_and_mismatched_programs_stay_partial() {
    let (_dir, conn) = db();
    conn.execute_batch("UPDATE hardware SET bus_current=NULL; UPDATE product SET text=''; UPDATE catalog_section SET parent_id='CS-child' WHERE id='CS-root'; UPDATE hardware2program SET hardware_id='foreign' WHERE id='HP-rich';").unwrap();
    let details = device_product_details(&conn, "P-rich", "HP-rich").unwrap();
    assert!(!details.iter().any(|s| matches!(
        s.kind.as_str(),
        "application" | "programLink" | "catalogItem"
    )));
    assert!(!details
        .iter()
        .flat_map(|s| &s.fields)
        .any(|f| f.key == "bus_current"));
    assert!(details
        .iter()
        .flat_map(|s| &s.fields)
        .any(|f| f.key == "text" && f.value.is_empty()));
    conn.execute("UPDATE hardware2program SET hardware_id='H-rich'", [])
        .unwrap();
    assert_eq!(
        device_product_details(&conn, "P-rich", "HP-rich")
            .unwrap()
            .iter()
            .filter(|s| s.kind == "catalogSection")
            .count(),
        2
    );
}
