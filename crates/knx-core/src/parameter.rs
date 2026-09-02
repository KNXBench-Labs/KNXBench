//! Parameter instances: retained but uninterpreted (DATA_MODEL §10). 1390
//! values in the reference project, dropped entirely by `xknxproject`. Held
//! as raw strings because interpreting them requires the `Dynamic` tree
//! grammar, which is unresearched (RESEARCH R3).

use crate::ids::{ParameterInstanceId, SourceRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterInstance {
    pub id: ParameterInstanceId,
    /// `RefId`.
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
            source: SourceRef {
                path: "t".into(),
                ets_id: "M-x_P-1".into(),
            },
            raw: "42".into(),
        };
        assert_eq!(p.raw, "42");
    }
}
