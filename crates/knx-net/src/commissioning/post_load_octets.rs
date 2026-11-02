//! Written octets a device's own firmware may change after load, and what verification saw.
//!
//! An explicit, source-bound opt-in, never a default. Each declaration names one octet that
//! the executed plan writes and the other values the device may hold there afterwards. Final
//! verification then accepts exactly those values at exactly those octets and reports every
//! tolerated difference as an observation; every other octet stays strictly compared.

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
}

impl std::fmt::Display for PostLoadOctetObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "object {} octet {:04X}h: wrote {:02X}h, device holds declared post-load value {:02X}h",
            self.object, self.address, self.written, self.observed
        )
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
                });
            }
        }
        (tolerated, observations)
    }
}

/// The last value `plan` writes to `address` of `object`, if it writes it at all.
fn written_value(plan: &PropertyDownloadPlan, object: u8, address: u16) -> Option<u8> {
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
