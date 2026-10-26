//! Typed, non-raw System-2 failure facts: closed reason, object, property and counts only.
//!
//! Built from the delivered Net `PointerWidthRefusal` by an explicit typed match. Never
//! derived from Display/Debug text, and never carries the refusal's private raw diagnostic.
use crate::property_validation_bundle::invalid;
use knx_net::commissioning::{
    pointer_width::{PointerWidthRefusal, PointerWidthRefusalReason},
    property_download::PropertySimulationFailure,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::io;

/// The PID 7 (table reference) property; the only property a pointer refusal names.
const PID_TABLE_REFERENCE: u8 = 7;

/// Closed reason of a refused PID 7 answer under the one-element two-or-four rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PointerFailureReason {
    ZeroElementsWithData,
    ElementCount,
    UnsupportedWidth,
}
/// Which run of one durable operation failed: the download or its explicit restore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FailureDiagnosticRun {
    Download,
    Restore,
}
/// Private journal form, stored inside `Failed`/`RestoreFailed`; the run is the variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointerFailureRecord {
    pub reason: PointerFailureReason,
    pub object: u8,
    pub property_id: u8,
    pub elements: u8,
    pub octets: u32,
}
impl PointerFailureRecord {
    /// The typed facts of a PID 7 width refusal; `None` for every other failure, and for
    /// a refusal whose facts break the closed reason/count grammar (nothing is invented).
    pub fn from_failure(failure: &PropertySimulationFailure) -> Option<Self> {
        let PropertySimulationFailure::PointerWidth(refusal) = failure else {
            return None;
        };
        let record = Self::from_refusal(refusal)?;
        record.validate().ok().map(|()| record)
    }
    fn from_refusal(refusal: &PointerWidthRefusal) -> Option<Self> {
        Some(Self {
            reason: match refusal.reason {
                PointerWidthRefusalReason::ZeroElementsWithData => {
                    PointerFailureReason::ZeroElementsWithData
                }
                PointerWidthRefusalReason::ElementCount => PointerFailureReason::ElementCount,
                PointerWidthRefusalReason::UnsupportedWidth => {
                    PointerFailureReason::UnsupportedWidth
                }
            },
            object: refusal.object,
            property_id: refusal.property_id,
            elements: refusal.elements,
            octets: u32::try_from(refusal.octets).ok()?,
        })
    }
    /// Reason, counts and property must agree exactly as Net's refusal rule produces them.
    pub(crate) fn validate(&self) -> io::Result<()> {
        let consistent = match self.reason {
            PointerFailureReason::ZeroElementsWithData => self.elements == 0 && self.octets > 0,
            PointerFailureReason::ElementCount => self.elements >= 2,
            PointerFailureReason::UnsupportedWidth => {
                self.elements == 1 && self.octets != 2 && self.octets != 4
            }
        };
        if !consistent || self.property_id != PID_TABLE_REFERENCE {
            return Err(invalid("inconsistent typed pointer failure record"));
        }
        Ok(())
    }
    pub(crate) fn diagnostic(&self, run: FailureDiagnosticRun) -> FailureDiagnostic {
        FailureDiagnostic {
            run,
            reason: self.reason,
            object: self.object,
            property_id: self.property_id,
            elements: self.elements,
            octets: self.octets,
        }
    }
}
/// Journal key reader: an absent key is `None` (via `default`), a present key must hold a
/// record. Explicit `null` is refused, so no version-1 checkpoint carries a key that an
/// older strict reader would reject without any populated record behind it.
pub(crate) fn present_record<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<PointerFailureRecord>, D::Error> {
    PointerFailureRecord::deserialize(deserializer).map(Some)
}
/// Shareable projection of one typed failure: exactly these six camelCase keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FailureDiagnostic {
    pub run: FailureDiagnosticRun,
    pub reason: PointerFailureReason,
    pub object: u8,
    pub property_id: u8,
    pub elements: u8,
    pub octets: u32,
}
