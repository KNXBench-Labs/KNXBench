//! Application services: opening and saving projects, commands, undo/redo,
//! search, selection, and reports.
//!
//! This crate orchestrates. It holds no domain rules (those live in
//! `knx-core`) and no storage or format knowledge (those live in
//! `knx-store`, `knx-etsproj` and `knx-productdb`). It is, deliberately,
//! the one crate that sees both `knx-etsproj` and `knx-store` — see
//! `import`'s own doc comment for why that matters.

pub mod import;

pub use import::{import_ets_project, AppError, ImportedProject};
