//! Reading and writing `.knxproj`: ZIP container, schema detection, tolerant
//! XML parsing, mapping to and from `knx-core`, and the import report.

pub mod container;
pub mod detect;

pub use container::{Container, ContainerError, EntryInfo};
pub use detect::{detect, DetectError, Detected, SchemaVersion};
