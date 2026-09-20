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
use crate::flags::{GroupLink, ObjectSize, ResolvedFlags};
use crate::ids::{ComObjectInstanceId, DeviceId, ModuleInstanceId, SourceRef};
use crate::provenance::{Override, Resolved};
use crate::string_table::Text;

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
    pub binary_data: Vec<BinaryDataRef>,
}

/// A reference from a device to one of the opaque blobs in
/// `<P-xxxx>/BinaryData/<guid>.dat`. The bytes live in the opaque store; only
/// the reference is modelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryDataRef {
    /// `BinaryData/@Id` — the GUID that names the `.dat` entry.
    pub id: String,
    /// `BinaryData/@Name`.
    pub name: String,
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
    pub text: Override<Text>,
    pub description: Override<Text>,
    pub dpt: Override<DptRef>,
    pub flags: ResolvedFlags,
    /// Never stated at instance level in schema 11; filled from the
    /// application program once the product database exists.
    pub size: Option<Resolved<ObjectSize>>,
    pub is_active: bool,
    pub links: Vec<GroupLink>,
    /// `None` for a schema-11-shaped monolithic-program device; `Some` for
    /// a module-based one (ADR-0013). Resolves this object's DPT/Text
    /// defaults through `ModuleInstance` → `ModuleDef` → `ComObjectRef` →
    /// `ComObject` instead of the direct `ApplicationProgram` chain.
    pub module_instance: Option<ModuleInstanceId>,
}

/// The program-layer value sitting behind one of `ComObjectInstance`'s own
/// `Override::Empty` slots — present in the source, explicitly cleared, but
/// with a value the application program itself still states (ADR-0012 gap
/// 2, ADR-0027, KNOWN_LIMITATIONS §12). Kept **beside** `ComObjectInstance`
/// (in `Devices`, keyed by `ComObjectInstanceId`) rather than folded into
/// `Override<T>` itself: the instance's own `Empty` state must never be
/// mistaken for having a value, and every one of `Override<T>`'s existing
/// call sites — the importer, the exporter, every match arm already
/// written against its four variants — stays exactly as it was.
///
/// Every field here is `Layer::Program` or `Layer::ProgramRef` by
/// construction (`knx_productdb::enrich` is the only writer), so
/// `Layer::is_exported()` already excludes it from export without this
/// type needing an opinion of its own.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProgramDefaults {
    pub text: Option<Resolved<Text>>,
    pub description: Option<Resolved<Text>>,
    pub dpt: Option<Resolved<DptRef>>,
}

impl ProgramDefaults {
    /// Whether every field is unset — the state a `ComObjectInstance` with
    /// no `Empty` slots, or one whose product database had nothing to say,
    /// leaves this in. `Devices` uses this to decide whether an entry is
    /// worth keeping at all.
    pub fn is_empty(&self) -> bool {
        self.text.is_none() && self.description.is_none() && self.dpt.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::ResolvedFlags;
    use crate::provenance::{Layer, Override};
    use crate::string_table::Text;

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
        let com = com_object_instance_fixture();
        assert!(com.dpt.value().is_none());
    }

    /// A minimal, valid `ComObjectInstance`: text and description resolved
    /// at program layer, no datapoint type, all six flags absent, and no
    /// size (unstated at instance level in schema 11, as `size` documents).
    fn com_object_instance_fixture() -> ComObjectInstance {
        ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Value(Resolved {
                value: Text::Literal("t".into()),
                layer: Layer::Program,
            }),
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        }
    }

    #[test]
    fn program_defaults_is_empty_only_with_all_three_fields_unset() {
        let mut defaults = ProgramDefaults::default();
        assert!(defaults.is_empty());
        defaults.dpt = Some(Resolved {
            value: DptRef {
                main: 1,
                sub: Some(1),
            },
            layer: Layer::Program,
        });
        assert!(!defaults.is_empty());
    }

    #[test]
    fn an_empty_datapoint_type_attribute_is_not_the_same_as_an_absent_one() {
        let mut com = com_object_instance_fixture();
        com.dpt = Override::Empty;
        assert!(com.dpt.value().is_none());
        assert!(com.dpt.is_present());
        com.dpt = Override::Absent;
        assert!(!com.dpt.is_present());
    }
}
