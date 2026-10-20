//! Bounded synthetic project streams; no external corpus or proprietary grammar.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

pub const EARLY: &str = "M-0001/M-0001_A-2.xml";
pub const LATE_MANUFACTURER: &str = "M-0001/Z-stream.xml";
pub const LATE_OPAQUE: &str = "z-stream.bin";
pub const NESTED_LEAF: &str = "P-0001/z-stream.bin";
pub const WITNESS: &[u8] = b"decoded project stream witness";

#[derive(Clone, Copy, Debug)]
pub enum Layer {
    Outer,
    PlainNested,
    ZipCrypto,
    CompressedCarrier,
}

pub struct Member {
    name: String,
    plain: Vec<u8>,
    stream: Option<Vec<u8>>,
    encrypted: bool,
}

pub fn stored(name: &str, plain: &[u8]) -> Member {
    Member {
        name: name.into(),
        plain: plain.into(),
        stream: None,
        encrypted: false,
    }
}

// One complete RFC-1951 stored block, deliberately noncanonical alignment bits.
// The ZIP method is still DEFLATE, not ZIP Store. Every fixture is < 64 KiB.
pub fn deflated(name: &str, plain: &[u8], trailer: bool) -> Member {
    let len = u16::try_from(plain.len()).unwrap();
    let mut stream = vec![0xf9];
    stream.extend(len.to_le_bytes());
    stream.extend((!len).to_le_bytes());
    stream.extend(plain);
    if trailer {
        stream.extend([0x00, 0xa5]);
    }
    Member {
        stream: Some(stream),
        ..stored(name, plain)
    }
}

fn crc_step(mut crc: u32, byte: u8) -> u32 {
    crc ^= u32::from(byte);
    for _ in 0..8 {
        crc = (crc >> 1) ^ if crc & 1 == 0 { 0 } else { 0xedb8_8320 };
    }
    crc
}
fn crc32(bytes: &[u8]) -> u32 {
    !bytes.iter().fold(!0, |crc, &byte| crc_step(crc, byte))
}

// Independent fixture encryption, not the production decryptor. Only the public
// knx-testsupport credential is used; header/check and plaintext are deterministic.
fn encrypted_stream(stream: &[u8], crc: u32) -> Vec<u8> {
    let mut keys = [0x1234_5678u32, 0x2345_6789, 0x3456_7890];
    let update = |keys: &mut [u32; 3], byte| {
        keys[0] = crc_step(keys[0], byte);
        keys[1] = keys[1]
            .wrapping_add(keys[0] & 255)
            .wrapping_mul(134_775_813)
            .wrapping_add(1);
        keys[2] = crc_step(keys[2], (keys[1] >> 24) as u8);
    };
    for &byte in knx_testsupport::ZIPCRYPTO_MINIMAL_PASSWORD.as_bytes() {
        update(&mut keys, byte);
    }
    let mut header = [0x5a; 12];
    header[11] = (crc >> 24) as u8;
    header
        .into_iter()
        .chain(stream.iter().copied())
        .map(|byte| {
            let temp = (keys[2] as u16) | 2;
            let result = byte ^ (temp.wrapping_mul(temp ^ 1) >> 8) as u8;
            update(&mut keys, byte);
            result
        })
        .collect()
}

// Reuse zip's writer for layout/offsets. Only the chosen stored raw payloads'
// method, plaintext CRC/size and encryption flag are patched in both headers.
// No second ZIP layout builder, descriptors, ZIP64 or guessed XML are needed.
pub fn archive(members: &[Member]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for member in members {
        writer.start_file(&member.name, options).unwrap();
        let raw = member.stream.as_ref().unwrap_or(&member.plain);
        if member.encrypted {
            writer
                .write_all(&encrypted_stream(raw, crc32(&member.plain)))
                .unwrap();
        } else {
            writer.write_all(raw).unwrap();
        }
    }
    let mut bytes = writer.finish().unwrap().into_inner();
    let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let offsets: Vec<_> = (0..zip.len())
        .map(|index| {
            let file = zip.by_index_raw(index).unwrap();
            (
                file.header_start() as usize,
                file.central_header_start() as usize,
            )
        })
        .collect();
    drop(zip);
    for (member, (local, central)) in members.iter().zip(offsets) {
        if member.stream.is_none() {
            continue;
        }
        for (offset, method_at, crc_at, size_at) in [(local, 8, 14, 22), (central, 10, 16, 24)] {
            bytes[offset + method_at..offset + method_at + 2].copy_from_slice(&8u16.to_le_bytes());
            bytes[offset + crc_at..offset + crc_at + 4]
                .copy_from_slice(&crc32(&member.plain).to_le_bytes());
            bytes[offset + size_at..offset + size_at + 4]
                .copy_from_slice(&u32::try_from(member.plain.len()).unwrap().to_le_bytes());
        }
        if member.encrypted {
            for at in [local + 6, central + 8] {
                let flags = u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap()) | 1;
                bytes[at..at + 2].copy_from_slice(&flags.to_le_bytes());
            }
        }
    }
    bytes
}

pub fn manufacturer(tag: &str) -> Vec<u8> {
    let mut template =
        zip::ZipArchive::new(Cursor::new(knx_testsupport::minimal_knxproj_bytes())).unwrap();
    let mut bytes = String::new();
    template
        .by_name("M-0001/M-0001_A-1.xml")
        .unwrap()
        .read_to_string(&mut bytes)
        .unwrap();
    // XML comment changes content identity without inventing manufacturer grammar.
    bytes
        .replace("</KNX>", &format!("<!-- synthetic {tag} -->\n</KNX>"))
        .into_bytes()
}

pub fn project(layer: Layer, trailer: bool, late_manufacturer: bool) -> Vec<u8> {
    project_with_damage(layer, trailer, late_manufacturer, false)
}

pub fn invalid_zipcrypto_project() -> Vec<u8> {
    project_with_damage(Layer::ZipCrypto, false, false, true)
}

fn project_with_damage(
    layer: Layer,
    trailer: bool,
    late_manufacturer: bool,
    invalid_encrypted: bool,
) -> Vec<u8> {
    let mut template =
        zip::ZipArchive::new(Cursor::new(knx_testsupport::minimal_knxproj_bytes())).unwrap();
    let mut outer = Vec::new();
    let mut inner = Vec::new();
    for index in 0..template.len() {
        let mut file = template.by_index(index).unwrap();
        let mut plain = Vec::new();
        file.read_to_end(&mut plain).unwrap();
        let name = file.name();
        if !matches!(layer, Layer::Outer) && name.starts_with("P-0001/") {
            inner.push(stored(name.strip_prefix("P-0001/").unwrap(), &plain));
        } else {
            outer.push(stored(name, &plain));
        }
    }
    outer.push(stored(EARLY, &manufacturer("early-new")));
    match layer {
        Layer::Outer => outer.push(if late_manufacturer {
            deflated(LATE_MANUFACTURER, &manufacturer("late-new"), trailer)
        } else {
            deflated(LATE_OPAQUE, WITNESS, trailer)
        }),
        Layer::PlainNested | Layer::ZipCrypto | Layer::CompressedCarrier => {
            let mut leaf = deflated(
                LATE_OPAQUE,
                WITNESS,
                trailer && !matches!(layer, Layer::CompressedCarrier),
            );
            leaf.encrypted = matches!(layer, Layer::ZipCrypto);
            if invalid_encrypted {
                assert!(leaf.encrypted);
                leaf.stream = Some(vec![0x07]); // Reserved BTYPE, after a valid cipher header.
            }
            inner.push(leaf);
            let payload = archive(&inner);
            outer.push(if matches!(layer, Layer::CompressedCarrier) {
                deflated("P-0001.zip", &payload, trailer)
            } else {
                stored("P-0001.zip", &payload)
            });
        }
    }
    archive(&outer)
}

pub fn password(layer: Layer) -> Option<knx_etsproj::ProjectPassword> {
    (!matches!(layer, Layer::Outer))
        .then(|| knx_etsproj::ProjectPassword::new(knx_testsupport::ZIPCRYPTO_MINIMAL_PASSWORD))
}

// A complete flat synthetic directory snapshot: paths, main DBs, all sidecar
// bytes/absence and fixed source archive. Errors other than absence are fatal.
// Tests use closed DELETE-journal DBs; this is not live-WAL concurrency evidence.
pub fn disk_snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            assert!(path.is_file(), "unexpected directory in synthetic snapshot");
            (
                path.strip_prefix(dir).unwrap().to_owned(),
                std::fs::read(path).unwrap(),
            )
        })
        .collect()
}
