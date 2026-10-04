//! Application services: opening and saving projects, commands, undo/redo,
//! search, selection, and reports.
//!
//! This crate orchestrates. It holds no domain rules (those live in
//! `knx-core`) and no storage or format knowledge (those live in
//! `knx-store`, `knx-etsproj` and `knx-productdb`). It is, deliberately,
//! the one crate that sees both `knx-etsproj` and `knx-store` — see
//! `import`'s own doc comment for why that matters.

pub mod access_key;
mod backup_directory;
pub mod comparison;
pub mod device_backup;
pub mod device_download;
pub mod documentation;
pub mod download_support;
pub mod import;
pub mod individual_address_programming_recovery;
pub mod individual_address_reset_recovery;
pub mod progress;
pub mod project_readiness;
pub mod serial_address_recovery;
pub mod serial_number;
pub mod service_control_backup;

pub use import::{
    import_ets_project, import_ets_project_observed, import_ets_project_with,
    import_ets_project_with_password, AppError, ImportOptions, ImportedProject,
};
pub use progress::{LoadObserver, LoadStage};
