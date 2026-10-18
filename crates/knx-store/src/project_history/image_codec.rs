//! Independently versioned, bounded encoding of native history image bytes.

use super::{HistoryError, MAX_DECODED_IMAGE_BYTES, MAX_IMAGE_BYTES};
use flate2::{write::ZlibEncoder, Compression, Decompress, FlushDecompress, Status};
use std::{
    borrow::Cow,
    io::{self, Write},
};

const MAGIC: &[u8; 8] = b"KNXHST\0\x01";
const HEADER_BYTES: usize = MAGIC.len() + 8;
const CHUNK_BYTES: usize = 64 * 1024;

pub(super) fn encode(bytes: &[u8], stored_limit: usize) -> Result<Vec<u8>, HistoryError> {
    // Compress once above the legacy threshold even when the context has a larger storage budget.
    encode_bounded(bytes, stored_limit, MAX_DECODED_IMAGE_BYTES)
}

pub(super) fn decode(bytes: &[u8], stored_limit: usize) -> Result<Cow<'_, [u8]>, HistoryError> {
    decode_bounded(bytes, stored_limit, MAX_DECODED_IMAGE_BYTES)
}

/// Prefix-only resource accounting. Unsupported encodings retain their stored
/// charge and are still refused by the normal hash/codec checks before decoding.
pub(super) fn decoded_size(prefix: &[u8], stored_size: usize) -> Result<usize, HistoryError> {
    if prefix.starts_with(MAGIC) {
        declared_length(prefix, MAX_DECODED_IMAGE_BYTES)
    } else {
        Ok(stored_size)
    }
}

fn declared_length(header: &[u8], decoded_limit: usize) -> Result<usize, HistoryError> {
    if header.len() < HEADER_BYTES || !header.starts_with(MAGIC) {
        return Err(HistoryError::Invalid(
            "unsupported or incomplete history image codec",
        ));
    }
    let length = u64::from_le_bytes(
        header[MAGIC.len()..HEADER_BYTES]
            .try_into()
            .expect("checked header"),
    );
    if length > decoded_limit as u64 {
        return Err(HistoryError::ImageLimit(
            "decoded native snapshot exceeds 4 GiB",
        ));
    }
    if length < 100 {
        return Err(HistoryError::Invalid(
            "invalid decoded native snapshot length",
        ));
    }
    Ok(length as usize)
}

struct BoundedBytes {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for BoundedBytes {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("encoded history image exceeds its budget"));
        }
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn encode_bounded(
    bytes: &[u8],
    stored_limit: usize,
    decoded_limit: usize,
) -> Result<Vec<u8>, HistoryError> {
    if bytes.len() > decoded_limit {
        return Err(HistoryError::ImageLimit(
            "decoded native snapshot exceeds 4 GiB",
        ));
    }
    if bytes.len() <= stored_limit.min(MAX_IMAGE_BYTES) {
        return Ok(bytes.to_vec());
    }
    if stored_limit < HEADER_BYTES {
        return Err(HistoryError::ImageLimit(
            "encoded native snapshot exceeds its model/context budget",
        ));
    }
    let mut header = MAGIC.to_vec();
    header.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    let output = BoundedBytes {
        bytes: header,
        limit: stored_limit,
    };
    let mut encoder = ZlibEncoder::new(output, Compression::fast());
    encoder.write_all(bytes).map_err(|_| {
        HistoryError::ImageLimit("encoded native snapshot exceeds its model/context budget")
    })?;
    let output = encoder.finish().map_err(|_| {
        HistoryError::ImageLimit("encoded native snapshot exceeds its model/context budget")
    })?;
    Ok(output.bytes)
}

fn decode_bounded(
    bytes: &[u8],
    stored_limit: usize,
    decoded_limit: usize,
) -> Result<Cow<'_, [u8]>, HistoryError> {
    if bytes.len() > stored_limit {
        return Err(HistoryError::ImageLimit(
            "encoded native snapshot exceeds its model/context budget",
        ));
    }
    if bytes.starts_with(b"SQLite format 3\0") {
        if bytes.len() > decoded_limit {
            return Err(HistoryError::ImageLimit(
                "decoded native snapshot exceeds 4 GiB",
            ));
        }
        return Ok(Cow::Borrowed(bytes));
    }
    let expected = declared_length(bytes, decoded_limit)?;
    let length = expected as u64;
    let input = &bytes[HEADER_BYTES..];
    let mut decoder = Decompress::new(true);
    let mut output = Vec::new();
    let mut chunk = [0u8; CHUNK_BYTES];
    loop {
        let before_in = decoder.total_in();
        let before_out = decoder.total_out();
        let status = decoder
            .decompress(
                &input[before_in as usize..],
                &mut chunk,
                FlushDecompress::None,
            )
            .map_err(|_| HistoryError::Invalid("corrupt compressed native snapshot"))?;
        if decoder.total_out() > length {
            return Err(HistoryError::Invalid(
                "decoded native snapshot exceeds declared length",
            ));
        }
        let produced = (decoder.total_out() - before_out) as usize;
        // Reserve only for bytes actually produced, never the untrusted header.
        output.extend_from_slice(&chunk[..produced]);
        if status == Status::StreamEnd {
            if decoder.total_in() as usize != input.len() || output.len() != expected {
                return Err(HistoryError::Invalid(
                    "compressed native snapshot length or trailing bytes mismatch",
                ));
            }
            return Ok(Cow::Owned(output));
        }
        if decoder.total_in() == before_in && decoder.total_out() == before_out {
            return Err(HistoryError::Invalid(
                "incomplete compressed native snapshot",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw() -> Vec<u8> {
        let mut bytes = vec![b'x'; 16 * 1024];
        bytes[..16].copy_from_slice(b"SQLite format 3\0");
        bytes
    }

    #[test]
    fn codec_retains_legacy_bytes_and_roundtrips_large_images() {
        let raw = raw();
        let legacy = encode_bounded(&raw, raw.len(), raw.len()).unwrap();
        assert_eq!(legacy, raw);
        assert!(matches!(
            decode_bounded(&legacy, raw.len(), raw.len()).unwrap(),
            Cow::Borrowed(_)
        ));
        let compressed = encode_bounded(&raw, 1024, raw.len()).unwrap();
        assert!(compressed.starts_with(MAGIC));
        assert_eq!(&*decode_bounded(&compressed, 1024, raw.len()).unwrap(), raw);
    }

    #[test]
    fn codec_refuses_unknown_short_corrupt_and_trailing_streams() {
        let good = encode_bounded(&raw(), 1024, raw().len()).unwrap();
        let mut variants = vec![
            good[..7].to_vec(),
            good[..15].to_vec(),
            good[..good.len() - 1].to_vec(),
        ];
        let mut unknown = good.clone();
        unknown[7] = 2;
        variants.push(unknown);
        let mut corrupt = good.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        variants.push(corrupt);
        let mut trailing = good.clone();
        trailing.push(0);
        variants.push(trailing);
        let mut concatenated = good.clone();
        concatenated.extend_from_slice(&good[HEADER_BYTES..]);
        variants.push(concatenated);
        for bytes in variants {
            assert!(decode_bounded(&bytes, 1024, raw().len()).is_err());
        }
    }

    #[test]
    fn codec_refuses_declared_length_overflow_underflow_and_excessive_budget() {
        let good = encode_bounded(&raw(), 1024, raw().len()).unwrap();
        for length in [
            0,
            99,
            (raw().len() - 1) as u64,
            (raw().len() + 1) as u64,
            u64::MAX,
        ] {
            let mut bytes = good.clone();
            bytes[8..16].copy_from_slice(&length.to_le_bytes());
            assert!(decode_bounded(&bytes, 1024, raw().len() + 1).is_err());
        }
        assert!(decode_bounded(&good, good.len() - 1, raw().len()).is_err());
        assert!(decode_bounded(&good, 1024, raw().len() - 1).is_err());
        assert!(encode_bounded(&raw(), 1024, raw().len() - 1).is_err());
        let mut bytes = raw();
        let mut state = 123456789u32;
        for b in &mut bytes[16..] {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *b = state as u8;
        }
        assert!(encode_bounded(&bytes, 1024, bytes.len()).is_err());
    }
}
