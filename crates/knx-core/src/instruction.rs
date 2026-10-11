//! Normalized installation instructions, independent of imported formatting.
//!
//! Pure value module; project/native admission is a separate integration gate.

pub const MAX_DOCUMENT_BYTES: usize = 64 * 1024;
pub const MAX_DOCUMENT_RUNS: usize = 4096;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InstructionStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionRun {
    pub text: String,
    pub style: InstructionStyle,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionError {
    ByteLimit,
    RunLimit,
    EmptyRun,
    AdjacentStyle,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstructionDocument {
    runs: Vec<InstructionRun>,
}
impl InstructionDocument {
    /// No trimming, Unicode normalization, markup interpretation or rewriting.
    pub fn new(runs: Vec<InstructionRun>) -> Result<Self, InstructionError> {
        if runs.len() > MAX_DOCUMENT_RUNS {
            return Err(InstructionError::RunLimit);
        }
        let mut bytes = 0_usize;
        let mut previous = None;
        for run in &runs {
            if run.text.is_empty() {
                return Err(InstructionError::EmptyRun);
            }
            bytes = bytes
                .checked_add(run.text.len())
                .ok_or(InstructionError::ByteLimit)?;
            if bytes > MAX_DOCUMENT_BYTES {
                return Err(InstructionError::ByteLimit);
            }
            if previous == Some(run.style) {
                return Err(InstructionError::AdjacentStyle);
            }
            previous = Some(run.style);
        }
        Ok(Self { runs })
    }
    pub fn runs(&self) -> &[InstructionRun] {
        &self.runs
    }
    pub fn plain_text(&self) -> String {
        self.runs.iter().map(|run| run.text.as_str()).collect()
    }
}
