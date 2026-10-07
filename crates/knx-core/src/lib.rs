//! The KNX domain model: entities, addresses, datapoint types, override
//! resolution and validation.
//!
//! This crate performs no IO. It does not parse XML, does not touch SQL, and
//! knows nothing about any file format or the user interface.

pub mod address;
pub mod allocation;
pub mod building;
pub mod command;
pub mod commissioning;
pub mod device;
pub mod devices;
pub mod dpt;
pub mod flags;
pub mod group;
pub mod group_address_names;
pub mod ids;
pub mod installation;
pub mod module;
pub mod parameter;
pub mod project;
pub mod provenance;
pub mod scan;
pub mod string_table;
pub mod topology;
pub mod validation;

pub use address::{
    is_project_excluded, AddressError, ContactableAddress, ExcludedAddress, GroupAddress,
    GroupAddressStyle, IndividualAddress, EXCLUDED_INDIVIDUAL_ADDRESSES,
};
pub use allocation::{free_line_addresses, AddressAllocationError};
pub use building::{BuildingPart, BuildingPartType};
pub use command::{
    Command, CommandError, CommandStack, CouplerEvidence, DevicePlacementSlot, IdKind,
    RemovedDevicePlacement, RemovedLineReference,
};
pub use commissioning::mutation::{WriteAuthorisation, WriteScope};
pub use commissioning::{
    CommissioningState, CompletionStatus, DeviceLoadStates, LoadDisagreement, LoadPart,
};
pub use device::{BinaryDataRef, ComObjectInstance, DeviceInstance, ProgramDefaults};
pub use devices::Devices;
pub use dpt::{
    decode, default_input_format, encode, encode_inferred_format, encoding_rulings,
    format_width_bits, group_address_dpt_from, group_address_type_from, resolve_group_address_dpt,
    resolve_group_address_type, resolve_project_group_address_dpts, validate_group_write_dpt,
    DptCodecError, DptEncodingRuling, DptInputFormat, DptParseError, DptRef, DptValue,
    GroupAddressDpt, GroupAddressType, GroupAddressTypeOutcome, GroupValue,
};
pub use flags::{ComFlagKind, ComFlags, Direction, GroupLink, ObjectSize, ResolvedFlags};
pub use group::{GroupAddressEntry, GroupRange};
pub use group_address_names::{resolve_project_group_address_names, GROUP_ADDRESS_NAME_SEPARATOR};
pub use ids::*;
pub use installation::Installation;
pub use module::ModuleInstance;
pub use parameter::ParameterInstance;
pub use project::{IdAllocationError, IdAllocators, Project, ProjectInfo, CURRENT_SCHEMA_VERSION};
pub use provenance::{Layer, Override, Resolved};
pub use scan::{ScanPlan, ScanPlanBuilder, ScanPlanError};
pub use string_table::{Language, LocalizedString, StringTable, Text, TranslationKey};
pub use topology::{Area, Line, Topology};
pub use validation::ValidationError;
