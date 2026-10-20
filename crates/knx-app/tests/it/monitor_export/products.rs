//! Read-only product metadata and current-project precedence tests.
use super::*;

#[test]
fn read_only_catalogue_metadata_fills_gaps_without_overwriting_current_values() {
    let p = fixture();
    let before = p.clone();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    conn.execute_batch("INSERT INTO manufacturer(id,name) VALUES('M-1','Synthetic maker');
        INSERT INTO hardware(id,manufacturer_id,name,source_sha256) VALUES('hardware','M-1','Hardware label','test');
        INSERT INTO product(id,manufacturer_id,hardware_id,text,order_number,source_sha256) VALUES('product','M-1','hardware','Product label','ORDER-1','test');
        INSERT INTO hardware2program(id,manufacturer_id,hardware_id,application_program_ref,source_sha256) VALUES('program','M-1','hardware','app','test');
        INSERT INTO application_program(id,manufacturer_id,name,source_sha256) VALUES('app','M-1','Application','test');
        INSERT INTO catalog_item(id,manufacturer_id,section_id,name,product_ref_id,hardware2program_ref_id,source_sha256) VALUES('item','M-1','section','Catalogue display','product','program','test');").unwrap();
    drop(conn);
    let bytes = std::fs::read(&path).unwrap();
    let db = knx_productdb::open_read_only(&path).unwrap();
    let b = build(&p, Some(&db.conn), &ExportOptions::default()).unwrap();
    let (v, r, _) = unpack(&b.bytes);
    assert_eq!(v["devices"]["1.1.9"]["name"], "Current device");
    assert_eq!(v["devices"]["1.1.9"]["hardware_name"], "Catalogue display");
    assert_eq!(v["devices"]["1.1.9"]["order_number"], "ORDER-1");
    assert_eq!(
        v["devices"]["1.1.9"]["manufacturer_name"],
        "Synthetic maker"
    );
    assert_eq!(v["devices"]["1.1.9"]["application"], "app");
    assert_eq!(
        v["communication_objects"]["1.1.9/O-1"]["device_application"],
        "app"
    );
    assert_eq!(
        v["communication_objects"]["1.1.9/O-1"]["text"],
        "Current {name}"
    );
    assert_eq!(p, before);
    assert!(r.issues.iter().any(|i| i.code == "catalogue_metadata"));
    drop(db);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}
