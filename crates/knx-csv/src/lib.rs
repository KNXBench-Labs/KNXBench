//! Native group-address CSV exchange and bounded address-only import profiles.
//!
//! The writer produces "KNXBench group-address CSV v1". The reader additionally
//! accepts four fixed column profiles; unsupported metadata is diagnosed and
//! never granted mutation authority. See `docs/contracts/group-address-csv-profiles.md`.
//! Neither path claims general interoperability with another application.
//!
//! `knx-csv` is pure: text in, typed rows/plans out, and project data in,
//! text out. It knows nothing of the filesystem, SQLite, HTTP, or
//! `AppState` — orchestration lives in the callers (`apps/knx-server`,
//! `apps/knx-cli`).

mod plan;
mod profile;
#[cfg(test)]
mod profile_tests;
mod read;
#[cfg(test)]
mod testutil;
mod write;

pub use plan::{
    plan_import, plan_import_into, CsvAffectedLink, CsvDestructiveAction, CsvDestructiveChange,
    CsvImportReport, ImportPlan,
};
pub use read::{
    parse_group_addresses, CsvAction, CsvProblem, CsvRow, IgnoredColumn, IgnoredColumnReason,
    ParsedCsv, Severity,
};
pub use write::{
    export_group_addresses, export_group_addresses_from, CsvExport, UnknownInstallation,
};
