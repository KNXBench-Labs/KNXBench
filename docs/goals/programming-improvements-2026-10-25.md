# Programming correctness improvements — 25 October 2026

## Scope and authority

Improve the existing regular memory-download workflow without broadening device
admission. This is source development and offline/simulator validation only;
there is no authority here for live-device operations, deployment or a release.
KNX Secure is outside this workstream.

Three bounded outcomes are admitted for this implementation round:

1. **Scope advice.** Derive the smallest application-side download from the
   project's existing programming flags. Application pending means complete;
   otherwise select parameters, group communication, both, or no application
   mutation. Report individual-address and medium-configuration needs separately.
   CLI and web must consume the same application decision. Imported/project
   flags remain planning hints, never live-device evidence. Bind approval to a
   concrete plan; a failed partial precondition must not trigger a full download.
2. **Hardware identity response width.** For the Device Object's hardware-type
   comparison, require the complete six-octet response. Preserve supported
   zero-padded product comparison declarations and existing generic comparison
   behavior for resources whose response width is not established. Refusal must
   precede backup persistence and any download mutation.
3. **Post-transfer observation.** After a successful regular transfer and its
   existing restart/disconnect wait, use a fresh read-only session to observe
   target/application identity and the plan's load-state machines. Preserve
   partial observations and unknown/unavailable states. Transfer verification,
   restart acknowledgment, renewed reachability and loaded state are separate
   claims. Loaded does not prove that an application is running or that the
   installation functions. No restart/reset fallback or mutating retry follows
   an unavailable observation.

## Acceptance

Each outcome needs a focused commit, independent source review, discriminating
regressions and maintained integrated package-gate acceptance before publication.
Current acceptance is **pending**; implementation preparation is not delivery.

- Scope tests cover all loaded-flag combinations, a missing address, no-op,
  explicit-scope compatibility, project freshness and refusal without escalation.
- Identity tests retain valid six-octet/padded comparisons, reject short and long
  responses through the actual executor, and prove the backup/write barrier.
- Observation tests retain one persistent simulated device through transfer and
  follow-up, distinguish mismatches from unavailable reads, retain earlier
  readings on later failure, and prove that the follow-up sends no mutation.
- API, CLI, web and durable history must agree about the facts they expose.
  Missing older result fields must remain unavailable, not become success.
- Build, documentation, local-information and privacy checks apply to the final
  combined source. Simulator acceptance is not hardware acceptance.

## Boundaries retained

No additional device family, application slot, programming-address mutation,
access-key mutation, medium configuration, batch execution or firmware path is
admitted by these changes. Existing product/procedure, backup and recovery gates
remain mandatory.

Parameter-policy interpretation is a separate prerequisite: source declarations
must be handled according to an independently established contract or refused.
Agreement with template bytes alone cannot establish an inactive/default/Union
write rule. This workstream does not claim to resolve those semantics.

Project status remains in [open work](../OPEN_WORK.md),
[implementation status](../IMPLEMENTATION_STATUS.md) and the
[limitations catalogue](../KNOWN_LIMITATIONS.md).
