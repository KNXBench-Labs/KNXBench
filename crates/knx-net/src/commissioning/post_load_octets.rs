//! Written octets a device's own firmware may change after load, and what verification saw.
//!
//! An explicit, source-bound opt-in, never a default. Each declaration names one octet that
//! the bound download plan writes and the other values the device may hold there afterwards.
//! Final verification of exactly that plan accepts only those values at only those octets and
//! returns each tolerated difference as an observation of a verified outcome; every other
//! octet, and every recovery plan that differs from the download, stays strictly compared.

use super::property_download::PropertySimulationFailure;
use super::property_recovery::{invalid, PropertyRecoveryRegion};
use knx_core::commissioning::property_download::{PropertyDownloadPlan, PropertyOperation};

/// One written octet and the values the device may hold there after load besides it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostLoadOctet {
    pub object: u8,
    pub address: u16,
    /// Distinct values other than the written one; never empty.
    pub accepted: Vec<u8>,
}

/// Which written octets final verification may find changed by the device itself.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PostLoadOctetAdmission {
    /// Every octet must read back exactly as expected.
    #[default]
    Legacy,
    /// Only these octets, each only to one of its accepted values.
    Declared(Vec<PostLoadOctet>),
}

/// A written octet that read back as one of its declared post-load values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PostLoadOctetObservation {
    pub object: u8,
    pub address: u16,
    pub written: u8,
    pub observed: u8,
    /// The octet before the download; `None` when the allocation prestate was unreadable.
    /// Equal to `observed`, the read-back cannot tell a firmware change from a write that
    /// never took effect.
    pub prestate: Option<u8>,
}

impl std::fmt::Display for PostLoadOctetObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "object {} octet {:04X}h: wrote {:02X}h, device holds declared post-load value {:02X}h",
            self.object, self.address, self.written, self.observed
        )?;
        match self.prestate {
            Some(prestate) => write!(f, " (held {prestate:02X}h before the download)"),
            None => write!(f, " (prestate unreadable)"),
        }
    }
}

impl PostLoadOctetAdmission {
    /// Every declaration must name a distinct octet that `plan` writes for that object,
    /// with distinct accepted values none of which equals the value written there.
    pub(crate) fn validate(
        &self,
        plan: &PropertyDownloadPlan,
    ) -> Result<(), PropertySimulationFailure> {
        let Self::Declared(octets) = self else {
            return Ok(());
        };
        if octets.is_empty() {
            return Err(invalid("post-load octet declaration names no octet"));
        }
        for (index, octet) in octets.iter().enumerate() {
            if octets[..index]
                .iter()
                .any(|other| other.object == octet.object && other.address == octet.address)
            {
                return Err(invalid("post-load octet declared twice"));
            }
            let Some(written) = written_value(plan, octet.object, octet.address) else {
                return Err(invalid("post-load octet is not written by this plan"));
            };
            if octet.accepted.is_empty() {
                return Err(invalid("post-load octet accepts no other value"));
            }
            for (position, value) in octet.accepted.iter().enumerate() {
                if *value == written || octet.accepted[..position].contains(value) {
                    return Err(invalid(
                        "post-load values must be distinct and differ from the written value",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Per allocation offset: whether `actual` holds a declared post-load value of a
    /// written octet where it differs from `expected`, with one observation each.
    pub(crate) fn tolerated(
        &self,
        allocation: &PropertyRecoveryRegion,
        prestate_unreadable: bool,
        expected: &[u8],
        written: &[bool],
        actual: &[u8],
    ) -> (Vec<bool>, Vec<PostLoadOctetObservation>) {
        let mut tolerated = vec![false; actual.len()];
        let mut observations = Vec::new();
        let Self::Declared(octets) = self else {
            return (tolerated, observations);
        };
        for octet in octets.iter().filter(|o| o.object == allocation.object) {
            let Some(offset) = octet
                .address
                .checked_sub(allocation.address)
                .map(usize::from)
                .filter(|offset| *offset < actual.len())
            else {
                continue;
            };
            if written[offset]
                && actual[offset] != expected[offset]
                && octet.accepted.contains(&actual[offset])
            {
                tolerated[offset] = true;
                observations.push(PostLoadOctetObservation {
                    object: octet.object,
                    address: octet.address,
                    written: expected[offset],
                    observed: actual[offset],
                    prestate: (!prestate_unreadable).then(|| allocation.data[offset]),
                });
            }
        }
        (tolerated, observations)
    }
}

/// The last value `plan` writes to `address` of `object`, if it writes it at all.
pub fn written_value(plan: &PropertyDownloadPlan, object: u8, address: u16) -> Option<u8> {
    plan.steps()
        .iter()
        .rev()
        .find_map(|step| match &step.operation {
            PropertyOperation::WriteMemory {
                object: written,
                address: start,
                data,
            } if *written == object && address >= *start => {
                data.get(usize::from(address - *start)).copied()
            }
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allocation(object: u8) -> PropertyRecoveryRegion {
        PropertyRecoveryRegion {
            operation_index: 0,
            source_index: 0,
            object,
            address: 0x3C00,
            data: vec![9, 8, 7],
        }
    }

    fn flag(object: u8, address: u16) -> PostLoadOctetAdmission {
        PostLoadOctetAdmission::Declared(vec![PostLoadOctet {
            object,
            address,
            accepted: vec![0],
        }])
    }

    const EXPECTED: [u8; 3] = [9, 1, 7];
    const WRITTEN: [bool; 3] = [false, true, false];

    #[test]
    fn only_the_declared_written_octet_of_the_same_object_is_tolerated() {
        let actual = [9, 0, 7];
        let (tolerated, observed) =
            flag(3, 0x3C01).tolerated(&allocation(3), false, &EXPECTED, &WRITTEN, &actual);
        assert_eq!(tolerated, [false, true, false]);
        assert_eq!(
            observed,
            [PostLoadOctetObservation {
                object: 3,
                address: 0x3C01,
                written: 1,
                observed: 0,
                prestate: Some(8),
            }]
        );
        for (admission, allocation) in [
            (flag(1, 0x3C01), allocation(3)),
            (flag(3, 0x3C01), allocation(1)),
            (flag(3, 0x3C05), allocation(3)),
            (flag(3, 0x3BFF), allocation(3)),
            (PostLoadOctetAdmission::Legacy, allocation(3)),
        ] {
            let (tolerated, observed) =
                admission.tolerated(&allocation, false, &EXPECTED, &WRITTEN, &actual);
            assert_eq!(
                tolerated, [false; 3],
                "{admission:?} on object {}",
                allocation.object
            );
            assert!(observed.is_empty());
        }
    }

    #[test]
    fn an_unwritten_or_unchanged_octet_is_never_tolerated() {
        // Offset 0 is unwritten although it holds an accepted value.
        let (tolerated, observed) =
            flag(3, 0x3C00).tolerated(&allocation(3), false, &EXPECTED, &WRITTEN, &[0, 1, 7]);
        assert_eq!(tolerated, [false; 3]);
        assert!(observed.is_empty());
        // Expected already equals the accepted value: nothing to tolerate or observe.
        let (tolerated, observed) =
            flag(3, 0x3C01).tolerated(&allocation(3), false, &[9, 0, 7], &WRITTEN, &[9, 0, 7]);
        assert_eq!(tolerated, [false; 3]);
        assert!(observed.is_empty());
    }

    #[test]
    fn an_unreadable_prestate_reports_no_prestate() {
        let (_, observed) =
            flag(3, 0x3C01).tolerated(&allocation(3), true, &EXPECTED, &WRITTEN, &[9, 0, 7]);
        assert_eq!(observed[0].prestate, None);
    }
}
