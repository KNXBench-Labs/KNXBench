//! Stage 6 (reverse direction): writing `knx_core::Project` back out to a
//! `.knxproj` container. `schema11` is the schema-11 XML writer (Task 18);
//! the container-level orchestration (`export_knxproj`) arrives in Task 19.

pub mod schema11;

pub use schema11::{write_installation_xml, write_project_xml, ExportError};
