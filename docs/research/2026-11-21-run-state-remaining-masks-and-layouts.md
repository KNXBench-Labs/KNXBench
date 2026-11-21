# Remaining run-state masks, object layouts and memory encoding — 21 November 2026 (PRG-01-N4)

## Question

After `0700h`, `0701h`, `0705h` (and `0702h`, delivered in PRG-01-N1/N2), is any
further run-state observation simulator-verifiable without inventing semantics?

## Evidence reused (not re-inspected)

All facts come from the [0702h source review](2026-11-20-property-run-state-observation-0702.md)
and the [0700h investigation](2026-10-29-property-description-run-state-0700.md) of the
original KNX Standard v3.0.0 PDFs; licensed extracts stay private.

| Candidate gap | What the sources establish | What is missing |
| --- | --- | --- |
| Other `070nh` masks (`0703h`, `0704h`, `0706h`–`070Fh`) | Resources Table 7 approves only `0700h`/`0701h`/`0705h`; PROF profiles those plus `5705h`; MP §3.31.2 names the `070nh` family only for the load-control procedure. | No source approves, profiles or describes any other low nibble, so there is no datatype rule (Table 95 footnote 94 names only `0020h`/`0701h` as legacy exceptions) and no capability claim to bind. |
| Other object layouts | Candidate index 3 is a documented default, validated by PID 1 and a PID 13 reread. MP §3.28 keeps object scanning a separate, conditional procedure. | No source-backed rule for another index, and a scan would add bus traffic whose bounds nothing specifies. |
| Memory run-state encoding | Resources §4.24.3 leaves the memory-mapped Type-2 realization unspecified; MP §3.36.2 names `0101h`/`0102h` for memory run state. | The `070nh` memory encoding itself. Decoding any byte would be a guess. |
| Dynamic/Union/default-token parameter policies | Existing contracts refuse them. | Physical inactive-byte survival and customer-write rules need hardware proof (not claimed). |

## Finding

No remaining run-state gap is both source-backed and simulator-verifiable. The
simulator can only pin the **refusal**: an unadmitted mask produces no
run-state frame beyond the identity read. Existing tests pinned only `0702h`
and `070Fh`; this slice pins every other unadmitted low nibble and masks
outside the family, including precedence over a failed load read.

Boundaries unchanged: no hardware claim, no new session/retry/wait, KNX-compatible
wording only, `regular-070n-exclusive-inactive-storage-survival` is not claimed.
