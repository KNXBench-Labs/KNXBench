//! Persists ordered plain-text notes and their independent allocation high-water mark.

use crate::StoreError;
use knx_core::note::{
    validate_notes, ProjectNote, ProjectNoteTarget as Target, MAX_NOTES_TEXT_BYTES,
    MAX_NOTE_BODY_BYTES, MAX_NOTE_TITLE_BYTES, MAX_PROJECT_NOTES,
};
use knx_core::{
    AreaId, BuildingPartId, ComObjectInstanceId, DeviceId, GroupAddressId, GroupRangeId,
    InstallationId, LineId, Project, ProjectNoteId,
};
use rusqlite::{params, Connection};
fn invalid(reason: impl Into<String>) -> StoreError {
    StoreError::NoteIntegrity(reason.into())
}

pub(crate) fn check(project: &Project) -> Result<(), StoreError> {
    validate_notes(project, &project.notes).map_err(|e| invalid(format!("{e:?}")))?;
    if project
        .notes
        .iter()
        .any(|n| n.id().0 > project.ids.peek_project_note())
    {
        return Err(invalid("note allocator is below an existing note id"));
    }
    Ok(())
}
pub(crate) fn write(conn: &Connection, project: &Project) -> Result<(), StoreError> {
    check(project)?;
    conn.execute(
        "INSERT INTO project_note_state(id,last_id) VALUES(0,?1)",
        [project.ids.peek_project_note()],
    )?;
    let mut statement=conn.prepare("INSERT INTO project_note(id,position,target_kind,target_id,title,body,include_in_documentation) VALUES(?1,?2,?3,?4,?5,?6,?7)")?;
    for (position, note) in project.notes.iter().enumerate() {
        let (kind, id) = encode_target(note.target());
        statement.execute(params![
            note.id().0,
            position as i64,
            kind,
            id,
            note.title(),
            note.body(),
            note.include_in_documentation()
        ])?;
    }
    Ok(())
}
fn encode_target(target: Target) -> (&'static str, Option<u32>) {
    match target {
        Target::Project => ("project", None),
        Target::Installation(id) => ("installation", Some(u32::from(id.0))),
        Target::Area(id) => ("area", Some(id.0)),
        Target::Line(id) => ("line", Some(id.0)),
        Target::BuildingPart(id) => ("buildingPart", Some(id.0)),
        Target::Device(id) => ("device", Some(id.0)),
        Target::GroupRange(id) => ("groupRange", Some(id.0)),
        Target::GroupAddress(id) => ("groupAddress", Some(id.0)),
        Target::ComObject(id) => ("comObject", Some(id.0)),
    }
}
fn decode_target(kind: &str, id: Option<i64>) -> Result<Target, StoreError> {
    if kind == "project" {
        return if id.is_none() {
            Ok(Target::Project)
        } else {
            Err(invalid("project note target must not have an entity id"))
        };
    }
    let id = u32::try_from(id.ok_or_else(|| invalid("entity note target has no id"))?)
        .map_err(|_| invalid("note target id exceeds its domain"))?;
    Ok(match kind {
        "installation" => Target::Installation(InstallationId(
            u8::try_from(id).map_err(|_| invalid("note installation id exceeds u8"))?,
        )),
        "area" => Target::Area(AreaId(id)),
        "line" => Target::Line(LineId(id)),
        "buildingPart" => Target::BuildingPart(BuildingPartId(id)),
        "device" => Target::Device(DeviceId(id)),
        "groupRange" => Target::GroupRange(GroupRangeId(id)),
        "groupAddress" => Target::GroupAddress(GroupAddressId(id)),
        "comObject" => Target::ComObject(ComObjectInstanceId(id)),
        _ => return Err(invalid("unknown note target kind")),
    })
}
pub(crate) fn load(conn: &Connection, project: &mut Project) -> Result<(), StoreError> {
    let state_rows: i64 =
        conn.query_row("SELECT count(*) FROM project_note_state", [], |r| r.get(0))?;
    if state_rows != 1 {
        return Err(invalid("note allocator singleton is missing or duplicated"));
    }
    let counter: i64 = conn.query_row(
        "SELECT last_id FROM project_note_state WHERE id=0",
        [],
        |r| r.get(0),
    )?;
    let counter = u32::try_from(counter).map_err(|_| invalid("note allocator is outside u32"))?;
    let (count,total):(i64,i64)=conn.query_row("SELECT count(*),coalesce(sum(length(CAST(title AS BLOB))+length(CAST(body AS BLOB))),0) FROM project_note",[],|r|Ok((r.get(0)?,r.get(1)?)))?;
    if count > MAX_PROJECT_NOTES as i64 || total > MAX_NOTES_TEXT_BYTES as i64 {
        return Err(invalid("note collection exceeds native load budget"));
    }
    let mut statement=conn.prepare("SELECT id,position,target_kind,target_id,length(CAST(title AS BLOB)),length(CAST(body AS BLOB)),title,body,include_in_documentation FROM project_note ORDER BY position")?;
    let mut rows = statement.query([])?;
    let mut notes = Vec::new();
    while let Some(row) = rows.next()? {
        let id = ProjectNoteId(
            u32::try_from(row.get::<_, i64>(0)?).map_err(|_| invalid("note id exceeds u32"))?,
        );
        if row.get::<_, i64>(1)? != notes.len() as i64 {
            return Err(invalid("note positions are not contiguous"));
        }
        let title_size: i64 = row.get(4)?;
        let body_size: i64 = row.get(5)?;
        if title_size > MAX_NOTE_TITLE_BYTES as i64 || body_size > MAX_NOTE_BODY_BYTES as i64 {
            return Err(invalid("note text exceeds native load budget"));
        }
        let target = decode_target(&row.get::<_, String>(2)?, row.get(3)?)?;
        let include: i64 = row.get(8)?;
        if !matches!(include, 0 | 1) {
            return Err(invalid("note documentation inclusion is not boolean"));
        }
        let note = ProjectNote::new(id, target, row.get(6)?, row.get(7)?)
            .map_err(|e| invalid(format!("{e:?}")))?
            .with_documentation(include == 1);
        notes.push(note);
    }
    validate_notes(project, &notes).map_err(|e| invalid(format!("{e:?}")))?;
    project.notes = notes;
    project.ids = project.ids.clone().with_project_note_count(counter);
    Ok(())
}
