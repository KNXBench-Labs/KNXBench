//! The bytes behind a final System-2 allocation mismatch: where they differ and what each held.
//!
//! Recorded only. Producing this value is the strict comparison itself, so the verification
//! outcome, its message and its payload-free classification stay exactly as before.

use super::property_recovery::PropertyRecoveryRegion;

/// How one differing byte relates to the executed plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifferenceClass {
    /// The executed plan wrote this byte.
    Written,
    /// The plan left this byte alone; its expectation is the captured prestate.
    Unwritten,
    /// The plan left this byte alone and the allocation's prestate was unreadable: the
    /// expectation is a zero placeholder, not a prediction. Never a failure on its own.
    UnwrittenUnpredicted,
}

/// One maximal run of differing bytes of one class, as offsets into the allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifferingRange {
    pub offset: usize,
    pub length: usize,
    pub class: DifferenceClass,
}

/// The complete comparison of the one allocation that failed final verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationMismatchEvidence {
    /// Index into the recovery snapshot's `allocations`.
    pub allocation_index: usize,
    pub object: u8,
    /// Absolute start address of the allocation.
    pub address: u16,
    /// The prestate read was refused or short, so unwritten expected bytes are placeholders.
    pub prestate_unreadable: bool,
    /// The whole allocation: captured prestate overlaid with the executed plan's writes.
    pub expected: Vec<u8>,
    /// The whole allocation as read back by the final verification.
    pub actual: Vec<u8>,
    /// Per byte: whether the executed plan wrote it.
    pub written: Vec<bool>,
    /// Every differing byte in address order, split where the class changes.
    pub differing: Vec<DifferingRange>,
}

impl AllocationMismatchEvidence {
    /// Allocated length in octets.
    pub fn length(&self) -> usize {
        self.expected.len()
    }

    /// The strict final comparison of one allocation: `Some` exactly when it fails.
    ///
    /// With an unreadable prestate only written bytes decide, because nothing predicts the
    /// others; once a written byte differs every differing byte is still recorded. Lengths
    /// are equal by construction (`read_region` refuses a short answer).
    pub(crate) fn compare(
        allocation_index: usize,
        allocation: &PropertyRecoveryRegion,
        prestate_unreadable: bool,
        expected: Vec<u8>,
        written: Vec<bool>,
        actual: Vec<u8>,
        tolerated: &[bool],
    ) -> Option<Self> {
        // A tolerated offset holds a declared post-load value of a written octet: it
        // neither fails nor appears as a difference; `actual` still keeps its real value.
        let differs =
            actual
                .iter()
                .zip(&expected)
                .zip(&written)
                .enumerate()
                .any(|(offset, ((a, e), w))| {
                    a != e && !tolerated[offset] && (*w || !prestate_unreadable)
                });
        if !differs {
            return None;
        }
        let differing =
            differing_ranges(&expected, &actual, &written, prestate_unreadable, tolerated);
        Some(Self {
            allocation_index,
            object: allocation.object,
            address: allocation.address,
            prestate_unreadable,
            expected,
            actual,
            written,
            differing,
        })
    }
}

fn differing_ranges(
    expected: &[u8],
    actual: &[u8],
    written: &[bool],
    prestate_unreadable: bool,
    tolerated: &[bool],
) -> Vec<DifferingRange> {
    let mut ranges: Vec<DifferingRange> = Vec::new();
    for (offset, ((e, a), w)) in expected.iter().zip(actual).zip(written).enumerate() {
        if e == a || tolerated[offset] {
            continue;
        }
        let class = match (*w, prestate_unreadable) {
            (true, _) => DifferenceClass::Written,
            (false, false) => DifferenceClass::Unwritten,
            (false, true) => DifferenceClass::UnwrittenUnpredicted,
        };
        match ranges.last_mut() {
            Some(last) if last.class == class && last.offset + last.length == offset => {
                last.length += 1;
            }
            _ => ranges.push(DifferingRange {
                offset,
                length: 1,
                class,
            }),
        }
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;
    use DifferenceClass::*;

    fn region(length: usize) -> PropertyRecoveryRegion {
        PropertyRecoveryRegion {
            operation_index: 4,
            source_index: 4,
            object: 3,
            address: 0x3C00,
            data: vec![0; length],
        }
    }

    fn range(offset: usize, length: usize, class: DifferenceClass) -> DifferingRange {
        DifferingRange {
            offset,
            length,
            class,
        }
    }

    #[test]
    fn runs_split_where_the_class_changes_and_where_equal_bytes_interrupt() {
        let written = [false, false, true, true, true, false, false, false];
        let expected = vec![0, 1, 2, 3, 4, 5, 6, 7];
        let actual = vec![0, 9, 9, 3, 9, 9, 6, 9];
        let evidence = AllocationMismatchEvidence::compare(
            2,
            &region(8),
            false,
            expected,
            written.to_vec(),
            actual,
            &[false; 8],
        )
        .unwrap();
        assert_eq!(
            evidence.differing,
            [
                range(1, 1, Unwritten),
                range(2, 1, Written),
                range(4, 1, Written),
                range(5, 1, Unwritten),
                range(7, 1, Unwritten),
            ]
        );
        assert_eq!(evidence.length(), 8);
        assert_eq!((evidence.object, evidence.address), (3, 0x3C00));
    }

    #[test]
    fn an_unreadable_prestate_fails_only_on_a_written_byte_and_then_records_everything() {
        let written = vec![false, true, true, false];
        assert_eq!(
            AllocationMismatchEvidence::compare(
                0,
                &region(4),
                true,
                vec![0, 1, 2, 0],
                written.clone(),
                vec![7, 1, 2, 7],
                &[false; 4],
            ),
            None,
            "placeholders never fail verification"
        );
        let evidence = AllocationMismatchEvidence::compare(
            0,
            &region(4),
            true,
            vec![0, 1, 2, 0],
            written,
            vec![7, 1, 9, 7],
            &[false; 4],
        )
        .unwrap();
        assert_eq!(
            evidence.differing,
            [
                range(0, 1, UnwrittenUnpredicted),
                range(2, 1, Written),
                range(3, 1, UnwrittenUnpredicted),
            ]
        );
    }

    #[test]
    fn equal_bytes_produce_nothing() {
        assert_eq!(
            AllocationMismatchEvidence::compare(
                0,
                &region(2),
                false,
                vec![1, 2],
                vec![true, false],
                vec![1, 2],
                &[false; 2],
            ),
            None
        );
    }

    #[test]
    fn a_tolerated_offset_neither_fails_nor_appears_but_keeps_its_real_value() {
        let written = vec![true, true, false];
        let tolerated = [false, true, false];
        assert_eq!(
            AllocationMismatchEvidence::compare(
                0,
                &region(3),
                false,
                vec![1, 1, 5],
                written.clone(),
                vec![1, 0, 5],
                &tolerated,
            ),
            None,
            "only the tolerated octet differs"
        );
        let evidence = AllocationMismatchEvidence::compare(
            0,
            &region(3),
            false,
            vec![1, 1, 5],
            written,
            vec![9, 0, 5],
            &tolerated,
        )
        .unwrap();
        assert_eq!(evidence.differing, [range(0, 1, Written)]);
        assert_eq!(evidence.actual, [9, 0, 5], "the real readback is kept");
        // Unreadable prestate: only written octets decide, a tolerated one included.
        let unpredicted = AllocationMismatchEvidence::compare(
            0,
            &region(3),
            true,
            vec![0, 1, 0],
            vec![false, true, false],
            vec![7, 0, 7],
            &[false, true, false],
        );
        assert_eq!(unpredicted, None);
        let evidence = AllocationMismatchEvidence::compare(
            0,
            &region(3),
            true,
            vec![0, 1, 0],
            vec![false, true, false],
            vec![7, 0x7F, 7],
            &[false; 3],
        )
        .unwrap();
        assert_eq!(
            evidence.differing.len(),
            3,
            "untolerated written octet decides"
        );
    }
}
