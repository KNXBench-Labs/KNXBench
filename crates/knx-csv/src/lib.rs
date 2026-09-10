//! Reader/writer for **"KNXBench group-address CSV v1"** — a format this
//! project defines and owns.
//!
//! This is deliberately *not* a claim of compatibility with ETS's own
//! "Export Group Addresses" feature. No sample of that export exists
//! anywhere in this repository or in the KNX Standard v3.0.0 corpus, so
//! there is nothing to be compatible *with* — see
//! `docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md`
//! §1 for the full accounting. Never describe this format as "ETS CSV" or
//! imply interoperability with it in code, comments, or UI text.
//!
//! `knx-csv` is pure: text in, typed rows out (and, eventually, typed rows
//! in, text out). It knows nothing of the filesystem, SQLite, HTTP, or
//! `AppState` — orchestration lives in the callers (`apps/knx-server`,
//! `apps/knx-cli`).

mod read;

pub use read::{
    parse_group_addresses, CsvProblem, CsvRow, IgnoredColumn, IgnoredColumnReason, ParsedCsv,
    Severity,
};
