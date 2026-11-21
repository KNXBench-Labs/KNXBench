# PRG-01-N4: unadmitted run-state masks — acceptance record

Observed: 21 November 2026. Simulator/loopback only; no bus, device or hardware write.

Scoping slice: no source-backed, simulator-verifiable new semantics exist
([research](../research/2026-11-21-run-state-remaining-masks-and-layouts.md),
[contract](../contracts/post-download-run-state-unadmitted-masks.md),
[ADR-0182](../adr/0182-unadmitted-run-state-masks-stay-refused.md)). Delivered:
refusal tests `commissioning::post_download::tests::run_state::unadmitted_masks`.
Verification output and package-gate receipt: see the PR description.

Not hardware acceptance, not closure of PRG-01;
`regular-070n-exclusive-inactive-storage-survival` is not claimed.
