//! Native group-address CSV exchange and bounded address-only import profiles.
//!
//! The writer produces "KNXBench group-address CSV v1". The reader additionally
//! accepts eight fixed column profiles; unsupported metadata is diagnosed and
//! never granted mutation authority. See `docs/contracts/group-address-csv-profiles.md`.
//! Neither path claims general interoperability with another application.
//! Import bytes are decoded in the encoding the user chose ([`CsvEncoding`]).
//!
//! `knx-csv` is pure: text in, typed rows/plans out, and project data in,
//! text out. It knows nothing of the filesystem, SQLite, HTTP, or
//! `AppState` — orchestration lives in the callers (`apps/knx-server`,
//! `apps/knx-cli`).

mod encoding;
mod plan;
mod profile;
#[cfg(test)]
mod profile_tests;
mod read;
#[cfg(test)]
mod testutil;
mod write;

pub use encoding::{decode_csv_bytes, CsvEncoding};
pub use plan::{
    plan_import, plan_import_into, plan_import_with, CsvAffectedLink, CsvDestructiveAction,
    CsvDestructiveChange, CsvImportOptions, CsvImportReport, ImportPlan,
};
pub use read::{
    decode_csv_text, parse_group_addresses, CsvAction, CsvProblem, CsvRow, IgnoredColumn,
    IgnoredColumnReason, ParsedCsv, Severity,
};
pub use write::{
    export_group_addresses, export_group_addresses_from, CsvExport, UnknownInstallation,
};
