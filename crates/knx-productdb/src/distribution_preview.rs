//! Decodes budgeted single-frame PNG pixels and re-encodes owned passive previews without metadata.

use super::*;
use base64::Engine;
use std::io::{Cursor, Write};

/// Independent inclusive raster/metadata ceilings, always clamped to the default profile.
#[derive(Debug, Clone, Copy)]
pub struct DistributionResourceLimits {
    pub max_metadata_bytes: usize,
    pub max_encoded_bytes: usize,
    pub max_dimension: u32,
    pub max_pixels: u64,
    pub max_decoded_bytes: usize,
    pub max_decoder_bytes: usize,
    pub max_preview_bytes: usize,
    pub max_work_bytes: usize,
    pub max_png_chunks: usize,
}
impl Default for DistributionResourceLimits {
    fn default() -> Self {
        Self {
            max_metadata_bytes: 8 * 1024 * 1024,
            max_encoded_bytes: 8 * 1024 * 1024,
            max_dimension: 4096,
            max_pixels: 4 * 1024 * 1024,
            max_decoded_bytes: 16 * 1024 * 1024,
            max_decoder_bytes: 16 * 1024 * 1024,
            max_preview_bytes: 8 * 1024 * 1024,
            max_work_bytes: 128 * 1024 * 1024,
            max_png_chunks: 4096,
        }
    }
}
impl DistributionResourceLimits {
    pub(super) fn bounded(self) -> Self {
        let d = Self::default();
        Self {
            max_metadata_bytes: self.max_metadata_bytes.min(d.max_metadata_bytes),
            max_encoded_bytes: self.max_encoded_bytes.min(d.max_encoded_bytes),
            max_dimension: self.max_dimension.min(d.max_dimension),
            max_pixels: self.max_pixels.min(d.max_pixels),
            max_decoded_bytes: self.max_decoded_bytes.min(d.max_decoded_bytes),
            max_decoder_bytes: self.max_decoder_bytes.min(d.max_decoder_bytes),
            max_preview_bytes: self.max_preview_bytes.min(d.max_preview_bytes),
            max_work_bytes: self.max_work_bytes.min(d.max_work_bytes),
            max_png_chunks: self.max_png_chunks.min(d.max_png_chunks),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionResourcePreview {
    pub member_ordinal: u32,
    pub original_sha256: String,
    pub derived_sha256: String,
    pub width: u32,
    pub height: u32,
    pub transformation: String,
    pub png_base64: String,
}

fn png_error(e: png::DecodingError) -> &'static str {
    match e {
        png::DecodingError::LimitsExceeded => "resourceLimitDecoderBytes",
        _ => "pngDecodeFailed",
    }
}
fn charge(work: &mut usize, bytes: usize, cap: usize) -> std::result::Result<(), &'static str> {
    *work = work
        .checked_add(bytes)
        .filter(|&n| n <= cap)
        .ok_or("resourceLimitWorkBytes")?;
    Ok(())
}
/// Scan every chunk and its CRC with constant auxiliary memory before constructing a decoder.
fn chunks(data: &[u8], cap: usize) -> std::result::Result<(), &'static str> {
    let mut offset = 8usize;
    let mut count = 0usize;
    while offset < data.len() {
        count += 1;
        if count > cap {
            return Err("resourceLimitPngChunks");
        }
        let head = data
            .get(offset..offset.saturating_add(8))
            .ok_or("pngDecodeFailed")?;
        let len = u32::from_be_bytes(head[..4].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(12)
            .and_then(|n| n.checked_add(len))
            .filter(|&n| n <= data.len())
            .ok_or("pngDecodeFailed")?;
        let kind = &head[4..8];
        if matches!(kind, b"acTL" | b"fcTL" | b"fdAT") {
            return Err("animatedPngUnsupported");
        }
        if crc32fast::hash(&data[offset + 4..end - 4])
            != u32::from_be_bytes(data[end - 4..end].try_into().unwrap())
        {
            return Err("pngDecodeFailed");
        }
        if kind == b"IEND" {
            return if len == 0 && end == data.len() {
                Ok(())
            } else {
                Err("pngDecodeFailed")
            };
        }
        offset = end;
    }
    Err("pngDecodeFailed")
}
struct BoundedPng {
    bytes: Vec<u8>,
    cap: usize,
}
impl Write for BoundedPng {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > self.cap)
        {
            return Err(std::io::Error::other("preview byte ceiling"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) fn decode(
    data: &[u8],
    ordinal: u32,
    original: &str,
    limits: &DistributionLimits,
    work: &mut usize,
) -> std::result::Result<DistributionResourcePreview, &'static str> {
    let l = limits.resource;
    if data.len() > l.max_encoded_bytes {
        return Err("resourceLimitEncodedBytes");
    }
    charge(work, data.len().saturating_mul(2), l.max_work_bytes)?;
    chunks(data, l.max_png_chunks)?;
    let mut decoder = png::Decoder::new_with_limits(
        Cursor::new(data),
        png::Limits {
            bytes: l.max_decoder_bytes,
        },
    );
    decoder.set_ignore_text_chunk(true);
    decoder.set_ignore_iccp_chunk(true);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let info = decoder.read_header_info().map_err(png_error)?;
    let (width, height) = (info.width, info.height);
    if width > l.max_dimension || height > l.max_dimension {
        return Err("resourceLimitDimensions");
    }
    let pixels = u64::from(width) * u64::from(height);
    if pixels > l.max_pixels {
        return Err("resourceLimitPixels");
    }
    // RGBA8 is a conservative upper bound for the expanded/stripped output below.
    let decoded = pixels
        .checked_mul(4)
        .and_then(|n| usize::try_from(n).ok())
        .filter(|&n| n <= l.max_decoded_bytes)
        .ok_or("resourceLimitDecodedBytes")?;
    charge(work, decoded.saturating_mul(4), l.max_work_bytes)?;
    let mut reader = decoder.read_info().map_err(png_error)?;
    if reader.info().animation_control.is_some() {
        return Err("animatedPngUnsupported");
    }
    let size = reader
        .output_buffer_size()
        .filter(|&n| n <= l.max_decoded_bytes && n <= decoded)
        .ok_or("resourceLimitDecodedBytes")?;
    let mut pixels = vec![0; size];
    let frame = reader.next_frame(&mut pixels).map_err(png_error)?;
    reader.finish().map_err(png_error)?;
    let cap = l.max_preview_bytes.min(limits.max_derived_bytes);
    let mut output = BoundedPng {
        bytes: Vec::new(),
        cap,
    };
    {
        let mut encoder = png::Encoder::new(&mut output, width, height);
        encoder.set_color(frame.color_type);
        encoder.set_depth(frame.bit_depth);
        encoder.set_filter(png::Filter::NoFilter);
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder
            .write_header()
            .map_err(|_| "resourceLimitPreviewBytes")?;
        writer
            .write_image_data(&pixels[..frame.buffer_size()])
            .map_err(|_| "resourceLimitPreviewBytes")?;
        writer.finish().map_err(|_| "resourceLimitPreviewBytes")?;
    }
    Ok(DistributionResourcePreview {
        member_ordinal: ordinal,
        original_sha256: original.into(),
        derived_sha256: crate::sha256_hex(&output.bytes),
        width,
        height,
        transformation: "decoded-pixels-to-png-v1".into(),
        png_base64: base64::engine::general_purpose::STANDARD.encode(&output.bytes),
    })
}
