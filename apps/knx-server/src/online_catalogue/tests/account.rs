//! Account persistence, authenticated encryption and origin-bound secret lifecycle regressions.

use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use super::super::test_server::{Script, TestServer};
use super::super::transport::HttpsClient;
use super::super::vault::Credentials;
use super::super::{store, OnlineCatalogue};
use super::support::*;

#[tokio::test]
async fn both_account_modes_survive_restart_encrypted_and_separate_from_the_key() {
    for credentials in [
        password(),
        Credentials::Token {
            token: TOKEN.into(),
        },
    ] {
        let f = Fixture::new(vec![one_index()]).await;
        f.source.set_credentials(credentials.clone()).await.unwrap();
        let encrypted = f.source_dir().join("credentials.enc");
        let first = std::fs::read(&encrypted).unwrap();
        let envelope: serde_json::Value = serde_json::from_slice(&first).unwrap();
        assert_eq!(envelope["alg"], "AES-256-GCM");
        for secret in [PASSWORD, TOKEN] {
            assert!(!String::from_utf8_lossy(&first).contains(secret));
        }
        assert_ne!(encrypted.parent(), f.key_file().parent());
        assert_eq!(std::fs::metadata(f.key_file()).unwrap().len(), 32);
        for path in [&encrypted, &f.key_file()] {
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                std::fs::metadata(path.parent().unwrap())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }
        let reopened = f.reopen();
        let source = reopened.source("reko").unwrap();
        assert_eq!(source.load_credentials().unwrap(), credentials);
        assert_eq!(source.status().credentials.state, "stored");
        assert_redacted(&serde_json::to_value(source.status()).unwrap());
        // The same credentials still require a new random nonce on every store.
        source.set_credentials(credentials).await.unwrap();
        let second: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&encrypted).unwrap()).unwrap();
        assert_ne!(envelope["nonce"], second["nonce"]);
        assert_ne!(envelope["ciphertext"], second["ciphertext"]);
        assert!(f.server.requests().is_empty());
    }
}

#[tokio::test]
async fn missing_key_never_recreates_or_overwrites_an_unreadable_account() {
    let f = Fixture::new(vec![one_index()]).await;
    f.password().await;
    let encrypted = f.source_dir().join("credentials.enc");
    let before = std::fs::read(&encrypted).unwrap();
    std::fs::remove_file(f.key_file()).unwrap();
    let reopened = f.reopen();
    let source = reopened.source("reko").unwrap();
    assert_eq!(source.status().credentials.problem, Some("vaultKeyMissing"));
    assert_eq!(
        source.load_credentials().unwrap_err().kind(),
        "vaultKeyMissing"
    );
    assert_eq!(
        source.set_credentials(password()).await.unwrap_err().kind(),
        "vaultKeyMissing"
    );
    assert!(!f.key_file().exists());
    assert_eq!(std::fs::read(&encrypted).unwrap(), before);
    assert!(f.server.requests().is_empty());
}

#[tokio::test]
async fn wrong_key_and_tampered_ciphertext_fail_authentication_without_destroying_evidence() {
    for tamper_key in [true, false] {
        let f = Fixture::new(vec![one_index()]).await;
        f.token().await;
        let encrypted = f.source_dir().join("credentials.enc");
        if tamper_key {
            let mut bytes = std::fs::read(f.key_file()).unwrap();
            bytes[0] ^= 0x80;
            std::fs::write(f.key_file(), bytes).unwrap();
        } else {
            let mut body: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&encrypted).unwrap()).unwrap();
            let mut ciphertext = body["ciphertext"].as_str().unwrap().as_bytes().to_vec();
            ciphertext[0] = if ciphertext[0] == b'A' { b'B' } else { b'A' };
            body["ciphertext"] = json!(String::from_utf8(ciphertext).unwrap());
            std::fs::write(&encrypted, serde_json::to_vec(&body).unwrap()).unwrap();
        }
        let before = std::fs::read(&encrypted).unwrap();
        let key_before = std::fs::read(f.key_file()).unwrap();
        let reopened = f.reopen();
        let source = reopened.source("reko").unwrap();
        assert_eq!(source.status().credentials.state, "problem");
        assert_eq!(source.load_credentials().unwrap_err().kind(), "vault");
        assert_eq!(std::fs::read(&encrypted).unwrap(), before);
        assert_eq!(std::fs::read(f.key_file()).unwrap(), key_before);
        // An unreadable record may only be replaced after an explicit forget.
        assert_eq!(
            source.set_credentials(password()).await.unwrap_err().kind(),
            "vault"
        );
        assert_eq!(std::fs::read(&encrypted).unwrap(), before);
        assert_eq!(std::fs::read(f.key_file()).unwrap(), key_before);
        source.forget_credentials().await.unwrap();
        source.set_credentials(password()).await.unwrap();
        assert_eq!(source.load_credentials().unwrap(), password());
        assert!(f.server.requests().is_empty());
    }
}

#[tokio::test]
async fn an_accepted_credential_envelope_always_stays_within_the_read_cap() {
    let f = Fixture::new(vec![one_index()]).await;
    // Worst case for JSON+Base64 expansion inside the sealed envelope.
    let maxed = Credentials::Password {
        user: "\\".repeat(4096),
        password: "\\".repeat(4096),
    };
    assert_eq!(
        f.source.set_credentials(maxed).await.unwrap_err().kind(),
        "vault"
    );
    // A rejected save must leave both the credential file and the key untouched.
    assert!(!f.source_dir().join("credentials.enc").exists());
    assert!(!f.key_file().exists());
    f.source
        .set_credentials(Credentials::Token {
            token: "t".repeat(4096),
        })
        .await
        .unwrap();
    assert!(
        f.source_dir()
            .join("credentials.enc")
            .metadata()
            .unwrap()
            .len()
            <= 16 * 1024
    );
    assert_eq!(
        f.reopen()
            .source("reko")
            .unwrap()
            .status()
            .credentials
            .state,
        "stored"
    );
}

#[tokio::test]
async fn forgetting_an_account_invalidates_the_in_memory_password_token() {
    let f = Fixture::new(vec![
        login("fictional-login-token-one"),
        login("fictional-login-token-two"),
    ])
    .await;
    f.password().await;
    assert_eq!(
        f.source.token(&password(), None).await.unwrap(),
        "fictional-login-token-one"
    );
    f.source.forget_credentials().await.unwrap();
    assert_eq!(f.source.status().credentials.state, "empty");
    assert_eq!(
        f.source.load_credentials().unwrap_err().kind(),
        "needsLogin"
    );
    assert!(!f.source_dir().join("credentials.enc").exists());
    f.password().await;
    assert_eq!(
        f.source.token(&password(), None).await.unwrap(),
        "fictional-login-token-two"
    );
    assert_eq!(f.server.requests().len(), 2);
}

#[tokio::test]
async fn changing_an_account_invalidates_the_previous_token_without_automatic_login() {
    let f = Fixture::new(vec![
        login("fictional-login-token-one"),
        login("fictional-login-token-two"),
    ])
    .await;
    f.password().await;
    f.source.token(&password(), None).await.unwrap();
    let changed = Credentials::Password {
        user: "different-fictional-user".into(),
        password: "different-fictional-password".into(),
    };
    f.source.set_credentials(changed.clone()).await.unwrap();
    assert_eq!(f.server.requests().len(), 1);
    assert_eq!(
        f.source.token(&changed, None).await.unwrap(),
        "fictional-login-token-two"
    );
    let seen = f.server.requests();
    assert_eq!(seen.len(), 2);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&seen[1].body).unwrap()["user"],
        "different-fictional-user"
    );
}

#[tokio::test]
async fn a_new_origin_cannot_receive_the_previous_origins_credentials_or_token() {
    let second = TestServer::start(vec![one_index(), Script::reply(200, &[], CONTENT)]).await;
    let f = Fixture::new(vec![one_index(), login("fictional-login-token-one")]).await;
    f.source.sync().await.unwrap();
    f.password().await;
    f.source.token(&password(), None).await.unwrap();
    let mut roots = f.server.roots();
    roots.roots.extend(second.roots().roots);
    let catalogue = OnlineCatalogue::with_client(
        f.temp.path().join("online-data"),
        f.temp.path().join("separate-config"),
        Arc::new(HttpsClient::with_roots(roots, Duration::ZERO)),
    );
    let source = catalogue.source("reko").unwrap();
    let sealed = std::fs::read(f.source_dir().join("credentials.enc")).unwrap();
    source
        .set_base_url(Some(&second.base_text()))
        .await
        .unwrap();
    assert!(second.requests().is_empty());
    assert_eq!(
        source.status().credentials.problem,
        Some("vaultOriginMismatch")
    );
    assert_eq!(source.status().index_problem, Some("originChanged"));
    assert_eq!(
        source.search("", None, 0, 10).await.err().unwrap().kind(),
        "noIndex"
    );
    source.sync().await.unwrap();
    assert_eq!(
        source
            .start_download("sample", &store::sha256_hex(CONTENT))
            .await
            .unwrap_err()
            .kind(),
        "vaultOriginMismatch"
    );
    assert_eq!(second.requests().len(), 1);
    assert!(!second.requests()[0].headers.contains_key("authorization"));
    assert_eq!(
        std::fs::read(f.source_dir().join("credentials.enc")).unwrap(),
        sealed
    );
    source
        .set_credentials(Credentials::Token {
            token: "new-origin-fictional-token".into(),
        })
        .await
        .unwrap();
    source
        .start_download("sample", &store::sha256_hex(CONTENT))
        .await
        .unwrap();
    assert_eq!(settled(source).await.state, "done");
    let seen = second.requests();
    assert_eq!(seen.len(), 2);
    assert_eq!(
        seen[1].headers["authorization"],
        "Bearer new-origin-fictional-token"
    );
    assert_eq!(f.server.requests().len(), 2);
}
