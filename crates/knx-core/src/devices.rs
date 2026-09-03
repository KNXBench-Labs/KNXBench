//! `Devices` is the sole owner of `DeviceInstance` and `ComObjectInstance`
//! values. `Topology` and `Buildings` reference devices by id only
//! (DATA_MODEL §5).

use std::collections::BTreeMap;

use crate::device::{ComObjectInstance, DeviceInstance};
use crate::ids::{ComObjectInstanceId, DeviceId};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Devices {
    by_id: BTreeMap<DeviceId, DeviceInstance>,
    com_objects: BTreeMap<ComObjectInstanceId, ComObjectInstance>,
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

    pub fn com_object(&self, id: ComObjectInstanceId) -> Option<&ComObjectInstance> {
        self.com_objects.get(&id)
    }

    pub fn com_object_mut(&mut self, id: ComObjectInstanceId) -> Option<&mut ComObjectInstance> {
        self.com_objects.get_mut(&id)
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
    fn devices_is_the_sole_owner_lookup_by_id() {
        let mut d = Devices::new();
        d.insert(device(DeviceId(1)));
        assert!(d.get(DeviceId(1)).is_some());
        assert!(d.get(DeviceId(2)).is_none());
    }
}
