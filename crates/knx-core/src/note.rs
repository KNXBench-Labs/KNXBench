//! Validates bounded plain-text project notes with typed entity targets.

use std::collections::BTreeSet;
use std::fmt;

use crate::{
    AreaId, BuildingPartId, ComObjectInstanceId, DeviceId, GroupAddressId, GroupRangeId,
    InstallationId, LineId, Project, ProjectNoteId,
};

pub const MAX_PROJECT_NOTES: usize = 4096;
pub const MAX_NOTE_TITLE_BYTES: usize = 512;
pub const MAX_NOTE_BODY_BYTES: usize = 64 * 1024;
pub const MAX_NOTES_TEXT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProjectNoteTarget {
    Project,
    Installation(InstallationId),
    Area(AreaId),
    Line(LineId),
    BuildingPart(BuildingPartId),
    Device(DeviceId),
    GroupRange(GroupRangeId),
    GroupAddress(GroupAddressId),
    ComObject(ComObjectInstanceId),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteError {
    InvalidId,
    EmptyTitle,
    TitleBudget,
    BodyBudget,
    ControlCharacter,
    NoteCountBudget,
    TotalTextBudget,
    DuplicateId(ProjectNoteId),
    MissingTarget(ProjectNoteTarget),
    AmbiguousTarget(ProjectNoteTarget),
}
/// A note an edit would leave pointing at an entity the edit removes.
///
/// [`crate::CommandError::NotesOrphaned`] lists these so a refusal can name
/// every affected note instead of only the first dangling target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrphanedNote {
    pub id: ProjectNoteId,
    pub target: ProjectNoteTarget,
    pub title: String,
}
impl fmt::Display for ProjectNoteTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Project => f.write_str("the project"),
            Self::Installation(id) => write!(f, "installation {id}"),
            Self::Area(id) => write!(f, "area {id}"),
            Self::Line(id) => write!(f, "line {id}"),
            Self::BuildingPart(id) => write!(f, "building part {id}"),
            Self::Device(id) => write!(f, "device {id}"),
            Self::GroupRange(id) => write!(f, "group range {id}"),
            Self::GroupAddress(id) => write!(f, "group address {id}"),
            Self::ComObject(id) => write!(f, "communication object {id}"),
        }
    }
}
impl fmt::Display for NoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId => f.write_str("note id 0 is not a valid identity"),
            Self::EmptyTitle => f.write_str("a note needs a title that is not blank"),
            Self::TitleBudget => write!(f, "the title exceeds {MAX_NOTE_TITLE_BYTES} bytes"),
            Self::BodyBudget => write!(f, "the note text exceeds {MAX_NOTE_BODY_BYTES} bytes"),
            Self::ControlCharacter => f.write_str(
                "the text contains a control character; only tab and line breaks are allowed",
            ),
            Self::NoteCountBudget => write!(f, "a project holds at most {MAX_PROJECT_NOTES} notes"),
            Self::TotalTextBudget => {
                write!(
                    f,
                    "all notes together would exceed {MAX_NOTES_TEXT_BYTES} bytes of text"
                )
            }
            Self::DuplicateId(id) => write!(f, "note id {id} is used twice"),
            Self::MissingTarget(target) => write!(f, "its target {target} does not exist"),
            Self::AmbiguousTarget(target) => {
                write!(
                    f,
                    "its target {target} occurs more than once in the project"
                )
            }
        }
    }
}
impl std::error::Error for NoteError {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectNote {
    id: ProjectNoteId,
    target: ProjectNoteTarget,
    title: String,
    body: String,
    include_in_documentation: bool,
}
impl ProjectNote {
    pub fn new(
        id: ProjectNoteId,
        target: ProjectNoteTarget,
        title: String,
        body: String,
    ) -> Result<Self, NoteError> {
        if id.0 == 0 {
            return Err(NoteError::InvalidId);
        }
        if title.len() > MAX_NOTE_TITLE_BYTES {
            return Err(NoteError::TitleBudget);
        }
        if body.len() > MAX_NOTE_BODY_BYTES {
            return Err(NoteError::BodyBudget);
        }
        if title.trim().is_empty() {
            return Err(NoteError::EmptyTitle);
        }
        if title
            .chars()
            .chain(body.chars())
            .any(|c| c.is_control() && !matches!(c, '\t' | '\r' | '\n'))
        {
            return Err(NoteError::ControlCharacter);
        }
        Ok(Self {
            id,
            target,
            title,
            body,
            include_in_documentation: false,
        })
    }
    pub fn id(&self) -> ProjectNoteId {
        self.id
    }
    pub fn target(&self) -> ProjectNoteTarget {
        self.target
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn body(&self) -> &str {
        &self.body
    }
    pub fn include_in_documentation(&self) -> bool {
        self.include_in_documentation
    }
    pub fn with_documentation(mut self, include: bool) -> Self {
        self.include_in_documentation = include;
        self
    }
    /// The same note (identity, text, inclusion) attached to `target`.
    /// Target existence is checked where the note is admitted, not here.
    pub fn with_target(mut self, target: ProjectNoteTarget) -> Self {
        self.target = target;
        self
    }
}
/// Notes that resolved in `before`, still carry the same target in `after`
/// and now point at an entity `after` no longer has, in `after`'s order.
pub(crate) fn orphaned_notes(before: &Project, after: &Project) -> Vec<OrphanedNote> {
    after
        .notes
        .iter()
        .filter(|note| {
            before
                .notes
                .iter()
                .any(|old| old.id == note.id && old.target == note.target)
                && note.target.validate(before).is_ok()
                && matches!(
                    note.target.validate(after),
                    Err(NoteError::MissingTarget(_))
                )
        })
        .map(|note| OrphanedNote {
            id: note.id,
            target: note.target,
            title: note.title.clone(),
        })
        .collect()
}
impl ProjectNoteTarget {
    pub fn validate(self, project: &Project) -> Result<(), NoteError> {
        let count = match self {
            Self::Project => 1,
            Self::Installation(id) => project.installations.iter().filter(|i| i.id == id).count(),
            Self::Area(id) => project
                .installations
                .iter()
                .flat_map(|i| &i.topology.areas)
                .filter(|v| v.id == id)
                .count(),
            Self::Line(id) => project
                .installations
                .iter()
                .flat_map(|i| &i.topology.lines)
                .filter(|v| v.id == id)
                .count(),
            Self::BuildingPart(id) => project
                .installations
                .iter()
                .flat_map(|i| &i.buildings)
                .filter(|v| v.id == id)
                .count(),
            Self::Device(id) => usize::from(project.devices.get(id).is_some()),
            Self::GroupRange(id) => project
                .installations
                .iter()
                .flat_map(|i| &i.group_ranges)
                .filter(|v| v.id == id)
                .count(),
            Self::GroupAddress(id) => project
                .installations
                .iter()
                .flat_map(|i| &i.group_addresses)
                .filter(|v| v.id == id)
                .count(),
            Self::ComObject(id) => usize::from(
                project
                    .devices
                    .com_object(id)
                    .is_some_and(|object| project.devices.get(object.device).is_some()),
            ),
        };
        match count {
            0 => Err(NoteError::MissingTarget(self)),
            1 => Ok(()),
            _ => Err(NoteError::AmbiguousTarget(self)),
        }
    }
}
/// Checks collection-wide budgets, identity uniqueness and exact target existence.
pub fn validate_notes(project: &Project, notes: &[ProjectNote]) -> Result<(), NoteError> {
    if notes.len() > MAX_PROJECT_NOTES {
        return Err(NoteError::NoteCountBudget);
    }
    let mut ids = BTreeSet::new();
    let mut bytes = 0usize;
    for note in notes {
        if !ids.insert(note.id) {
            return Err(NoteError::DuplicateId(note.id));
        }
        bytes += note.title.len() + note.body.len();
        if bytes > MAX_NOTES_TEXT_BYTES {
            return Err(NoteError::TotalTextBudget);
        }
        note.target.validate(project)?;
    }
    Ok(())
}
