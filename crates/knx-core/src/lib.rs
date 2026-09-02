//! The KNX domain model: entities, addresses, datapoint types, override
//! resolution and validation.
//!
//! This crate performs no IO. It does not parse XML, does not touch SQL, and
//! knows nothing about any file format or the user interface.

pub mod address;
pub mod ids;
pub mod provenance;

pub use address::{AddressError, GroupAddress, GroupAddressStyle, IndividualAddress};
pub use ids::*;
pub use provenance::{Layer, Resolved};
