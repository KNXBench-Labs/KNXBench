//! Downloaded parameter, flag and link evidence, held outside model history (ADR-0173/0174).
//!
//! A baseline is keyed by device and lives beside the project, never inside
//! `Project`, its undo history or a version image: no undo, redo, snapshot
//! recovery or version restore may bring an older one back. Only download
//! settlement records it; settlement, a manual mark and pruning at a native
//! save remove it. It is "the value at last download", never a device read.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::{DeviceId, DeviceInstance, Project};

/// Effective values by `ParameterRef` id: the chosen value, else the
/// product default, exactly as the image builder consumed them.
pub type ParameterValues = BTreeMap<String, String>;

/// One module instantiation's written value: the program-side `Module/@Id`
/// (e.g. `MD-2_M-1_MI-3`) plus the declared `ParameterRef` id. The stored
/// module-qualified id is decomposed once, on the write side, via
/// `instance_counter` (ADR-0177); the baseline keeps the two halves so a
/// re-numbered instance never silently adopts another's evidence.
pub type ScopedParameterValues = BTreeMap<(String, String), String>;

/// What one completed download wrote parameters from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterBaseline {
    /// The resolved application program the run wrote.
    pub program_id: String,
    /// The written request's values, defaults resolved in the same catalogue
    /// lease that proved the source unchanged.
    pub values: ParameterValues,
    /// Module-scoped values the written request carried, keyed by
    /// `(Module/@Id, ParameterRef id)` — present but empty when the run
    /// wrote no module-scoped parameter (WP-37, schema 22).
    pub scoped_values: ScopedParameterValues,
    /// Unknown is distinct from a completed write with no active objects.
    pub object_flags: Option<ObjectFlagValues>,
    /// When the run's settlement recorded it.
    pub at: DateTime<Utc>,
}

/// The instruction a settlement gives about one device's baseline (ADR-0173 §3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BaselineChange {
    /// Leave whatever baseline the device has.
    #[default]
    Keep,
    /// The device may hold other parameter values now: forget the baseline.
    Remove,
    /// A completed run wrote these values from an unchanged source.
    Record(ParameterBaseline),
}

/// The five flags actually written; read-on-init is unwritten in admitted formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WrittenObjectFlags {
    pub read: bool,
    pub write: bool,
    pub transmit: bool,
    pub update: bool,
    pub communication: bool,
    pub read_on_init: Option<bool>,
}

pub type ObjectFlagValues = BTreeMap<u8, WrittenObjectFlags>;

/// One association exactly as the written request built its tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineLink {
    pub object: u8,
    pub group_address: crate::GroupAddress,
    pub sending: bool,
}

/// Independently written Group-addresses evidence, outside model history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkBaseline {
    pub program_id: String,
    pub links: Vec<BaselineLink>,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LinkBaselineChange {
    #[default]
    Keep,
    Remove,
    Record(LinkBaseline),
}

impl BaselineChange {
    pub fn edit_for(&self, device: DeviceId) -> Option<BaselineEdit> {
        (!matches!(self, Self::Keep)).then(|| BaselineEdit {
            device,
            parameter: self.clone(),
            links: LinkBaselineChange::Keep,
        })
    }
}

/// Explicit independent edits: an unselected section genuinely keeps its time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineEdit {
    pub device: DeviceId,
    pub parameter: BaselineChange,
    pub links: LinkBaselineChange,
}

impl BaselineEdit {
    pub fn remove(device: DeviceId) -> Self {
        Self {
            device,
            parameter: BaselineChange::Remove,
            links: LinkBaselineChange::Remove,
        }
    }
}

/// Independent sections of one project incarnation, by device (including link-only owners).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParameterBaselines {
    by_device: BTreeMap<DeviceId, ParameterBaseline>,
    links: BTreeMap<DeviceId, LinkBaseline>,
}

impl ParameterBaselines {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, device: DeviceId) -> Option<&ParameterBaseline> {
        self.by_device.get(&device)
    }

    pub fn len(&self) -> usize {
        self.device_ids().len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_device.is_empty() && self.links.is_empty()
    }

    /// Parameter headers only; lifecycle enumeration must use `device_ids`.
    pub fn iter(&self) -> impl Iterator<Item = (DeviceId, &ParameterBaseline)> {
        self.by_device
            .iter()
            .map(|(device, baseline)| (*device, baseline))
    }

    pub fn get_links(&self, device: DeviceId) -> Option<&LinkBaseline> {
        self.links.get(&device)
    }

    pub fn iter_links(&self) -> impl Iterator<Item = (DeviceId, &LinkBaseline)> {
        self.links.iter().map(|(id, baseline)| (*id, baseline))
    }

    /// Union of independently stored section owners, in deterministic order.
    pub fn device_ids(&self) -> std::collections::BTreeSet<DeviceId> {
        self.by_device
            .keys()
            .chain(self.links.keys())
            .copied()
            .collect()
    }

    pub fn would_change(&self, edit: &BaselineEdit) -> bool {
        let parameter = match &edit.parameter {
            BaselineChange::Keep => false,
            BaselineChange::Remove => self.get(edit.device).is_some(),
            BaselineChange::Record(value) => self.get(edit.device) != Some(value),
        };
        let links = match &edit.links {
            LinkBaselineChange::Keep => false,
            LinkBaselineChange::Remove => self.get_links(edit.device).is_some(),
            LinkBaselineChange::Record(value) => self.get_links(edit.device) != Some(value),
        };
        parameter || links
    }

    pub fn apply(&mut self, edit: &BaselineEdit) {
        match &edit.parameter {
            BaselineChange::Keep => {}
            BaselineChange::Record(value) => {
                self.by_device.insert(edit.device, value.clone());
            }
            BaselineChange::Remove => {
                self.by_device.remove(&edit.device);
            }
        }
        match &edit.links {
            LinkBaselineChange::Keep => {}
            LinkBaselineChange::Record(value) => {
                self.links.insert(edit.device, value.clone());
            }
            LinkBaselineChange::Remove => {
                self.links.remove(&edit.device);
            }
        }
    }

    /// Keeps only the baselines of devices `project` contains: what a native
    /// save does with the rows of deleted devices (ADR-0173 §2).
    pub fn retain_devices_of(&mut self, project: &Project) {
        self.links
            .retain(|device, _| project.devices.get(*device).is_some());
        self.by_device
            .retain(|device, _| project.devices.get(*device).is_some());
    }
}

impl FromIterator<(DeviceId, ParameterBaseline)> for ParameterBaselines {
    fn from_iter<T: IntoIterator<Item = (DeviceId, ParameterBaseline)>>(iter: T) -> Self {
        Self {
            by_device: iter.into_iter().collect(),
            links: BTreeMap::new(),
        }
    }
}

/// Whether two devices under one id are the same device: same source
/// element, same product and same application program reference.
pub fn same_device(a: &DeviceInstance, b: &DeviceInstance) -> bool {
    a.source == b.source && a.product_ref == b.product_ref && a.program_ref == b.program_ref
}

/// One ref whose value at the last download is not its current value. A
/// side without a value is `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterDifference {
    pub ref_id: String,
    pub downloaded: Option<String>,
    pub current: Option<String>,
}

/// One module-scoped ref whose value at the last download is not its current
/// value: the `Module/@Id`, the declared ref id, and both sides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedParameterDifference {
    pub module_id: String,
    pub ref_id: String,
    pub downloaded: Option<String>,
    pub current: Option<String>,
}

/// The refs whose effective values differ between `downloaded` and
/// `current`, by id. Values compare as exact raw strings; a ref with a value
/// on exactly one side differs, a ref on neither side is not compared
/// (ADR-0173 §4).
pub fn differences(
    downloaded: &ParameterValues,
    current: &ParameterValues,
) -> Vec<ParameterDifference> {
    let mut ids: Vec<&String> = downloaded.keys().chain(current.keys()).collect();
    ids.sort();
    ids.dedup();
    ids.into_iter()
        .filter_map(|id| {
            let (was, now) = (downloaded.get(id), current.get(id));
            (was != now).then(|| ParameterDifference {
                ref_id: id.clone(),
                downloaded: was.cloned(),
                current: now.cloned(),
            })
        })
        .collect()
}

/// The module-scoped refs whose values differ, ordered by `(module_id,
/// ref_id)`: same exact-raw-string rule as [`differences`].
pub fn scoped_differences(
    downloaded: &ScopedParameterValues,
    current: &ScopedParameterValues,
) -> Vec<ScopedParameterDifference> {
    let mut keys: Vec<&(String, String)> = downloaded.keys().chain(current.keys()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .filter_map(|(module_id, ref_id)| {
            let (was, now) = (downloaded.get(&(module_id.clone(), ref_id.clone())), current.get(&(module_id.clone(), ref_id.clone())));
            (was != now).then(|| ScopedParameterDifference {
                module_id: module_id.clone(),
                ref_id: ref_id.clone(),
                downloaded: was.cloned(),
                current: now.cloned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&str, &str)]) -> ParameterValues {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn equal_values_and_refs_absent_on_both_sides_never_differ() {
        let same = values(&[("R-1", "1"), ("R-2", "")]);
        assert!(differences(&same, &same).is_empty());
        assert!(differences(&values(&[]), &values(&[])).is_empty());
    }

    #[test]
    fn a_changed_value_and_a_value_on_one_side_only_differ_in_id_order() {
        let downloaded = values(&[("R-1", "1"), ("R-3", "x"), ("R-4", "same")]);
        let current = values(&[("R-1", "01"), ("R-2", "new"), ("R-4", "same")]);
        assert_eq!(
            differences(&downloaded, &current),
            vec![
                ParameterDifference {
                    ref_id: "R-1".into(),
                    downloaded: Some("1".into()),
                    current: Some("01".into()),
                },
                ParameterDifference {
                    ref_id: "R-2".into(),
                    downloaded: None,
                    current: Some("new".into()),
                },
                ParameterDifference {
                    ref_id: "R-3".into(),
                    downloaded: Some("x".into()),
                    current: None,
                },
            ]
        );
    }

    #[test]
    fn keep_edits_nothing_remove_and_record_edit_the_device() {
        let baseline = ParameterBaseline {
            object_flags: None,
            program_id: "P".into(),
            values: values(&[("R-1", "1")]),
            scoped_values: ScopedParameterValues::new(),
            at: DateTime::<Utc>::UNIX_EPOCH,
        };
        let device = DeviceId(4);
        assert_eq!(BaselineChange::Keep.edit_for(device), None);
        let mut all = ParameterBaselines::new();
        all.apply(
            &BaselineChange::Record(baseline.clone())
                .edit_for(device)
                .unwrap(),
        );
        assert_eq!(all.get(device), Some(&baseline));
        all.apply(&BaselineChange::Remove.edit_for(device).unwrap());
        assert!(all.is_empty());
    }

    fn scoped(pairs: &[(&str, &str, &str)]) -> ScopedParameterValues {
        pairs
            .iter()
            .map(|(m, r, v)| ((m.to_string(), r.to_string()), v.to_string()))
            .collect()
    }

    #[test]
    fn scoped_differences_compare_exact_raws_in_module_ref_order() {
        let downloaded = scoped(&[
            ("MD-2_M-1_MI-1", "P-1_R-1", "1"),
            ("MD-2_M-1_MI-2", "P-1_R-1", "x"),
            ("MD-2_M-1_MI-1", "P-1_R-2", "same"),
        ]);
        let current = scoped(&[
            ("MD-2_M-1_MI-1", "P-1_R-1", "01"),
            ("MD-2_M-1_MI-2", "P-1_R-1", "x"),
            ("MD-2_M-1_MI-1", "P-1_R-3", "new"),
            ("MD-2_M-1_MI-1", "P-1_R-2", "same"),
        ]);
        assert_eq!(
            scoped_differences(&downloaded, &current),
            vec![
                ScopedParameterDifference {
                    module_id: "MD-2_M-1_MI-1".into(),
                    ref_id: "P-1_R-1".into(),
                    downloaded: Some("1".into()),
                    current: Some("01".into()),
                },
                ScopedParameterDifference {
                    module_id: "MD-2_M-1_MI-1".into(),
                    ref_id: "P-1_R-3".into(),
                    downloaded: None,
                    current: Some("new".into()),
                },
            ]
        );
        // The MI-2 row is identical on both sides, so one instantiation's
        // evidence never leaks into another's comparison.
        assert!(scoped_differences(&downloaded, &downloaded).is_empty());
    }
}
