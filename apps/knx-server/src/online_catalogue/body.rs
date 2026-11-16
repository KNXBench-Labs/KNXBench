//! A request body that streams a local file while re-checking its length and SHA-256 at the end.

use std::fs::File;
use std::io::Read;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::body::{Body, Bytes};
use http_body::{Body as HttpBody, Frame};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;

const CHUNK_BYTES: usize = 64 * 1024;
/// Chunks in flight between the reader thread and the consumer.
const QUEUE_CHUNKS: usize = 4;

struct ChannelBody {
    rx: mpsc::Receiver<std::io::Result<Bytes>>,
}

impl HttpBody for ChannelBody {
    type Data = Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, std::io::Error>>> {
        self.rx
            .poll_recv(cx)
            .map(|item| item.map(|r| r.map(Frame::data)))
    }
}

/// Streams `file` and ends the body with an error instead of a clean end when the bytes read are
/// not exactly `size` bytes with digest `sha256_hex`: a file changed since it was verified never
/// reaches the install job as a complete upload.
pub fn verified_file_body(mut file: File, size: u64, sha256_hex: String) -> Body {
    let (tx, rx) = mpsc::channel(QUEUE_CHUNKS);
    tokio::task::spawn_blocking(move || {
        let mut hasher = Sha256::new();
        let mut total = 0u64;
        let mut buffer = vec![0u8; CHUNK_BYTES];
        loop {
            let n = match file.read(&mut buffer) {
                Ok(n) => n,
                Err(e) => {
                    let _ = tx.blocking_send(Err(e));
                    return;
                }
            };
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > size {
                let _ = tx.blocking_send(Err(std::io::Error::other("the stored copy grew")));
                return;
            }
            hasher.update(&buffer[..n]);
            if tx
                .blocking_send(Ok(Bytes::copy_from_slice(&buffer[..n])))
                .is_err()
            {
                return;
            }
        }
        if total != size || format!("{:x}", hasher.finalize()) != sha256_hex {
            let _ = tx.blocking_send(Err(std::io::Error::other(
                "the stored copy no longer matches its digest",
            )));
        }
    });
    Body::new(ChannelBody { rx })
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use std::io::Write;

    fn file_with(bytes: &[u8]) -> File {
        let mut f = tempfile::tempfile().unwrap();
        f.write_all(bytes).unwrap();
        std::io::Seek::rewind(&mut f).unwrap();
        f
    }

    fn sha(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    #[tokio::test]
    async fn a_matching_file_streams_completely() {
        let data = vec![7u8; 200_000];
        let body = verified_file_body(file_with(&data), data.len() as u64, sha(&data));
        let collected = body.collect().await.unwrap().to_bytes();
        assert_eq!(collected.as_ref(), data.as_slice());
    }

    #[tokio::test]
    async fn a_changed_or_wrong_length_file_ends_in_an_error() {
        let data = vec![7u8; 1000];
        let wrong_digest = verified_file_body(file_with(&data), 1000, sha(b"other"));
        assert!(wrong_digest.collect().await.is_err());
        let short = verified_file_body(file_with(&data), 2000, sha(&data));
        assert!(short.collect().await.is_err());
        let long = verified_file_body(file_with(&data), 10, sha(&data));
        assert!(long.collect().await.is_err());
    }
}
