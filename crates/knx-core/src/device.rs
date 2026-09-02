//! Device instances and the communication object instances they own.
//!
//! `ComObjectInstance` is the entity the override chain hangs off
//! (DATA_MODEL §3/§4): its `text`, `description`, `dpt`, `flags` and `size`
//! fields are `Resolved<T>`, never a bare value, because no resolved scalar
//! exists without knowing which of `ComObject`, `ComObjectRef` or
//! `ComObjectInstanceRef` it came from.

use crate::address::IndividualAddress;
use crate::commissioning::CommissioningState;
use crate::dpt::DptRef;
use crate::flags::{ComFlags, GroupLink, ObjectSize};
use crate::ids::{ComObjectInstanceId, DeviceId, SourceRef};
use crate::provenance::Resolved;
use crate::string_table::LocalizedString;

/// A device placed in a project. Owned exclusively by `Devices` — `Topology`
/// and `Buildings` reference a device by id, never embed it.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceInstance {
    pub id: DeviceId,
    pub source: SourceRef,
    pub name: String,
    pub description: Option<String>,
    /// A device without an individual address is valid project state.
    pub address: Option<IndividualAddress>,
    /// `ProductRefId` — an opaque product reference, not interpreted here.
    pub product_ref: String,
    /// `Hardware2ProgramRefId` — an opaque product reference, not
    /// interpreted here.
    pub program_ref: String,
    pub commissioning: CommissioningState,
    pub visibility_calculated: bool,
    pub com_objects: Vec<ComObjectInstanceId>,
}

/// One communication object as it exists on one device, after the
/// `ComObject` → `ComObjectRef` → `ComObjectInstanceRef` override chain has
/// been resolved (DATA_MODEL §3).
#[derive(Debug, Clone, PartialEq)]
pub struct ComObjectInstance {
    pub id: ComObjectInstanceId,
    pub source: SourceRef,
    pub device: DeviceId,
    /// From `_O-<n>` in the source `RefId`.
    pub number: u16,
    pub text: Resolved<LocalizedString>,
    pub description: Option<Resolved<LocalizedString>>,
    /// A communication object instance may have no resolved datapoint type
    /// at all; this is normal, not an error (DATA_MODEL §9).
    pub dpt: Option<Resolved<DptRef>>,
    pub flags: Resolved<ComFlags>,
    pub size: Resolved<ObjectSize>,
    pub is_active: bool,
    pub links: Vec<GroupLink>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::Layer;
    use crate::string_table::TranslationKey;

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    #[test]
    fn instance_layer_dpt_overrides_program_layer_dpt() {
        let program_dpt = Resolved {
            value: DptRef {
                main: 1,
                sub: Some(1),
            },
            layer: Layer::Program,
        };
        let instance_dpt = Resolved {
            value: DptRef {
                main: 5,
                sub: Some(1),
            },
            layer: Layer::Instance,
        };
        // The override chain resolves to the last (highest-precedence)
        // layer present. `ComObjectInstance` always stores the
        // already-resolved value plus its layer; resolving from raw layers
        // is the importer's job (Session 3). This test documents and locks
        // the precedence a resolver must honour.
        let precedence = |l: Layer| match l {
            Layer::Program => 0,
            Layer::ProgramRef => 1,
            Layer::Instance => 2,
            Layer::Inferred => 3,
            Layer::UserEdit => 4,
        };
        assert!(precedence(instance_dpt.layer) > precedence(program_dpt.layer));
        assert!(!program_dpt.layer.is_exported());
        assert!(instance_dpt.layer.is_exported());
    }

    #[test]
    fn com_object_instance_without_dpt_is_constructible() {
        let com = ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Resolved {
                value: LocalizedString(TranslationKey("t".into())),
                layer: Layer::Program,
            },
            description: None,
            dpt: None,
            flags: Resolved {
                value: ComFlags {
                    read: false,
                    write: true,
                    transmit: false,
                    update: false,
                    communication: true,
                },
                layer: Layer::Program,
            },
            size: Resolved {
                value: ObjectSize::Bit(1),
                layer: Layer::Program,
            },
            is_active: true,
            links: vec![],
        };
        assert!(com.dpt.is_none());
    }
}
