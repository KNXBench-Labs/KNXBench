//! Credentialed download of one catalogue file into a verified, size-bounded local cache.

use std::fs::{DirBuilder, File};
use std::io::Read;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::http::{header, HeaderValue, Method};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;

use super::error::OnlineError;
use super::index::{Entry, MAX_FILE_BYTES};
use super::service::{error_fields, transport, SourceService};
use super::transport::{IDLE_TIMEOUT, SMALL_BODY_BYTES};

const DOWNLOADS_DIR: &str = "downloads";
const PART_SUFFIX: &str = ".part";
const FILE_SUFFIX: &str = ".bin";
/// The cache keeps at most this many verified files and bytes; the oldest go first.
const MAX_CACHED_FILES: usize = 16;
const MAX_CACHED_BYTES: u64 = 2 * MAX_FILE_BYTES;
const INVALID_TOKEN: &str = "INVALID_TOKEN";
/// Independent of the idle timeout, a trickling response cannot keep a download alive forever.
const DOWNLOAD_TOTAL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3600);

pub(super) enum DownloadSlot {
    Idle,
    Running {
        entry_id: String,
        total: u64,
        cancel: watch::Sender<bool>,
    },
    Done {
        entry_id: String,
        size: u64,
    },
    Failed {
        entry_id: String,
        error: OnlineError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInfo {
    pub kind: &'static str,
    pub message: String,
    pub reset_at: Option<String>,
    pub retry_after_secs: Option<u64>,
}

impl ErrorInfo {
    pub fn from_error(error: &OnlineError) -> Self {
        let (reset_at, retry_after_secs) = match error {
            OnlineError::DailyLimit {
                reset_at,
                retry_after_secs,
            } => (reset_at.clone(), *retry_after_secs),
            OnlineError::RateLimited { retry_after_secs } => (None, *retry_after_secs),
            _ => (None, None),
        };
        Self {
            kind: error.kind(),
            message: error.message(),
            reset_at,
            retry_after_secs,
        }
    }
}

/// The single download slot's public state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStatus {
    /// `idle`, `running`, `done` or `failed`.
    pub state: &'static str,
    pub entry_id: Option<String>,
    pub received_bytes: u64,
    pub total_bytes: Option<u64>,
    pub error: Option<ErrorInfo>,
}

impl DownloadStatus {
    fn idle() -> Self {
        Self {
            state: "idle",
            entry_id: None,
            received_bytes: 0,
            total_bytes: None,
            error: None,
        }
    }
}

fn downloads_dir(dir: &Path) -> PathBuf {
    dir.join(DOWNLOADS_DIR)
}

/// Removes half-written files an earlier run (or crash) left behind.
pub(super) fn clear_partial_files(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(downloads_dir(dir)) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().ends_with(PART_SUFFIX) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

pub(super) fn is_cached_at(dir: &Path, entry: &Entry) -> bool {
    std::fs::symlink_metadata(downloads_dir(dir).join(format!("{}{FILE_SUFFIX}", entry.id)))
        .is_ok_and(|m| m.is_file() && m.nlink() == 1 && m.len() == entry.size)
}

impl SourceService {
    fn cache_path(&self, entry_id: &str) -> PathBuf {
        downloads_dir(&self.dir).join(format!("{entry_id}{FILE_SUFFIX}"))
    }

    /// Whether a file of the advertised size is stored (verified when it was written).
    pub(super) fn is_cached(&self, entry: &Entry) -> bool {
        is_cached_at(&self.dir, entry)
    }

    pub(super) fn download_running(&self) -> bool {
        matches!(
            *self.download.lock().expect("download"),
            DownloadSlot::Running { .. }
        )
    }

    pub fn download_status(&self) -> DownloadStatus {
        match &*self.download.lock().expect("download") {
            DownloadSlot::Idle => DownloadStatus::idle(),
            DownloadSlot::Running {
                entry_id, total, ..
            } => DownloadStatus {
                state: "running",
                entry_id: Some(entry_id.clone()),
                received_bytes: self.received.load(Ordering::Relaxed),
                total_bytes: Some(*total),
                error: None,
            },
            DownloadSlot::Done { entry_id, size } => DownloadStatus {
                state: "done",
                entry_id: Some(entry_id.clone()),
                received_bytes: *size,
                total_bytes: Some(*size),
                error: None,
            },
            DownloadSlot::Failed { entry_id, error } => DownloadStatus {
                state: "failed",
                entry_id: Some(entry_id.clone()),
                received_bytes: 0,
                total_bytes: None,
                error: Some(ErrorInfo::from_error(error)),
            },
        }
    }

    /// Starts downloading `entry_id` in the background. A verified cached copy is reused without
    /// any network request, so it spends none of the account's daily allowance.
    pub async fn start_download(
        self: &Arc<Self>,
        entry_id: &str,
        expected_sha256: &str,
    ) -> Result<DownloadStatus, OnlineError> {
        let _operation = self.sync_gate.try_lock().map_err(|_| OnlineError::Busy)?;
        let entry = self.entry(entry_id).await?;
        if entry.sha256 != expected_sha256 {
            return Err(OnlineError::SelectionChanged);
        }
        if self.download_running() {
            return Err(OnlineError::Busy);
        }
        if self.verified_copy(&entry).await {
            *self.download.lock().expect("download") = DownloadSlot::Done {
                entry_id: entry.id.clone(),
                size: entry.size,
            };
            return Ok(self.download_status());
        }
        let credentials = self.load_credentials()?;
        self.check_blocked(true)?;
        let (cancel, cancelled) = watch::channel(false);
        {
            let mut slot = self.download.lock().expect("download");
            if matches!(*slot, DownloadSlot::Running { .. }) {
                return Err(OnlineError::Busy);
            }
            self.received.store(0, Ordering::Relaxed);
            *slot = DownloadSlot::Running {
                entry_id: entry.id.clone(),
                total: entry.size,
                cancel,
            };
        }
        let service = Arc::clone(self);
        tokio::spawn(async move {
            let result = tokio::select! {
                result = service.run_download(&entry, credentials) => result,
                () = cancelled_signal(cancelled) => Err(OnlineError::Cancelled),
                () = tokio::time::sleep(DOWNLOAD_TOTAL_TIMEOUT) =>
                    Err(OnlineError::Transport("the download exceeded its time budget")),
            };
            *service.download.lock().expect("download") = match result {
                Ok(()) => DownloadSlot::Done {
                    entry_id: entry.id.clone(),
                    size: entry.size,
                },
                Err(error) => DownloadSlot::Failed {
                    entry_id: entry.id.clone(),
                    error,
                },
            };
        });
        Ok(self.download_status())
    }

    /// Asks a running download to stop; it ends as `failed` with kind `cancelled`.
    pub fn cancel_download(&self) {
        if let DownloadSlot::Running { cancel, .. } = &*self.download.lock().expect("download") {
            let _ = cancel.send(true);
        }
    }

    async fn verified_copy(&self, entry: &Entry) -> bool {
        if !self.is_cached(entry) {
            return false;
        }
        let path = self.cache_path(&entry.id);
        let expected = entry.sha256.clone();
        let size = entry.size;
        let removed = path.clone();
        let ok = tokio::task::spawn_blocking(move || {
            digest_of(&path, size).is_ok_and(|digest| digest == expected)
        })
        .await
        .unwrap_or(false);
        if !ok {
            let _ = std::fs::remove_file(removed);
        }
        ok
    }

    async fn run_download(
        &self,
        entry: &Entry,
        credentials: super::vault::Credentials,
    ) -> Result<(), OnlineError> {
        let base = self.base();
        let path = base
            .join(&entry.download_url)
            .map_err(OnlineError::InvalidBaseUrl)?;
        let mut token = self.token(&credentials, None).await?;
        let mut renewed = false;
        let mut reply = loop {
            self.check_blocked(true)?;
            let bearer = HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|_| OnlineError::Integrity("the token cannot be sent"))?;
            let reply = self
                .client
                .request(
                    &base,
                    Method::GET,
                    path.as_str(),
                    &[
                        (header::AUTHORIZATION, bearer),
                        (
                            header::ACCEPT,
                            HeaderValue::from_static("application/octet-stream"),
                        ),
                    ],
                    None,
                )
                .await
                .map_err(transport)?;
            match reply.status {
                200 => break reply,
                401 => {
                    let body = reply
                        .read_bounded(SMALL_BODY_BYTES, IDLE_TIMEOUT)
                        .await
                        .map_err(transport)?;
                    let (code, _) = error_fields(&body);
                    if code.as_deref() != Some(INVALID_TOKEN) {
                        return Err(OnlineError::Transport(
                            "the server refused the request's authorization",
                        ));
                    }
                    // One renewal and one repeat, never a loop.
                    if renewed {
                        return Err(OnlineError::TokenRejected);
                    }
                    renewed = true;
                    token = self.token(&credentials, Some(&token)).await?;
                }
                _ => return Err(self.failure(reply).await),
            }
        };
        let encoding = reply
            .header("content-encoding")
            .unwrap_or("identity")
            .to_ascii_lowercase();
        if encoding != "identity" {
            return Err(OnlineError::Integrity("unexpected content encoding"));
        }
        if let Some(length) = reply
            .header("content-length")
            .and_then(|v| v.parse::<u64>().ok())
        {
            if length != entry.size {
                return Err(OnlineError::Integrity(
                    "the file size differs from the index",
                ));
            }
        }
        let directory = downloads_dir(&self.dir);
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&directory)
            .map_err(|_| OnlineError::Storage("the download folder could not be created"))?;
        // The blocking operation returns an owning RAII guard: if cancellation drops its
        // waiter, the eventual result still unlinks itself, even after a late create.
        let partial = partial_file(directory.clone()).await?;
        let handle = partial
            .as_file()
            .try_clone()
            .map_err(|_| OnlineError::Storage("the download file could not be opened"))?;
        let mut file = tokio::fs::File::from_std(handle);
        let mut hasher = Sha256::new();
        let mut received = 0u64;
        while let Some(chunk) = reply.chunk(IDLE_TIMEOUT).await.map_err(transport)? {
            received = received
                .checked_add(chunk.len() as u64)
                .ok_or(OnlineError::Integrity("the download size overflowed"))?;
            if received > entry.size || received > MAX_FILE_BYTES {
                return Err(OnlineError::Integrity(
                    "the server sent more than the index announced",
                ));
            }
            hasher.update(&chunk);
            file.write_all(&chunk)
                .await
                .map_err(|_| OnlineError::Storage("the download could not be written"))?;
            self.received.store(received, Ordering::Relaxed);
        }
        if received != entry.size {
            return Err(OnlineError::Integrity("the download ended early"));
        }
        if format!("{:x}", hasher.finalize()) != entry.sha256 {
            return Err(OnlineError::Integrity(
                "the download does not match its announced digest",
            ));
        }
        file.sync_all()
            .await
            .map_err(|_| OnlineError::Storage("the download could not be stored"))?;
        drop(file);
        partial
            .persist(self.cache_path(&entry.id))
            .map_err(|_| OnlineError::Storage("the download could not be stored"))?;
        let keep = entry.id.clone();
        let dir = directory;
        let _ = tokio::task::spawn_blocking(move || evict(&dir, &keep)).await;
        Ok(())
    }

    /// The verified cached file of `entry_id` for streaming into an install job.
    pub async fn open_cached(&self, entry_id: &str) -> Result<(File, Entry), OnlineError> {
        let entry = self.entry(entry_id).await?;
        let meta = std::fs::symlink_metadata(self.cache_path(&entry.id))
            .map_err(|_| OnlineError::NotFound)?;
        if !meta.is_file() || meta.nlink() != 1 || meta.len() != entry.size {
            return Err(OnlineError::Integrity("the stored copy is not usable"));
        }
        let file = super::store::open_regular(&self.cache_path(&entry.id), entry.size)
            .map_err(|_| OnlineError::Storage("the stored copy could not be opened"))?;
        Ok((file, entry))
    }

    /// The cached file's bytes after a fresh digest check (for the legacy import path).
    pub async fn read_verified(&self, entry_id: &str) -> Result<(Vec<u8>, Entry), OnlineError> {
        let entry = self.entry(entry_id).await?;
        let path = self.cache_path(&entry.id);
        let expected = entry.sha256.clone();
        let size = entry.size;
        let bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, OnlineError> {
            let meta = std::fs::symlink_metadata(&path).map_err(|_| OnlineError::NotFound)?;
            if !meta.is_file() || meta.len() != size {
                return Err(OnlineError::Integrity("the stored copy is not usable"));
            }
            let mut bytes = Vec::with_capacity(size as usize);
            super::store::open_regular(&path, size)
                .and_then(|f| f.take(size + 1).read_to_end(&mut bytes))
                .map_err(|_| OnlineError::Storage("the stored copy could not be read"))?;
            if bytes.len() as u64 != size || format!("{:x}", Sha256::digest(&bytes)) != expected {
                return Err(OnlineError::Integrity(
                    "the stored copy no longer matches its digest",
                ));
            }
            Ok(bytes)
        })
        .await
        .map_err(|_| OnlineError::Storage("the stored copy could not be read"))??;
        Ok((bytes, entry))
    }
}

/// A detached blocking create must retain cleanup ownership after its async waiter is gone.
async fn partial_file(directory: PathBuf) -> Result<tempfile::NamedTempFile, OnlineError> {
    tokio::task::spawn_blocking(move || {
        tempfile::Builder::new()
            .prefix("download-")
            .suffix(PART_SUFFIX)
            .tempfile_in(directory)
    })
    .await
    .map_err(|_| OnlineError::Storage("the download file could not be created"))?
    .map_err(|_| OnlineError::Storage("the download file could not be created"))
}

async fn cancelled_signal(mut cancelled: watch::Receiver<bool>) {
    while !*cancelled.borrow() {
        if cancelled.changed().await.is_err() {
            // The sender lives in the slot until the task ends; a dropped sender never cancels.
            std::future::pending::<()>().await;
        }
    }
}

fn digest_of(path: &Path, size: u64) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    let mut file = super::store::open_regular(path, size)?.take(size + 1);
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        hasher.update(&buffer[..n]);
    }
    if total != size {
        return Err(std::io::Error::other("size differs"));
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Keeps the cache within its file and byte bounds, removing the oldest files, never `keep`.
fn evict(dir: &Path, keep: &str) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(std::time::SystemTime, u64, PathBuf, bool)> = read
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(FILE_SUFFIX))
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            let is_keep = e.file_name().to_string_lossy() == format!("{keep}{FILE_SUFFIX}");
            Some((meta.modified().ok()?, meta.len(), e.path(), is_keep))
        })
        .collect();
    files.sort_by_key(|f| f.0);
    let mut count = files.len();
    let mut bytes: u64 = files.iter().map(|f| f.1).sum();
    for (_, len, path, is_keep) in files {
        if count <= MAX_CACHED_FILES && bytes <= MAX_CACHED_BYTES {
            break;
        }
        if is_keep {
            continue;
        }
        if std::fs::remove_file(path).is_ok() {
            count -= 1;
            bytes -= len;
        }
    }
}
