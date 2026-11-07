//! Read-only captured-source views with current owner binding checked once per device detail.

use knx_core::{
    ComObjectInstance, DeviceId, SavedInstancePriority, SavedPriorityBindings,
    SavedPriorityOmission, SavedPriorityTokenStatus, SavedPriorityUnavailable,
};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "status", rename_all = "lowercase")]
#[ts(export)]
pub enum SavedInstancePriorityView {
    Unavailable {
        reason: SavedPriorityUnavailableView,
    },
    Absent {
        omission: SavedPriorityOmissionView,
        source: CapturedSavedInstanceSource,
    },
    Declared {
        raw: String,
        token_status: SavedPriorityTokenStatusView,
        source: CapturedSavedInstanceSource,
    },
}
impl Default for SavedInstancePriorityView {
    fn default() -> Self {
        Self::Unavailable {
            reason: SavedPriorityUnavailableView::NotCaptured,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum SavedPriorityUnavailableView {
    NotCaptured,
    UnsupportedSourceSchema,
    ModuleScope,
    AmbiguousOwner,
    AmbiguousDeclaration,
    SourceIntegrity,
    ResourceLimit,
    SourceBindingMismatch,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum SavedPriorityOmissionView {
    AttributeOmitted,
    InstanceElementOmitted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum SavedPriorityTokenStatusView {
    Known,
    Empty,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum SavedPrioritySourceKind {
    CapturedSavedInstance,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SavedPrioritySourceRef {
    pub path: String,
    pub ets_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CapturedSavedInstanceSource {
    pub kind: SavedPrioritySourceKind,
    pub source_schema: u32,
    pub device_source: SavedPrioritySourceRef,
    pub object_source: SavedPrioritySourceRef,
    pub owner_xpath: String,
    /// Original import member identity, not a rehash of current source or live device state.
    pub topology_sha256: String,
}
impl From<&knx_core::SavedPrioritySource> for CapturedSavedInstanceSource {
    fn from(s: &knx_core::SavedPrioritySource) -> Self {
        Self {
            kind: SavedPrioritySourceKind::CapturedSavedInstance,
            source_schema: s.source_schema,
            device_source: SavedPrioritySourceRef {
                path: s.device_source.path.clone(),
                ets_id: s.device_source.ets_id.clone(),
            },
            object_source: SavedPrioritySourceRef {
                path: s.object_source.path.clone(),
                ets_id: s.object_source.ets_id.clone(),
            },
            owner_xpath: s.owner_xpath.clone(),
            topology_sha256: s.topology_sha256.clone(),
        }
    }
}

pub(crate) fn view(
    com: &ComObjectInstance,
    owner: DeviceId,
    bindings: &SavedPriorityBindings<'_>,
) -> SavedInstancePriorityView {
    use SavedPriorityUnavailableView as R;
    if let SavedInstancePriority::Unavailable { reason } = com.saved_instance_priority {
        let reason = match reason {
            SavedPriorityUnavailable::NotCaptured => R::NotCaptured,
            SavedPriorityUnavailable::UnsupportedSourceSchema => R::UnsupportedSourceSchema,
            SavedPriorityUnavailable::ModuleScope => R::ModuleScope,
            SavedPriorityUnavailable::AmbiguousOwner => R::AmbiguousOwner,
            SavedPriorityUnavailable::AmbiguousDeclaration => R::AmbiguousDeclaration,
            SavedPriorityUnavailable::SourceIntegrity => R::SourceIntegrity,
            SavedPriorityUnavailable::ResourceLimit => R::ResourceLimit,
        };
        return SavedInstancePriorityView::Unavailable { reason };
    }
    if !bindings.captured_binding(com, owner) {
        return SavedInstancePriorityView::Unavailable {
            reason: R::SourceBindingMismatch,
        };
    }
    match &com.saved_instance_priority {
        SavedInstancePriority::Unavailable { .. } => unreachable!("handled above"),
        SavedInstancePriority::Absent(value) => SavedInstancePriorityView::Absent {
            omission: match value.omission {
                SavedPriorityOmission::AttributeOmitted => {
                    SavedPriorityOmissionView::AttributeOmitted
                }
                SavedPriorityOmission::InstanceElementOmitted => {
                    SavedPriorityOmissionView::InstanceElementOmitted
                }
            },
            source: (&value.source).into(),
        },
        SavedInstancePriority::Declared(value) => SavedInstancePriorityView::Declared {
            raw: value.raw.clone(),
            token_status: match SavedPriorityTokenStatus::classify(&value.raw) {
                SavedPriorityTokenStatus::Known => SavedPriorityTokenStatusView::Known,
                SavedPriorityTokenStatus::Empty => SavedPriorityTokenStatusView::Empty,
                SavedPriorityTokenStatus::Unknown => SavedPriorityTokenStatusView::Unknown,
            },
            source: (&value.source).into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::*;
    fn project() -> Project {
        let mut p = Project::new(Language("en".into()));
        let ds = SourceRef {
            path: "P-SYNTHETIC/0.xml".into(),
            ets_id: "P-SYNTHETIC-0_DI-1".into(),
        };
        let os = SourceRef {
            path: ds.path.clone(),
            ets_id: "O-7_R-1".into(),
        };
        let source = SavedPrioritySource {
            source_schema: 23,
            device_source: ds.clone(),
            object_source: os.clone(),
            owner_xpath: "/original/owner/object".into(),
            topology_sha256: "a".repeat(64),
        };
        p.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: ds,
            name: "Original".into(),
            description: None,
            address: None,
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: Default::default(),
            visibility_calculated: true,
            com_objects: vec![ComObjectInstanceId(1)],
            binary_data: vec![],
        });
        p.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(1),
            device: DeviceId(1),
            source: os,
            number: 7,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
            saved_instance_priority: SavedInstancePriority::Declared(Box::new(
                SavedPriorityDeclared {
                    raw: "High".into(),
                    source,
                },
            )),
        });
        p
    }
    fn projected(p: &Project) -> SavedInstancePriorityView {
        crate::build_device_detail(p, DeviceId(1))
            .unwrap()
            .com_objects[0]
            .saved_instance_priority
            .clone()
    }
    #[test]
    fn saved_instance_priority_source_binding_mismatch_withholds_raw_without_rewriting_fact() {
        let original = project();
        let expected = projected(&original);
        assert!(
            matches!(&expected,SavedInstancePriorityView::Declared{raw,token_status:SavedPriorityTokenStatusView::Known,source} if raw=="High" && source.kind==SavedPrioritySourceKind::CapturedSavedInstance)
        );
        let captured = original
            .devices
            .com_object(ComObjectInstanceId(1))
            .unwrap()
            .saved_instance_priority
            .clone();
        for case in 0..11 {
            let mut p = original.clone();
            match case {
                0 => p.devices.get_mut(DeviceId(1)).unwrap().source.path = "scoped/0.xml".into(),
                1 => {
                    p.devices
                        .com_object_mut(ComObjectInstanceId(1))
                        .unwrap()
                        .source
                        .ets_id = "O-8_R-1".into()
                }
                2 => p
                    .devices
                    .get_mut(DeviceId(1))
                    .unwrap()
                    .com_objects
                    .push(ComObjectInstanceId(1)),
                3 => {
                    p.devices
                        .com_object_mut(ComObjectInstanceId(1))
                        .unwrap()
                        .device = DeviceId(2)
                }
                4 => {
                    p.devices.com_object_mut(ComObjectInstanceId(1)).unwrap().id =
                        ComObjectInstanceId(2)
                }
                5 => {
                    let mut d = p.devices.get(DeviceId(1)).unwrap().clone();
                    d.id = DeviceId(2);
                    d.source.ets_id = "OTHER".into();
                    p.devices.insert(d);
                }
                6 => {
                    let mut d = p.devices.get(DeviceId(1)).unwrap().clone();
                    d.id = DeviceId(2);
                    d.com_objects.clear();
                    p.devices.insert(d);
                }
                7 => {
                    let mut c = p
                        .devices
                        .com_object(ComObjectInstanceId(1))
                        .unwrap()
                        .clone();
                    c.id = ComObjectInstanceId(2);
                    p.devices.insert_com_object(c);
                }
                8 => {
                    p.devices
                        .com_object_mut(ComObjectInstanceId(1))
                        .unwrap()
                        .module_instance = Some(ModuleInstanceId(1))
                }
                9 => {
                    let mut c = p
                        .devices
                        .com_object(ComObjectInstanceId(1))
                        .unwrap()
                        .clone();
                    c.id = ComObjectInstanceId(2);
                    c.source.ets_id = "O-8_R-1".into();
                    p.devices.insert_com_object(c);
                    p.devices.com_object_mut(ComObjectInstanceId(2)).unwrap().id =
                        ComObjectInstanceId(1);
                }
                10 => {
                    let mut d = p.devices.get(DeviceId(1)).unwrap().clone();
                    d.id = DeviceId(2);
                    d.source.ets_id = "OTHER".into();
                    d.com_objects.clear();
                    p.devices.insert(d);
                    p.devices.get_mut(DeviceId(2)).unwrap().id = DeviceId(1);
                }
                _ => unreachable!(),
            }
            assert_eq!(
                projected(&p),
                SavedInstancePriorityView::Unavailable {
                    reason: SavedPriorityUnavailableView::SourceBindingMismatch
                }
            );
            assert_eq!(
                p.devices
                    .com_object(ComObjectInstanceId(1))
                    .unwrap()
                    .saved_instance_priority,
                captured
            );
        }
        let mut renamed = original.clone();
        let owner = renamed.devices.get_mut(DeviceId(1)).unwrap();
        owner.name = "Renamed".into();
        owner.address = Some(IndividualAddress::new(1, 1, 7).unwrap());
        owner.product_ref = "Different product".into();
        owner.program_ref = "Different program".into();
        renamed
            .devices
            .com_object_mut(ComObjectInstanceId(1))
            .unwrap()
            .is_active = false;
        renamed.strings = StringTable::new(Language("de".into()));
        assert_eq!(projected(&renamed), expected);
    }
}
