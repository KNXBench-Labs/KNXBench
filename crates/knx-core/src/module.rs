//! Module instances (ADR-0013): a schema-≥21 device's modular application
//! programs. Mirrors `parameter.rs`'s shape — retained but uninterpreted.
//! `repeat_index` and `arguments` feed the `Dynamic/choose/when` grammar
//! (KNOWN_LIMITATIONS §3) — documented, not unresearched, since RESEARCH
//! §4.3 (`knx-productdb` evaluates it headlessly as of T18 slice 1,
//! 2026-09-11, but `Module` expansion is not part of that); import never
//! needs to evaluate it, because `GroupObjectTree` already carries ETS's
//! own answer (ADR-0014).

use crate::ids::{DeviceId, ModuleInstanceId, SourceRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleInstance {
    pub id: ModuleInstanceId,
    pub device: DeviceId,
    /// The `ModuleInstance` element this came from. `ets_id` is the raw
    /// `RefId` (e.g. `"MD-2_M-1"`), never interpreted here — resolving it to
    /// an application program's `ModuleDef`/`Module` chain is
    /// `knx-etsproj`'s/`knx-productdb`'s job, not this crate's.
    pub source: SourceRef,
    /// `RepeatIndex`, e.g. `"6x1"`. Retained opaque (ADR-0013) — its
    /// `"NxM"` shape is not parsed.
    pub repeat_index: String,
    /// `Arguments/Argument`, as `(source, value)` pairs. Uninterpreted, same
    /// policy as `ParameterInstanceRef::value`.
    pub arguments: Vec<(SourceRef, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(ets_id: &str) -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: ets_id.into(),
        }
    }

    #[test]
    fn a_module_instance_holds_its_arguments_uninterpreted() {
        let m = ModuleInstance {
            id: ModuleInstanceId(1),
            device: DeviceId(1),
            source: source("MD-2_M-1"),
            repeat_index: "6x1".into(),
            arguments: vec![
                (source("MD-2_A-1"), "1".into()),
                (source("MD-2_A-2"), "1".into()),
            ],
        };
        assert_eq!(m.arguments.len(), 2);
        assert_eq!(m.repeat_index, "6x1");
    }

    #[test]
    fn two_module_instances_with_different_arguments_are_not_equal() {
        let base = ModuleInstance {
            id: ModuleInstanceId(1),
            device: DeviceId(1),
            source: source("MD-2_M-1"),
            repeat_index: "6x1".into(),
            arguments: vec![(source("MD-2_A-1"), "1".into())],
        };
        let mut other = base.clone();
        other.arguments[0].1 = "2".into();
        assert_ne!(base, other);
    }
}
