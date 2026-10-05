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
mod render;
mod testutil;

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use knx_core::{ComObjectInstanceId, DeviceId, Project};

/// Renders `project` into one self-contained HTML "project documentation"
/// document, using `options.generated_at` as the only source of "now"
/// (Task 3). A two-line wrapper so the crate's one public entry point has
/// an obvious, stable location; all the actual walking lives in
/// `render.rs`.
pub fn render_html(project: &Project, options: &ReportOptions) -> HtmlReport {
    render::render(project, options)
}

/// Caller-supplied inputs to [`render_html`] that must not be re-derived
/// inside this crate.
pub struct ReportOptions {
    /// Printed in the document header and nowhere read from the system
    /// clock — the caller (server, CLI, or a test) is the only source of
    /// "now", so the same project and the same timestamp always produce
    /// the same bytes.
    pub generated_at: DateTime<Utc>,
    /// Language for report-owned prose. Product strings are already
    /// resolved by the caller into [`ReportDeviceData`] in this language.
    pub language: ReportLanguage,
    /// Content sections to include. Header, contents, and the limitations /
    /// warning section are structural and always remain present.
    pub sections: BTreeSet<ReportSection>,
    /// Product-database and parameter projections prepared by an outer
    /// application layer. A `BTreeMap` makes iteration deterministic even
    /// if a future renderer needs to walk this collection directly.
    pub device_data: BTreeMap<DeviceId, ReportDeviceData>,
}

impl ReportOptions {
    pub fn new(generated_at: DateTime<Utc>) -> Self {
        Self {
            generated_at,
            language: ReportLanguage::English,
            sections: ReportSection::ALL.into_iter().collect(),
            device_data: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportLanguage {
    English,
    German,
}

impl ReportLanguage {
    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::German => "de",
        }
    }

    pub(crate) fn text<'a>(self, english: &'a str, german: &'a str) -> &'a str {
        match self {
            Self::English => english,
            Self::German => german,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReportSection {
    Summary,
    Topology,
    Buildings,
    GroupAddresses,
    Devices,
}

impl ReportSection {
    pub const ALL: [Self; 5] = [
        Self::Summary,
        Self::Topology,
        Self::Buildings,
        Self::GroupAddresses,
        Self::Devices,
    ];
}

/// Caller-resolved, display-only data for one device. Raw project
/// identifiers remain in the document independently of this projection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportDeviceData {
    /// Raw manufacturer id used when its display name is absent or blank.
    pub manufacturer_reference: Option<String>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub application_program: Option<String>,
    /// Product/program relationship problems found by the composing layer.
    pub problems: Vec<String>,
    pub parameters: Vec<ReportField>,
    pub module_arguments: Vec<ReportField>,
    /// Caller-translated communication-object texts in the report language
    /// (AR10, KNOWN_LIMITATIONS §37). An absent entry or field keeps the
    /// project's own resolved text; the caller decides which texts may be
    /// translated at all.
    pub com_object_texts: BTreeMap<ComObjectInstanceId, ReportComObjectText>,
}

/// Display-only replacement texts for one communication object.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportComObjectText {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// One parameter value or module argument. `raw_value` is mandatory and is
/// always rendered. `display_value` may add a human-readable representation;
/// `problem` makes a failed resolution visible inline and as a warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportField {
    pub reference: String,
    pub name: Option<String>,
    pub raw_value: String,
    pub display_value: Option<String>,
    pub problem: Option<String>,
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
    /// A human-readable sentence naming what's odd, e.g. `"assigned to no
    /// line (Topology::unassigned)"`, `"parent building part 99 does not
    /// exist"`, or `"dpt is malformed and was kept verbatim: \"1.xxx\""`.
    pub detail: String,
}
