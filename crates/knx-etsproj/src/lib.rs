//! Reading and writing `.knxproj`: ZIP container, schema detection, tolerant
//! XML parsing, mapping to and from `knx-core`, and the import report.

pub mod container;
pub mod detect;
pub mod known;
pub mod parse;
pub mod source;

pub use container::{Container, ContainerError, EntryInfo};
pub use detect::{detect, DetectError, Detected, SchemaVersion};
pub use known::{known_schema, KnownElement, KnownSchema};
pub use parse::{parse_installation, ParseError, ParseOutput, UnknownConstruct, UnknownKind};
pub use source::{
    RetainedAttribute, RetainedElement, SourceArea, SourceBinaryDataRef, SourceBuildingPart,
    SourceComObjectInstance, SourceDevice, SourceDocument, SourceGroupAddress, SourceGroupRange,
    SourceInstallation, SourceLine, SourceParameterInstance, SourceProjectInfo,
};
