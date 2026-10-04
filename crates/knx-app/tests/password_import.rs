//! Password-protected `.knxproj` through the application import service (AR08, KL-13).

use knx_app::{import_ets_project_with_password, AppError, ImportOptions};
use knx_etsproj::{ContainerError, ImportFailure, ProjectPassword};
use knx_store::{load_opaque, load_project, open_and_migrate, save_project};
use knx_testsupport::{write_zipcrypto_minimal_knxproj, ZIPCRYPTO_MINIMAL_PASSWORD};

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w == needle.as_bytes())
}

#[test]
fn a_protected_project_imports_saves_and_reopens_without_keeping_the_password() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_zipcrypto_minimal_knxproj(dir.path());
    let native_path = dir.path().join("protected.knxdb");
    let products_path = dir.path().join("products.sqlite");
    let password = ProjectPassword::new(ZIPCRYPTO_MINIMAL_PASSWORD);
    {
        let store = open_and_migrate(&native_path).unwrap();
        let products = knx_productdb::open_and_migrate(&products_path).unwrap();
        let imported = import_ets_project_with_password(
            &source,
            &store,
            ImportOptions {
                product_db: Some(&products),
            },
            Some(&password),
            &(),
        )
        .expect("the right password imports");
        assert_eq!(imported.project.info.name, "Minimal");
        save_project(&store, &imported.project).unwrap();
        assert_eq!(load_project(&store).unwrap(), imported.project);
        assert!(!load_opaque(&store).unwrap().is_empty());
    }
    // Every file the import left behind (store, product database, any
    // SQLite side file, and the protected source itself) is free of it.
    let mut scanned = 0;
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let path = entry.unwrap().path();
        let bytes = std::fs::read(&path).unwrap();
        assert!(
            !contains(&bytes, ZIPCRYPTO_MINIMAL_PASSWORD),
            "{} holds the password",
            path.display()
        );
        scanned += 1;
    }
    assert!(
        scanned >= 3,
        "store, product database and source were scanned"
    );
    assert!(std::fs::metadata(&native_path).unwrap().len() > 0);
    assert!(std::fs::metadata(&products_path).unwrap().len() > 0);
}

#[test]
fn a_wrong_password_leaves_the_store_empty() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_zipcrypto_minimal_knxproj(dir.path());
    let store = open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    let wrong = ProjectPassword::new("not-the-password");
    let result = import_ets_project_with_password(
        &source,
        &store,
        ImportOptions::default(),
        Some(&wrong),
        &(),
    );
    assert!(matches!(
        result,
        Err(AppError::Import(ImportFailure::Container(
            ContainerError::WrongPassword { .. }
        )))
    ));
    assert!(load_opaque(&store).unwrap().is_empty());
}

#[test]
fn without_a_password_the_service_reports_the_protection() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_zipcrypto_minimal_knxproj(dir.path());
    let store = open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    let result =
        import_ets_project_with_password(&source, &store, ImportOptions::default(), None, &());
    assert!(matches!(
        result,
        Err(AppError::Import(ImportFailure::Container(
            ContainerError::PasswordProtected { .. }
        )))
    ));
    assert!(load_opaque(&store).unwrap().is_empty());
}
