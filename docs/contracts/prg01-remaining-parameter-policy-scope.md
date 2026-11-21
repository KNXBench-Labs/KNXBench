# PRG-01 remaining parameter policies: scope and refusal rule

Status: **scoping contract, no code change**, 21 November 2026. Evidence:
[research](../research/2026-11-21-prg01-n5-remaining-parameter-policy-evidence.md);
decision: [ADR-0183](../adr/0183-prg01-remaining-parameter-policies-need-evidence.md).
Related: [customer-adjustable](customer-adjustable-write-admission.md),
[inactive bytes](inactive-byte-write-selection.md).

## Rules

1. **Refuse rather than guess.** Until the evidence named in the research record
   exists, KNXBench adds no behaviour for: further `DownloadInvisibleParameters`
   tokens, customer-adjustable markers outside the contract's contexts, editable
   Union cases or storage-survival claims.
2. **Unchanged baselines stay authoritative.** `Background` is the only
   invisible-parameter policy that acts (ADR-0161, ADR-0171); other present
   values write no invisible parameter on Easy3/070n and refuse on System2.
   Customer-adjustable presence in admitted contexts refuses regular writes.
   Inactive whole-byte omission keeps its byte-precise fallback.
3. **Admission needs a named artifact.** A later increment may widen a policy
   only when it cites (a) an ETS observation or primary-source statement and (b)
   a simulator test whose RED fails on the old behaviour. A simulator test alone
   never justifies new write semantics.
4. **Wording.** "KNX-compatible, hardware-untested". Never "certified" or "full
   ETS compatibility". `regular-070n-exclusive-inactive-storage-survival` stays
   OPEN and is not claimed by any simulator result.

## Verification of this scope

Document-only. The existing pins that make rule 2 testable are the unit tests
beside `DownloadInvisibleParameters` in `knx-productdb` `image.rs` (unknown
token writes nothing; Background assign; System2 refusal) and the
customer-adjustable suite; they are unchanged and were not rerun for this
document.
