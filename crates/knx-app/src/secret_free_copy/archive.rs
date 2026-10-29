//! Rewrites a `.knxproj` (any ZIP) with secrets removed and unreadable members withheld.

use std::io::{Cursor, Read, Write};
use std::path::Path;

use knx_etsproj::Container;

use super::xml::{self, Base, Content};
use super::{io_error, Findings, Needles, SecretFreeCopy, SecretFreeCopyError};

/// The outer archive is level 0; a `.knxproj`'s project payload is level 1
/// and a product's plug-in archive inside it can be level 2. Anything
/// deeper is withheld rather than unpacked without end.
const MAX_NESTING: usize = 3;

/// The largest source accepted: one retained file (ADR-0142).
const MAX_SOURCE_BYTES: u64 = knx_core::limits::MAX_RETAINED_BYTES;

pub(super) fn copy(source: &Path) -> Result<SecretFreeCopy, SecretFreeCopyError> {
    // Pass 1 learns every value the redaction removes; pass 2 redacts
    // again and withholds any member that still contains one. Reading the
    // file twice keeps one copy of it in memory, not two.
    let mut learned = Findings::default();
    let first =
        Container::open_outer(read_source(source)?).map_err(SecretFreeCopyError::Archive)?;
    rewrite(first, "", 0, &mut learned, None)?;
    let needles = learned.needles();

    let original = read_source(source)?;
    let container =
        Container::open_outer(original.clone()).map_err(SecretFreeCopyError::Archive)?;
    let mut findings = Findings::default();
    let rewritten = rewrite(container, "", 0, &mut findings, Some(&needles))?;
    let unchanged = findings.redactions.is_empty() && findings.withheld.is_empty();
    Ok(findings.finish(if unchanged { original } else { rewritten }))
}

fn read_source(source: &Path) -> Result<Vec<u8>, SecretFreeCopyError> {
    let file = std::fs::File::open(source).map_err(io_error("opening the import file"))?;
    let declared = file
        .metadata()
        .map_err(io_error("reading the import file's size"))?
        .len();
    if declared > MAX_SOURCE_BYTES {
        return Err(SecretFreeCopyError::TooLarge {
            bytes: declared,
            limit: MAX_SOURCE_BYTES,
        });
    }
    let mut bytes = Vec::new();
    file.take(MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error("reading the import file"))?;
    if bytes.len() as u64 > MAX_SOURCE_BYTES {
        return Err(SecretFreeCopyError::TooLarge {
            bytes: bytes.len() as u64,
            limit: MAX_SOURCE_BYTES,
        });
    }
    Ok(bytes)
}

/// The archive with every member redacted, kept or withheld. `prefix`
/// names the member this archive is (empty for the outer file).
fn rewrite(
    mut container: Container,
    prefix: &str,
    depth: usize,
    findings: &mut Findings,
    residual: Option<&Needles<'_>>,
) -> Result<Vec<u8>, SecretFreeCopyError> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let entries = container.entries().to_vec();
    for entry in entries {
        let location = if prefix.is_empty() {
            entry.path.clone()
        } else {
            format!("{prefix}!{}", entry.path)
        };
        let mut member = Findings::default();
        let outcome = redact_member(
            &mut container,
            &entry.path,
            &location,
            depth,
            &mut member,
            residual,
        )?;
        match outcome {
            Ok(bytes) => {
                findings.absorb(member);
                let options = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated)
                    .last_modified_time(zip::DateTime::default())
                    .large_file(bytes.len() as u64 >= u64::from(u32::MAX));
                writer
                    .start_file(entry.path.as_str(), options)
                    .and_then(|()| writer.write_all(&bytes).map_err(Into::into))
                    .map_err(|error| SecretFreeCopyError::Write(error.to_string()))?;
            }
            Err(reason) => {
                findings.values.append(&mut member.values);
                findings.withhold(&location, &reason);
            }
        }
    }
    let cursor = writer
        .finish()
        .map_err(|error| SecretFreeCopyError::Write(error.to_string()))?;
    Ok(cursor.into_inner())
}

/// One member's copy (`Ok`) or the reason it is withheld (`Err`). The outer
/// `Result` is a failure of the whole copy.
fn redact_member(
    container: &mut Container,
    path: &str,
    location: &str,
    depth: usize,
    findings: &mut Findings,
    residual: Option<&Needles<'_>>,
) -> Result<Result<Vec<u8>, String>, SecretFreeCopyError> {
    // `Container::read` looks names up case-insensitively; a second member
    // differing only in case would read the first one's bytes.
    if container.find(path).map(|found| found.path.as_str()) != Some(path) {
        return Ok(Err(
            "another member has the same name in different case".into()
        ));
    }
    let Ok(bytes) = container.read(path) else {
        return Ok(Err(
            "cannot be read (encrypted, damaged or too large)".into()
        ));
    };
    let content = xml::sniff(&bytes);
    let redacted = match content {
        Content::Xml => {
            let base = format!("{location}:");
            let base = Base {
                text: &base,
                root_in_base: false,
            };
            match xml::redact(&bytes, &base, findings) {
                Ok(Some(redacted)) => redacted,
                Ok(None) => bytes,
                Err(reason) => return Ok(Err(reason.into())),
            }
        }
        Content::Zip if depth + 1 >= MAX_NESTING => {
            return Ok(Err("archive nested too deeply to inspect".into()));
        }
        Content::Zip => match nested(bytes, location, depth + 1, findings, residual)? {
            Ok(redacted) => redacted,
            Err(reason) => return Ok(Err(reason)),
        },
        Content::WideText => return Ok(Err("UTF-16/32 text is not inspected".into())),
        Content::Other => bytes,
    };
    // A nested archive's own members were checked one by one; its
    // compressed bytes would hide a value anyway.
    if content != Content::Zip && residual.is_some_and(|needles| needles.any_in(&redacted)) {
        return Ok(Err("still contains a removed secret value".into()));
    }
    Ok(Ok(redacted))
}

/// A nested archive, rewritten; its original bytes when nothing inside it
/// changed, so an untouched payload stays byte-identical.
pub(super) fn nested(
    bytes: Vec<u8>,
    location: &str,
    depth: usize,
    findings: &mut Findings,
    residual: Option<&Needles<'_>>,
) -> Result<Result<Vec<u8>, String>, SecretFreeCopyError> {
    let Ok(container) = Container::open_outer(bytes.clone()) else {
        return Ok(Err(
            "nested archive cannot be read (encrypted, password-protected or damaged)".into(),
        ));
    };
    let mut inner = Findings::default();
    let rewritten = rewrite(container, location, depth, &mut inner, residual)?;
    let unchanged = inner.redactions.is_empty() && inner.withheld.is_empty();
    findings.absorb(inner);
    Ok(Ok(if unchanged { bytes } else { rewritten }))
}
