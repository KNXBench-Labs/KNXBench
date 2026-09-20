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
    /// Whether a value carrying this layer is the project's own, rather than
    /// something supplied or guessed on its behalf.
    ///
    /// Program-level values belong to the product database and inferred values
    /// are ours, not the user's; neither counts. The name is older than its
    /// remaining callers — `knx-diff` and `knx-etsproj::compare` use it to
    /// decide what is worth comparing, and the CSV and documentation exports
    /// use it to decide what is worth writing. `.knxproj` writing, which
    /// named it, was withdrawn (ADR-0028).
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

/// A source attribute in one of its three real states.
///
/// ETS distinguishes an attribute it never wrote from an attribute it wrote
/// empty: 497 of the reference project's 907 `ComObjectInstanceRef` elements
/// carry `DatapointType=""`, and 82 of its `Description` attributes are
/// likewise empty. Collapsing both into `None` loses that distinction, and
/// export then writes a file that differs from the one that was read.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Override<T> {
    /// The attribute was not present in the source.
    #[default]
    Absent,
    /// The attribute was present and its value was the empty string.
    Empty,
    /// The attribute was present and carried a value.
    Value(Resolved<T>),
    /// The attribute was present and carried a non-empty value this
    /// application could not parse into `T`. The raw text is kept exactly
    /// as it was read, so export writes back what the source file said
    /// instead of dropping the attribute — an unreadable value is still
    /// the user's data. An importer that produces this state must also
    /// report the problem; the model itself carries no report.
    Malformed(String),
}

impl<T> Override<T> {
    pub fn value(&self) -> Option<&Resolved<T>> {
        match self {
            Override::Value(r) => Some(r),
            _ => None,
        }
    }

    pub fn layer(&self) -> Option<Layer> {
        self.value().map(|r| r.layer)
    }

    /// The raw text of a value that was present but could not be parsed.
    pub fn malformed(&self) -> Option<&str> {
        match self {
            Override::Malformed(raw) => Some(raw.as_str()),
            _ => None,
        }
    }

    /// Whether the attribute appeared in the source at all, empty or not,
    /// readable or not.
    pub fn is_present(&self) -> bool {
        !matches!(self, Override::Absent)
    }
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

    #[test]
    fn a_malformed_value_is_present_carries_no_value_and_keeps_its_raw_text() {
        let malformed: Override<u8> = Override::Malformed("not-a-number".into());
        assert!(malformed.is_present());
        assert!(malformed.value().is_none());
        assert_eq!(malformed.malformed(), Some("not-a-number"));
        assert_ne!(malformed, Override::Absent);
        assert_ne!(malformed, Override::Empty);
    }

    #[test]
    fn absent_and_empty_are_distinct_and_neither_carries_a_value() {
        let absent: Override<u8> = Override::Absent;
        let empty: Override<u8> = Override::Empty;
        assert_ne!(absent, empty);
        assert!(absent.value().is_none());
        assert!(empty.value().is_none());
        assert!(!absent.is_present());
        assert!(empty.is_present());
    }

    #[test]
    fn a_value_override_reports_its_layer() {
        let o = Override::Value(Resolved {
            value: 7u8,
            layer: Layer::Instance,
        });
        assert_eq!(o.layer(), Some(Layer::Instance));
        assert_eq!(o.value().map(|r| r.value), Some(7));
        assert!(o.is_present());
    }
}
