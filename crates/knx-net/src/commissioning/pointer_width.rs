//! Raw PID 7 pointer values and the explicit width policy that decides which replies are admitted.

use super::property_download::PropertySimulationFailure;
use super::property_recovery::PropertyResourcePointer;

/// A PID_TABLE_REFERENCE answer exactly as the device sent it. A two-octet reply is never
/// padded to four octets, so the observed width stays evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyPointerValue {
    Four([u8; 4]),
    Two([u8; 2]),
}
impl PropertyPointerValue {
    /// `Two` equals a source expectation only when the expectation's high two octets are
    /// zero; byte order is the unverified big-endian reading, never a width guess.
    pub fn matches_source(&self, expected: &[u8; 4]) -> bool {
        match self {
            Self::Four(octets) => octets == expected,
            Self::Two(octets) => expected[..2] == [0, 0] && octets == &expected[2..],
        }
    }
    pub fn same_width(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Four(_), Self::Four(_)) | (Self::Two(_), Self::Two(_))
        )
    }
    pub fn octets(&self) -> &[u8] {
        match self {
            Self::Four(octets) => octets,
            Self::Two(octets) => octets,
        }
    }
}
impl From<[u8; 4]> for PropertyPointerValue {
    fn from(octets: [u8; 4]) -> Self {
        Self::Four(octets)
    }
}
/// Eight hex digits for `Four`, four for `Two`: the width is visible in the text.
impl std::fmt::Display for PropertyPointerValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.octets()
            .iter()
            .try_for_each(|octet| write!(f, "{octet:02x}"))
    }
}

/// Which PID 7 widths a hardware record admits. The width is part of the hashed
/// hardware record, so a replay is bound to the rule that was in force at preparation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PointerWidthAdmission {
    /// Four octets only; the historical behaviour, byte for byte.
    #[default]
    Legacy,
    /// On the listed objects exactly one element of two or four octets. The list is
    /// sorted, unique, non-empty and a subset of the plan's touched objects.
    OneElementTwoOrFour { objects: Vec<u8> },
}
impl PointerWidthAdmission {
    /// Checked constructor; `validate` repeats the plan binding at every entry point.
    pub fn one_element_two_or_four(objects: Vec<u8>) -> Result<Self, PropertySimulationFailure> {
        let admission = Self::OneElementTwoOrFour { objects };
        admission.validate_shape()?;
        Ok(admission)
    }
    fn validate_shape(&self) -> Result<(), PropertySimulationFailure> {
        match self {
            Self::Legacy => Ok(()),
            Self::OneElementTwoOrFour { objects } => {
                if objects.is_empty() || objects.windows(2).any(|w| w[0] >= w[1]) {
                    return Err(super::property_recovery::invalid(
                        "pointer width admission needs a sorted, unique, non-empty object list",
                    ));
                }
                Ok(())
            }
        }
    }
    pub(crate) fn validate(
        &self,
        source_pointers: &[PropertyResourcePointer],
    ) -> Result<(), PropertySimulationFailure> {
        self.validate_shape()?;
        if let Self::OneElementTwoOrFour { objects } = self {
            if objects
                .iter()
                .any(|object| !source_pointers.iter().any(|p| p.object == *object))
            {
                return Err(super::property_recovery::invalid(
                    "pointer width admission names an object the plan does not touch",
                ));
            }
        }
        Ok(())
    }
    /// The one-element two-or-four rule governs this object's PID 7 answers.
    pub fn applies_to(&self, object: u8) -> bool {
        matches!(self, Self::OneElementTwoOrFour { objects } if objects.contains(&object))
    }
}

/// Closed, non-raw reason for a refused PID 7 answer under the one-element rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerWidthRefusalReason {
    /// Zero elements, yet octets followed: a malformed negative answer.
    ZeroElementsWithData,
    /// An element count other than one for a one-element request.
    ElementCount,
    /// One element whose length is neither two nor four octets.
    UnsupportedWidth,
}

/// A refused PID 7 answer: typed observation facts plus a capped raw diagnostic.
/// The raw octets (a hex prefix of at most 16 octets, true length stated) live only in
/// `diagnostic`. That text reaches the private operation journal and the local error text of
/// the caller, exactly like the legacy `Recovery(String)` refusal; this type adds no wider
/// exposure and its `Debug` keeps the text on purpose, because the journal retains it.
/// A future public projection must use the typed fields only, never `diagnostic`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointerWidthRefusal {
    pub reason: PointerWidthRefusalReason,
    pub object: u8,
    pub property_id: u8,
    pub elements: u8,
    pub octets: usize,
    pub(crate) diagnostic: String,
}
impl PointerWidthRefusal {
    /// Private: includes a capped hex prefix of the raw answer. For the private journal and
    /// local error text only; a public projection takes the typed fields instead.
    pub fn private_diagnostic(&self) -> &str {
        &self.diagnostic
    }
}
