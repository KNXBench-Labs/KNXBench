//! Canonical SHA-256 content digest of the scoped write plan a download runs (WP-48.1, ADR-0179).
//!
//! Plan counts are not identity: a same-width parameter edit or a replaced group link keeps
//! them. The digest covers every octet and address the plan writes, so a report binds to the
//! content it proved. The CLI (`prepare_report`) and the server's reconcile both call
//! [`plan_content_digest`]; there is no second encoder.
//!
//! # Canonical encoding (`knxbench-plan-digest/1`)
//!
//! SHA-256 over the byte string below, rendered as 64 lowercase hex digits. Integers are
//! big-endian; `bytes` is a `u32` length followed by the octets; `str` is `bytes` of UTF-8.
//!
//! ```text
//! "knxbench-plan-digest/1\0"
//! str(program id) str(scope code)  u16(manufacturer) u16(mask)  u32(step count)
//! per step: u8(tag) then the fields below, in order
//!   0 Connect
//!   1 CompareProperty            u8 object, u8 property, bytes inline
//!   2 Delay                      u16 milliseconds
//!   3 WriteProperty              u8 object, u8 property, bytes inline
//!   4 MasterResetCommand         u8 erase code, u8 object, u8 property, u8 command
//!   5 WritePropertyParameters    u8 object, u8 property, u16 elements, bytes octets, bytes owned
//!   6 LoadRecord                 11 record octets
//!   7 WriteMemory                u16 address, bytes octets
//!   8 RequireLoaded              u8 state machine type number
//!   9 RequireExemplar            6 serial octets
//!  10 Restart
//!  11 Disconnect
//! ```
//!
//! Group addresses, associations and parameter bytes are inside the `WriteMemory` /
//! `WriteProperty*` steps, so the digest covers table contents exactly as written, and the
//! scope code covers which parts the run was for. Parameters or links a partial scope does
//! not write are, correctly, not part of that scope's digest.

use knx_core::commissioning::memory_download::{MemoryDownloadPlan, MemoryDownloadStep};
use sha2::{Digest, Sha256};

use crate::device_download::PreparedDownload;
use crate::download_support::scope_code;

const DOMAIN: &[u8] = b"knxbench-plan-digest/1\0";

fn put_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(bytes);
}

fn put_step(out: &mut Vec<u8>, step: &MemoryDownloadStep) {
    match step {
        MemoryDownloadStep::Connect => out.push(0),
        MemoryDownloadStep::CompareProperty {
            object_index,
            property_id,
            inline_data,
        } => {
            out.extend_from_slice(&[1, *object_index, *property_id]);
            put_bytes(out, inline_data);
        }
        MemoryDownloadStep::Delay { milliseconds } => {
            out.push(2);
            out.extend_from_slice(&milliseconds.to_be_bytes());
        }
        MemoryDownloadStep::WriteProperty {
            object_index,
            property_id,
            inline_data,
        } => {
            out.extend_from_slice(&[3, *object_index, *property_id]);
            put_bytes(out, inline_data);
        }
        MemoryDownloadStep::MasterResetCommand {
            erase_code,
            object_index,
            property_id,
            command,
        } => out.extend_from_slice(&[4, *erase_code, *object_index, *property_id, *command]),
        MemoryDownloadStep::WritePropertyParameters {
            object_index,
            property_id,
            elements,
            octets,
            owned,
        } => {
            out.extend_from_slice(&[5, *object_index, *property_id]);
            out.extend_from_slice(&elements.to_be_bytes());
            put_bytes(out, octets);
            put_bytes(out, owned);
        }
        MemoryDownloadStep::LoadRecord(record) => {
            out.push(6);
            out.extend_from_slice(record.octets());
        }
        MemoryDownloadStep::WriteMemory { address, octets } => {
            out.push(7);
            out.extend_from_slice(&address.to_be_bytes());
            put_bytes(out, octets);
        }
        MemoryDownloadStep::RequireLoaded(machine) => {
            out.extend_from_slice(&[8, machine.type_number()]);
        }
        MemoryDownloadStep::RequireExemplar(serial) => {
            out.push(9);
            out.extend_from_slice(&serial.octets());
        }
        MemoryDownloadStep::Restart => out.push(10),
        MemoryDownloadStep::Disconnect => out.push(11),
    }
}

/// The digest of `plan` as run for `program_id` in the scope `scope` (a
/// [`scope_code`](crate::download_support::scope_code) value).
pub fn plan_digest(plan: &MemoryDownloadPlan, program_id: &str, scope: &str) -> String {
    let mut out = DOMAIN.to_vec();
    put_bytes(&mut out, program_id.as_bytes());
    put_bytes(&mut out, scope.as_bytes());
    out.extend_from_slice(&plan.manufacturer.to_be_bytes());
    out.extend_from_slice(&plan.mask.0.to_be_bytes());
    out.extend_from_slice(&(plan.steps.len() as u32).to_be_bytes());
    for step in &plan.steps {
        put_step(&mut out, step);
    }
    Sha256::digest(&out)
        .iter()
        .map(|octet| format!("{octet:02x}"))
        .collect()
}

/// The digest of the plan `prepared` runs, in its scope: the value a report carries and the
/// server recomputes.
pub fn plan_content_digest(prepared: &PreparedDownload) -> String {
    let scope = scope_code(prepared.partial.as_ref().map(|(parts, _)| *parts));
    plan_digest(&prepared.plan, &prepared.request.program_id, scope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::commissioning::load_control_memory::event_record;
    use knx_core::commissioning::load_control_memory::MemoryLoadStateMachine;
    use knx_core::commissioning::load_state::{LoadEvent, MaskVersion};
    use knx_core::commissioning::memory_download::ExemplarSerial;

    fn plan(address: u16, octets: &[u8]) -> MemoryDownloadPlan {
        MemoryDownloadPlan {
            mask: MaskVersion(0x0701),
            manufacturer: 0xFFF6,
            steps: vec![
                MemoryDownloadStep::Connect,
                MemoryDownloadStep::RequireLoaded(MemoryLoadStateMachine::ApplicationProgram),
                MemoryDownloadStep::WriteMemory {
                    address,
                    octets: octets.to_vec(),
                },
                MemoryDownloadStep::Restart,
                MemoryDownloadStep::Disconnect,
            ],
        }
    }

    #[test]
    fn digest_is_deterministic_lowercase_hex() {
        let a = plan_digest(&plan(0x4000, &[1, 2, 3]), "P", "complete");
        assert_eq!(a, plan_digest(&plan(0x4000, &[1, 2, 3]), "P", "complete"));
        assert_eq!(a.len(), 64);
        assert!(a
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
    }

    #[test]
    fn a_single_octet_address_step_program_or_scope_change_changes_the_digest() {
        let base = plan_digest(&plan(0x4000, &[1, 2, 3]), "P", "complete");
        assert_ne!(
            base,
            plan_digest(&plan(0x4000, &[1, 2, 4]), "P", "complete")
        );
        assert_ne!(
            base,
            plan_digest(&plan(0x4001, &[1, 2, 3]), "P", "complete")
        );
        assert_ne!(
            base,
            plan_digest(&plan(0x4000, &[1, 2, 3]), "Q", "complete")
        );
        let mut extra = plan(0x4000, &[1, 2, 3]);
        extra
            .steps
            .insert(3, MemoryDownloadStep::Delay { milliseconds: 1 });
        assert_ne!(base, plan_digest(&extra, "P", "complete"));
        let mut mask = plan(0x4000, &[1, 2, 3]);
        mask.mask = MaskVersion(0x0702);
        assert_ne!(base, plan_digest(&mask, "P", "complete"));
    }

    #[test]
    fn the_same_plan_in_another_scope_has_another_digest() {
        let p = plan(0x4000, &[1, 2, 3]);
        let scopes = [
            "complete",
            "partial-parameters",
            "partial-group-addresses",
            "partial-both",
        ];
        let digests: std::collections::BTreeSet<_> =
            scopes.iter().map(|s| plan_digest(&p, "P", s)).collect();
        assert_eq!(digests.len(), scopes.len());
    }

    #[test]
    fn splitting_octets_differently_is_a_different_plan() {
        let one = plan(0x4000, &[1, 2, 3, 4]);
        let mut two = plan(0x4000, &[1, 2]);
        two.steps.insert(
            3,
            MemoryDownloadStep::WriteMemory {
                address: 0x4002,
                octets: vec![3, 4],
            },
        );
        assert_ne!(
            plan_digest(&one, "P", "complete"),
            plan_digest(&two, "P", "complete")
        );
    }

    fn step_digest(step: MemoryDownloadStep) -> String {
        let plan = MemoryDownloadPlan {
            mask: MaskVersion(0x0701),
            manufacturer: 0xFFF6,
            steps: vec![step],
        };
        plan_digest(&plan, "P", "complete")
    }

    fn distinct(kind: &str, variants: Vec<MemoryDownloadStep>) {
        let count = variants.len();
        let digests: std::collections::BTreeSet<_> =
            variants.into_iter().map(step_digest).collect();
        assert_eq!(
            digests.len(),
            count,
            "{kind}: a hashed field did not change the digest"
        );
    }

    #[test]
    fn every_hashed_field_of_every_step_kind_changes_the_digest() {
        use MemoryDownloadStep as S;
        let record = |event| event_record(MemoryLoadStateMachine::AddressTable, event).unwrap();
        distinct(
            "Connect/Restart/Disconnect",
            vec![S::Connect, S::Restart, S::Disconnect],
        );
        let cmp = |o, p, d: &[u8]| S::CompareProperty {
            object_index: o,
            property_id: p,
            inline_data: d.to_vec(),
        };
        distinct(
            "CompareProperty",
            vec![
                cmp(0, 1, &[1]),
                cmp(1, 1, &[1]),
                cmp(0, 2, &[1]),
                cmp(0, 1, &[2]),
                cmp(0, 1, &[1, 1]),
            ],
        );
        distinct(
            "Delay",
            vec![S::Delay { milliseconds: 1 }, S::Delay { milliseconds: 2 }],
        );
        let wp = |o, p, d: &[u8]| S::WriteProperty {
            object_index: o,
            property_id: p,
            inline_data: d.to_vec(),
        };
        distinct(
            "WriteProperty",
            vec![
                wp(0, 1, &[1]),
                wp(1, 1, &[1]),
                wp(0, 2, &[1]),
                wp(0, 1, &[2]),
                wp(0, 1, &[1, 1]),
            ],
        );
        let mr = |e, o, p, c| S::MasterResetCommand {
            erase_code: e,
            object_index: o,
            property_id: p,
            command: c,
        };
        distinct(
            "MasterResetCommand",
            vec![
                mr(1, 2, 3, 4),
                mr(9, 2, 3, 4),
                mr(1, 9, 3, 4),
                mr(1, 2, 9, 4),
                mr(1, 2, 3, 9),
            ],
        );
        let wpp = |o, p, e, oc: &[u8], ow: &[u8]| S::WritePropertyParameters {
            object_index: o,
            property_id: p,
            elements: e,
            octets: oc.to_vec(),
            owned: ow.to_vec(),
        };
        distinct(
            "WritePropertyParameters",
            vec![
                wpp(1, 2, 3, &[4], &[5]),
                wpp(9, 2, 3, &[4], &[5]),
                wpp(1, 9, 3, &[4], &[5]),
                wpp(1, 2, 9, &[4], &[5]),
                wpp(1, 2, 3, &[9], &[5]),
                wpp(1, 2, 3, &[4], &[9]),
            ],
        );
        distinct(
            "LoadRecord",
            vec![
                S::LoadRecord(record(LoadEvent::Unload)),
                S::LoadRecord(record(LoadEvent::StartLoading)),
                S::LoadRecord(
                    event_record(MemoryLoadStateMachine::PeiProgram, LoadEvent::Unload).unwrap(),
                ),
            ],
        );
        let wm = |a, o: &[u8]| S::WriteMemory {
            address: a,
            octets: o.to_vec(),
        };
        distinct("WriteMemory", vec![wm(1, &[1]), wm(2, &[1]), wm(1, &[2])]);
        distinct(
            "RequireLoaded",
            MemoryLoadStateMachine::ALL
                .iter()
                .map(|m| S::RequireLoaded(*m))
                .collect(),
        );
        let ex = |n: u8| S::RequireExemplar(ExemplarSerial::new([n, 2, 3, 4, 5, 6]));
        let last = S::RequireExemplar(ExemplarSerial::new([1, 2, 3, 4, 5, 9]));
        distinct("RequireExemplar", vec![ex(1), ex(2), last]);
    }

    #[test]
    fn the_digest_of_a_small_fixed_plan_is_pinned() {
        assert_eq!(
            plan_digest(&plan(0x4000, &[1, 2, 3]), "P", "complete"),
            "7b2865b60bd5188c678fb06d6607b9011159fe1b79120ee103bbab0946f30d56"
        );
    }
}
