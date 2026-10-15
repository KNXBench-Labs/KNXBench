//! Source-bound System-2 tool metadata, never an executable download plan.
//!
//! The historical tool-only AbsCObjSeg record and exact XML pointer witnesses
//! support this bounded projection, not a universal LsmIdx-to-object mapping.
//! See docs/research/2026-10-14-system2-archive-mirrors.md. No step is removed
//! from ProgramCode and no public download admission changes.

use std::fmt;

use crate::code::{LoadStep, ProgramCode};

/// One source position, retained in its original order.
#[derive(Debug, PartialEq, Eq)]
pub enum ProjectedStep<'a> {
    /// A source declaration, not a translated or executable wire operation.
    Retained(&'a LoadStep),
    /// Exact TaskSegment/5 pointer witness; tool-side interpretation only.
    ToolTableReference {
        /// Absolute communication-object table address.
        address: u16,
    },
}

/// A source view; deliberately unrelated to executable download plan types.
#[derive(Debug, PartialEq, Eq)]
pub struct ToolMetadata<'a> {
    /// Resolved, independently cross-checked source table pointer.
    pub table_address: u16,
    /// Every original source position, including tool metadata.
    pub steps: Vec<ProjectedStep<'a>>,
}

/// Why the exact metadata witness cannot be established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataError {
    /// Not one of the witnessed System-2 product-procedure masks.
    UnsupportedProgram,
    /// Needs exactly one unmerged source procedure.
    ProcedureShape,
    /// Missing, ambiguous, out-of-bounds or unresolvable table placement.
    TablePlacement,
    /// Missing or ambiguous TaskCtrl2 on application machine 3.
    TaskControl,
    /// Source pointer declarations disagree.
    PointerMismatch,
    /// Missing or repeated tool-side reference.
    ReferenceCount,
    /// An unmodelled step or another use of machine 5 must not be skipped.
    UnsupportedStep {
        /// Zero-based source position.
        index: usize,
    },
}

impl fmt::Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "System-2 tool metadata is not established: {self:?}")
    }
}

impl std::error::Error for MetadataError {}

/// Cross-checks the exact source witness without constructing any wire step.
///
/// This is a documented source inference, not normative XML execution semantics.
/// Even success leaves System-2 public programming refused.
pub fn project_tool_metadata(code: &ProgramCode) -> Result<ToolMetadata<'_>, MetadataError> {
    if !matches!(
        code.mask_version.as_deref(),
        Some("MV-0020" | "MV-0021" | "MV-0025")
    ) || code.load_procedure_style.as_deref() != Some("ProductProcedure")
    {
        return Err(MetadataError::UnsupportedProgram);
    }
    let [procedure] = code.load_procedures.as_slice() else {
        return Err(MetadataError::ProcedureShape);
    };
    if procedure.merge_id.is_some() {
        return Err(MetadataError::ProcedureShape);
    }
    let placement = code
        .com_object_table
        .as_ref()
        .ok_or(MetadataError::TablePlacement)?;
    let id = placement
        .code_segment
        .as_ref()
        .ok_or(MetadataError::TablePlacement)?;
    let offset = placement.offset.ok_or(MetadataError::TablePlacement)?;
    let mut matching = code.segments.iter().filter(|segment| segment.id == *id);
    let segment = matching.next().ok_or(MetadataError::TablePlacement)?;
    if matching.next().is_some() || offset >= segment.size {
        return Err(MetadataError::TablePlacement);
    }
    let table_address = segment
        .address
        .checked_add(offset)
        .and_then(|address| u16::try_from(address).ok())
        .ok_or(MetadataError::TablePlacement)?;
    let mut controls = procedure.steps.iter().filter_map(|step| match step {
        LoadStep::TaskCtrl2 { lsm, address, .. } => Some((*lsm, *address)),
        _ => None,
    });
    let (lsm, control_address) = controls.next().ok_or(MetadataError::TaskControl)?;
    if lsm != 3 || controls.next().is_some() {
        return Err(MetadataError::TaskControl);
    }
    if control_address != table_address {
        return Err(MetadataError::PointerMismatch);
    }
    let mut references = 0;
    let mut steps = Vec::with_capacity(procedure.steps.len());
    for (index, step) in procedure.steps.iter().enumerate() {
        let projected = match step {
            LoadStep::TaskSegment { lsm: 5, address } => {
                if *address != table_address {
                    return Err(MetadataError::PointerMismatch);
                }
                references += 1;
                ProjectedStep::ToolTableReference { address: *address }
            }
            LoadStep::Unload { lsm: 5 }
            | LoadStep::Load { lsm: 5 }
            | LoadStep::LoadCompleted { lsm: 5 }
            | LoadStep::AbsSegment { lsm: 5, .. }
            | LoadStep::TaskCtrl1 { lsm: 5, .. }
            | LoadStep::TaskCtrl2 { lsm: 5, .. }
            | LoadStep::CompareProp {
                object_index: 5, ..
            }
            | LoadStep::Unmodelled { .. } => return Err(MetadataError::UnsupportedStep { index }),
            _ => ProjectedStep::Retained(step),
        };
        steps.push(projected);
    }
    if references != 1 {
        return Err(MetadataError::ReferenceCount);
    }
    Ok(ToolMetadata {
        table_address,
        steps,
    })
}
