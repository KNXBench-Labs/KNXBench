//! `Devices` is the sole owner of `DeviceInstance` and `ComObjectInstance`
//! values. `Topology` and `Buildings` reference devices by id only
//! (DATA_MODEL §5).

use std::collections::BTreeMap;

use crate::device::{ComObjectInstance, DeviceInstance, ProgramDefaults};
use crate::ids::{ComObjectInstanceId, DeviceId, ModuleInstanceId};
use crate::module::ModuleInstance;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Devices {
    by_id: BTreeMap<DeviceId, DeviceInstance>,
    com_objects: BTreeMap<ComObjectInstanceId, ComObjectInstance>,
    module_instances: BTreeMap<ModuleInstanceId, ModuleInstance>,
    /// Populated only by `knx_productdb::enrich`, and only for a
    /// communication object carrying at least one `Override::Empty` slot
    /// with a resolvable program value behind it (ADR-0012 gap 2,
    /// ADR-0027). Absence means "nothing to show", not "not enriched yet" —
    /// callers distinguish those two, if they need to, via
    /// `EnrichmentReport`.
    program_defaults: BTreeMap<ComObjectInstanceId, ProgramDefaults>,
}

impl Devices {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, device: DeviceInstance) {
        self.by_id.insert(device.id, device);
    }

    pub fn get(&self, id: DeviceId) -> Option<&DeviceInstance> {
        self.by_id.get(&id)
    }

    pub fn get_mut(&mut self, id: DeviceId) -> Option<&mut DeviceInstance> {
        self.by_id.get_mut(&id)
    }

    pub fn remove(&mut self, id: DeviceId) -> Option<DeviceInstance> {
        self.by_id.remove(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &DeviceInstance> {
        self.by_id.values()
    }

    pub fn insert_com_object(&mut self, com: ComObjectInstance) {
        self.com_objects.insert(com.id, com);
    }

    pub fn remove_com_object(&mut self, id: ComObjectInstanceId) -> Option<ComObjectInstance> {
        self.program_defaults.remove(&id);
        self.com_objects.remove(&id)
    }

    pub fn com_object(&self, id: ComObjectInstanceId) -> Option<&ComObjectInstance> {
        self.com_objects.get(&id)
    }

    pub fn com_object_mut(&mut self, id: ComObjectInstanceId) -> Option<&mut ComObjectInstance> {
        self.com_objects.get_mut(&id)
    }

    /// Every communication object instance this `Devices` owns, in id order —
    /// including any not currently named by its device's `com_objects` list.
    /// Persistence needs the full set to be able to notice such an orphan
    /// instead of silently dropping it, the same reason `StringTable::iter`
    /// exists.
    pub fn com_objects(&self) -> impl Iterator<Item = &ComObjectInstance> {
        self.com_objects.values()
    }

    /// The program-layer defaults lifted for a communication object with
    /// at least one `Override::Empty` slot (ADR-0012 gap 2, ADR-0027).
    /// `None` for every com object `knx_productdb::enrich` had nothing to
    /// add for — which is most of them.
    pub fn program_defaults(&self, id: ComObjectInstanceId) -> Option<&ProgramDefaults> {
        self.program_defaults.get(&id)
    }

    /// Replaces the program defaults recorded for `id`. An empty
    /// `ProgramDefaults` (see `ProgramDefaults::is_empty`) removes the entry
    /// instead of keeping a pointless placeholder around.
    pub fn set_program_defaults(&mut self, id: ComObjectInstanceId, defaults: ProgramDefaults) {
        if defaults.is_empty() {
            self.program_defaults.remove(&id);
        } else {
            self.program_defaults.insert(id, defaults);
        }
    }

    pub fn insert_module_instance(&mut self, m: ModuleInstance) {
        self.module_instances.insert(m.id, m);
    }

    pub fn module_instance(&self, id: ModuleInstanceId) -> Option<&ModuleInstance> {
        self.module_instances.get(&id)
    }

    pub fn module_instances(&self) -> impl Iterator<Item = &ModuleInstance> {
        self.module_instances.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commissioning::CommissioningState;
    use crate::ids::SourceRef;

    fn device(id: DeviceId) -> DeviceInstance {
        DeviceInstance {
            id,
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            name: "Dev".into(),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        }
    }

    #[test]
    fn com_objects_enumerates_every_instance_including_an_unlinked_one() {
        use crate::device::ComObjectInstance;
        use crate::flags::ResolvedFlags;
        use crate::ids::ComObjectInstanceId;
        use crate::provenance::Override;

        let mut d = Devices::new();
        d.insert(device(DeviceId(1))); // its `com_objects` list stays empty
        d.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(7),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        });
        let ids: Vec<_> = d.com_objects().map(|c| c.id).collect();
        assert_eq!(ids, vec![ComObjectInstanceId(7)]);
    }

    #[test]
    fn program_defaults_is_absent_until_set_and_cleared_by_removal() {
        use crate::device::ProgramDefaults;
        use crate::dpt::DptRef;
        use crate::ids::ComObjectInstanceId;
        use crate::provenance::{Layer, Resolved};

        let mut d = Devices::new();
        let id = ComObjectInstanceId(1);
        assert!(d.program_defaults(id).is_none());

        d.set_program_defaults(
            id,
            ProgramDefaults {
                text: None,
                description: None,
                dpt: Some(Resolved {
                    value: DptRef {
                        main: 1,
                        sub: Some(1),
                    },
                    layer: Layer::Program,
                }),
            },
        );
        assert!(d.program_defaults(id).is_some());

        // An empty ProgramDefaults clears rather than lingering as a
        // pointless placeholder.
        d.set_program_defaults(id, ProgramDefaults::default());
        assert!(d.program_defaults(id).is_none());
    }

    #[test]
    fn devices_is_the_sole_owner_lookup_by_id() {
        let mut d = Devices::new();
        d.insert(device(DeviceId(1)));
        assert!(d.get(DeviceId(1)).is_some());
        assert!(d.get(DeviceId(2)).is_none());
    }

    #[test]
    fn module_instances_are_retrievable_by_id() {
        let mut d = Devices::new();
        d.insert_module_instance(crate::module::ModuleInstance {
            id: ModuleInstanceId(1),
            device: DeviceId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "MD-2_M-1".into(),
            },
            repeat_index: "6x1".into(),
            instance_ets_id: "MD-2_M-1_MI-1".into(),
            arguments: vec![],
        });
        assert!(d.module_instance(ModuleInstanceId(1)).is_some());
        assert_eq!(d.module_instances().count(), 1);
    }
}
