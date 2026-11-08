//! Shared retained context, separately verified from each normalized model image.

use super::trust::KnownContext;
use super::{image_hash, HistoryError, NativeSnapshot, FORMAT_VERSION};
use knx_core::{Language, Project};
use rusqlite::{params, Connection, OptionalExtension};

fn empty_project() -> Project {
    Project::new(Language("en".into()))
}

pub(super) fn model_image(project: &Project) -> Result<Vec<u8>, HistoryError> {
    NativeSnapshot {
        project: project.clone(),
        opaque: vec![],
        manufacturer_refs: vec![],
    }
    .encode()
}

/// Stores the retained context of `snapshot` unless present and returns it as
/// known. A `known` context holding the same value is confirmed by the SHA-256
/// of its stored bytes instead of being encoded and decoded again, or taken as
/// is when this transaction already verified it (`verified`).
pub(super) fn remember(
    conn: &Connection,
    snapshot: &NativeSnapshot,
    known: Option<&KnownContext>,
    verified: bool,
) -> Result<KnownContext, HistoryError> {
    if let Some(known) = known.filter(|known| known.holds(snapshot)) {
        if verified || verify_known(conn, known)? {
            return Ok(known.clone());
        }
    }
    let context = NativeSnapshot {
        project: empty_project(),
        opaque: snapshot.opaque.clone(),
        manufacturer_refs: snapshot.manufacturer_refs.clone(),
    };
    let (image, content_hash) = context.encode_with_identity()?;
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM project_history_context WHERE content_hash = ?1)",
        [&content_hash],
        |row| row.get(0),
    )?;
    let stored_hash = if exists {
        let (stored, stored_hash) = read_with_image_hash(conn, &content_hash)?;
        if stored != context {
            return Err(HistoryError::Invalid("retained context identity collision"));
        }
        stored_hash
    } else {
        let stored_hash = image_hash(&image);
        conn.execute("INSERT INTO project_history_context (content_hash, format_version, image, image_hash) VALUES (?1, ?2, ?3, ?4)", params![content_hash, FORMAT_VERSION, image, stored_hash])?;
        stored_hash
    };
    Ok(KnownContext {
        opaque: context.opaque,
        manufacturer_refs: context.manufacturer_refs,
        content_hash,
        image_hash: stored_hash,
    })
}

/// Whether the stored context row `known` names still holds exactly the bytes
/// it was known by; see [`verify_stored`].
pub(super) fn verify_known(conn: &Connection, known: &KnownContext) -> Result<bool, HistoryError> {
    verify_stored(conn, &known.content_hash, &known.image_hash)
}

/// Whether the stored context `content_hash` still holds the bytes it was
/// verified with: same recorded SHA-256 `expected_image`, and bytes with that
/// SHA-256. A row recorded differently answers `false`; bytes that no longer
/// match their recorded hash refuse.
pub(super) fn verify_stored(
    conn: &Connection,
    content_hash: &str,
    expected_image: &str,
) -> Result<bool, HistoryError> {
    let row: Option<(i64, Vec<u8>, String)> = conn
        .query_row(
            "SELECT format_version, image, image_hash FROM project_history_context WHERE content_hash = ?1",
            [content_hash],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let Some((format, image, stored_hash)) = row else {
        return Ok(false);
    };
    if format != FORMAT_VERSION || stored_hash != expected_image {
        return Ok(false);
    }
    if image_hash(&image) != stored_hash {
        return Err(HistoryError::Invalid(
            "retained context image hash mismatch",
        ));
    }
    Ok(true)
}

pub(super) fn read(conn: &Connection, hash: &str) -> Result<NativeSnapshot, HistoryError> {
    read_with_image_hash(conn, hash).map(|(context, _)| context)
}

/// [`read`] that also returns the SHA-256 recorded for the stored bytes.
pub(super) fn read_with_image_hash(
    conn: &Connection,
    hash: &str,
) -> Result<(NativeSnapshot, String), HistoryError> {
    let (format, image, image_hash): (i64, Vec<u8>, String) = conn.query_row(
        "SELECT format_version, image, image_hash FROM project_history_context WHERE content_hash = ?1", [hash],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional()?.ok_or(HistoryError::Invalid("referenced retained context is missing"))?;
    if format != FORMAT_VERSION {
        return Err(HistoryError::Invalid(
            "unsupported retained-context version",
        ));
    }
    let context = NativeSnapshot::decode_context(&image, &image_hash)?;
    if context.project != empty_project() || context.semantic_hash()? != hash {
        return Err(HistoryError::Invalid(
            "retained context contains a model or has an invalid semantic identity",
        ));
    }
    Ok((context, image_hash))
}

/// A context without value, for checks that only need every state to attach
/// the same one.
pub(super) fn unvalued() -> NativeSnapshot {
    NativeSnapshot {
        project: empty_project(),
        opaque: vec![],
        manufacturer_refs: vec![],
    }
}

pub(super) fn attach(
    image: &[u8],
    image_hash: &str,
    context: &NativeSnapshot,
) -> Result<NativeSnapshot, HistoryError> {
    let mut snapshot = NativeSnapshot::decode(image, image_hash)?;
    if !snapshot.opaque.is_empty() || !snapshot.manufacturer_refs.is_empty() {
        return Err(HistoryError::Invalid(
            "model image contains unbound retained context",
        ));
    }
    snapshot.opaque = context.opaque.clone();
    snapshot.manufacturer_refs = context.manufacturer_refs.clone();
    Ok(snapshot)
}

pub(super) fn collect_unreferenced(conn: &Connection) -> Result<(), HistoryError> {
    conn.execute(
        "DELETE FROM project_history_context WHERE content_hash NOT IN (
        SELECT context_hash FROM project_history_state UNION
        SELECT context_hash FROM project_history_stack UNION
        SELECT context_hash FROM project_history_version)",
        [],
    )?;
    Ok(())
}
