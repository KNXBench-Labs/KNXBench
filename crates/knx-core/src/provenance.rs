//! Where a resolved value came from.

/// Where a resolved value came from.
///
/// A communication object's effective properties resolve through three layers
/// in the source data (`ComObject` → `ComObjectRef` → `ComObjectInstanceRef`).
/// 758 of 907 instances in the reference project override the datapoint type at
/// instance level, so a model without provenance cannot decide what to write
/// back on export. See the architecture spec, section 5.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// Default from the application program's `ComObject`.
    Program,
    /// Per-variant override from the application program's `ComObjectRef`.
    ProgramRef,
    /// Per-device override that was present in the imported project.
    Instance,
    /// Derived by this application, for example a datapoint type taken from
    /// linked communication objects. Never written back as if the user set it.
    Inferred,
    /// Changed in this application.
    UserEdit,
}

impl Layer {
    /// Whether a value carrying this layer is written back to the project file
    /// on export.
    ///
    /// Program-level values belong to the product database and inferred values
    /// are ours, not the user's; neither is exported.
    pub fn is_exported(self) -> bool {
        matches!(self, Layer::Instance | Layer::UserEdit)
    }
}

/// A value together with the layer it was resolved from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Resolved<T> {
    pub value: T,
    pub layer: Layer,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_layer_values_are_not_exported() {
        let from_program = Resolved {
            value: 1u8,
            layer: Layer::Program,
        };
        let from_instance = Resolved {
            value: 1u8,
            layer: Layer::Instance,
        };
        let inferred = Resolved {
            value: 1u8,
            layer: Layer::Inferred,
        };
        let edited = Resolved {
            value: 1u8,
            layer: Layer::UserEdit,
        };

        assert!(!from_program.layer.is_exported());
        assert!(!inferred.layer.is_exported());
        assert!(from_instance.layer.is_exported());
        assert!(edited.layer.is_exported());
    }
}
