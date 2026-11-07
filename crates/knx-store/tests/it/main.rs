//! knx-store integration tests in one binary: one link, one test pool (ADR-0144).

mod command_persistence;
mod completion_vocabulary;
mod functions;
mod history_compare_and_save;
mod instruction_predecessor;
mod instructions;
mod lossless_save;
mod notes_admission;
mod priority_edit;
mod project_history;
mod read_only_open;
mod reference_project;
mod saved_instance_priority;
mod saved_instance_priority_predecessor;
mod store_integrity;
