//! Shared authored catalogue fixtures for App and mounted-server caption settlement regressions.
use super::fixture as original;
use knx_app::system2_validation_preparation::{
    describe_project_source_closure, ProjectProductSelector, ProjectValidationProfile,
};
use knx_core::{DeviceId, Project};
use std::{
    io::{Cursor, Read, Write},
    path::Path,
};

pub const DEVICE: DeviceId = DeviceId(1);
pub const PROGRAM: &str = original::PROGRAM;
pub const MEMBER: &str = "M-0001/Application.xml";
pub struct Inputs {
    pub dir: tempfile::TempDir,
    pub conn: knx_productdb::Connection,
    pub project: Project,
    pub profile: Vec<u8>,
    pub package: Vec<u8>,
}
pub fn members(package: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(package)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let name = file.name().unwrap().into_owned();
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            (name, bytes)
        })
        .collect()
}
pub fn package(members: &[(String, Vec<u8>)], comment: &str) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer.set_comment(comment).unwrap();
    for (name, data) in members {
        if name.ends_with('/') {
            assert!(data.is_empty());
            writer
                .add_directory(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            continue;
        }
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
pub fn changed(package_bytes: &[u8], edit: impl FnOnce(String) -> String) -> Vec<u8> {
    let mut entries = members(package_bytes);
    let (_, bytes) = entries.iter_mut().find(|(name, _)| name == MEMBER).unwrap();
    *bytes = edit(String::from_utf8(bytes.clone()).unwrap()).into_bytes();
    package(&entries, "")
}
pub fn caption(package: &[u8]) -> Vec<u8> {
    changed(package, |s| {
        s.replacen("Name=\"Synthetic\"", "Name=\"Revised caption\"", 1)
    })
}
pub fn replacement(path: &Path, current: &[u8], old: &[u8], ambiguous: bool) {
    let conn = knx_productdb::open_and_migrate(path).unwrap();
    knx_productdb::install_package(&conn, "current.knxprod", current).unwrap();
    knx_productdb::install_package(&conn, "old-retained.knxprod", old).unwrap();
    if ambiguous {
        let other = package(&members(current), "second actual container");
        assert_ne!(
            knx_productdb::sha256_hex(current),
            knx_productdb::sha256_hex(&other)
        );
        knx_productdb::install_package(&conn, "second.knxprod", &other).unwrap();
    }
}
pub fn inputs(namespace: u8, retained: bool) -> Inputs {
    let (dir, original, project, profile) = original::configured_inputs();
    let mut profile = ProjectValidationProfile::from_json(&profile).unwrap();
    let ProjectProductSelector::StandalonePackage { package_sha256 } = &profile.product_input
    else {
        panic!("fixture family")
    };
    let old: Vec<u8> = original
        .query_row(
            "SELECT bytes FROM package WHERE sha256=?1",
            [package_sha256],
            |r| r.get(0),
        )
        .unwrap();
    let mut entries = members(&old)
        .into_iter()
        .map(|(path, bytes)| {
            (
                path,
                String::from_utf8(bytes)
                    .unwrap()
                    .replace("project/20", &format!("project/{namespace}"))
                    .into_bytes(),
            )
        })
        .collect::<Vec<_>>();
    entries.push(("empty/".into(), Vec::new()));
    let package = package(&entries, "");
    drop(original);
    let conn =
        knx_productdb::open_and_migrate(&dir.path().join("caption-products.sqlite")).unwrap();
    if retained {
        // Two independently installed originals: no package rows or invented common archive.
        for (path, bytes) in &entries {
            if path.ends_with("Hardware.xml") || path.ends_with("Application.xml") {
                assert!(matches!(
                    knx_productdb::ingest_file(&conn, path, bytes).unwrap(),
                    knx_productdb::IngestOutcome::Ingested { .. }
                ));
            }
        }
        let packages: i64 = conn
            .query_row("SELECT COUNT(*) FROM package", [], |r| r.get(0))
            .unwrap();
        assert_eq!(packages, 0);
    } else {
        knx_productdb::install_package(&conn, "original.knxprod", &package).unwrap();
    }
    profile.schema_namespace = format!("http://knx.org/xml/project/{namespace}");
    profile.source_sha256 =
        knx_productdb::sha256_hex(&entries.iter().find(|(path, _)| path == MEMBER).unwrap().1);
    profile.product_input = if retained {
        ProjectProductSelector::RetainedSourceMembers {
            closure_sha256: describe_project_source_closure(
                &conn,
                &project,
                DEVICE,
                &profile.schema_namespace,
            )
            .unwrap()
            .sha256()
            .unwrap(),
        }
    } else {
        ProjectProductSelector::StandalonePackage {
            package_sha256: knx_productdb::sha256_hex(&package),
        }
    };
    Inputs {
        dir,
        conn,
        project,
        profile: serde_json::to_vec(&profile).unwrap(),
        package,
    }
}
impl Inputs {
    pub fn replace(self, current: &[u8], ambiguous: bool) -> Self {
        let path = self.dir.path().join("caption-replacement.sqlite");
        replacement(&path, current, &self.package, ambiguous);
        drop(self.conn);
        let live = self.dir.path().join("caption-products.sqlite");
        knx_productdb::restore_product_db_backup(&live, &path).unwrap();
        let conn = knx_productdb::open_and_migrate(&live).unwrap();
        Self { conn, ..self }
    }
}
