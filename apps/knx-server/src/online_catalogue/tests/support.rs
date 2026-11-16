//! Shared isolated catalogue fixtures, mounted requests and bounded worker settlement.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use super::super::download::DownloadStatus;
use super::super::index::fixtures::{document, entry};
use super::super::test_server::{Script, TestServer};
use super::super::transport::HttpsClient;
use super::super::vault::Credentials;
use super::super::{OnlineCatalogue, SourceService};

pub const CONTENT: &[u8] = b"fictional catalogue payload, never real manufacturer data";
pub const PASSWORD: &str = "fictional-password-not-for-real-accounts";
pub const TOKEN: &str = "fictional-manual-bearer-secret";
pub const PREFIX: &str = "/api/online-catalogue/reko";
pub const WAIT: Duration = Duration::from_secs(10);

pub struct Fixture {
    pub temp: tempfile::TempDir,
    pub server: TestServer,
    pub catalogue: Arc<OnlineCatalogue>,
    pub source: Arc<SourceService>,
}

impl Fixture {
    pub async fn new(script: Vec<Script>) -> Self {
        Self::new_trusting(script, None).await
    }

    pub async fn new_trusting(
        script: Vec<Script>,
        extra_roots: Option<rustls::RootCertStore>,
    ) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let server = TestServer::start(script).await;
        let mut roots = server.roots();
        if let Some(extra) = extra_roots {
            roots.roots.extend(extra.roots);
        }
        let catalogue = Arc::new(OnlineCatalogue::with_client(
            temp.path().join("online-data"),
            temp.path().join("separate-config"),
            Arc::new(HttpsClient::with_roots(roots, Duration::ZERO)),
        ));
        let source = Arc::clone(catalogue.source("reko").unwrap());
        source
            .set_base_url(Some(&server.base_text()))
            .await
            .unwrap();
        Self {
            temp,
            server,
            catalogue,
            source,
        }
    }

    pub fn reopen(&self) -> Arc<OnlineCatalogue> {
        Arc::new(OnlineCatalogue::with_client(
            self.temp.path().join("online-data"),
            self.temp.path().join("separate-config"),
            Arc::new(HttpsClient::with_roots(self.server.roots(), Duration::ZERO)),
        ))
    }

    pub fn source_dir(&self) -> PathBuf {
        self.temp.path().join("online-data/reko")
    }

    pub fn cache_file(&self, id: &str) -> PathBuf {
        self.source_dir()
            .join("downloads")
            .join(format!("{id}.bin"))
    }

    pub fn key_file(&self) -> PathBuf {
        self.temp
            .path()
            .join("separate-config/online-catalogue-reko.key")
    }

    pub async fn token(&self) {
        self.source
            .set_credentials(Credentials::Token {
                token: TOKEN.into(),
            })
            .await
            .unwrap();
    }

    pub async fn password(&self) {
        self.source.set_credentials(password()).await.unwrap();
    }

    pub fn state(&self, database: bool) -> crate::SharedState {
        let data = self.temp.path().join("app-data");
        std::fs::create_dir_all(&data).unwrap();
        let mut state = if database {
            crate::AppState::with_product_db_at(data.clone(), Some(data.join("catalogue.sqlite")))
        } else {
            crate::AppState::new(data)
        };
        state.online_catalogue = Some(Arc::clone(&self.catalogue));
        Arc::new(state)
    }

    pub fn assert_no_partial(&self) {
        let dir = self.source_dir().join("downloads");
        if let Ok(entries) = std::fs::read_dir(dir) {
            assert!(entries
                .flatten()
                .all(|e| e.path().extension().is_none_or(|v| v != "part")));
        }
    }
}

pub fn password() -> Credentials {
    Credentials::Password {
        user: "fictional-user".into(),
        password: PASSWORD.into(),
    }
}

pub fn file_entry(id: &str, kind: &str, bytes: &[u8]) -> Value {
    let mut value = entry(id, kind, &format!("Fictional {id}"));
    value["size"] = json!(bytes.len());
    value["sha256"] = json!(super::super::store::sha256_hex(bytes));
    value
}

pub fn index_reply(entries: Vec<Value>) -> Script {
    Script::reply(
        200,
        &[
            ("content-type", "application/json"),
            ("etag", "W/\"fictional-v1\""),
        ],
        &document(entries),
    )
}

pub fn one_index() -> Script {
    index_reply(vec![file_entry("sample", "knxprod", CONTENT)])
}

pub fn login(token: &str) -> Script {
    Script::json(
        200,
        json!({"token": token, "tokenType": "Bearer", "expiresIn": 3600}),
    )
}

pub fn invalid_token() -> Script {
    Script::json(
        401,
        json!({"code": "INVALID_TOKEN", "message": "fictional remote text"}),
    )
}

pub async fn settled(source: &SourceService) -> DownloadStatus {
    tokio::time::timeout(WAIT, async {
        loop {
            let status = source.download_status();
            if status.state != "running" {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .expect("catalogue worker must settle")
}

pub async fn requests(server: &TestServer, count: usize) {
    tokio::time::timeout(WAIT, async {
        while server.requests().len() < count {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .expect("scripted HTTPS request not observed");
}

pub async fn request(
    router: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let (code, value, _) = request_raw(router, method, uri, body).await;
    (code, value)
}

pub async fn request_raw(
    router: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(value) => {
            builder = builder.header("content-type", "application/json");
            Body::from(serde_json::to_vec(&value).unwrap())
        }
        None => Body::empty(),
    };
    let response = tokio::time::timeout(WAIT, router.clone().oneshot(builder.body(body).unwrap()))
        .await
        .expect("mounted route must not queue indefinitely")
        .unwrap();
    let code = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"raw": String::from_utf8_lossy(&bytes)}));
    (code, value, bytes.to_vec())
}

/// Persist the exact mounted response bytes only after the caller's contract assertions pass.
/// Existing captures are never replaced, so a later run cannot silently rebind browser evidence.
pub fn capture_raw(name: &str, bytes: &[u8]) {
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    let Some(root) = std::env::var_os("KNXBENCH_REKO_CAPTURE_DIR") else {
        return;
    };
    let root = PathBuf::from(root);
    assert!(
        root.is_absolute(),
        "capture directory must be an explicit private absolute path"
    );
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&root)
        .unwrap();
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(root.join(name))
        .unwrap();
    output.write_all(bytes).unwrap();
    output.sync_all().unwrap();
}

pub fn assert_redacted(value: &Value) {
    let text = value.to_string();
    for secret in [
        PASSWORD,
        TOKEN,
        "fictional-login-token-one",
        "fictional-login-token-two",
    ] {
        assert!(!text.contains(secret), "response leaked a fictional secret");
    }
    for field in ["password", "token", "ciphertext", "nonce"] {
        assert!(
            !text.contains(&format!("\"{field}\":")),
            "response exposed secret field {field}"
        );
    }
}
