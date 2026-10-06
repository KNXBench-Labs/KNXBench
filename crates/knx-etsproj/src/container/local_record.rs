//! Checks that a ZIP record's local header and data agree with its central record.
//
// AR18 re-check round 3, N11. The `zip` reader takes every size from the
// central record and reads the local header only to skip its name and extra
// field. A central record that claims fewer bytes than its local record
// holds therefore hides them; with an Info-ZIP Unicode Path the hidden
// record was the real topology document. Other readers (a streaming one
// such as `bsdtar`) follow the local headers instead, so any disagreement
// means two readers see two archives. The rules mirror the product reader's
// (`knx-productdb`'s package validator); this crate may not depend on it.

use std::ops::Range;

use flate2::{Decompress, FlushDecompress, Status};

/// The local header names a member other than its central record.
pub(super) const NAME: &str = "its local header names a different member";
/// The local header, its name or its extra field runs past the archive.
pub(super) const TRUNCATED: &str = "its local header is missing, truncated or malformed";
/// Flags or compression method differ.
pub(super) const METADATA: &str =
    "its local header disagrees with the central record (flags/method)";
/// CRC or sizes differ (with bit 3: are neither zeros nor the central values).
pub(super) const SIZES: &str = "its local header disagrees with the central record (sizes/CRC)";
/// No data descriptor carrying the central CRC and sizes follows the data.
pub(super) const DESCRIPTOR: &str =
    "its data descriptor disagrees with the central record (sizes/CRC)";
/// The Unicode Path fields (0x7075) of the two headers differ.
pub(super) const UNICODE: &str = "its local and central Unicode Path fields differ";
/// The record leaves bytes between itself and the next record (or the
/// central directory), or overlaps it.
pub(super) const LAYOUT: &str = "its local record does not end where the next record begins";

const LOCAL_SIGNATURE: &[u8] = b"PK\x03\x04";
const DESCRIPTOR_SIGNATURE: &[u8] = b"PK\x07\x08";
const LOCAL_HEADER_LEN: usize = 30;
const DATA_DESCRIPTOR_FLAG: u16 = 1 << 3;
const DEFLATED: u16 = 8;
const ZIP64_FIELD: u16 = 0x0001;
const UNICODE_PATH_FIELD: u16 = 0x7075;
/// The 32-bit size value that defers to a zip64 extra field.
const ZIP64_PLACEHOLDER: u32 = u32::MAX;

/// What the central directory says about one record: the raw fields from
/// the archive's own walk, and the CRC and sizes the `zip` reader resolved
/// (zip64 included) and will read with.
pub(super) struct Central<'a> {
    pub name: &'a [u8],
    pub flags: u16,
    pub method: u16,
    pub extra: &'a [u8],
    pub crc: u32,
    pub compressed: u64,
    pub size: u64,
}

/// Where one record's parts sit in the archive.
pub(super) struct Local {
    /// The member's stored (possibly compressed) bytes.
    pub data: Range<usize>,
    /// The whole record: header, data and any data descriptor.
    pub record: Range<usize>,
}

/// Compares the local record at `start` in `bytes` with `central`:
/// name, flags, method, CRC and both sizes (zip64-resolved); with bit 3,
/// zeros or the central values in the local header and a descriptor
/// (signature optional) right after the data that carries the central
/// values; and equal Unicode Path fields, or none in either header.
/// Returns the record's extent.
pub(super) fn check(bytes: &[u8], start: usize, central: &Central) -> Result<Local, &'static str> {
    let header = bytes
        .get(start..start.saturating_add(LOCAL_HEADER_LEN))
        .filter(|header| header.starts_with(LOCAL_SIGNATURE))
        .ok_or(TRUNCATED)?;
    let u16_at = |at: usize| u16::from_le_bytes([header[at], header[at + 1]]);
    let u32_at = |at: usize| {
        u32::from_le_bytes([header[at], header[at + 1], header[at + 2], header[at + 3]])
    };
    let name_start = start + LOCAL_HEADER_LEN;
    let extra_start = name_start + usize::from(u16_at(26));
    let data_start = extra_start + usize::from(u16_at(28));
    let name = bytes.get(name_start..extra_start).ok_or(TRUNCATED)?;
    let extra = bytes.get(extra_start..data_start).ok_or(TRUNCATED)?;
    if name != central.name {
        return Err(NAME);
    }
    if (u16_at(6), u16_at(8)) != (central.flags, central.method) {
        return Err(METADATA);
    }
    let local_fields = extra_fields(extra).ok_or(TRUNCATED)?;
    let zip64 = local_fields
        .iter()
        .find(|(id, _)| *id == ZIP64_FIELD)
        .map(|(_, body)| *body);
    let (size, compressed) = local_sizes(u32_at(22), u32_at(18), zip64).ok_or(TRUNCATED)?;
    let data_end = usize::try_from(central.compressed)
        .ok()
        .and_then(|len| data_start.checked_add(len))
        .filter(|end| *end <= bytes.len())
        .ok_or(TRUNCATED)?;
    let declared = (u32_at(14), compressed, size);
    let actual = (central.crc, central.compressed, central.size);
    let end = if central.flags & DATA_DESCRIPTOR_FLAG != 0 {
        // APPNOTE 4.4.4 wants zeros here, but Info-ZIP's `zip -e` writes
        // the real values as well as the descriptor; either is truthful.
        if declared != (0, 0, 0) && declared != actual {
            return Err(SIZES);
        }
        descriptor_end(bytes, data_end, central, zip64.is_some()).ok_or(DESCRIPTOR)?
    } else {
        if declared != actual {
            return Err(SIZES);
        }
        data_end
    };
    let central_fields = extra_fields(central.extra).ok_or(TRUNCATED)?;
    if unicode_paths(&local_fields) != unicode_paths(&central_fields) {
        return Err(UNICODE);
    }
    Ok(Local {
        data: data_start..data_end,
        record: start..end,
    })
}

/// Whether a directory record holds anything (round 3, N12). Java's
/// `ZipOutputStream` and `jar` write an empty directory as a deflated empty
/// stream, which carries nothing; a stream that inflates to bytes, or that
/// ends before its stored bytes do, carries data. `data` is the record's
/// stored bytes, already checked against both headers by [`check`].
pub(super) fn directory_carries_data(central: &Central, data: &[u8]) -> bool {
    if central.size > 0 {
        return true;
    }
    if data.is_empty() {
        return false;
    }
    central.method != DEFLATED || !inflates_to_nothing(data)
}

/// True for a complete deflate stream that produces no byte and ends
/// exactly at the end of `stream`.
fn inflates_to_nothing(stream: &[u8]) -> bool {
    let mut inflater = Decompress::new(false);
    let mut out = [0u8; 1];
    matches!(
        inflater.decompress(stream, &mut out, FlushDecompress::Finish),
        Ok(Status::StreamEnd)
    ) && inflater.total_out() == 0
        && inflater.total_in() == stream.len() as u64
}

/// The local sizes `(uncompressed, compressed)`, a zip64 placeholder taken
/// from the local zip64 field, which APPNOTE 4.5.3 requires to carry both
/// sizes in a local header (uncompressed first). `None` when a placeholder
/// has no value to stand for.
fn local_sizes(size: u32, compressed: u32, zip64: Option<&[u8]>) -> Option<(u64, u64)> {
    if size != ZIP64_PLACEHOLDER && compressed != ZIP64_PLACEHOLDER {
        return Some((size.into(), compressed.into()));
    }
    let body = zip64?;
    let wide = |at: usize| {
        body.get(at..at + 8)
            .map(|b| u64::from_le_bytes(b.try_into().expect("eight bytes")))
    };
    let size = match size {
        ZIP64_PLACEHOLDER => wide(0)?,
        size => size.into(),
    };
    let compressed = match compressed {
        ZIP64_PLACEHOLDER => wide(8)?,
        compressed => compressed.into(),
    };
    Some((size, compressed))
}

/// The end of the data descriptor at `at`, with or without its signature,
/// if it carries the central CRC and sizes. Sizes are eight bytes wide
/// when the local header has a zip64 field (APPNOTE 4.3.9.2).
fn descriptor_end(bytes: &[u8], at: usize, central: &Central, wide: bool) -> Option<usize> {
    let width = if wide { 8 } else { 4 };
    let sized = |b: &[u8]| -> u64 {
        b.iter()
            .rev()
            .fold(0u64, |value, byte| (value << 8) | u64::from(*byte))
    };
    for signature in [DESCRIPTOR_SIGNATURE.len(), 0] {
        if signature > 0 && bytes.get(at..at + signature) != Some(DESCRIPTOR_SIGNATURE) {
            continue;
        }
        let start = at + signature;
        let end = start + 4 + 2 * width;
        let Some(body) = bytes.get(start..end) else {
            continue;
        };
        let crc = u32::from_le_bytes([body[0], body[1], body[2], body[3]]);
        let values = (crc, sized(&body[4..4 + width]), sized(&body[4 + width..]));
        if values == (central.crc, central.compressed, central.size) {
            return Some(end);
        }
    }
    None
}

/// The fields of an extra block as `(id, body)`, or `None` when one runs
/// past the block. Fewer than four trailing bytes are padding, which `zip`
/// skips as well.
fn extra_fields(block: &[u8]) -> Option<Vec<(u16, &[u8])>> {
    let mut fields = Vec::new();
    let mut at = 0;
    while block.len() - at >= 4 {
        let id = u16::from_le_bytes([block[at], block[at + 1]]);
        let len = usize::from(u16::from_le_bytes([block[at + 2], block[at + 3]]));
        fields.push((id, block.get(at + 4..at + 4 + len)?));
        at += 4 + len;
    }
    Some(fields)
}

fn unicode_paths<'a>(fields: &[(u16, &'a [u8])]) -> Vec<&'a [u8]> {
    fields
        .iter()
        .filter(|(id, _)| *id == UNICODE_PATH_FIELD)
        .map(|(_, body)| *body)
        .collect()
}
