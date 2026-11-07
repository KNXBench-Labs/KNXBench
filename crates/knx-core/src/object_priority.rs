//! The user's project priority token for one object, separate from captured and wire priority.

/// `ComObjectPriority_t` of Project Schema23 §1.1.2.4: `Low`, `High`, `Alert`.
///
/// A project token, not `knx_net`'s or the image builder's transmission
/// priority: nothing casts one into the other (ADR-0158). Stored as the exact
/// schema literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObjectPriority {
    Low,
    High,
    Alert,
}

impl ObjectPriority {
    pub const ALL: [Self; 3] = [Self::Low, Self::High, Self::Alert];

    /// The exact schema literal.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::High => "High",
            Self::Alert => "Alert",
        }
    }

    /// Parses an exact schema literal; no trimming or case folding.
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|token| token.as_str() == raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_the_exact_schema_literals() {
        for token in ObjectPriority::ALL {
            assert_eq!(ObjectPriority::parse(token.as_str()), Some(token));
        }
        for other in ["", "low", " Low", "Low ", "Urgent", "System", "Normal"] {
            assert_eq!(ObjectPriority::parse(other), None, "{other:?}");
        }
    }
}
