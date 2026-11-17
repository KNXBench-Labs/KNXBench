//! Real 429 responses remain scoped to their origin or account across explicit settings changes.

use std::time::Duration;

use serde_json::json;

use super::super::store::sha256_hex;
use super::super::test_server::{Script, TestServer};
use super::super::vault::Credentials;
use super::support::*;

fn daily() -> Script {
    Script::reply(
        429,
        &[("retry-after", "120")],
        &serde_json::to_vec(&json!({
            "code":"DOWNLOAD_DAILY_LIMIT",
            "resetAt": (chrono::Utc::now() + chrono::Duration::hours(2)).to_rfc3339(),
        }))
        .unwrap(),
    )
}

#[tokio::test]
async fn a_non_timestamp_reset_at_is_dropped_and_never_reaches_status() {
    let f = Fixture::new_trusting(
        vec![
            one_index(),
            Script::reply(
                429,
                &[("retry-after", "120")],
                &serde_json::to_vec(&json!({
                    "code":"DOWNLOAD_DAILY_LIMIT",
                    "resetAt":"provider-controlled text",
                }))
                .unwrap(),
            ),
        ],
        None,
    )
    .await;
    f.source.sync().await.unwrap();
    f.token().await;
    f.source
        .start_download("sample", &sha256_hex(CONTENT))
        .await
        .unwrap();
    let failed = settled(&f.source).await;
    assert_eq!(failed.state, "failed");
    let error = failed.error.unwrap();
    assert_eq!(error.kind, "dailyLimit");
    assert_eq!(error.reset_at, None);
    assert_eq!(error.retry_after_secs, Some(120));
    let status = f.source.status();
    assert_eq!(status.limit.as_ref().unwrap().reset_at, None);
    assert_eq!(status.limit.as_ref().unwrap().retry_after_secs, 120);
    let text = serde_json::to_string(&status).unwrap();
    assert!(!text.contains("provider-controlled"));
}

#[tokio::test]
async fn changing_origin_discards_old_rate_daily_and_download_state_before_an_explicit_new_sync() {
    for response in [
        Script::reply(429, &[("retry-after", "120")], b"{}"),
        daily(),
    ] {
        let second = TestServer::start(vec![one_index(), Script::reply(200, &[], CONTENT)]).await;
        let f = Fixture::new_trusting(vec![one_index(), response], Some(second.roots())).await;
        f.source.sync().await.unwrap();
        f.token().await;
        f.source
            .start_download("sample", &sha256_hex(CONTENT))
            .await
            .unwrap();
        let failed = settled(&f.source).await;
        assert_eq!(failed.state, "failed");
        assert!(f.source.status().limit.is_some());
        assert_eq!(f.server.requests().len(), 2);
        f.source
            .set_base_url(Some(&second.base_text()))
            .await
            .unwrap();
        assert!(
            second.requests().is_empty(),
            "settings changes must not fetch"
        );
        assert!(
            f.source.status().limit.is_none(),
            "a different origin has independent blocks"
        );
        assert_eq!(
            f.source.download_status().state,
            "idle",
            "old-origin download status is obsolete"
        );
        f.source.sync().await.unwrap();
        assert_eq!(second.requests().len(), 1);
        assert!(!second.requests()[0].headers.contains_key("authorization"));
        f.source
            .set_credentials(Credentials::Token {
                token: "fictional-new-origin-token".into(),
            })
            .await
            .unwrap();
        f.source
            .start_download("sample", &sha256_hex(CONTENT))
            .await
            .unwrap();
        assert_eq!(settled(&f.source).await.state, "done");
        assert_eq!(f.source.read_verified("sample").await.unwrap().0, CONTENT);
        assert_eq!(second.requests().len(), 2);
        assert_eq!(
            second.requests()[1].headers["authorization"],
            "Bearer fictional-new-origin-token"
        );
        assert_eq!(f.server.requests().len(), 2);
    }
}

#[tokio::test]
async fn replacing_or_forgetting_an_account_clears_only_the_previous_accounts_daily_limit() {
    for forget_first in [false, true] {
        let f = Fixture::new(vec![one_index(), daily(), Script::reply(200, &[], CONTENT)]).await;
        f.source.sync().await.unwrap();
        f.token().await;
        f.source
            .start_download("sample", &sha256_hex(CONTENT))
            .await
            .unwrap();
        assert_eq!(settled(&f.source).await.error.unwrap().kind, "dailyLimit");
        if forget_first {
            f.source.forget_credentials().await.unwrap();
            assert!(
                f.source.status().limit.is_none(),
                "forgotten account has no active daily quota"
            );
        }
        f.source
            .set_credentials(Credentials::Token {
                token: "fictional-replacement-account-token".into(),
            })
            .await
            .unwrap();
        assert!(
            f.source.status().limit.is_none(),
            "a replacement account has its own daily quota"
        );
        assert_eq!(
            f.server.requests().len(),
            2,
            "account mutation must not resume the download"
        );
        f.source
            .start_download("sample", &sha256_hex(CONTENT))
            .await
            .unwrap();
        assert_eq!(settled(&f.source).await.state, "done");
        assert_eq!(f.server.requests().len(), 3);
        assert_eq!(
            f.server.requests()[2].headers["authorization"],
            "Bearer fictional-replacement-account-token"
        );
    }
}

#[tokio::test]
async fn replacing_or_forgetting_an_account_retains_the_same_origins_rate_block() {
    for forget_first in [false, true] {
        let f = Fixture::new(vec![
            one_index(),
            Script::reply(429, &[("retry-after", "120")], b"{}"),
            Script::reply(200, &[], CONTENT),
        ])
        .await;
        f.source.sync().await.unwrap();
        f.token().await;
        f.source
            .start_download("sample", &sha256_hex(CONTENT))
            .await
            .unwrap();
        assert_eq!(settled(&f.source).await.error.unwrap().kind, "rateLimited");
        if forget_first {
            f.source.forget_credentials().await.unwrap();
        }
        f.source
            .set_credentials(Credentials::Token {
                token: "fictional-replacement-account-token".into(),
            })
            .await
            .unwrap();
        assert_eq!(f.source.status().limit.unwrap().kind, "rateLimited");
        assert_eq!(
            f.source
                .start_download("sample", &sha256_hex(CONTENT))
                .await
                .unwrap_err()
                .kind(),
            "rateLimited"
        );
        assert_eq!(f.server.requests().len(), 2);
        tokio::time::pause();
        tokio::time::advance(Duration::from_secs(121)).await;
        tokio::task::yield_now().await;
        assert!(f.source.status().limit.is_none());
        assert_eq!(
            f.server.requests().len(),
            2,
            "expiry cannot schedule an action"
        );
        tokio::time::resume();
        f.source
            .start_download("sample", &sha256_hex(CONTENT))
            .await
            .unwrap();
        assert_eq!(settled(&f.source).await.state, "done");
        assert_eq!(f.server.requests().len(), 3);
    }
}
