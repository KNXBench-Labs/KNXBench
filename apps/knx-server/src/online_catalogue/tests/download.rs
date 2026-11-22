//! Credentialed acquisition, retry budgets, integrity, cancellation and cache reuse regressions.

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use super::super::store::sha256_hex;
use super::super::test_server::Script;
use super::support::*;

async fn start(f: &Fixture) {
    f.source
        .start_download("sample", &sha256_hex(CONTENT))
        .await
        .unwrap();
}

fn failure(status: super::super::download::DownloadStatus, kind: &str) {
    assert_eq!(status.state, "failed", "{status:?}");
    assert_eq!(status.error.unwrap().kind, kind);
}

#[tokio::test]
async fn download_requires_an_account_but_sync_and_search_do_not() {
    let f = Fixture::new(vec![one_index()]).await;
    f.source.sync().await.unwrap();
    assert_eq!(f.source.search("", None, 0, 1).await.unwrap().total, 1);
    assert_eq!(
        f.source
            .start_download("sample", &sha256_hex(CONTENT))
            .await
            .unwrap_err()
            .kind(),
        "needsLogin"
    );
    assert_eq!(
        f.source
            .start_download("unknown", &sha256_hex(CONTENT))
            .await
            .unwrap_err()
            .kind(),
        "unknownEntry"
    );
    assert_eq!(f.server.requests().len(), 1);
}

#[tokio::test]
async fn selected_digest_must_still_match_before_a_download_starts() {
    let f = Fixture::new(vec![one_index()]).await;
    f.source.sync().await.unwrap();
    f.token().await;
    let wrong = sha256_hex(b"previous snapshot's fictional bytes");
    assert_eq!(
        f.source
            .start_download("sample", &wrong)
            .await
            .unwrap_err()
            .kind(),
        "selectionChanged"
    );
    assert_eq!(f.source.download_status().state, "idle");
    assert_eq!(f.server.requests().len(), 1);
    assert!(!f.cache_file("sample").exists());
}

#[tokio::test]
async fn verified_download_preserves_relative_query_and_reuses_cache_without_credentials() {
    let f = Fixture::new(vec![one_index(), Script::reply(200, &[], CONTENT)]).await;
    f.source.sync().await.unwrap();
    f.token().await;
    start(&f).await;
    let status = settled(&f.source).await;
    assert_eq!(status.state, "done", "{status:?}");
    assert_eq!(status.received_bytes, CONTENT.len() as u64);
    assert_eq!(std::fs::read(f.cache_file("sample")).unwrap(), CONTENT);
    let seen = f.server.requests();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[1].method, "GET");
    assert_eq!(
        seen[1].path,
        "/api/download?path=Hersteller%2F0042_Contoso%2Fsample.knxprod"
    );
    assert_eq!(seen[1].headers["authorization"], format!("Bearer {TOKEN}"));
    f.source.forget_credentials().await.unwrap();
    let cached = f
        .source
        .start_download("sample", &sha256_hex(CONTENT))
        .await
        .unwrap();
    assert_eq!(cached.state, "done");
    let reopened = f.reopen();
    let source = reopened.source("reko").unwrap();
    assert_eq!(
        source
            .start_download("sample", &sha256_hex(CONTENT))
            .await
            .unwrap()
            .state,
        "done"
    );
    assert_eq!(source.read_verified("sample").await.unwrap().0, CONTENT);
    assert!(source.search("", None, 0, 1).await.unwrap().entries[0].1);
    assert_eq!(f.server.requests().len(), 2);
    f.assert_no_partial();
}

#[tokio::test]
async fn same_size_corrupt_cached_bytes_are_rejected_then_reacquired_and_reverified() {
    let f = Fixture::new(vec![one_index(), Script::reply(200, &[], CONTENT)]).await;
    f.source.sync().await.unwrap();
    f.token().await;
    start(&f).await;
    assert_eq!(settled(&f.source).await.state, "done");
    std::fs::write(f.cache_file("sample"), vec![b'!'; CONTENT.len()]).unwrap();
    assert_eq!(
        f.source.read_verified("sample").await.err().unwrap().kind(),
        "integrity"
    );
    start(&f).await;
    assert_eq!(settled(&f.source).await.state, "done");
    assert_eq!(f.source.read_verified("sample").await.unwrap().0, CONTENT);
    assert_eq!(f.server.requests().len(), 3);
    f.assert_no_partial();
}

async fn downloaded(f: &Fixture) -> bool {
    f.source.search("", None, 0, 1).await.unwrap().entries[0].1
}

async fn cached_fixture() -> Fixture {
    cached_fixture_with(Vec::new()).await
}

async fn cached_fixture_with(later: Vec<Script>) -> Fixture {
    let mut script = vec![one_index(), Script::reply(200, &[], CONTENT)];
    script.extend(later);
    let f = Fixture::new(script).await;
    f.source.sync().await.unwrap();
    f.token().await;
    start(&f).await;
    assert_eq!(settled(&f.source).await.state, "done");
    f
}

#[tokio::test]
async fn fresh_download_retains_digest_metadata_and_reports_downloaded() {
    let f = cached_fixture().await;
    assert_eq!(
        std::fs::read_to_string(f.digest_file("sample")).unwrap(),
        format!("{}\n", sha256_hex(CONTENT))
    );
    assert!(downloaded(&f).await);
    f.assert_no_partial();
}

#[tokio::test]
async fn same_id_and_size_with_a_new_digest_is_not_downloaded_until_reacquired() {
    let other = vec![b'?'; CONTENT.len()];
    let f = cached_fixture_with(vec![
        index_reply(vec![file_entry("sample", "knxprod", &other)]),
        Script::reply(200, &[], &other),
    ])
    .await;
    f.source.sync().await.unwrap();
    assert!(!downloaded(&f).await);
    f.source
        .start_download("sample", &sha256_hex(&other))
        .await
        .unwrap();
    assert_eq!(settled(&f.source).await.state, "done");
    assert!(downloaded(&f).await);
    assert_eq!(f.source.read_verified("sample").await.unwrap().0, other);
    assert_eq!(
        std::fs::read_to_string(f.digest_file("sample")).unwrap(),
        format!("{}\n", sha256_hex(&other))
    );
}

#[tokio::test]
async fn legacy_file_without_metadata_is_not_downloaded_until_verified_without_network() {
    let f = cached_fixture().await;
    std::fs::remove_file(f.digest_file("sample")).unwrap();
    assert!(!downloaded(&f).await);
    let before = f.server.requests().len();
    f.source.forget_credentials().await.unwrap();
    let status = f
        .source
        .start_download("sample", &sha256_hex(CONTENT))
        .await
        .unwrap();
    assert_eq!(status.state, "done");
    assert_eq!(f.server.requests().len(), before);
    assert!(f.digest_file("sample").is_file());
    assert!(downloaded(&f).await);
}

#[tokio::test]
async fn legacy_file_with_wrong_bytes_is_removed_by_verification() {
    let f = cached_fixture().await;
    std::fs::remove_file(f.digest_file("sample")).unwrap();
    std::fs::write(f.cache_file("sample"), vec![b'!'; CONTENT.len()]).unwrap();
    assert!(!downloaded(&f).await);
    f.source.forget_credentials().await.unwrap();
    assert_eq!(
        f.source
            .start_download("sample", &sha256_hex(CONTENT))
            .await
            .unwrap_err()
            .kind(),
        "needsLogin"
    );
    assert!(!f.cache_file("sample").exists());
    assert!(!f.digest_file("sample").exists());
}

#[tokio::test]
async fn tampered_or_linked_digest_metadata_is_not_downloaded() {
    use std::os::unix::fs::symlink;
    let f = cached_fixture().await;
    let meta = f.digest_file("sample");
    let good = std::fs::read(&meta).unwrap();
    for bad in [
        format!("{}\n", sha256_hex(b"other")).into_bytes(),
        good[..good.len() - 1].to_vec(),
        [good.clone(), b"x".to_vec()].concat(),
        Vec::new(),
    ] {
        std::fs::write(&meta, bad).unwrap();
        assert!(!downloaded(&f).await);
    }
    std::fs::remove_file(&meta).unwrap();
    let target = f.temp.path().join("elsewhere");
    std::fs::write(&target, &good).unwrap();
    symlink(&target, &meta).unwrap();
    assert!(!downloaded(&f).await);
    std::fs::remove_file(&meta).unwrap();
    std::fs::hard_link(&target, &meta).unwrap();
    assert!(!downloaded(&f).await);
    std::fs::remove_file(&meta).unwrap();
    std::fs::write(&meta, &good).unwrap();
    assert!(downloaded(&f).await);
}

#[tokio::test]
async fn failed_verification_removes_file_and_digest_metadata() {
    let f = cached_fixture().await;
    std::fs::write(f.cache_file("sample"), vec![b'!'; CONTENT.len()]).unwrap();
    assert!(
        downloaded(&f).await,
        "metadata-only search cannot see same-size tampering"
    );
    f.source.forget_credentials().await.unwrap();
    assert_eq!(
        f.source
            .start_download("sample", &sha256_hex(CONTENT))
            .await
            .unwrap_err()
            .kind(),
        "needsLogin"
    );
    assert!(!f.cache_file("sample").exists());
    assert!(!f.digest_file("sample").exists());
}

#[tokio::test]
async fn hash_size_truncation_and_encoding_failures_never_publish_a_cached_file() {
    let replies = vec![
        (
            Script::reply(200, &[], &vec![b'!'; CONTENT.len()]),
            "integrity",
        ),
        (Script::reply(200, &[], b"too short"), "integrity"),
        (Script::truncated(b"short", CONTENT.len()), "transport"),
        (
            Script::reply(200, &[("content-encoding", "gzip")], CONTENT),
            "integrity",
        ),
    ];
    for (reply, kind) in replies {
        let f = Fixture::new(vec![one_index(), reply]).await;
        f.source.sync().await.unwrap();
        f.token().await;
        start(&f).await;
        failure(settled(&f.source).await, kind);
        assert!(!f.cache_file("sample").exists());
        assert_eq!(
            f.source.read_verified("sample").await.err().unwrap().kind(),
            "notFound"
        );
        assert!(!f.source.search("", None, 0, 1).await.unwrap().entries[0].1);
        assert_eq!(f.server.requests().len(), 2);
        f.assert_no_partial();
    }
}

#[tokio::test]
async fn login_401_is_attempted_exactly_once_without_a_download() {
    let f = Fixture::new(vec![
        one_index(),
        Script::json(401, json!({"code":"BAD_PASSWORD", "message": PASSWORD})),
    ])
    .await;
    f.source.sync().await.unwrap();
    f.password().await;
    start(&f).await;
    let status = settled(&f.source).await;
    assert_redacted(&serde_json::to_value(&status).unwrap());
    failure(status, "invalidCredentials");
    let seen = f.server.requests();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[1].path, "/api/auth/token");
    assert_eq!(seen[1].method, "POST");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&seen[1].body).unwrap(),
        json!({"user":"fictional-user", "password":PASSWORD})
    );
    assert!(!seen[1].headers.contains_key("authorization"));
    f.assert_no_partial();
}

#[tokio::test]
async fn invalid_token_allows_one_password_renewal_and_one_repeat_only() {
    let f = Fixture::new(vec![
        one_index(),
        login("fictional-login-token-one"),
        invalid_token(),
        login("fictional-login-token-two"),
        invalid_token(),
    ])
    .await;
    f.source.sync().await.unwrap();
    f.password().await;
    start(&f).await;
    let status = settled(&f.source).await;
    assert_redacted(&serde_json::to_value(&status).unwrap());
    failure(status, "tokenRejected");
    let seen = f.server.requests();
    assert_eq!(seen.len(), 5);
    assert_eq!(
        seen.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
        vec![
            "/api/catalog/index.json",
            "/api/auth/token",
            "/api/download?path=Hersteller%2F0042_Contoso%2Fsample.knxprod",
            "/api/auth/token",
            "/api/download?path=Hersteller%2F0042_Contoso%2Fsample.knxprod",
        ]
    );
    assert_eq!(
        seen[2].headers["authorization"],
        "Bearer fictional-login-token-one"
    );
    assert_eq!(
        seen[4].headers["authorization"],
        "Bearer fictional-login-token-two"
    );
    assert!(!f.cache_file("sample").exists());
}

#[tokio::test]
async fn renewed_password_token_can_finish_the_original_download() {
    let f = Fixture::new(vec![
        one_index(),
        login("fictional-login-token-one"),
        invalid_token(),
        login("fictional-login-token-two"),
        Script::reply(200, &[], CONTENT),
    ])
    .await;
    f.source.sync().await.unwrap();
    f.password().await;
    start(&f).await;
    assert_eq!(settled(&f.source).await.state, "done");
    assert_eq!(f.source.read_verified("sample").await.unwrap().0, CONTENT);
    assert_eq!(f.server.requests().len(), 5);
}

#[tokio::test]
async fn token_only_invalid_token_and_other_401_do_not_attempt_password_login() {
    for (reply, kind) in [
        (invalid_token(), "tokenRejected"),
        (
            Script::json(401, json!({"code":"OTHER_REASON"})),
            "transport",
        ),
    ] {
        let f = Fixture::new(vec![one_index(), reply]).await;
        f.source.sync().await.unwrap();
        f.token().await;
        start(&f).await;
        failure(settled(&f.source).await, kind);
        let seen = f.server.requests();
        assert_eq!(seen.len(), 2);
        assert!(seen.iter().all(|r| r.path != "/api/auth/token"));
        f.assert_no_partial();
    }
}

#[tokio::test]
async fn forbidden_missing_unavailable_and_redirect_are_terminal_without_retries() {
    for (code, kind) in [
        (403, "forbidden"),
        (404, "notFound"),
        (503, "serviceUnavailable"),
        (302, "transport"),
    ] {
        let f = Fixture::new(vec![
            one_index(),
            Script::reply(
                code,
                &[("location", "https://outside.example/secret")],
                PASSWORD.as_bytes(),
            ),
        ])
        .await;
        f.source.sync().await.unwrap();
        f.token().await;
        start(&f).await;
        let status = settled(&f.source).await;
        assert_redacted(&serde_json::to_value(&status).unwrap());
        failure(status, kind);
        assert_eq!(f.server.requests().len(), 2);
        assert!(!f.cache_file("sample").exists());
        f.assert_no_partial();
    }
}

#[tokio::test]
async fn rate_limit_blocks_new_network_actions_until_retry_after_without_auto_restart() {
    let f = Fixture::new(vec![
        one_index(),
        Script::reply(429, &[("retry-after", "120")], b"{}"),
        Script::reply(200, &[], CONTENT),
    ])
    .await;
    f.source.sync().await.unwrap();
    f.token().await;
    start(&f).await;
    failure(settled(&f.source).await, "rateLimited");
    let limit = f.source.status().limit.unwrap();
    assert_eq!(limit.kind, "rateLimited");
    assert!((1..=121).contains(&limit.retry_after_secs));
    assert_eq!(
        f.source
            .start_download("sample", &sha256_hex(CONTENT))
            .await
            .unwrap_err()
            .kind(),
        "rateLimited"
    );
    assert_eq!(f.source.sync().await.unwrap_err().kind(), "rateLimited");
    assert_eq!(f.server.requests().len(), 2);
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(121)).await;
    assert!(f.source.status().limit.is_none());
    tokio::task::yield_now().await;
    assert_eq!(
        f.server.requests().len(),
        2,
        "expiry must never schedule a download"
    );
    tokio::time::resume();
    start(&f).await;
    assert_eq!(settled(&f.source).await.state, "done");
    assert_eq!(f.server.requests().len(), 3);
}

#[tokio::test]
async fn daily_limit_preserves_server_reset_at_and_does_not_block_manual_index_sync() {
    let reset_at = (chrono::Utc::now() + chrono::Duration::hours(2)).to_rfc3339();
    let canonical = chrono::DateTime::parse_from_rfc3339(&reset_at)
        .unwrap()
        .with_timezone(&chrono::Utc)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let body =
        serde_json::to_vec(&json!({"code":"DOWNLOAD_DAILY_LIMIT", "resetAt":reset_at})).unwrap();
    let f = Fixture::new(vec![
        one_index(),
        Script::reply(429, &[("retry-after", "120")], &body),
        one_index(),
    ])
    .await;
    f.source.sync().await.unwrap();
    f.token().await;
    start(&f).await;
    let status = settled(&f.source).await;
    let error = status.error.unwrap();
    assert_eq!(error.kind, "dailyLimit");
    assert_eq!(error.reset_at.as_deref(), Some(canonical.as_str()));
    let limit = f.source.status().limit.unwrap();
    assert_eq!(limit.kind, "dailyLimit");
    assert_eq!(limit.reset_at.as_deref(), Some(canonical.as_str()));
    assert_eq!(
        f.source
            .start_download("sample", &sha256_hex(CONTENT))
            .await
            .unwrap_err()
            .kind(),
        "dailyLimit"
    );
    assert_eq!(f.server.requests().len(), 2);
    f.source.sync().await.unwrap();
    assert_eq!(f.server.requests().len(), 3);
}

#[tokio::test]
async fn daily_reset_at_not_zero_retry_after_controls_the_daily_download_block() {
    let reset_at = (chrono::Utc::now() + chrono::Duration::hours(2)).to_rfc3339();
    let body =
        serde_json::to_vec(&json!({"code":"DOWNLOAD_DAILY_LIMIT", "resetAt":reset_at})).unwrap();
    let f = Fixture::new(vec![
        one_index(),
        Script::reply(429, &[("retry-after", "0")], &body),
    ])
    .await;
    f.source.sync().await.unwrap();
    f.token().await;
    start(&f).await;
    failure(settled(&f.source).await, "dailyLimit");
    assert_eq!(
        f.source.status().limit.as_ref().map(|l| l.kind),
        Some("dailyLimit")
    );
    assert_eq!(
        f.source
            .start_download("sample", &sha256_hex(CONTENT))
            .await
            .unwrap_err()
            .kind(),
        "dailyLimit"
    );
    assert_eq!(f.server.requests().len(), 2);
}

#[tokio::test]
async fn daily_block_honours_the_later_of_retry_after_and_reset_at_in_both_orders() {
    // retry-after later than resetAt
    let reset_soon = (chrono::Utc::now() + chrono::Duration::seconds(30)).to_rfc3339();
    let body = serde_json::to_vec(&json!({
        "code":"DOWNLOAD_DAILY_LIMIT",
        "resetAt": reset_soon,
    }))
    .unwrap();
    let f = Fixture::new(vec![
        one_index(),
        Script::reply(429, &[("retry-after", "7200")], &body),
    ])
    .await;
    f.source.sync().await.unwrap();
    f.token().await;
    start(&f).await;
    failure(settled(&f.source).await, "dailyLimit");
    let limit = f.source.status().limit.unwrap();
    assert!(
        limit.retry_after_secs >= 7000,
        "later retry-after wins: {limit:?}"
    );
    assert_eq!(
        f.source
            .start_download("sample", &sha256_hex(CONTENT))
            .await
            .unwrap_err()
            .kind(),
        "dailyLimit"
    );

    // resetAt later than retry-after
    let reset_late = (chrono::Utc::now() + chrono::Duration::hours(3)).to_rfc3339();
    let body = serde_json::to_vec(&json!({
        "code":"DOWNLOAD_DAILY_LIMIT",
        "resetAt": reset_late,
    }))
    .unwrap();
    let f = Fixture::new(vec![
        one_index(),
        Script::reply(429, &[("retry-after", "10")], &body),
    ])
    .await;
    f.source.sync().await.unwrap();
    f.token().await;
    start(&f).await;
    failure(settled(&f.source).await, "dailyLimit");
    let limit = f.source.status().limit.unwrap();
    assert!(
        limit.retry_after_secs >= 10000,
        "later resetAt wins: {limit:?}"
    );
}

#[tokio::test]
async fn cancellation_removes_a_real_partial_file_and_releases_download_admission() {
    let f = Fixture::new(vec![
        one_index(),
        Script::holding_body(&CONTENT[..8], CONTENT.len()),
        Script::reply(200, &[], CONTENT),
    ])
    .await;
    f.source.sync().await.unwrap();
    f.token().await;
    start(&f).await;
    requests(&f.server, 2).await;
    tokio::time::timeout(WAIT, async {
        while f.source.download_status().received_bytes < 8 {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .expect("actual body prefix must reach the partial file");
    let partial = std::fs::read_dir(f.source_dir().join("downloads"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|ext| ext == "part"))
        .expect("a real partial file must exist while the download holds");
    assert!(partial.is_file());
    assert_eq!(
        f.source
            .start_download("sample", &sha256_hex(CONTENT))
            .await
            .unwrap_err()
            .kind(),
        "busy"
    );
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
    assert_eq!(f.source.sync().await.unwrap_err().kind(), "busy");
    // Acquisition never takes the installed catalogue's lock: unrelated local search remains usable.
    assert_eq!(f.source.search("", None, 0, 25).await.unwrap().total, 1);
    f.source.cancel_download();
    failure(settled(&f.source).await, "cancelled");
    f.assert_no_partial();
    assert!(!f.cache_file("sample").exists());
    start(&f).await;
    assert_eq!(settled(&f.source).await.state, "done");
    assert_eq!(f.server.requests().len(), 3);
}

#[tokio::test]
async fn opening_a_source_removes_only_abandoned_partial_files() {
    let f = Fixture::new(vec![one_index(), Script::reply(200, &[], CONTENT)]).await;
    f.source.sync().await.unwrap();
    f.token().await;
    start(&f).await;
    assert_eq!(settled(&f.source).await.state, "done");
    let abandoned = f.source_dir().join("downloads/abandoned.part");
    std::fs::write(&abandoned, b"unfinished fictional bytes").unwrap();
    let reopened = f.reopen();
    assert!(!abandoned.exists());
    assert_eq!(
        reopened
            .source("reko")
            .unwrap()
            .read_verified("sample")
            .await
            .unwrap()
            .0,
        CONTENT
    );
    assert_eq!(f.server.requests().len(), 2);
}

#[tokio::test]
async fn concurrent_password_token_consumers_share_exactly_one_login() {
    let f = Fixture::new(vec![login("fictional-login-token-one")]).await;
    f.password().await;
    let mut workers = Vec::new();
    for _ in 0..8 {
        let source = Arc::clone(&f.source);
        workers.push(tokio::spawn(async move {
            source.token(&password(), None).await
        }));
    }
    for worker in workers {
        assert_eq!(
            tokio::time::timeout(WAIT, worker)
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            "fictional-login-token-one"
        );
    }
    let seen = f.server.requests();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].path, "/api/auth/token");
}
