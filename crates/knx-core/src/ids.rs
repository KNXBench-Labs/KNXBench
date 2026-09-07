//! Internal identifiers and the source-provenance reference.
//!
//! Two kinds of identifier, never conflated (DATA_MODEL §2). Internal ids —
//! `DeviceId`, `GroupAddressId` and so on — are stable, project-unique and
//! persisted; these are the primary keys. `SourceRef` carries the original
//! ETS identifier string, which is never used as a primary key: ETS ids
//! collide across projects and change, and export needs the original string
//! to write a file ETS can read.

use std::fmt;

/// The original ETS identifier a value came from, and the path in the
/// source document it was read from.
///
/// Never a primary key. Preserved so export can write back an identifier ETS
/// recognises, and so provenance can explain where a value came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceRef {
    pub path: String,
    pub ets_id: String,
}

/// Declares a synthetic, project-unique internal id newtype wrapping `$repr`.
macro_rules! id_type {
    ($name:ident, $repr:ty) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub $repr);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

id_type!(AreaId, u32);
id_type!(LineId, u32);
id_type!(DeviceId, u32);
id_type!(ComObjectInstanceId, u32);
id_type!(GroupRangeId, u32);
id_type!(GroupAddressId, u32);
id_type!(BuildingPartId, u32);
id_type!(ParameterInstanceId, u32);
id_type!(ModuleInstanceId, u32);

// `InstallationId` mirrors ETS's own installation number (`InstallationId`
// in `0.xml`) rather than a synthetic counter: it is already stable and
// project-unique in the source data, so there is nothing to allocate.
id_type!(InstallationId, u8);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_with_different_values_are_not_equal() {
        assert_ne!(DeviceId(1), DeviceId(2));
        assert_eq!(DeviceId(1), DeviceId(1));
    }

    #[test]
    fn id_displays_as_its_raw_value() {
        assert_eq!(DeviceId(1).to_string(), "1");
    }

    #[test]
    fn module_instance_ids_with_different_values_are_not_equal() {
        assert_ne!(ModuleInstanceId(1), ModuleInstanceId(2));
    }
}
