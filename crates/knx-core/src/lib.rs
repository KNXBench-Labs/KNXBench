//! The KNX domain model: entities, addresses, datapoint types, override
//! resolution and validation.
//!
//! This crate performs no IO. It does not parse XML, does not touch SQL, and
//! knows nothing about any file format or the user interface.

pub mod address;
pub mod commissioning;
pub mod dpt;
pub mod flags;
pub mod ids;
pub mod provenance;
pub mod string_table;

pub use address::{AddressError, GroupAddress, GroupAddressStyle, IndividualAddress};
pub use commissioning::{CommissioningState, CompletionStatus};
pub use dpt::{DptParseError, DptRef};
pub use flags::{ComFlags, Direction, GroupLink, ObjectSize};
pub use ids::*;
pub use provenance::{Layer, Resolved};
pub use string_table::{Language, LocalizedString, StringTable, TranslationKey};
