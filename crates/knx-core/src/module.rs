//! Module instances (ADR-0013): a schema-≥21 device's modular application
//! programs. Mirrors `parameter.rs`'s shape — retained but uninterpreted.
//! `repeat_index` and `arguments` feed the `Dynamic/choose/when` grammar
//! (KNOWN_LIMITATIONS §3) — documented, not unresearched, since RESEARCH
//! §4.3 (`knx-productdb` evaluates it headlessly as of T18 slice 1,
//! 2026-09-11, and slice 2, same day, expands a `Module` node into its
//! `ModuleDef`'s own stored tree) and RESEARCH §4.4 (the R4 spike, same
//! day, which establishes how `Module`/`ModuleDef` naming, argument
//! binding and id-mangling actually work). None of that is wired into a
//! UI or a parameter editor yet, and this `ModuleInstance` type itself
//! stays exactly as retained-but-uninterpreted as `parameter.rs`'s;
//! import never needs to evaluate it, because `GroupObjectTree` already
//! carries ETS's own answer (ADR-0014).

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
    /// The verbatim `ModuleInstance/@Id` — e.g. `"MD-2_M-4_MI-1"`, the
    /// `RefId` (`source.ets_id`) plus the `_MI-<k>` component the id grammar
    /// puts on it ([D] Project Schema23 §1.2.5.18, [V] KV v2.5 demo).
    /// Retained uninterpreted, same policy as `repeat_index`; the parameter
    /// editor reads it to reconstruct the module-qualified `ets_id` a write
    /// must target, rather than guessing `MI-1`.
    pub instance_ets_id: String,
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
            instance_ets_id: "MD-2_M-1_MI-1".into(),
            arguments: vec![
                (source("MD-2_A-1"), "1".into()),
                (source("MD-2_A-2"), "1".into()),
            ],
        };
        assert_eq!(m.arguments.len(), 2);
        assert_eq!(m.repeat_index, "6x1");
    }

    /// D38: `instance_ets_id` is the verbatim `ModuleInstance/@Id`, distinct
    /// from `source.ets_id` (the `@RefId`) — the whole reason this field
    /// exists is that the two differ by exactly the `_MI-<k>` suffix.
    #[test]
    fn instance_ets_id_is_retained_distinct_from_source_ets_id() {
        let m = ModuleInstance {
            id: ModuleInstanceId(1),
            device: DeviceId(1),
            source: source("MD-2_M-4"),
            repeat_index: "32x1".into(),
            instance_ets_id: "MD-2_M-4_MI-1".into(),
            arguments: vec![],
        };
        assert_eq!(m.source.ets_id, "MD-2_M-4");
        assert_eq!(m.instance_ets_id, "MD-2_M-4_MI-1");
        assert_ne!(m.source.ets_id, m.instance_ets_id);
    }

    #[test]
    fn two_module_instances_with_different_arguments_are_not_equal() {
        let base = ModuleInstance {
            id: ModuleInstanceId(1),
            device: DeviceId(1),
            source: source("MD-2_M-1"),
            repeat_index: "6x1".into(),
            instance_ets_id: "MD-2_M-1_MI-1".into(),
            arguments: vec![(source("MD-2_A-1"), "1".into())],
        };
        let mut other = base.clone();
        other.arguments[0].1 = "2".into();
        assert_ne!(base, other);
    }
}
