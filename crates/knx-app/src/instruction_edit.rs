//! Previewed normalized instruction edits, independent of UI or protocol.
use crate::instructions::{InstructionOrigin, InstructionSourceError, OriginalInstructions};
use knx_core::instruction::InstructionDocument;
use knx_core::{CommandStack, DeviceId, Project};
use knx_store::project_history::NativeSnapshot;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionRevision {
    pub project_incarnation: String,
    pub generation: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionEditError {
    Source(InstructionSourceError),
    InvalidRevision,
    Conflict,
    ConversionConsentRequired,
    Command,
    Persistence(String),
}

pub struct InstructionEditPreview {
    before: Project,
    revision: InstructionRevision,
    device: DeviceId,
    origin: InstructionOrigin,
    replacement: Option<InstructionDocument>,
    conversion_required: bool,
    no_op: bool,
}
impl InstructionEditPreview {
    pub fn device(&self) -> DeviceId {
        self.device
    }
    /// The imported original; `None` for a device made in KNXBench (Goal B4).
    pub fn original(&self) -> Option<&OriginalInstructions> {
        match &self.origin {
            InstructionOrigin::Imported(original) => Some(original),
            InstructionOrigin::Native => None,
        }
    }
    pub fn origin(&self) -> &InstructionOrigin {
        &self.origin
    }
    pub fn replacement(&self) -> Option<&InstructionDocument> {
        self.replacement.as_ref()
    }
    pub fn conversion_required(&self) -> bool {
        self.conversion_required
    }
    pub fn is_no_op(&self) -> bool {
        self.no_op
    }
}
impl std::fmt::Debug for InstructionEditPreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstructionEditPreview")
            .field("device", &self.device)
            .field("conversion_required", &self.conversion_required)
            .field("no_op", &self.no_op)
            .finish_non_exhaustive()
    }
}
#[derive(Debug, Clone, Copy)]
pub struct InstructionNativeTarget<'a> {
    pub path: &'a Path,
    pub history_generation: i64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionEditOutcome {
    pub changed: bool,
    pub native_generation: Option<i64>,
}

pub fn preview_instruction_edit(
    snapshot: &NativeSnapshot,
    revision: &InstructionRevision,
    device: DeviceId,
    replacement: Option<InstructionDocument>,
) -> Result<InstructionEditPreview, InstructionEditError> {
    if revision.project_incarnation.is_empty()
        || revision.project_incarnation.len() > 128
        || revision.generation > 9_007_199_254_740_991
    {
        return Err(InstructionEditError::InvalidRevision);
    }
    let origin =
        crate::instructions::instruction_origin(&snapshot.project, &snapshot.opaque, device)
            .map_err(InstructionEditError::Source)?;
    let current = snapshot.project.instructions.get(&device);
    let mut conversion_required = false;
    let mut no_op = current == replacement.as_ref();
    // Only an imported original can be shown in place of no override; a
    // native device without an override simply has no instructions.
    let imported_hints = match &origin {
        InstructionOrigin::Imported(original) => original.hints.as_ref(),
        InstructionOrigin::Native => None,
    };
    if current.is_none() && replacement.is_some() {
        if let Some(hints) = imported_hints {
            match &hints.preview {
                Ok(view) if view.unsupported.is_empty() => {
                    let runs = view
                        .runs
                        .iter()
                        .map(|run| knx_core::instruction::InstructionRun {
                            text: run.text.clone(),
                            style: knx_core::instruction::InstructionStyle {
                                bold: run.style.bold,
                                italic: run.style.italic,
                                underline: run.style.underline,
                            },
                        })
                        .collect();
                    if let Ok(document) = InstructionDocument::new(runs) {
                        no_op = replacement.as_ref() == Some(&document);
                    } else {
                        conversion_required = true;
                    }
                }
                _ => conversion_required = true,
            }
        }
    }
    Ok(InstructionEditPreview {
        before: snapshot.project.clone(),
        revision: revision.clone(),
        device,
        origin,
        replacement,
        conversion_required,
        no_op,
    })
}

pub fn apply_instruction_edit(
    snapshot: &mut NativeSnapshot,
    stack: &mut CommandStack,
    revision: &InstructionRevision,
    preview: &InstructionEditPreview,
    confirm_conversion: bool,
    native: Option<InstructionNativeTarget<'_>>,
) -> Result<InstructionEditOutcome, InstructionEditError> {
    use InstructionEditError as E;
    if revision != &preview.revision || snapshot.project != preview.before {
        return Err(E::Conflict);
    }
    let origin = crate::instructions::instruction_origin(
        &snapshot.project,
        &snapshot.opaque,
        preview.device,
    )
    .map_err(E::Source)?;
    if origin != preview.origin {
        return Err(E::Conflict);
    }
    if preview.conversion_required && !confirm_conversion {
        return Err(E::ConversionConsentRequired);
    }
    if preview.no_op {
        return Ok(InstructionEditOutcome {
            changed: false,
            native_generation: None,
        });
    }
    // Persist private candidate state first. No acknowledged failure may
    // mutate the caller's model, retained originals, or either history stack.
    let mut candidate = snapshot.clone();
    let mut history = stack.clone();
    history
        .do_command(
            &mut candidate.project,
            knx_core::Command::SetDeviceInstructions {
                device: preview.device,
                document: preview.replacement.clone(),
            },
        )
        .map_err(|_| E::Command)?;
    let native_generation = if let Some(target) = native {
        Some(
            crate::project_history::persist_working(
                target.path,
                &candidate,
                &history,
                target.history_generation,
            )
            .map_err(E::Persistence)?,
        )
    } else {
        None
    };
    *snapshot = candidate;
    *stack = history;
    Ok(InstructionEditOutcome {
        changed: true,
        native_generation,
    })
}
