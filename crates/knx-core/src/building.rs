//! Building parts: a recursive typed hierarchy that references devices, but
//! does not own them — `Devices` is the sole owner (DATA_MODEL §5).

use crate::ids::{BuildingPartId, DeviceId, LineId, SourceRef};
use crate::CompletionStatus;

/// `BuildingPart/@Type`. Observed values in the reference project:
/// `Building` (1), `Floor` (3), `Room` (14), `Corridor` (2),
/// `DistributionBoard` (1), `BuildingPart` (1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildingPartType {
    Building,
    Floor,
    Room,
    Corridor,
    DistributionBoard,
    BuildingPart,
}

/// A node in the building hierarchy. Nests via `children`, not containment
/// — a `BuildingPart` does not embed its child parts, it references their
/// ids, and likewise for `devices` (`DeviceInstanceRef`, a reference, not
/// ownership).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildingPart {
    pub id: BuildingPartId,
    pub source: SourceRef,
    pub name: String,
    pub number: Option<String>,
    pub kind: BuildingPartType,
    pub default_line: Option<LineId>,
    pub completion: CompletionStatus,
    pub children: Vec<BuildingPartId>,
    pub devices: Vec<DeviceId>,
    pub parent: Option<BuildingPartId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn building_part_nests_via_children_not_containment() {
        let floor = BuildingPart {
            id: BuildingPartId(2),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            name: "Floor 1".into(),
            number: Some("1".into()),
            kind: BuildingPartType::Floor,
            default_line: None,
            completion: CompletionStatus::FinishedDesign,
            children: vec![],
            devices: vec![],
            parent: Some(BuildingPartId(1)),
        };
        assert_eq!(floor.parent, Some(BuildingPartId(1)));
    }
}
