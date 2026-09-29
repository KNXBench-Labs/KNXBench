//! K3 acceptance: a download request read from a saved project, not written into a test.
//!
//! `1.1.67` (MDT push button 2-fold Plus, `A-0027-15-0BAC`) is in no
//! imported project, so KNXBench builds it itself, the way a user would:
//! new project, catalog package, device from the catalog, the option-C
//! parameter values through the parameter panel, the one link through the
//! link route, and **Save As**. The saved file is loaded back with the
//! storage crate and handed to
//! [`knx_productdb::image_request::image_request_for_device`]. The result
//! must equal the request `apps/knx-cli/tests/memory_download_simulated.rs`
//! writes by hand, and the image built from it must equal that test's image
//! octet for octet. No bus anywhere.
//!
//! `#[ignore]`d: it needs the private product corpus
//! (`KNXBENCH_PRODUCT_CORPUS` or the gitignored `OriginalData/`).

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{DeviceId, GroupAddress, IndividualAddress};
use knx_productdb::dynamic::{evaluate, load_program_trees, resolve_values};
use knx_productdb::enrich::com_object_lookup_id;
use knx_productdb::image::{build_download_image, ImageRequest, Link};
use knx_productdb::image_request::{image_request_for_device, DeviceRequestError};
use serde_json::{json, Value};
use tower::ServiceExt;

const BOUNDARY: &str = "knx-k3-project-boundary";
const PACKAGE: &str = "MDT_KP_BE_01_Push_Button_V15a.knxprod";
const PROGRAM: &str = "M-0083_A-0027-15-0BAC";
/// `Catalog.xml` of `PACKAGE`: "BE-TA55P2.01 Push button 2-fold / Plus",
/// whose `Hardware2Program` points at `PROGRAM`.
const CATALOG_ITEM: &str = "M-0083_H-39-1_HP-0027-15-0BAC_CI-BE.2DTA55P2.2E01-1";
/// Option C, as `memory_download_simulated.rs` writes it by hand. Set in
/// this order: the second and third only appear once the first is `2`.
const OPTION_C: [(&str, &str); 3] = [
    ("P-1007_R-1007", "2"),
    ("UP-5500_R-5500", "0"),
    ("UP-5501_R-5501", "1"),
];

fn corpus_root() -> PathBuf {
    std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../OriginalData/ProductDatabases")
        })
}

fn multipart(filename: &str, bytes: &[u8]) -> Request<Body> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    Request::builder()
        .method("POST")
        .uri("/api/catalog/install")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(body))
        .unwrap()
}

fn post(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

async fn call(app: &axum::Router, request: Request<Body>) -> Value {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

/// The hand-written request of `memory_download_simulated.rs`, verbatim.
fn hand_written() -> ImageRequest {
    ImageRequest {
        program_id: PROGRAM.to_string(),
        individual_address: IndividualAddress::new(1, 1, 67).expect("valid"),
        values: OPTION_C
            .into_iter()
            .map(|(short, value)| (format!("{PROGRAM}_{short}"), value.to_string()))
            .collect(),
        links: vec![Link {
            object: 0,
            group_address: GroupAddress::from_raw(0x1035), // 2/0/53
            sending: true,
        }],
        flag_overrides: Default::default(),
    }
}

/// Sets one parameter through the panel, the way the inspector does: the
/// field must be on the panel and editable, and the write goes to the id
/// the panel names.
async fn set_parameter(app: &axum::Router, device: u64, short: &str, raw: &str) {
    let panel = call(app, get(&format!("/api/device/{device}/parameters"))).await;
    let wanted = format!("{PROGRAM}_{short}");
    let field = panel["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["fields"].as_array().unwrap())
        .find(|f| f["etsId"] == wanted.as_str())
        .unwrap_or_else(|| panic!("{wanted} is not on the panel"));
    let write = field["writeEtsId"]
        .as_str()
        .unwrap_or_else(|| panic!("{wanted} is not editable"));
    call(
        app,
        post(
            &format!("/api/device/{device}/parameters"),
            json!({ "etsId": write, "raw": raw }),
        ),
    )
    .await;
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ product corpus (or KNXBENCH_PRODUCT_CORPUS); run with --ignored"]
async fn a_saved_project_yields_exactly_the_hand_written_option_c_image() {
    let root = corpus_root();
    assert!(
        root.exists(),
        "product corpus {} not present (OriginalData/ is gitignored, local-only); \
         set KNXBENCH_PRODUCT_CORPUS to a directory holding {PACKAGE}",
        root.display()
    );
    let package = knx_testsupport::find_corpus_file(&root, PACKAGE)
        .unwrap_or_else(|| panic!("{PACKAGE} not found under {}", root.display()));
    let bytes = std::fs::read(&package).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(knx_server::AppState {
        product_db: Some(std::sync::Mutex::new(
            knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap(),
        )),
        data_dir: dir.path().to_path_buf(),
        ..Default::default()
    });
    let app = knx_server::app(state.clone(), None);

    // Project, topology, the group address, the product package.
    call(&app, post("/api/project/new", json!({}))).await;
    let tree = call(
        &app,
        post("/api/areas", json!({ "name": "Area 1", "address": 1 })),
    )
    .await;
    let area = tree["installations"][0]["topology"][0]["id"].clone();
    let tree = call(
        &app,
        post(
            "/api/lines",
            json!({ "areaId": area, "name": "Line 1", "address": 1, "mediumRef": "MT-0" }),
        ),
    )
    .await;
    let line = tree["installations"][0]["topology"][0]["lines"][0]["id"].clone();
    let tree = call(
        &app,
        post(
            "/api/group-addresses",
            json!({ "name": "Button 1", "address": "2/0/53" }),
        ),
    )
    .await;
    let ga = tree["installations"][0]["group_addresses"][0]["id"].clone();
    call(&app, multipart(PACKAGE, &bytes)).await;

    // The device from the catalog, with its address.
    let created = call(
        &app,
        post(
            "/api/devices",
            json!({ "lineId": line, "catalogItemId": CATALOG_ITEM, "name": "Push button 2-fold Plus" }),
        ),
    )
    .await;
    let device = created["tree"]["installations"][0]["topology"][0]["lines"][0]["devices"][0]["id"]
        .as_u64()
        .expect("the device sits on line 1.1");
    call(
        &app,
        post(
            "/api/individual-address",
            json!({ "deviceId": device, "address": "1.1.67" }),
        ),
    )
    .await;

    // Before any parameter is set the project yields a request without
    // values, and nothing is filled in.
    {
        let project = state.project.lock().unwrap();
        let conn = state.product_db.as_ref().unwrap().lock().unwrap();
        let request =
            image_request_for_device(&conn, project.as_ref().unwrap(), DeviceId(device as u32))
                .expect("maps");
        assert!(request.values.is_empty(), "{:?}", request.values);
        assert!(request.links.is_empty());
    }

    for (short, raw) in OPTION_C {
        set_parameter(&app, device, short, raw).await;
    }

    // The link goes on the object instance number 0 the parameters
    // activate; the program declares many for number 0.
    let com_object = {
        let project = state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        let conn = state.product_db.as_ref().unwrap().lock().unwrap();
        let supplied = hand_written().values.into_iter().collect();
        let values = resolve_values(&conn, PROGRAM, &supplied).unwrap();
        let activation = evaluate(&load_program_trees(&conn, PROGRAM).unwrap(), &values);
        let active: Vec<&str> = activation
            .com_object_refs
            .iter()
            .filter(|a| a.scope.is_none())
            .map(|a| a.ref_id.as_str())
            .collect();
        let instance = project.devices.get(DeviceId(device as u32)).unwrap();
        let candidates: Vec<(u32, String)> = instance
            .com_objects
            .iter()
            .map(|id| project.devices.com_object(*id).unwrap())
            .filter(|o| o.number == 0)
            .map(|o| {
                (
                    o.id.0,
                    com_object_lookup_id(PROGRAM, &o.source.ets_id, o.module_instance.is_some()),
                )
            })
            .collect();
        assert!(candidates.len() > 1, "number 0 is declared several times");
        let active: Vec<u32> = candidates
            .iter()
            .filter(|(_, ref_id)| active.contains(&ref_id.as_str()))
            .map(|(id, _)| *id)
            .collect();
        assert_eq!(active.len(), 1, "option C activates one object 0");
        let inactive = candidates
            .iter()
            .find(|(id, _)| *id != active[0])
            .unwrap()
            .0;
        (active[0], inactive)
    };

    // A link on an inactive object 0 is refused by name, not moved.
    call(
        &app,
        post(
            "/api/group-links",
            json!({ "comObjectId": com_object.1, "gaId": ga, "direction": "Send" }),
        ),
    )
    .await;
    {
        let project = state.project.lock().unwrap();
        let conn = state.product_db.as_ref().unwrap().lock().unwrap();
        let refused =
            image_request_for_device(&conn, project.as_ref().unwrap(), DeviceId(device as u32));
        assert!(
            matches!(
                refused,
                Err(DeviceRequestError::LinkOnInactiveObject { number: 0, .. })
            ),
            "{refused:?}"
        );
    }
    let delete = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/group-links")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "comObjectId": com_object.1, "gaId": ga, "direction": "Send" })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::OK);

    call(
        &app,
        post(
            "/api/group-links",
            json!({ "comObjectId": com_object.0, "gaId": ga, "direction": "Send" }),
        ),
    )
    .await;

    // Save As, then read the file back with the storage crate alone.
    call(
        &app,
        post("/api/project/save-as", json!({ "path": "option-c.knxdb" })),
    )
    .await;
    let saved = dir.path().join("option-c.knxdb");
    let loaded = knx_store::load_project(&knx_store::open_and_migrate(&saved).unwrap())
        .expect("the saved project loads");

    let conn = state.product_db.as_ref().unwrap().lock().unwrap();
    let from_project =
        image_request_for_device(&conn, &loaded, DeviceId(device as u32)).expect("maps");
    assert_eq!(from_project, hand_written());

    let project_image = build_download_image(&conn, &from_project).expect("builds");
    let hand_image = build_download_image(&conn, &hand_written()).expect("builds");
    assert_eq!(project_image.segments.len(), hand_image.segments.len());
    let differing: usize = project_image
        .segments
        .iter()
        .zip(&hand_image.segments)
        .map(|(a, b)| {
            assert_eq!((&a.id, a.address), (&b.id, b.address));
            assert_eq!(a.octets.len(), b.octets.len());
            a.octets
                .iter()
                .zip(&b.octets)
                .filter(|(x, y)| x != y)
                .count()
        })
        .sum();
    assert_eq!(differing, 0);
    assert_eq!(project_image, hand_image);

    // Kept for the later work packages that download from a project file.
    if let Some(keep) = std::env::var_os("KNXBENCH_K3_KEEP_PROJECT") {
        std::fs::copy(&saved, keep).expect("copies the saved project");
    }
}
