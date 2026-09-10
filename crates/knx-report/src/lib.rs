//! Renders a KNXBench project into one self-contained "project
//! documentation" HTML file — never called, described, or commit-messaged
//! as an ETS report or an ETS-compatible one, because no ETS-produced
//! report sample exists anywhere in this repository to be compatible with
//! (`docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`
//! §1).
//!
//! `knx-report` is pure: a `&knx_core::Project` in, a `String` and typed
//! warnings out. It knows nothing of the filesystem, HTTP, SQLite or the
//! system clock — the generation timestamp travels in through
//! [`ReportOptions`] so the same project renders to byte-identical HTML on
//! every call, which is what the determinism tests in Task 3 hold it to.
//! Depends on `knx-core` and `knx-projection` only (`xtask check-layering`
//! enforces this, the same rule `knx-csv` and `knx-projection` are each
//! held to).

pub mod html;
mod model;
mod testutil;

use chrono::{DateTime, Utc};

/// Caller-supplied inputs to [`render_html`] (Task 3) that must not be
/// re-derived inside this crate.
pub struct ReportOptions {
    /// Printed in the document header and nowhere read from the system
    /// clock — the caller (server, CLI, or a test) is the only source of
    /// "now", so the same project and the same timestamp always produce
    /// the same bytes.
    pub generated_at: DateTime<Utc>,
}

/// The rendered document plus everything structurally odd the walk found
/// along the way — devices in no line, group addresses in no range,
/// building parts with a dangling parent, and the like. Both are always
/// populated together: the document renders the same findings inline
/// (in its own "What this report does not contain" / anomalies text) so
/// the artifact carries its caveats even if `warnings` is discarded by a
/// caller (CLAUDE.md: never silently discard information).
pub struct HtmlReport {
    pub html: String,
    pub warnings: Vec<ReportWarning>,
}

/// One structurally odd thing the model walk found, e.g. a communication
/// object with no owning device, or a link to a group address that does
/// not exist.
pub struct ReportWarning {
    /// Where in the project, e.g. `"device 42"`, `"group address 1/2/3"`.
    pub location: String,
    pub detail: String,
}
