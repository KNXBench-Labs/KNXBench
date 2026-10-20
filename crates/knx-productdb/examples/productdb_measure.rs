//! Measures explicit current ProductDB setup and read-only AP1 resolution.
use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::Read,
    path::Path,
};

use knx_productdb::{
    open_and_migrate, open_read_only, procedure_resolution::resolve_package_ap1, Connection,
    CURRENT_PRODUCTDB_VERSION,
};
use serde_json::{json, Value};

type ProbeResult<T> = Result<T, &'static str>;
const MAX_ISSUE_CODES: usize = 32;
const MAX_CODE_BYTES: usize = 96;

fn absolute_path(path: &Path) -> ProbeResult<()> {
    if !path.is_absolute() {
        return Err("--product-db must be an absolute path");
    }
    Ok(())
}

// Only the quiescent, rollback-journal layout used by this study is admitted.
// Check before SQLite opens: even a read-only WAL reader can create sidecars.
fn no_sidecars(path: &Path) -> ProbeResult<()> {
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        match fs::symlink_metadata(Path::new(&sidecar)) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err("database sidecars are unsupported; require a quiescent catalogue"),
        }
    }
    Ok(())
}

fn current_disk_version(path: &Path) -> ProbeResult<i64> {
    absolute_path(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| "database must exist")?;
    if !metadata.file_type().is_file() {
        return Err("database must be a regular file, not a symlink");
    }
    no_sidecars(path)?;
    let mut header = [0; 20];
    File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .map_err(|_| "cannot read database header")?;
    if &header[..16] != b"SQLite format 3\0" || header[18..20] != [1, 1] {
        return Err("require a rollback-journal SQLite database");
    }
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| "cannot open database read-only")?;
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|_| "cannot read database schema version")?;
    conn.close()
        .map_err(|_| "cannot close schema admission connection")?;
    if version != CURRENT_PRODUCTDB_VERSION {
        return Err("database schema must already be current; migration is not measured");
    }
    Ok(version)
}

fn init_current(path: &Path) -> ProbeResult<i64> {
    absolute_path(path)?;
    if !path.parent().is_some_and(Path::is_dir) {
        return Err("database parent must already exist in an owned attempt");
    }
    no_sidecars(path)?;
    // The caller owns the parent and excludes replacement races for this study.
    // create_new also refuses dangling symlinks, unlike an exists() check.
    let reservation = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| "init requires a new database path")?;
    drop(reservation);
    let conn = open_and_migrate(path).map_err(|_| "cannot initialize database")?;
    conn.close()
        .map_err(|_| "cannot close initialized database")?;
    current_disk_version(path)
}

fn resolve_current(path: &Path, package: &str, program: &str) -> ProbeResult<Value> {
    if package.len() != 64
        || !package
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("--package-sha256 must be exactly 64 lowercase hexadecimal characters");
    }
    if program.is_empty() {
        return Err("--program-id must not be empty");
    }
    let schema = current_disk_version(path)?;
    let db = open_read_only(path).map_err(|_| "cannot open current catalogue read-only")?;
    if db.migrated_from.is_some() {
        return Err("unexpected in-memory migration");
    }
    let resolution =
        resolve_package_ap1(&db.conn, package, program).map_err(|_| "AP1 resolution failed")?;
    db.conn
        .close()
        .map_err(|_| "cannot close resolver connection")?;
    let result = match resolution.as_ref() {
        None => Value::Null,
        Some(resolution) => {
            if resolution.executable
                || !matches!(
                    resolution.status.as_str(),
                    "expanded" | "partial" | "unavailable"
                )
            {
                return Err("unexpected resolver status or execution capability");
            }
            let codes: Vec<&str> = resolution
                .issues
                .iter()
                .take(MAX_ISSUE_CODES)
                .map(|issue| issue.code.as_str())
                .collect();
            if codes.iter().any(|code| {
                code.len() > MAX_CODE_BYTES
                    || !code.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                    })
            }) {
                return Err("resolver issue code exceeds the bounded summary contract");
            }
            let omitted = resolution
                .omitted_issue_count
                .checked_add(resolution.issues.len() - codes.len())
                .ok_or("resolver issue count overflow")?;
            json!({
                "status": resolution.status,
                "executable": resolution.executable,
                "steps": resolution.steps.len(),
                "unplaced": resolution.unplaced_declarations.len(),
                "issueCodes": codes,
                "omittedIssueCount": omitted
            })
        }
    };
    Ok(json!({"format": 1, "operation": "resolve-ap1", "schema": schema, "result": result}))
}

fn run(args: &[OsString]) -> ProbeResult<Value> {
    let operation = args
        .first()
        .and_then(|arg| arg.to_str())
        .ok_or("expected init or resolve-ap1")?;
    if !matches!(operation, "init" | "resolve-ap1") || args.len() % 2 != 1 {
        return Err("expected init or resolve-ap1 with explicit option/value pairs");
    }
    let (mut path, mut package, mut program) = (None, None, None);
    for pair in args[1..].as_chunks::<2>().0 {
        let slot = match pair[0].to_str() {
            Some("--product-db") => &mut path,
            Some("--package-sha256") if operation == "resolve-ap1" => &mut package,
            Some("--program-id") if operation == "resolve-ap1" => &mut program,
            _ => return Err("unknown option"),
        };
        if slot.replace(pair[1].as_os_str()).is_some() {
            return Err("duplicate option");
        }
    }
    let path = Path::new(path.ok_or("--product-db is required")?);
    if operation == "init" {
        let schema = init_current(path)?;
        return Ok(json!({"format": 1, "operation": "init", "schema": schema,
            "sqliteVersion": rusqlite::version()}));
    }
    let package = package
        .and_then(|value| value.to_str())
        .ok_or("--package-sha256 is required and must be UTF-8")?;
    let program = program
        .and_then(|value| value.to_str())
        .ok_or("--program-id is required and must be UTF-8")?;
    resolve_current(path, package, program)
}

fn main() {
    match run(&std::env::args_os().skip(1).collect::<Vec<_>>()) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("productdb_measure: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_productdb::{
        open_and_migrate, procedure_resolution::MAX_XML_BYTES, Connection,
        CURRENT_PRODUCTDB_VERSION,
    };
    use rusqlite::types::Value as SqlValue;
    use std::{fs, path::PathBuf};

    const ID: &str = "M-0001_A-1";
    const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const NS: &str = "http://knx.org/xml/project/20";

    fn args(parts: &[&str]) -> Vec<OsString> {
        parts.iter().map(OsString::from).collect()
    }

    fn files(root: &Path) -> Vec<(OsString, Vec<u8>)> {
        let mut files: Vec<_> = fs::read_dir(root)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (entry.file_name(), fs::read(entry.path()).unwrap())
            })
            .collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        files
    }

    fn unchanged(root: &Path, before: &[(OsString, Vec<u8>)]) {
        assert!(files(root) == before, "database or sidecar bytes changed");
    }

    fn logical(path: &Path) -> Vec<(String, Vec<Vec<SqlValue>>)> {
        let conn =
            Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let mut tables = conn
            .prepare("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")
            .unwrap();
        let names: Vec<String> = tables
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        names
            .into_iter()
            .map(|name| {
                let query = format!("SELECT * FROM \"{}\"", name.replace('"', "\"\""));
                let mut statement = conn.prepare(&query).unwrap();
                let columns = statement.column_count();
                let rows = statement
                    .query_map([], |row| {
                        (0..columns)
                            .map(|i| row.get(i))
                            .collect::<Result<Vec<SqlValue>, _>>()
                    })
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                (name, rows)
            })
            .collect()
    }

    fn fixture(root: &Path, text_bytes: usize) -> (PathBuf, String, Vec<u8>, String, String) {
        fixture_with_steps(root, text_bytes, "")
    }

    fn fixture_with_steps(
        root: &Path,
        text_bytes: usize,
        extra_steps: &str,
    ) -> (PathBuf, String, Vec<u8>, String, String) {
        let path = root.join("catalogue.sqlite");
        let conn = open_and_migrate(&path).unwrap();
        let master = format!(
            r#"<KNX xmlns="{NS}"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Synthetic"/></Manufacturers><MaskVersions><MaskVersion Id="MV-07B0"><HawkConfigurationData><Procedures><Procedure ProcedureType="Load" ProcedureSubType="ap1"><LdCtrlConnect/><LdCtrlMerge MergeId="1"/><LdCtrlLoad LsmIdx="4"/><LdCtrlMerge MergeId="2"/><LdCtrlMerge MergeId="4"/><LdCtrlMerge MergeId="6"/><LdCtrlMerge MergeId="7"/><LdCtrlRestart/></Procedure></Procedures></HawkConfigurationData></MaskVersion></MaskVersions></MasterData></KNX>"#
        );
        let text = "x".repeat(text_bytes);
        let program = format!(
            r#"<KNX xmlns="{NS}"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="{ID}" MaskVersion="MV-07B0" LoadProcedureStyle="MergedProcedure"><Static><ParameterTypes><ParameterType Id="{ID}_PT-N"><TypeNumber Type="unsignedInt" SizeInBit="8" minInclusive="0" maxInclusive="255"/></ParameterType></ParameterTypes><Parameters><Parameter Id="{ID}_P-1" ParameterType="{ID}_PT-N" Text="private-tree-marker-{text}" Value="0"/></Parameters><LoadProcedures><LoadProcedure MergeId="2"><LdCtrlRelSegment LsmIdx="4" Size="16"/>{extra_steps}</LoadProcedure><LoadProcedure MergeId="4"><LdCtrlWriteRelMem ObjIdx="4" Size="16"/></LoadProcedure></LoadProcedures></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
        );
        let package = knx_testsupport::zip_with_entries(&[
            ("knx_master.xml", master.as_bytes()),
            ("M-0001/A.xml", program.as_bytes()),
            ("M-0001/signature.bin", b"\x00\xffsynthetic-opaque"),
        ]);
        let report = knx_productdb::install_package(&conn, "synthetic.knxprod", &package).unwrap();
        let count: i64 = conn
            .query_row("SELECT count(*) FROM parameter", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "fixture must install meaningful parameter data");
        drop(conn);
        (path, report.sha256, package, master, program)
    }

    #[test]
    fn init_creates_a_current_blank_db_and_closes_it() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("blank.sqlite");
        assert_eq!(init_current(&path).unwrap(), CURRENT_PRODUCTDB_VERSION);
        let conn = Connection::open(&path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_PRODUCTDB_VERSION);
        for table in ["package", "source_file", "application_program"] {
            let count: i64 = conn
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 0);
        }
        conn.close().unwrap();
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[test]
    fn init_refuses_existing_paths_without_changing_bytes() {
        for version in [
            0,
            CURRENT_PRODUCTDB_VERSION - 1,
            CURRENT_PRODUCTDB_VERSION,
            CURRENT_PRODUCTDB_VERSION + 1,
        ] {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("existing.sqlite");
            if version == 0 {
                fs::write(&path, []).unwrap();
            } else {
                let conn = open_and_migrate(&path).unwrap();
                conn.pragma_update(None, "user_version", version).unwrap();
                conn.close().unwrap();
            }
            let before = files(temp.path());
            assert!(init_current(&path).is_err());
            unchanged(temp.path(), &before);
        }
    }

    #[test]
    #[cfg(unix)]
    fn init_and_resolve_refuse_symlinks_including_dangling_ones() {
        for exists in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let target = temp.path().join("target.sqlite");
            if exists {
                open_and_migrate(&target).unwrap().close().unwrap();
            }
            let link = temp.path().join("link.sqlite");
            std::os::unix::fs::symlink(&target, &link).unwrap();
            let before = exists.then(|| fs::read(&target).unwrap());
            assert!(init_current(&link).is_err());
            assert!(resolve_current(&link, DIGEST, ID).is_err());
            assert!(fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink());
            assert_eq!(fs::read(&target).ok(), before);
        }
    }

    #[test]
    fn resolve_refuses_missing_old_future_empty_and_malformed_without_mutation() {
        for version in [
            -2,
            -1,
            0,
            CURRENT_PRODUCTDB_VERSION - 1,
            CURRENT_PRODUCTDB_VERSION + 1,
        ] {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("catalogue.sqlite");
            match version {
                -2 => {}
                -1 => fs::write(&path, b"not a database").unwrap(),
                0 => fs::write(&path, []).unwrap(),
                _ => {
                    let conn = open_and_migrate(&path).unwrap();
                    conn.pragma_update(None, "user_version", version).unwrap();
                    conn.close().unwrap();
                }
            }
            let before = files(temp.path());
            assert!(resolve_current(&path, DIGEST, ID).is_err());
            unchanged(temp.path(), &before);
        }
    }

    #[test]
    fn paths_must_be_absolute_and_parent_must_exist() {
        assert!(init_current(Path::new("relative.sqlite")).is_err());
        assert!(resolve_current(Path::new("relative.sqlite"), DIGEST, ID).is_err());
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("missing-parent/catalogue.sqlite");
        assert!(init_current(&path).is_err());
        assert!(!temp.path().join("missing-parent").exists());
        assert!(init_current(temp.path()).is_err());
        assert!(resolve_current(temp.path(), DIGEST, ID).is_err());
    }

    #[test]
    fn strict_commands_reject_missing_unknown_duplicate_and_malformed_arguments() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("catalogue.sqlite");
        let path = path.to_str().unwrap();
        let invalid = vec![
            args(&[]),
            args(&["install", "--product-db", path]),
            args(&["init"]),
            args(&["init", "--product-db"]),
            args(&["init", "--db", path]),
            args(&["init", "--product-db", path, "extra"]),
            args(&["init", "--product-db", path, "--product-db", path]),
            args(&["init", "--product-db", path, "--program-id", ID]),
            args(&[
                "resolve-ap1",
                "--product-db",
                path,
                "--package-sha256",
                DIGEST,
            ]),
            args(&[
                "resolve-ap1",
                "--product-db",
                path,
                "--package-sha256",
                DIGEST,
                "--program-id",
                "",
            ]),
            args(&[
                "resolve-ap1",
                "--product-db",
                path,
                "--package-sha256",
                &DIGEST.to_uppercase(),
                "--program-id",
                ID,
            ]),
            args(&[
                "resolve-ap1",
                "--product-db",
                path,
                "--package-sha256",
                "abcd",
                "--program-id",
                ID,
            ]),
            args(&[
                "resolve-ap1",
                "--product-db",
                path,
                "--package-sha256",
                DIGEST,
                "--program-id",
                ID,
                "--program-id",
                ID,
            ]),
        ];
        for argv in invalid {
            assert!(run(&argv).is_err());
            assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
        }
    }

    #[test]
    fn init_command_emits_only_the_setup_contract() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("catalogue.sqlite");
        let result = run(&args(&["init", "--product-db", path.to_str().unwrap()])).unwrap();
        assert_eq!(result.as_object().unwrap().len(), 4);
        assert_eq!(result["format"], 1);
        assert_eq!(result["operation"], "init");
        assert_eq!(result["schema"], CURRENT_PRODUCTDB_VERSION);
        assert_eq!(result["sqliteVersion"], rusqlite::version());
    }

    #[test]
    fn resolve_summary_distinguishes_none_unavailable_and_nonempty_expansion() {
        let temp = tempfile::tempdir().unwrap();
        let (path, sha, _, _, _) = fixture(temp.path(), 0);
        let expanded = run(&args(&[
            "resolve-ap1",
            "--program-id",
            ID,
            "--package-sha256",
            &sha,
            "--product-db",
            path.to_str().unwrap(),
        ]))
        .unwrap();
        assert_eq!(expanded["format"], 1);
        assert_eq!(expanded["operation"], "resolve-ap1");
        assert_eq!(expanded["schema"], CURRENT_PRODUCTDB_VERSION);
        assert_eq!(expanded["result"]["status"], "expanded");
        assert_eq!(expanded["result"]["steps"], 5);
        assert_eq!(expanded["result"]["unplaced"], 0);
        assert_eq!(expanded["result"]["executable"], false);
        assert_eq!(expanded["result"]["issueCodes"], serde_json::json!([]));
        assert_eq!(expanded["result"]["omittedIssueCount"], 0);
        let absent = resolve_current(&path, &sha, "missing-program").unwrap();
        assert!(absent["result"].is_null());
        let unavailable = resolve_current(&path, DIGEST, ID).unwrap();
        assert_eq!(unavailable["result"]["status"], "unavailable");
        assert_eq!(unavailable["result"]["steps"], 0);
        assert_eq!(unavailable["result"]["executable"], false);
        assert_eq!(
            unavailable["result"]["issueCodes"],
            serde_json::json!(["program-source-not-in-package"])
        );
    }

    #[test]
    fn summary_preserves_installed_source_and_all_logical_and_file_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let (path, sha, package, master, program) = fixture(temp.path(), 1024);
        let before = files(temp.path());
        let rows = logical(&path);
        let result = resolve_current(&path, &sha, ID).unwrap();
        let json = result.to_string();
        assert!(json.len() < 1024);
        assert!(!json.contains("private-tree-marker"));
        assert!(!json.contains(&sha));
        assert!(!json.contains(ID));
        assert_eq!(result.as_object().unwrap().len(), 4);
        assert_eq!(result["result"].as_object().unwrap().len(), 6);
        assert!(logical(&path) == rows, "logical catalogue changed");
        unchanged(temp.path(), &before);
        let conn =
            Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let retained: Vec<u8> = conn
            .query_row("SELECT bytes FROM package WHERE sha256=?1", [&sha], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(retained == package, "package bytes not preserved");
        for source in [master, program] {
            let key = knx_productdb::sha256_hex(source.as_bytes());
            let retained = knx_productdb::load_source_file(&conn, &key)
                .unwrap()
                .unwrap();
            assert!(retained == source.as_bytes(), "source bytes not preserved");
        }
    }

    #[test]
    fn partial_summary_bounds_issue_codes_and_accounts_for_omissions() {
        let temp = tempfile::tempdir().unwrap();
        let controls = r#"<UnknownControl Private="private-tree-marker"/>"#.repeat(64);
        let (path, sha, _, _, _) = fixture_with_steps(temp.path(), 0, &controls);
        let conn = knx_productdb::open_read_only(&path).unwrap().conn;
        let full = knx_productdb::procedure_resolution::resolve_package_ap1(&conn, &sha, ID)
            .unwrap()
            .unwrap();
        assert!(full.issues.len() > 32);
        assert_eq!(full.status, "partial");
        conn.close().unwrap();
        let before = files(temp.path());
        let summary = resolve_current(&path, &sha, ID).unwrap();
        let result = &summary["result"];
        assert_eq!(result["status"], "partial");
        assert_eq!(result["executable"], false);
        assert_eq!(result["steps"], full.steps.len());
        assert_eq!(result["issueCodes"].as_array().unwrap().len(), 32);
        assert_eq!(
            result["omittedIssueCount"],
            full.issues.len() - 32 + full.omitted_issue_count
        );
        assert!(summary.to_string().len() < 4096);
        assert!(!summary.to_string().contains("private-tree-marker"));
        unchanged(temp.path(), &before);
    }

    #[test]
    fn installed_oversize_source_remains_explicitly_unavailable_and_unchanged() {
        let temp = tempfile::tempdir().unwrap();
        let (path, sha, _, _, program) = fixture(temp.path(), MAX_XML_BYTES);
        assert!(program.len() > MAX_XML_BYTES);
        let before = files(temp.path());
        let result = resolve_current(&path, &sha, ID).unwrap();
        assert_eq!(result["result"]["status"], "unavailable");
        assert_eq!(result["result"]["steps"], 0);
        assert_eq!(result["result"]["executable"], false);
        assert_eq!(
            result["result"]["issueCodes"],
            serde_json::json!(["source-byte-limit"])
        );
        unchanged(temp.path(), &before);
    }

    #[test]
    fn sidecars_are_refused_without_opening_or_changing_them() {
        for suffix in ["-wal", "-shm", "-journal"] {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("catalogue.sqlite");
            open_and_migrate(&path).unwrap().close().unwrap();
            fs::write(
                temp.path().join(format!("catalogue.sqlite{suffix}")),
                b"sidecar-marker",
            )
            .unwrap();
            let before = files(temp.path());
            assert!(resolve_current(&path, DIGEST, ID).is_err());
            unchanged(temp.path(), &before);
            let new = temp.path().join("fresh.sqlite");
            fs::write(
                temp.path().join(format!("fresh.sqlite{suffix}")),
                b"orphan-marker",
            )
            .unwrap();
            let before = files(temp.path());
            assert!(init_current(&new).is_err());
            unchanged(temp.path(), &before);
        }
    }

    #[test]
    fn wal_header_is_refused_before_readonly_sqlite_can_create_sidecars() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("catalogue.sqlite");
        let conn = open_and_migrate(&path).unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        conn.close().unwrap();
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
        let before = files(temp.path());
        assert!(resolve_current(&path, DIGEST, ID).is_err());
        unchanged(temp.path(), &before);
    }
}
