//! Parameter values a completed download built its image from, held outside the model (ADR-0173).
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

/// What one completed download wrote parameters from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterBaseline {
    /// The resolved application program the run wrote.
    pub program_id: String,
    /// The written request's values, defaults resolved in the same catalogue
    /// lease that proved the source unchanged.
    pub values: ParameterValues,
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

impl BaselineChange {
    /// The durable edit this instruction makes for `device`; `None` keeps.
    pub fn edit_for(&self, device: DeviceId) -> Option<BaselineEdit> {
        match self {
            Self::Keep => None,
            Self::Remove => Some(BaselineEdit {
                device,
                baseline: None,
            }),
            Self::Record(baseline) => Some(BaselineEdit {
                device,
                baseline: Some(baseline.clone()),
            }),
        }
    }
}

/// One device's new baseline (`None`: removed), as written to storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineEdit {
    pub device: DeviceId,
    pub baseline: Option<ParameterBaseline>,
}

/// Every known baseline of one project incarnation, by device.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParameterBaselines {
    by_device: BTreeMap<DeviceId, ParameterBaseline>,
}

impl ParameterBaselines {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, device: DeviceId) -> Option<&ParameterBaseline> {
        self.by_device.get(&device)
    }

    pub fn len(&self) -> usize {
        self.by_device.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_device.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (DeviceId, &ParameterBaseline)> {
        self.by_device
            .iter()
            .map(|(device, baseline)| (*device, baseline))
    }

    /// Sets or removes one device's baseline.
    pub fn apply(&mut self, edit: &BaselineEdit) {
        match &edit.baseline {
            Some(baseline) => {
                self.by_device.insert(edit.device, baseline.clone());
            }
            None => {
                self.by_device.remove(&edit.device);
            }
        }
    }

    /// Keeps only the baselines of devices `project` contains: what a native
    /// save does with the rows of deleted devices (ADR-0173 §2).
    pub fn retain_devices_of(&mut self, project: &Project) {
        self.by_device
            .retain(|device, _| project.devices.get(*device).is_some());
    }
}

impl FromIterator<(DeviceId, ParameterBaseline)> for ParameterBaselines {
    fn from_iter<T: IntoIterator<Item = (DeviceId, ParameterBaseline)>>(iter: T) -> Self {
        Self {
            by_device: iter.into_iter().collect(),
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
            program_id: "P".into(),
            values: values(&[("R-1", "1")]),
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
}
