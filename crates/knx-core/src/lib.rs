//! The KNX domain model: entities, addresses, datapoint types, override
//! resolution and validation.
//!
//! This crate performs no IO. It does not parse XML, does not touch SQL, and
//! knows nothing about any file format or the user interface.

pub mod address;
pub mod building;
pub mod command;
pub mod commissioning;
pub mod device;
pub mod devices;
pub mod dpt;
pub mod flags;
pub mod group;
pub mod ids;
pub mod installation;
pub mod parameter;
pub mod project;
pub mod provenance;
pub mod string_table;
pub mod topology;
pub mod validation;

pub use address::{AddressError, GroupAddress, GroupAddressStyle, IndividualAddress};
pub use building::{BuildingPart, BuildingPartType};
pub use command::{Command, CommandError, CommandStack};
pub use commissioning::{CommissioningState, CompletionStatus};
pub use device::{BinaryDataRef, ComObjectInstance, DeviceInstance};
pub use devices::Devices;
pub use dpt::{DptParseError, DptRef};
pub use flags::{ComFlags, Direction, GroupLink, ObjectSize, ResolvedFlags};
pub use group::{GroupAddressEntry, GroupRange};
pub use ids::*;
pub use installation::Installation;
pub use parameter::ParameterInstance;
pub use project::{IdAllocators, Project, CURRENT_SCHEMA_VERSION};
pub use provenance::{Layer, Override, Resolved};
pub use string_table::{Language, LocalizedString, StringTable, Text, TranslationKey};
pub use topology::{Area, Line, Topology};
pub use validation::ValidationError;
