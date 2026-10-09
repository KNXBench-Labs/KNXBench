//! Shared retained context, separately verified from each normalized model image.

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

pub(super) fn remember(
    conn: &Connection,
    snapshot: &NativeSnapshot,
) -> Result<String, HistoryError> {
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
    if exists {
        if read(conn, &content_hash)? != context {
            return Err(HistoryError::Invalid("retained context identity collision"));
        }
    } else {
        conn.execute("INSERT INTO project_history_context (content_hash, format_version, image, image_hash) VALUES (?1, ?2, ?3, ?4)", params![content_hash, FORMAT_VERSION, image, image_hash(&image)])?;
    }
    Ok(content_hash)
}

pub(super) fn read(conn: &Connection, hash: &str) -> Result<NativeSnapshot, HistoryError> {
    let (format, image, image_hash): (i64, Vec<u8>, String) = conn.query_row(
        "SELECT format_version, image, image_hash FROM project_history_context WHERE content_hash = ?1", [hash],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional()?.ok_or(HistoryError::Invalid("referenced retained context is missing"))?;
    if format != FORMAT_VERSION {
        return Err(HistoryError::Invalid(
            "unsupported retained-context version",
        ));
    }
    let context = NativeSnapshot::decode(&image, &image_hash)?;
    if context.project != empty_project() || context.semantic_hash()? != hash {
        return Err(HistoryError::Invalid(
            "retained context contains a model or has an invalid semantic identity",
        ));
    }
    Ok(context)
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
