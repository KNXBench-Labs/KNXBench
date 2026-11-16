//! Verified download-to-job handoff and preview-only preservation of the installed database.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use super::super::store::sha256_hex;
use super::super::test_server::Script;
use super::support::*;
use crate::catalogue_jobs::{dto, http, JobId};

/// Detached logical rows, including schema/history, rather than a WAL-sensitive file hash.
pub(super) fn logical_database(path: &Path) -> Vec<(String, Vec<Vec<String>>)> {
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let tables: Vec<String> = connection
        .prepare("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    tables
        .into_iter()
        .map(|table| {
            let sql = format!("SELECT * FROM \"{}\"", table.replace('"', "\"\""));
            let mut query = connection.prepare(&sql).unwrap();
            let columns = query.column_count();
            let mut rows: Vec<Vec<String>> = query
                .query_map([], |row| {
                    (0..columns)
                        .map(|c| Ok(format!("{:?}", row.get_ref(c)?)))
                        .collect()
                })
                .unwrap()
                .map(Result::unwrap)
                .collect();
            rows.sort();
            (table, rows)
        })
        .collect()
}

fn fictional_package() -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("knx_master.xml", br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Fictional Example"/></Manufacturers></MasterData></KNX>"#.as_slice()),
        ("M-0001/Hardware.xml", br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Fictional Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#.as_slice()),
        ("M-0001/M-0001_A-0001.xml", br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-0001-01-0001" Name="Fictional preview only" ApplicationNumber="1" ApplicationVersion="1" ProgramType="ApplicationProgram" MaskVersion="MV-07B0" PeiType="0" LoadProcedureStyle="MergedProcedure" DefaultLanguage="en-US"><Static/></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#.as_slice()),
    ] {
        archive.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        archive.write_all(bytes).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

async fn job_request(
    router: &axum::Router,
    method: &str,
    uri: &str,
    incarnation: Option<&str>,
    serial: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("x-knx-catalogue-capability", "01".repeat(32));
    if let Some(incarnation) = incarnation {
        request = request
            .header("x-knx-catalogue-incarnation", incarnation)
            .header("x-knx-catalogue-serial", serial);
    }
    let body = if let Some(body) = body {
        request = request.header("content-type", "application/json");
        Body::from(serde_json::to_vec(&body).unwrap())
    } else {
        Body::empty()
    };
    let response = tokio::time::timeout(WAIT, router.clone().oneshot(request.body(body).unwrap()))
        .await
        .unwrap()
        .unwrap();
    let code = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (code, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn downloaded_file_reaches_the_existing_jobs_ready_preview_without_applying_or_mutating_db() {
    let package = fictional_package();
    let f = Fixture::new(vec![
        index_reply(vec![file_entry("fictional-package", "knxprod", &package)]),
        Script::reply(200, &[], &package),
    ])
    .await;
    let state = f.state(true);
    let router = crate::app(state.clone(), None);
    let database = state.data_dir.join("catalogue.sqlite");
    let before = logical_database(&database);
    f.source.sync().await.unwrap();
    f.token().await;
    let selected = sha256_hex(&package);
    let (code, _) = request(
        &router,
        "POST",
        &format!("{PREFIX}/entries/fictional-package/download?sha256={selected}"),
        None,
    )
    .await;
    assert_eq!(code, StatusCode::ACCEPTED);
    assert_eq!(settled(&f.source).await.state, "done");
    assert_eq!(
        f.source.read_verified("fictional-package").await.unwrap().0,
        package
    );
    assert_eq!(
        logical_database(&database),
        before,
        "acquisition must not install"
    );
    let (code, admission) =
        request(&router, "GET", &format!("{}/admission", http::PREFIX), None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(admission["admission"], "open");
    let incarnation = admission["serverIncarnation"].as_str().unwrap();
    let serial = admission["nextAdmissionSerial"].as_str().unwrap();
    let create = json!({
        "schemaVersion":1, "policyVersion":dto::POLICY_VERSION,
        "serverIncarnation":incarnation, "admissionSerial":serial,
        "sourceName":"fictional-package.knxprod", "declaredBytes":package.len().to_string(),
        "profile":"standard", "allowLargePackage":false
    });
    let (code, created) =
        job_request(&router, "POST", http::PREFIX, None, serial, Some(create)).await;
    assert_eq!(code, StatusCode::CREATED, "{created}");
    assert_eq!(created["phase"], "receiving");
    let id = created["handle"]["jobId"].as_str().unwrap();
    let job = format!("{}/{id}", http::PREFIX);
    let (code, feed) = job_request(
        &router,
        "POST",
        &format!("{PREFIX}/jobs/{id}/feed/fictional-package?sha256={selected}"),
        Some(incarnation),
        serial,
        None,
    )
    .await;
    assert_eq!(code, StatusCode::ACCEPTED, "{feed}");
    let ready = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let (code, status) =
                job_request(&router, "GET", &job, Some(incarnation), serial, None).await;
            assert_eq!(code, StatusCode::OK, "{status}");
            let typed: dto::Status = serde_json::from_value(status.clone()).unwrap();
            typed.validate().unwrap();
            assert_eq!(
                typed.state,
                dto::JobState::Running,
                "preview refused: {status}"
            );
            if typed.phase == Some(dto::Phase::Ready) {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the real managed preview must settle ready");
    assert_eq!(ready["preview"]["programs"], "1");
    assert_eq!(ready["commit"]["outcome"], "pending");
    assert!(ready["result"].is_null());
    assert_eq!(ready["source"]["sha256"], selected);
    assert_eq!(
        logical_database(&database),
        before,
        "preview is not apply consent"
    );
    assert!(state.project.lock().unwrap().is_none());
    assert_eq!(
        f.server.requests().len(),
        2,
        "feeding the job reuses verified bytes"
    );
    // Stop the preview-only worker through its ordinary job contract before TempDir teardown.
    let cancel = json!({"schemaVersion":1, "commandId":JobId::from_bytes([9;16]).to_string()});
    let (code, cancelled) = job_request(
        &router,
        "POST",
        &format!("{job}/cancel"),
        Some(incarnation),
        serial,
        Some(cancel),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{cancelled}");
    tokio::time::timeout(WAIT, async {
        loop {
            let (_, status) =
                job_request(&router, "GET", &job, Some(incarnation), serial, None).await;
            if status["state"] != "running" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(logical_database(&database), before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn installed_catalogue_remains_readable_during_network_acquisition_and_after_cancel() {
    let f = Fixture::new(vec![
        one_index(),
        Script::holding_body(&CONTENT[..8], CONTENT.len()),
    ])
    .await;
    let state = f.state(true);
    let router = crate::app(state.clone(), None);
    let database = state.data_dir.join("catalogue.sqlite");
    let before = logical_database(&database);
    f.source.sync().await.unwrap();
    f.token().await;
    f.source
        .start_download("sample", &sha256_hex(CONTENT))
        .await
        .unwrap();
    requests(&f.server, 2).await;
    let (code, response) = request(&router, "GET", "/api/catalog/manufacturers", None).await;
    assert_eq!(
        code,
        StatusCode::OK,
        "local catalogue read must not wait for HTTPS: {response}"
    );
    assert_eq!(logical_database(&database), before);
    f.source.cancel_download();
    assert_eq!(settled(&f.source).await.error.unwrap().kind, "cancelled");
    assert_eq!(logical_database(&database), before);
    f.assert_no_partial();
}
