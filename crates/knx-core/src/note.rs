//! Validates bounded plain-text project notes with typed entity targets.

use std::collections::BTreeSet;

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
