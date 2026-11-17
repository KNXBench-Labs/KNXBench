//! Manual synchronization, last-good cache publication and offline restart regressions.

use std::io::Write;

use flate2::{write::GzEncoder, Compression};
use serde_json::json;

use super::super::index::fixtures::{document, entry};
use super::super::service::SyncOutcome;
use super::super::store;
use super::super::test_server::Script;
use super::support::*;

#[tokio::test]
async fn opening_status_and_cold_search_never_fetch_an_index() {
    let f = Fixture::new(vec![one_index()]).await;
    for _ in 0..3 {
        assert!(f.source.status().index.is_none());
        assert_eq!(
            f.source
                .search("dimmer", None, 0, 25)
                .await
                .err()
                .unwrap()
                .kind(),
            "noIndex"
        );
    }
    let reopened = f.reopen();
    assert!(reopened.source("reko").unwrap().status().index.is_none());
    assert!(f.server.requests().is_empty());
    assert_eq!(
        f.source.sync().await.unwrap(),
        SyncOutcome::Updated { entry_count: 1 }
    );
    assert_eq!(f.server.requests().len(), 1);
}

#[tokio::test]
async fn gzip_etag_and_304_preserve_document_but_update_only_the_check_time() {
    let bytes = document(vec![file_entry("sample", "knxprod", CONTENT)]);
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&bytes).unwrap();
    let gzip = encoder.finish().unwrap();
    let etag = "W/\"fictional=opaque/one\"";
    let f = Fixture::new(vec![
        Script::reply(200, &[("content-encoding", "gzip"), ("etag", etag)], &gzip),
        Script::reply(304, &[], b""),
    ])
    .await;
    f.source.sync().await.unwrap();
    let mut meta = store::peek_meta(&f.source_dir()).unwrap();
    let old = "2020-01-01T00:00:00Z";
    // Seed an old successful check, then reopen: no sleep/wall-clock race is required.
    meta.fetched_at = old.into();
    meta.last_checked_at = old.into();
    store::save_meta(&f.source_dir(), &meta).unwrap();
    let reopened = f.reopen();
    let source = reopened.source("reko").unwrap();
    assert_eq!(source.sync().await.unwrap(), SyncOutcome::NotModified);
    let now = source.status().index.unwrap();
    assert_eq!(now.fetched_at, old);
    assert_ne!(now.last_checked_at, old);
    assert_eq!(now.generated_at, "2026-01-01T00:00:00+00:00");
    let current = store::peek_meta(&f.source_dir()).unwrap();
    assert_eq!(current.etag.as_deref(), Some(etag));
    assert_eq!(current.data_file, meta.data_file);
    assert_eq!(
        std::fs::read(f.source_dir().join(current.data_file)).unwrap(),
        bytes
    );
    let seen = f.server.requests();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].path, "/api/catalog/index.json");
    assert_eq!(seen[0].headers["accept-encoding"], "gzip");
    assert!(!seen[0].headers.contains_key("if-none-match"));
    assert_eq!(seen[1].headers["if-none-match"], etag);
    assert!(seen
        .iter()
        .all(|r| !r.headers.contains_key("authorization")));
}

#[tokio::test]
async fn failures_leave_last_good_search_bytes_validator_and_timestamps_unchanged() {
    let mut unknown_schema: serde_json::Value = serde_json::from_slice(&document(vec![])).unwrap();
    unknown_schema["schema"] = json!(77);
    let failures = vec![
        (Script::reply(200, &[], b"{"), "integrity"),
        (Script::json(200, unknown_schema), "integrity"),
        (
            Script::reply(200, &[("content-encoding", "br")], b"not brotli"),
            "integrity",
        ),
        (
            Script::reply(200, &[("content-encoding", "gzip")], b"bad gzip"),
            "integrity",
        ),
        (Script::truncated(b"{", 100), "transport"),
        // Content-Length is refused before allocation or waiting for a gigantic body.
        (Script::truncated(b"", 64 * 1024 * 1024 + 1), "transport"),
        (Script::reply(403, &[], b""), "forbidden"),
        (Script::reply(404, &[], b""), "notFound"),
        (Script::reply(503, &[], b""), "serviceUnavailable"),
    ];
    let expected: Vec<_> = failures.iter().map(|(_, kind)| *kind).collect();
    let mut script = vec![one_index()];
    script.extend(failures.into_iter().map(|(script, _)| script));
    let f = Fixture::new(script).await;
    f.source.sync().await.unwrap();
    let meta = store::peek_meta(&f.source_dir()).unwrap();
    let bytes = std::fs::read(f.source_dir().join(&meta.data_file)).unwrap();
    for (index, kind) in expected.into_iter().enumerate() {
        assert_eq!(
            f.source.sync().await.unwrap_err().kind(),
            kind,
            "failure row {index}"
        );
        assert_eq!(
            store::peek_meta(&f.source_dir()).unwrap(),
            meta,
            "failed sync changed metadata"
        );
        assert_eq!(
            std::fs::read(f.source_dir().join(&meta.data_file)).unwrap(),
            bytes
        );
        let hits = f
            .source
            .search("fictional sample", None, 0, 1)
            .await
            .unwrap();
        assert_eq!(hits.total, 1);
        assert_eq!(hits.entries[0].0.id, "sample");
        assert_eq!(
            f.server.requests().len(),
            index + 2,
            "a failure must not retry"
        );
    }
}

#[tokio::test]
async fn valid_cache_reopens_offline_and_local_search_does_not_revalidate() {
    let f = Fixture::new(vec![one_index(), Script::reply(503, &[], b"offline")]).await;
    f.source.sync().await.unwrap();
    let original = f.source.status().index;
    let reopened = f.reopen();
    let source = reopened.source("reko").unwrap();
    assert_eq!(source.status().index, original);
    assert_eq!(source.search("sample", None, 0, 25).await.unwrap().total, 1);
    assert_eq!(source.search("sample", None, 0, 25).await.unwrap().total, 1);
    assert_eq!(f.server.requests().len(), 1);
    assert_eq!(
        source.sync().await.unwrap_err().kind(),
        "serviceUnavailable"
    );
    assert_eq!(source.status().index, original);
    assert_eq!(source.search("sample", None, 0, 25).await.unwrap().total, 1);
    assert_eq!(f.server.requests().len(), 2);
}

#[tokio::test]
async fn a_304_without_a_usable_cache_is_refused_without_publication() {
    let f = Fixture::new(vec![Script::reply(304, &[], b"")]).await;
    assert_eq!(f.source.sync().await.unwrap_err().kind(), "integrity");
    assert!(f.source.status().index.is_none());
    assert!(!f.source_dir().join(store::META_FILE).exists());
    assert_eq!(f.server.requests().len(), 1);
}

#[tokio::test]
async fn damaged_restart_cache_is_not_searchable_or_offered_as_a_validator() {
    let f = Fixture::new(vec![one_index(), Script::reply(304, &[], b"")]).await;
    f.source.sync().await.unwrap();
    let meta = store::peek_meta(&f.source_dir()).unwrap();
    std::fs::write(
        f.source_dir().join(meta.data_file),
        b"damaged fictional cache",
    )
    .unwrap();
    let reopened = f.reopen();
    let source = reopened.source("reko").unwrap();
    assert_eq!(
        source.search("", None, 0, 1).await.err().unwrap().kind(),
        "noIndex"
    );
    assert_eq!(source.status().index_problem, Some("damaged"));
    assert_eq!(f.server.requests().len(), 1);
    assert_eq!(source.sync().await.unwrap_err().kind(), "integrity");
    let seen = f.server.requests();
    assert_eq!(seen.len(), 2);
    assert!(!seen[1].headers.contains_key("if-none-match"));
}

#[tokio::test]
async fn one_sync_in_flight_refuses_another_and_keeps_reads_local() {
    let (reply, release) = index_reply(vec![
        file_entry("sample", "knxprod", CONTENT),
        file_entry("new-version", "knxprod", CONTENT),
    ])
    .gated();
    let f = Fixture::new(vec![one_index(), reply]).await;
    f.source.sync().await.unwrap();
    let source = std::sync::Arc::clone(&f.source);
    let task = tokio::spawn(async move { source.sync().await });
    requests(&f.server, 2).await;
    assert!(f.source.status().sync_running);
    assert_eq!(f.source.sync().await.unwrap_err().kind(), "busy");
    assert_eq!(f.source.search("", None, 0, 10).await.unwrap().total, 1);
    assert_eq!(
        f.source
            .set_base_url(Some("https://changed.example"))
            .await
            .unwrap_err()
            .kind(),
        "busy"
    );
    assert_eq!(
        f.source
            .set_credentials(password())
            .await
            .unwrap_err()
            .kind(),
        "busy"
    );
    assert_eq!(
        f.source.forget_credentials().await.unwrap_err().kind(),
        "busy"
    );
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    // The admitted operation owns its lock beyond the dropped HTTP/service waiter.
    assert!(f.source.status().sync_running);
    assert_eq!(f.source.sync().await.unwrap_err().kind(), "busy");
    assert_eq!(
        f.source
            .set_credentials(password())
            .await
            .unwrap_err()
            .kind(),
        "busy"
    );
    release.notify_one();
    tokio::time::timeout(WAIT, async {
        while f.source.status().sync_running {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("admitted sync must finish after its response is released");
    assert_eq!(f.source.search("", None, 0, 10).await.unwrap().total, 2);
    assert_eq!(f.server.requests().len(), 2);
}

#[tokio::test]
async fn invalid_entries_are_reported_while_remaining_entries_stay_searchable() {
    let valid = file_entry("sample", "knxprod", CONTENT);
    let mut unsafe_entry = entry("unsafe", "knxprod", "Fictional Unsafe");
    unsafe_entry["download_url"] = json!("https://outside.example/secret");
    let f = Fixture::new(vec![index_reply(vec![valid.clone(), unsafe_entry, valid])]).await;
    f.source.sync().await.unwrap();
    let status = f.source.status().index.unwrap();
    assert_eq!((status.entry_count, status.skipped_entries), (3, 2));
    assert_eq!(f.source.search("", None, 0, 25).await.unwrap().total, 1);
    assert_eq!(f.server.requests().len(), 1);
}
