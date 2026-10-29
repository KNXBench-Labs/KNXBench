//! Private journal record of a final System-2 allocation mismatch's bytes and its typed view.
//!
//! Stored beside the final-step record (envelope format 4). Replay recomputes it from the
//! durable recovery snapshot, so only exactly what Net produces is accepted. Never part of
//! progress, status or the shareable reports; only a private diagnostic bundle reads it.
use crate::property_validation::{
    FailureDiagnosticRun, FinalStepCategory, FinalStepRecord, FinalStepStage,
};
use crate::property_validation_bundle::invalid;
use knx_core::commissioning::property_download::PropertyDownloadPlan;
use knx_net::commissioning::{
    final_step_evidence::{AllocationMismatchEvidence, DifferenceClass},
    property_download::PropertySimulationError,
    property_recovery::PropertyRecoverySnapshot,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::io;

/// How one differing byte relates to the executed plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DifferenceClassification {
    /// The executed plan wrote it.
    Written,
    /// The plan left it alone; its expectation is the captured prestate.
    Unwritten,
    /// The plan left it alone and the prestate was unreadable: the expected byte is a zero
    /// placeholder, not a prediction.
    UnwrittenUnpredicted,
}
/// A run of offsets into the allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OffsetRange {
    pub offset: usize,
    pub length: usize,
}
/// One maximal run of differing bytes of one class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DifferingRangeRecord {
    pub offset: usize,
    pub length: usize,
    pub classification: DifferenceClassification,
}
/// Private journal form, inside `Failed`/`RestoreFailed` next to a verification-mismatch
/// final-step record of the allocation stage; the run is the variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllocationEvidenceRecord {
    pub allocation_index: usize,
    pub object: u8,
    pub address: u16,
    pub length: usize,
    pub prestate_unreadable: bool,
    /// Lowercase hex of the whole allocation as expected, `length` octets.
    pub expected: String,
    /// Lowercase hex of the whole allocation as read back, `length` octets.
    pub actual: String,
    /// The offsets the executed plan wrote, as maximal runs.
    pub written: Vec<OffsetRange>,
    pub differing: Vec<DifferingRangeRecord>,
}
impl AllocationEvidenceRecord {
    /// The bytes Net attached to the stop, kept only beside the allocation-mismatch record
    /// the same stop produced; `None` otherwise (nothing is invented or reattributed).
    pub fn from_error(
        error: &PropertySimulationError,
        final_step: Option<&FinalStepRecord>,
    ) -> Option<Self> {
        let evidence = error.final_step_evidence.as_deref()?;
        is_allocation_mismatch(final_step?).then(|| Self::from_evidence(evidence))
    }
    fn from_evidence(evidence: &AllocationMismatchEvidence) -> Self {
        let mut written: Vec<OffsetRange> = Vec::new();
        for (offset, _) in evidence.written.iter().enumerate().filter(|(_, w)| **w) {
            match written.last_mut() {
                Some(last) if last.offset + last.length == offset => last.length += 1,
                _ => written.push(OffsetRange { offset, length: 1 }),
            }
        }
        Self {
            allocation_index: evidence.allocation_index,
            object: evidence.object,
            address: evidence.address,
            length: evidence.length(),
            prestate_unreadable: evidence.prestate_unreadable,
            expected: hex(&evidence.expected),
            actual: hex(&evidence.actual),
            written,
            differing: evidence
                .differing
                .iter()
                .map(|range| DifferingRangeRecord {
                    offset: range.offset,
                    length: range.length,
                    classification: match range.class {
                        DifferenceClass::Written => DifferenceClassification::Written,
                        DifferenceClass::Unwritten => DifferenceClassification::Unwritten,
                        DifferenceClass::UnwrittenUnpredicted => {
                            DifferenceClassification::UnwrittenUnpredicted
                        }
                    },
                })
                .collect(),
        }
    }
    /// Strict replay: the record must equal the comparison Net computes from the durable
    /// snapshot, the run's executed plan and the recorded actual bytes. This refuses
    /// malformed hex, wrong lengths, foreign allocations, edited expectations or ranges,
    /// and a record of bytes that would have passed verification.
    pub(crate) fn validate(
        &self,
        snapshot: &PropertyRecoverySnapshot,
        executed_plan: &PropertyDownloadPlan,
    ) -> io::Result<()> {
        decode(&self.expected, self.length)?;
        let actual = decode(&self.actual, self.length)?;
        let replayed = snapshot
            .allocation_mismatch(self.allocation_index, executed_plan, actual)
            .ok_or_else(|| invalid("allocation evidence is not a failing final comparison"))?;
        if Self::from_evidence(&replayed) != *self {
            return Err(invalid(
                "allocation evidence differs from its replayed comparison",
            ));
        }
        Ok(())
    }
    fn view(&self) -> AllocationMismatchEvidenceView {
        // Validated on apply, so both sides decode to `length` octets.
        let expected = decode(&self.expected, self.length).unwrap_or_default();
        let actual = decode(&self.actual, self.length).unwrap_or_default();
        let slice = |bytes: &[u8], r: &DifferingRangeRecord| {
            hex(bytes.get(r.offset..r.offset + r.length).unwrap_or_default())
        };
        AllocationMismatchEvidenceView {
            allocation_index: self.allocation_index,
            object: self.object,
            address: self.address,
            length: self.length,
            prestate_unreadable: self.prestate_unreadable,
            expected: self.expected.clone(),
            actual: self.actual.clone(),
            written: self.written.clone(),
            differing_ranges: self
                .differing
                .iter()
                .map(|r| DifferingRangeView {
                    offset: r.offset,
                    length: r.length,
                    classification: r.classification,
                    expected: slice(&expected, r),
                    actual: slice(&actual, r),
                })
                .collect(),
        }
    }
}
/// The one final-step record an evidence record may accompany.
pub(crate) fn is_allocation_mismatch(record: &FinalStepRecord) -> bool {
    record.category == FinalStepCategory::VerificationMismatch
        && record.stage == FinalStepStage::Allocation
}
/// Journal key reader: absent is `None` (via `default`); explicit `null` is refused.
pub(crate) fn present_record<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<AllocationEvidenceRecord>, D::Error> {
    AllocationEvidenceRecord::deserialize(deserializer).map(Some)
}

/// Why a typed allocation mismatch has no recorded bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceUnavailableReason {
    /// The failure checkpoint carries no evidence record: written before envelope format 4,
    /// when the bytes were not kept. Never reconstructed from text or guessed.
    OlderJournal,
}
/// Whether the compared bytes of one allocation mismatch exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "availability", rename_all = "camelCase")]
pub enum AllocationEvidenceAvailability {
    Recorded(AllocationMismatchEvidenceView),
    Unavailable { reason: EvidenceUnavailableReason },
}
/// One typed final allocation mismatch of one run and its evidence availability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocationMismatchEvidenceEntry {
    pub run: FailureDiagnosticRun,
    pub evidence: AllocationEvidenceAvailability,
}
/// Private, complete view of a recorded mismatch: bytes as lowercase hex.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocationMismatchEvidenceView {
    pub allocation_index: usize,
    pub object: u8,
    pub address: u16,
    pub length: usize,
    pub prestate_unreadable: bool,
    pub expected: String,
    pub actual: String,
    pub written: Vec<OffsetRange>,
    pub differing_ranges: Vec<DifferingRangeView>,
}
/// One differing run with both sides' bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DifferingRangeView {
    pub offset: usize,
    pub length: usize,
    pub classification: DifferenceClassification,
    pub expected: String,
    pub actual: String,
}
/// The journal's record of one run's allocation mismatch, or its absence.
pub(crate) fn entry(
    run: FailureDiagnosticRun,
    record: Option<&AllocationEvidenceRecord>,
) -> AllocationMismatchEvidenceEntry {
    AllocationMismatchEvidenceEntry {
        run,
        evidence: match record {
            Some(record) => AllocationEvidenceAvailability::Recorded(record.view()),
            None => AllocationEvidenceAvailability::Unavailable {
                reason: EvidenceUnavailableReason::OlderJournal,
            },
        },
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
/// Lowercase hex of exactly `length` octets; anything else is refused.
fn decode(text: &str, length: usize) -> io::Result<Vec<u8>> {
    let digit = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    if text.len() != length.saturating_mul(2) {
        return Err(invalid("allocation evidence length differs from its bytes"));
    }
    // The length check above leaves no remainder.
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|[high, low]| Some(digit(*high)? << 4 | digit(*low)?))
        .collect::<Option<Vec<u8>>>()
        .ok_or_else(|| invalid("allocation evidence bytes are not lowercase hex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_is_lowercase_exact_and_length_bound() {
        assert_eq!(hex(&[0x00, 0xAB, 0x7F]), "00ab7f");
        assert_eq!(decode("00ab7f", 3).unwrap(), [0x00, 0xAB, 0x7F]);
        for (text, length) in [
            ("00AB7F", 3),
            ("00ab7", 3),
            ("00ab7f", 2),
            ("0g", 1),
            ("", 1),
        ] {
            assert!(decode(text, length).is_err(), "{text:?}/{length}");
        }
        assert_eq!(decode("", 0).unwrap(), Vec::<u8>::new());
    }
}
