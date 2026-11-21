# PRG-01 mutation-effects observation: scope and refusal rule

Status: **scoping contract, no code change**, 21 November 2026. Evidence:
[research](../research/2026-11-21-prg01-n6-mutation-effects-observation.md);
decision: [ADR-0184](../adr/0184-prg01-mutation-effects-observation-scope.md).
Related: [post-download check](post-download-check.md),
[remaining parameter-policy scope](prg01-remaining-parameter-policy-scope.md) (N5 branch).

## Rules

1. **The observation set is closed.** After a regular memory download the only
   effects KNXBench reports are the ones the post-download check lists: written
   and read-back octets inside the transfer, restart outcome, reach, identity,
   load states and the bounded run-state property. Nothing else is implied by
   them; none is installation function, device health or the same physical exemplar.
2. **No post-restart memory comparison.** No step re-reads written ranges after
   the restart and no history/report/web field reports a content match or
   mismatch, until the evidence in the research record exists. A simulator result
   alone never admits it.
3. **Refusals stay refusals.** Image forms that refuse today keep refusing; no
   simulator-only fixture may relax one.
4. **Admission needs a named artifact:** a cited observation or primary-source
   statement plus a semantic RED failing on the current behaviour, with
   CLI/HTTP/history/report/web consumers in the same package (the run-state rule).
5. **Wording.** "KNX-compatible, hardware-untested". Never "certified" or "full
   ETS compatibility". `regular-070n-exclusive-inactive-storage-survival`, physical
   function and whole-mutation recovery stay OPEN.

## Verification of this scope

Document-only. The statement in rule 2 rests on the module documentation of
`knx-net` `commissioning/post_download.rs` ("No memory region is read or
compared") and on the existing post-download tests, which assert device memory
is unchanged by the check (`memory_reads` lists only identity/state addresses).
They were not rerun for this document.
