//! Parameter instances: retained but uninterpreted (DATA_MODEL §10). 1390
//! values in the reference project, dropped entirely by `xknxproject`. Held
//! as raw strings because interpreting them requires the `Dynamic` tree
//! grammar, which is unresearched (RESEARCH R3).

use crate::ids::{DeviceId, ParameterInstanceId, SourceRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterInstance {
    pub id: ParameterInstanceId,
    pub device: DeviceId,
    /// `RefId`, verbatim. Two forms occur: `<app>_P-<n>_R-<n>` for a plain
    /// parameter and `<app>_UP-<n>_R-<n>` for a union parameter, 1174 and 216
    /// respectively in the reference project. Neither is interpreted here.
    pub source: SourceRef,
    /// `Value`, uninterpreted.
    pub raw: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameter_instance_holds_raw_value_uninterpreted() {
        let p = ParameterInstance {
            id: ParameterInstanceId(1),
            device: DeviceId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "M-x_P-1".into(),
            },
            raw: "42".into(),
        };
        assert_eq!(p.raw, "42");
    }

    #[test]
    fn a_union_parameter_ref_id_is_stored_verbatim() {
        let p = ParameterInstance {
            id: ParameterInstanceId(1),
            device: DeviceId(3),
            source: SourceRef {
                path: "P-0512/0.xml".into(),
                ets_id: "M-0083_A-0026-15-7565_UP-411_R-411".into(),
            },
            raw: "1".into(),
        };
        assert!(p.source.ets_id.contains("_UP-"));
    }
}
