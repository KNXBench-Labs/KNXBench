//! One catalogue source: manual index sync, local search, account and token handling, status.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::{header, HeaderValue, Method};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use super::download::DownloadSlot;
use super::error::{OnlineError, VaultError};
use super::index::{self, Entry, Snapshot, MAX_COMPRESSED_BYTES, MAX_DECODED_BYTES};
use super::source::{BaseUrl, SourceSpec};
use super::store::{self, Loaded, Meta};
use super::transport::{HttpsClient, Reply, TransportError, INDEX_TOTAL_TIMEOUT, SMALL_BODY_BYTES};
use super::vault::{Credentials, Vault, VaultState};

/// Longest `Retry-After` honoured; a larger value is clamped, never trusted blindly.
const MAX_RETRY_AFTER_SECS: u64 = 24 * 3600;
/// Safety margin before a token's announced end of life.
const TOKEN_MARGIN: Duration = Duration::from_secs(60);
const SMALL_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_TOKEN_LIFE: Duration = Duration::from_secs(3600);
const MAX_TOKEN_BYTES: usize = 4096;

pub(super) enum Index {
    Unloaded,
    Ready(Box<Meta>, Arc<Snapshot>),
    /// No stored index, or one fetched from another address.
    Absent,
    Damaged,
}

pub(super) struct State {
    pub base: BaseUrl,
    pub index: Index,
}

/// A login's token held only in memory.
pub(super) struct Session {
    pub token: String,
    pub expires: Option<Instant>,
}

#[derive(Default)]
pub(super) struct Blocks {
    /// The per-address request rate was exceeded: nothing may be sent until it passes.
    pub rate: Option<Instant>,
    /// The account's daily download limit: only downloads wait.
    pub daily: Option<(Instant, Option<String>)>,
}

pub struct SourceService {
    pub(super) spec: &'static SourceSpec,
    pub(super) dir: PathBuf,
    pub(super) vault: Vault,
    pub(super) client: Arc<HttpsClient>,
    pub(super) state: Mutex<State>,
    pub(super) sync_gate: tokio::sync::Mutex<()>,
    sync_active: AtomicBool,
    pub(super) session: tokio::sync::Mutex<Option<Session>>,
    pub(super) blocks: Mutex<Blocks>,
    pub(super) download: Mutex<DownloadSlot>,
    pub(super) received: Arc<AtomicU64>,
}

/// How a manual synchronisation ended successfully.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "outcome"
)]
pub enum SyncOutcome {
    Updated { entry_count: u64 },
    NotModified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatus {
    pub entry_count: u64,
    pub skipped_entries: u64,
    pub version: String,
    pub generated_at: String,
    /// When the server last confirmed this index (the last successful synchronisation).
    pub last_checked_at: String,
    /// When the stored document last changed.
    pub fetched_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    pub state: &'static str,
    pub mode: Option<&'static str>,
    pub user: Option<String>,
    pub problem: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitStatus {
    pub kind: &'static str,
    pub retry_after_secs: u64,
    pub reset_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub source: &'static str,
    pub label: &'static str,
    pub default_base_url: &'static str,
    pub base_url: String,
    pub base_url_is_default: bool,
    pub index: Option<IndexStatus>,
    /// Why a stored index is not in use: `originChanged` or `damaged`.
    pub index_problem: Option<&'static str>,
    pub credentials: CredentialStatus,
    pub limit: Option<LimitStatus>,
    pub sync_running: bool,
    pub download: super::download::DownloadStatus,
}

pub fn now_text() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

impl SourceService {
    pub fn open(
        spec: &'static SourceSpec,
        data_dir: PathBuf,
        key_file: PathBuf,
        client: Arc<HttpsClient>,
    ) -> Self {
        let dir = data_dir.join(spec.id);
        let configured = store::read_base_url(&dir).and_then(|u| BaseUrl::parse(&u).ok());
        let base = configured.unwrap_or_else(|| {
            BaseUrl::parse(spec.default_base_url).expect("default address is valid")
        });
        super::download::clear_partial_files(&dir);
        Self {
            vault: Vault::new(dir.join("credentials.enc"), key_file, spec.id),
            spec,
            dir,
            client,
            state: Mutex::new(State {
                base,
                index: Index::Unloaded,
            }),
            sync_gate: tokio::sync::Mutex::new(()),
            sync_active: AtomicBool::new(false),
            session: tokio::sync::Mutex::new(None),
            blocks: Mutex::new(Blocks::default()),
            download: Mutex::new(DownloadSlot::Idle),
            received: Arc::new(AtomicU64::new(0)),
        }
    }

    pub(super) fn base(&self) -> BaseUrl {
        self.state.lock().expect("state").base.clone()
    }

    // ---- status ---------------------------------------------------------------------------

    pub fn status(&self) -> Status {
        let (base, index, problem) = {
            let state = self.state.lock().expect("state");
            let origin = state.base.canonical();
            let (index, problem) = match &state.index {
                Index::Ready(meta, _) => (Some(index_status(meta)), None),
                Index::Damaged => (None, Some("damaged")),
                Index::Absent | Index::Unloaded => match store::peek_meta(&self.dir) {
                    Some(meta) if meta.origin == origin => (Some(index_status(&meta)), None),
                    Some(_) => (None, Some("originChanged")),
                    None => (None, None),
                },
            };
            (state.base.clone(), index, problem)
        };
        Status {
            source: self.spec.id,
            label: self.spec.label,
            default_base_url: self.spec.default_base_url,
            base_url: base.canonical(),
            base_url_is_default: base.canonical()
                == BaseUrl::parse(self.spec.default_base_url)
                    .map(|b| b.canonical())
                    .unwrap_or_default(),
            index,
            index_problem: problem,
            credentials: credential_status(self.vault.state(&base.canonical())),
            limit: self.limit_status(),
            sync_running: self.sync_active.load(Ordering::Relaxed),
            download: self.download_status(),
        }
    }

    fn limit_status(&self) -> Option<LimitStatus> {
        let now = Instant::now();
        let blocks = self.blocks.lock().expect("blocks");
        if let Some((until, reset_at)) = &blocks.daily {
            if *until > now {
                return Some(LimitStatus {
                    kind: "dailyLimit",
                    retry_after_secs: (*until - now).as_secs() + 1,
                    reset_at: reset_at.clone(),
                });
            }
        }
        match blocks.rate {
            Some(until) if until > now => Some(LimitStatus {
                kind: "rateLimited",
                retry_after_secs: (until - now).as_secs() + 1,
                reset_at: None,
            }),
            _ => None,
        }
    }

    // ---- configuration --------------------------------------------------------------------

    /// Sets (or with `None` resets to the default) the catalogue address. The stored index and
    /// session belong to the old address and stop being used; credentials are never sent on.
    pub async fn set_base_url(self: &Arc<Self>, raw: Option<&str>) -> Result<(), OnlineError> {
        let service = Arc::clone(self);
        let raw = raw.map(str::to_string);
        tokio::spawn(async move { service.set_base_url_inner(raw.as_deref()).await })
            .await
            .map_err(|_| OnlineError::Storage("the address operation failed"))?
    }

    async fn set_base_url_inner(&self, raw: Option<&str>) -> Result<(), OnlineError> {
        let base = match raw {
            Some(raw) => BaseUrl::parse(raw).map_err(OnlineError::InvalidBaseUrl)?,
            None => BaseUrl::parse(self.spec.default_base_url).expect("default address is valid"),
        };
        let _sync = self.sync_gate.try_lock().map_err(|_| OnlineError::Busy)?;
        if self.download_running() {
            return Err(OnlineError::Busy);
        }
        let dir = self.dir.clone();
        let stored = raw.map(|_| base.canonical());
        tokio::task::spawn_blocking(move || store::write_base_url(&dir, stored.as_deref()))
            .await
            .map_err(|_| OnlineError::Storage("the address could not be saved"))?
            .map_err(|_| OnlineError::Storage("the address could not be saved"))?;
        *self.session.lock().await = None;
        let mut state = self.state.lock().expect("state");
        if state.base != base {
            state.index = Index::Unloaded;
            *self.blocks.lock().expect("blocks") = Blocks::default();
            *self.download.lock().expect("download") = DownloadSlot::Idle;
            self.received.store(0, Ordering::Relaxed);
        }
        state.base = base;
        Ok(())
    }

    // ---- local index ----------------------------------------------------------------------

    pub(super) async fn snapshot(&self) -> Result<Arc<Snapshot>, OnlineError> {
        loop {
            let (origin, needs_load) = {
                let state = self.state.lock().expect("state");
                match &state.index {
                    Index::Ready(_, snapshot) => return Ok(Arc::clone(snapshot)),
                    Index::Unloaded => (state.base.canonical(), true),
                    Index::Absent | Index::Damaged => (state.base.canonical(), false),
                }
            };
            if !needs_load {
                return Err(OnlineError::NoIndex);
            }
            let dir = self.dir.clone();
            let loaded = tokio::task::spawn_blocking({
                let origin = origin.clone();
                move || store::load_index(&dir, &origin)
            })
            .await
            .map_err(|_| OnlineError::Storage("the stored index could not be read"))?;
            let mut state = self.state.lock().expect("state");
            if state.base.canonical() != origin || !matches!(state.index, Index::Unloaded) {
                continue;
            }
            return match loaded {
                Loaded::Ready(meta, snapshot) => {
                    let snapshot = Arc::new(*snapshot);
                    state.index = Index::Ready(meta, Arc::clone(&snapshot));
                    Ok(snapshot)
                }
                Loaded::Empty | Loaded::OtherOrigin => {
                    state.index = Index::Absent;
                    Err(OnlineError::NoIndex)
                }
                Loaded::Damaged => {
                    state.index = Index::Damaged;
                    Err(OnlineError::NoIndex)
                }
            };
        }
    }

    /// One page of local search hits.
    pub async fn search(
        &self,
        query: &str,
        file_type: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> Result<SearchResult, OnlineError> {
        let snapshot = self.snapshot().await?;
        let query = query.to_string();
        let file_type = file_type.map(str::to_string);
        let dir = self.dir.clone();
        tokio::task::spawn_blocking(move || {
            let page = snapshot.search(&query, file_type.as_deref(), offset, limit);
            SearchResult {
                total: page.total,
                entries: page
                    .entries
                    .into_iter()
                    .map(|entry| {
                        let cached = super::download::is_cached_at(&dir, entry);
                        (entry.clone(), cached)
                    })
                    .collect(),
            }
        })
        .await
        .map_err(|_| OnlineError::Storage("the local search could not be completed"))
    }

    pub(super) async fn entry(&self, id: &str) -> Result<Entry, OnlineError> {
        self.snapshot()
            .await?
            .find(id)
            .cloned()
            .ok_or(OnlineError::UnknownEntry)
    }

    // ---- account --------------------------------------------------------------------------

    pub async fn set_credentials(
        self: &Arc<Self>,
        credentials: Credentials,
    ) -> Result<(), OnlineError> {
        let service = Arc::clone(self);
        tokio::spawn(async move { service.set_credentials_inner(credentials).await })
            .await
            .map_err(|_| OnlineError::Storage("the account operation failed"))?
    }

    async fn set_credentials_inner(&self, credentials: Credentials) -> Result<(), OnlineError> {
        let _operation = self.sync_gate.try_lock().map_err(|_| OnlineError::Busy)?;
        if self.download_running() {
            return Err(OnlineError::Busy);
        }
        credentials
            .validate()
            .map_err(|_| OnlineError::Integrity("the credentials are not acceptable"))?;
        let origin = self.base().canonical();
        let vault = self.vault.clone();
        tokio::task::spawn_blocking(move || vault.store(&origin, &credentials))
            .await
            .map_err(|_| OnlineError::Vault(VaultError::Io))?
            .map_err(OnlineError::Vault)?;
        *self.session.lock().await = None;
        // Daily quotas belong to the explicitly replaced/forgotten account; the origin's
        // per-IP rate block still applies and cannot be bypassed by changing credentials.
        self.blocks.lock().expect("blocks").daily = None;
        Ok(())
    }

    pub async fn forget_credentials(self: &Arc<Self>) -> Result<(), OnlineError> {
        let service = Arc::clone(self);
        tokio::spawn(async move { service.forget_credentials_inner().await })
            .await
            .map_err(|_| OnlineError::Storage("the account operation failed"))?
    }

    async fn forget_credentials_inner(&self) -> Result<(), OnlineError> {
        let _operation = self.sync_gate.try_lock().map_err(|_| OnlineError::Busy)?;
        if self.download_running() {
            return Err(OnlineError::Busy);
        }
        let vault = self.vault.clone();
        tokio::task::spawn_blocking(move || vault.forget())
            .await
            .map_err(|_| OnlineError::Vault(VaultError::Io))?
            .map_err(OnlineError::Vault)?;
        *self.session.lock().await = None;
        // Daily quotas belong to the explicitly replaced/forgotten account; the origin's
        // per-IP rate block still applies and cannot be bypassed by changing credentials.
        self.blocks.lock().expect("blocks").daily = None;
        Ok(())
    }

    pub(super) fn load_credentials(&self) -> Result<Credentials, OnlineError> {
        match self.vault.load(&self.base().canonical()) {
            Ok(Some(credentials)) => Ok(credentials),
            Ok(None) => Err(OnlineError::NeedsLogin),
            Err(error) => Err(OnlineError::Vault(error)),
        }
    }

    /// A token for the next request. A password login happens at most once per call, under one
    /// lock, so concurrent callers share a single login. `rejected` names a token the server
    /// just refused: it is never handed out again.
    pub(super) async fn token(
        &self,
        credentials: &Credentials,
        rejected: Option<&str>,
    ) -> Result<String, OnlineError> {
        let mut session = self.session.lock().await;
        if let Some(current) = session.as_ref() {
            let fresh = current.expires.is_none_or(|t| t > Instant::now());
            if fresh && Some(current.token.as_str()) != rejected {
                return Ok(current.token.clone());
            }
        }
        match credentials {
            Credentials::Token { token } => {
                if rejected == Some(token.as_str()) {
                    return Err(OnlineError::TokenRejected);
                }
                *session = Some(Session {
                    token: token.clone(),
                    expires: None,
                });
                Ok(token.clone())
            }
            Credentials::Password { user, password } => {
                *session = None;
                let fresh = self.login(user, password).await?;
                let token = fresh.token.clone();
                *session = Some(fresh);
                Ok(token)
            }
        }
    }

    async fn login(&self, user: &str, password: &str) -> Result<Session, OnlineError> {
        self.check_blocked(false)?;
        let base = self.base();
        let body = serde_json::to_vec(&serde_json::json!({ "user": user, "password": password }))
            .map_err(|_| OnlineError::Storage("the login request could not be built"))?;
        let path = base
            .join("/api/auth/token")
            .map_err(OnlineError::InvalidBaseUrl)?;
        let reply = self
            .client
            .request(
                &base,
                Method::POST,
                path.as_str(),
                &[
                    (
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    ),
                    (header::ACCEPT, HeaderValue::from_static("application/json")),
                ],
                Some(body),
            )
            .await
            .map_err(transport)?;
        match reply.status {
            200 => {}
            401 => return Err(OnlineError::InvalidCredentials),
            _ => return Err(self.failure(reply).await),
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Login {
            token: String,
            token_type: String,
            #[serde(default)]
            expires_in: Option<u64>,
            #[serde(default)]
            expires_at: Option<String>,
        }
        let bytes = reply
            .read_bounded(SMALL_BODY_BYTES, SMALL_TIMEOUT)
            .await
            .map_err(transport)?;
        let login: Login = serde_json::from_slice(&bytes)
            .map_err(|_| OnlineError::Integrity("the login answer is not understood"))?;
        let token_ok = !login.token.is_empty()
            && login.token.len() <= MAX_TOKEN_BYTES
            && !login
                .token
                .chars()
                .any(|c| c.is_control() || c.is_whitespace());
        if !token_ok || !login.token_type.eq_ignore_ascii_case("bearer") {
            return Err(OnlineError::Integrity(
                "the login answer is not a bearer token",
            ));
        }
        let absolute_life = login
            .expires_at
            .as_deref()
            .and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
            .map(|expiry| {
                (expiry.with_timezone(&chrono::Utc) - chrono::Utc::now())
                    .to_std()
                    .unwrap_or_default()
            });
        let announced = match (login.expires_in.map(Duration::from_secs), absolute_life) {
            (Some(relative), Some(absolute)) => relative.min(absolute),
            (Some(relative), None) => relative,
            (None, Some(absolute)) => absolute,
            (None, None) => DEFAULT_TOKEN_LIFE,
        };
        let life = announced
            .saturating_sub(TOKEN_MARGIN)
            .min(Duration::from_secs(MAX_RETRY_AFTER_SECS));
        Ok(Session {
            token: login.token,
            expires: Some(Instant::now() + life),
        })
    }

    // ---- synchronisation ------------------------------------------------------------------

    /// Fetches the index once, on explicit request. A failure leaves the stored index untouched.
    pub async fn sync(self: &Arc<Self>) -> Result<SyncOutcome, OnlineError> {
        let service = Arc::clone(self);
        // Own admitted work beyond the HTTP waiter: cancellation must not release the operation
        // gate while a blocking file publication is still in flight.
        tokio::spawn(async move { service.sync_inner().await })
            .await
            .map_err(|_| OnlineError::Storage("the sync operation failed"))?
    }

    async fn sync_inner(&self) -> Result<SyncOutcome, OnlineError> {
        let _gate = self.sync_gate.try_lock().map_err(|_| OnlineError::Busy)?;
        if self.download_running() {
            return Err(OnlineError::Busy);
        }
        self.check_blocked(false)?;
        self.sync_active.store(true, Ordering::Relaxed);
        let _active = SyncActivity(&self.sync_active);
        let base = self.base();
        let origin = base.canonical();
        // Make sure a matching stored index is known (and only then offer its validator).
        let known = match self.snapshot().await {
            Ok(_) => store::peek_meta(&self.dir).filter(|m| m.origin == origin),
            Err(_) => None,
        };
        let mut headers = vec![
            (header::ACCEPT_ENCODING, HeaderValue::from_static("gzip")),
            (header::ACCEPT, HeaderValue::from_static("application/json")),
        ];
        if let Some(etag) = known.as_ref().and_then(|m| m.etag.as_deref()) {
            if let Ok(value) = HeaderValue::from_str(etag) {
                headers.push((header::IF_NONE_MATCH, value));
            }
        }
        let path = base
            .join("/api/catalog/index.json")
            .map_err(OnlineError::InvalidBaseUrl)?;
        let reply = self
            .client
            .request(&base, Method::GET, path.as_str(), &headers, None)
            .await
            .map_err(transport)?;
        match reply.status {
            304 => {
                let Some(mut meta) = known else {
                    return Err(OnlineError::Integrity(
                        "a 304 answer without a stored index",
                    ));
                };
                meta.last_checked_at = now_text();
                self.persist_meta(meta).await?;
                Ok(SyncOutcome::NotModified)
            }
            200 => self.accept_index(reply, &origin).await,
            _ => Err(self.failure(reply).await),
        }
    }

    async fn accept_index(&self, reply: Reply, origin: &str) -> Result<SyncOutcome, OnlineError> {
        let encoding = reply
            .header("content-encoding")
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        if !matches!(encoding.as_str(), "" | "identity" | "gzip") {
            return Err(OnlineError::Integrity("unsupported content encoding"));
        }
        let etag = reply
            .header("etag")
            .map(str::to_string)
            .filter(|e| e.len() <= 256);
        let gzip = encoding == "gzip";
        let raw = reply
            .read_bounded(MAX_COMPRESSED_BYTES, INDEX_TOTAL_TIMEOUT)
            .await
            .map_err(transport)?;
        let dir = self.dir.clone();
        let origin = origin.to_string();
        let built =
            tokio::task::spawn_blocking(move || -> Result<(Meta, Snapshot), OnlineError> {
                let decoded = if gzip {
                    index::gunzip_bounded(&raw, MAX_DECODED_BYTES)?
                } else {
                    if raw.len() > MAX_DECODED_BYTES {
                        return Err(OnlineError::Integrity("the index is larger than the limit"));
                    }
                    raw
                };
                let parsed = index::parse(&decoded)?;
                let sha = store::sha256_hex(&decoded);
                let now = now_text();
                let meta = Meta {
                    schema: 1,
                    origin,
                    etag,
                    version: parsed.version.clone(),
                    generated_at: parsed.generated_at.clone(),
                    entry_count: parsed.entries.len() as u64 + parsed.skipped,
                    skipped: parsed.skipped,
                    data_file: store::data_file_name(&sha),
                    data_sha256: sha,
                    fetched_at: now.clone(),
                    last_checked_at: now,
                };
                store::save_index(&dir, &meta, &decoded)
                    .map_err(|_| OnlineError::Storage("the index could not be stored"))?;
                drop(decoded);
                Ok((meta, Snapshot::new(parsed)))
            })
            .await
            .map_err(|_| OnlineError::Storage("the index could not be processed"))??;
        let (meta, snapshot) = built;
        let count = meta.entry_count;
        let snapshot = Arc::new(snapshot);
        let mut state = self.state.lock().expect("state");
        // Only publish when the address is still the one this index came from.
        if state.base.canonical() == meta.origin {
            state.index = Index::Ready(Box::new(meta), snapshot);
        }
        Ok(SyncOutcome::Updated { entry_count: count })
    }

    async fn persist_meta(&self, meta: Meta) -> Result<(), OnlineError> {
        let dir = self.dir.clone();
        let saved = meta.clone();
        tokio::task::spawn_blocking(move || store::save_meta(&dir, &saved))
            .await
            .map_err(|_| OnlineError::Storage("the index status could not be stored"))?
            .map_err(|_| OnlineError::Storage("the index status could not be stored"))?;
        let mut state = self.state.lock().expect("state");
        if let Index::Ready(current, _) = &mut state.index {
            **current = meta;
        }
        Ok(())
    }

    // ---- error mapping --------------------------------------------------------------------

    /// Refuses before any request while a server-announced wait is in force.
    pub(super) fn check_blocked(&self, download: bool) -> Result<(), OnlineError> {
        let now = Instant::now();
        let blocks = self.blocks.lock().expect("blocks");
        if download {
            if let Some((until, reset_at)) = &blocks.daily {
                if *until > now {
                    return Err(OnlineError::DailyLimit {
                        reset_at: reset_at.clone(),
                        retry_after_secs: Some((*until - now).as_secs() + 1),
                    });
                }
            }
        }
        match blocks.rate {
            Some(until) if until > now => Err(OnlineError::RateLimited {
                retry_after_secs: Some((until - now).as_secs() + 1),
            }),
            _ => Ok(()),
        }
    }

    /// Maps a non-success reply to a refusal, recording a 429's wait. The body is only read for
    /// the machine-readable `code`/`resetAt`; it is never echoed.
    pub(super) async fn failure(&self, reply: Reply) -> OnlineError {
        let status = reply.status;
        let retry_after = reply
            .header("retry-after")
            .and_then(retry_after_seconds)
            .map(|s| s.min(MAX_RETRY_AFTER_SECS));
        let body = reply
            .read_bounded(SMALL_BODY_BYTES, SMALL_TIMEOUT)
            .await
            .ok();
        let (code, reset_at) = body.as_deref().map(error_fields).unwrap_or_default();
        match status {
            401 => OnlineError::Transport("the server refused the request's authorization"),
            403 => OnlineError::Forbidden,
            404 => OnlineError::NotFound,
            429 => {
                let reset_wait = reset_at
                    .as_deref()
                    .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
                    .and_then(|t| {
                        (t.with_timezone(&chrono::Utc) - chrono::Utc::now())
                            .to_std()
                            .ok()
                    })
                    .map(|wait| wait.as_secs().saturating_add(1).min(MAX_RETRY_AFTER_SECS));
                let mut blocks = self.blocks.lock().expect("blocks");
                if code.as_deref() == Some("DOWNLOAD_DAILY_LIMIT") {
                    // Neither a zero/short Retry-After nor clock arithmetic may move the
                    // provider's announced daily reset earlier. Honour the later boundary.
                    let seconds = match (retry_after, reset_wait) {
                        (Some(retry), Some(reset)) => retry.max(reset),
                        (Some(retry), None) => retry,
                        (None, Some(reset)) => reset,
                        (None, None) => 60,
                    };
                    blocks.daily = Some((
                        Instant::now() + Duration::from_secs(seconds),
                        reset_at.clone(),
                    ));
                    OnlineError::DailyLimit {
                        reset_at,
                        retry_after_secs: retry_after,
                    }
                } else {
                    blocks.rate =
                        Some(Instant::now() + Duration::from_secs(retry_after.unwrap_or(60)));
                    OnlineError::RateLimited {
                        retry_after_secs: retry_after,
                    }
                }
            }
            503 => OnlineError::ServiceUnavailable,
            300..=399 => OnlineError::Transport("the server answered with a redirect"),
            _ => OnlineError::Transport("the server answered with an unexpected status"),
        }
    }
}

struct SyncActivity<'a>(&'a AtomicBool);
impl Drop for SyncActivity<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

/// One search page: hits with whether a verified copy is already stored locally.
pub struct SearchResult {
    pub total: usize,
    pub entries: Vec<(Entry, bool)>,
}

fn index_status(meta: &Meta) -> IndexStatus {
    IndexStatus {
        entry_count: meta.entry_count,
        skipped_entries: meta.skipped,
        version: meta.version.clone(),
        generated_at: meta.generated_at.clone(),
        last_checked_at: meta.last_checked_at.clone(),
        fetched_at: meta.fetched_at.clone(),
    }
}

fn credential_status(state: VaultState) -> CredentialStatus {
    match state {
        VaultState::Empty => CredentialStatus {
            state: "empty",
            mode: None,
            user: None,
            problem: None,
        },
        VaultState::Stored { mode, user } => CredentialStatus {
            state: "stored",
            mode: Some(mode),
            user,
            problem: None,
        },
        VaultState::Problem(error) => CredentialStatus {
            state: "problem",
            mode: None,
            user: None,
            problem: Some(OnlineError::Vault(error).kind()),
        },
    }
}

pub(super) fn transport(error: TransportError) -> OnlineError {
    OnlineError::Transport(error.describe())
}

/// Retry delay from a numeric or HTTP-date header, bounded by callers.
pub(super) fn retry_after_seconds(value: &str) -> Option<u64> {
    value.trim().parse::<u64>().ok().or_else(|| {
        let time = chrono::DateTime::parse_from_rfc2822(value).ok()?;
        let wait = (time.with_timezone(&chrono::Utc) - chrono::Utc::now())
            .to_std()
            .unwrap_or_default();
        Some(wait.as_secs().saturating_add(1))
    })
}

/// `code` and a canonical `resetAt` of an error body, when it is the expected small JSON
/// object. Only a parseable RFC 3339 timestamp is kept; arbitrary provider text never leaves
/// the backend.
pub(super) fn error_fields(body: &[u8]) -> (Option<String>, Option<String>) {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) else {
        return (None, None);
    };
    let text = |key: &str| {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .filter(|s| s.len() <= 128 && !s.chars().any(char::is_control))
            .map(str::to_string)
    };
    let reset_at = text("resetAt").and_then(|raw| {
        chrono::DateTime::parse_from_rfc3339(&raw).ok().map(|t| {
            t.with_timezone(&chrono::Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        })
    });
    (text("code"), reset_at)
}
