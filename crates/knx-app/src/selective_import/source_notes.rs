//! Carries source notes along their remapped targets, by explicit choice (ADR-0138).
//!
//! A note whose target the merge brings in (or reuses) follows that entity's
//! destination identity. Every other note, including notes on the source
//! project itself, is outside the selection: the caller decides once whether
//! those are skipped (one report line each) or attached to the destination
//! project. Without that decision the merge is refused, so no note can
//! disappear or move without being reported.

use knx_core::note::{validate_notes, ProjectNote, ProjectNoteTarget as Target};
use knx_core::{
    AreaId, BuildingPartId, ComObjectInstanceId, DeviceId, GroupAddressId, GroupRangeId,
    InstallationId, LineId, Project,
};

use super::merge::mapping;
use super::{Counts, IdMapping, Selection, UnselectedNotes};

/// Where a source note's target went, or `None` when the merge does not
/// bring that entity along.
fn destination(target: Target, selection: &Selection, mappings: &[IdMapping]) -> Option<Target> {
    let mapped = |kind: &str, id: u32| {
        mappings
            .iter()
            .find(|m| m.kind == kind && m.source == id)
            .map(|m| m.target)
    };
    Some(match target {
        Target::Project => return None,
        Target::Installation(id) => {
            if id.0 != selection.source_installation {
                return None;
            }
            Target::Installation(InstallationId(selection.target_installation))
        }
        Target::Area(id) => Target::Area(AreaId(mapped("area", id.0)?)),
        Target::Line(id) => Target::Line(LineId(mapped("line", id.0)?)),
        Target::BuildingPart(id) => {
            Target::BuildingPart(BuildingPartId(mapped("building_part", id.0)?))
        }
        Target::Device(id) => Target::Device(DeviceId(mapped("device", id.0)?)),
        Target::GroupRange(id) => Target::GroupRange(GroupRangeId(mapped("group_range", id.0)?)),
        Target::GroupAddress(id) => {
            Target::GroupAddress(GroupAddressId(mapped("group_address", id.0)?))
        }
        Target::ComObject(id) => {
            Target::ComObject(ComObjectInstanceId(mapped("communication_object", id.0)?))
        }
    })
}

/// Appends the carried notes to `after` under fresh identities, in source
/// order, after every entity mapping is known. Counts and report lines say
/// what happened to each source note.
pub(super) fn carry(
    source: &Project,
    selection: &Selection,
    after: &mut Project,
    mappings: &mut Vec<IdMapping>,
    counts: &mut Counts,
    report: &mut Vec<String>,
) -> Result<(), String> {
    let resolved: Vec<_> = source
        .notes
        .iter()
        .map(|note| (note, destination(note.target(), selection, mappings)))
        .collect();
    let outside = resolved.iter().filter(|(_, to)| to.is_none()).count();
    let policy = match (outside, selection.unselected_notes) {
        (0, policy) => policy,
        (_, Some(policy)) => Some(policy),
        (count, None) => {
            let noun = if count == 1 { "note is" } else { "notes are" };
            return Err(format!(
                "{count} source {noun} attached to entities outside the selection; skip them or attach them to the project; nothing was imported"
            ));
        }
    };
    for (note, to) in resolved {
        let target = match (to, policy) {
            (Some(target), _) => {
                counts.mapped_notes += 1;
                target
            }
            (None, Some(UnselectedNotes::AttachToProject)) => {
                counts.attached_notes += 1;
                report.push(format!(
                    "Source note {} {:?} on {} is attached to the project: its target is outside the selection.",
                    note.id(),
                    note.title(),
                    note.target()
                ));
                Target::Project
            }
            (None, _) => {
                counts.skipped_notes += 1;
                report.push(format!(
                    "Source note {} {:?} on {} was skipped as chosen: its target is outside the selection.",
                    note.id(),
                    note.title(),
                    note.target()
                ));
                continue;
            }
        };
        let id = after
            .ids
            .next_project_note_id()
            .map_err(|e| e.to_string())?;
        let copied = ProjectNote::new(id, target, note.title().into(), note.body().into())
            .map_err(|e| format!("source note {} refused: {e}", note.id()))?
            .with_documentation(note.include_in_documentation());
        after.notes.push(copied);
        mappings.push(mapping("note", note.id().0, id.0, false));
    }
    validate_notes(after, &after.notes).map_err(|e| format!("imported source notes refused: {e}"))
}
