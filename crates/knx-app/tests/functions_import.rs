//! ADR-0124: an imported schema-23 Function persists through the real
//! import service: modeled fields in the native tables, the whole source
//! element (including `Puid`/`Implements`/`DefaulGroupRange`) in the opaque
//! store, both unchanged after reopening.

use knx_store::{load_opaque, load_project, open_and_migrate, open_existing_read_only};

const LIGHT: &str = r#"<Function Id="P-0001-0_F-1" Name="LivingroomLight" Type="FT-1" Implements="FT-1" DefaulGroupRange="P-0001-0_GR-1" Puid="11"><GroupAddressRef Id="P-0001-0_GF-1" Name="" Role="SwitchOnOff" RefId="P-0001-0_GA-1" Puid="15"/><GroupAddressRef Id="P-0001-0_GF-2" Name="" Role="InfoOnOff" RefId="P-0001-0_GA-2" Puid="17"/></Function>"#;

#[test]
fn an_imported_function_and_its_raw_source_survive_native_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("functions.knxproj");
    std::fs::write(
        &source,
        knx_testsupport::functions_knxproj_bytes(23, "", LIGHT),
    )
    .unwrap();
    let native = dir.path().join("functions.knxdb");
    let store = open_and_migrate(&native).unwrap();
    let imported = knx_app::import_ets_project(&source, &store).unwrap();
    assert_eq!(imported.project.functions.len(), 1);
    knx_store::save_project(&store, &imported.project).unwrap();
    drop(store);

    let reopened = open_existing_read_only(&native).unwrap();
    assert_eq!(load_project(&reopened.conn).unwrap(), imported.project);
    let raw: Vec<_> = load_opaque(&reopened.conn)
        .unwrap()
        .into_iter()
        .filter(|e| e.xpath.ends_with("Function[@Id='P-0001-0_F-1']"))
        .collect();
    assert_eq!(raw.len(), 1);
    let text = String::from_utf8(raw[0].bytes.clone()).unwrap();
    assert_eq!(text, LIGHT, "the source element is kept byte for byte");
}
