# PRG-01-N6: which mutation effects can a simulator observe without inventing semantics?

Date: 21 November 2026. Baseline: `origin/main` `95e4284d7`. Software and document
inspection only; no bus, device, ETS VM or manufacturer contact; KNX Secure is
excluded. Decision: [ADR-0184](../adr/0184-prg01-mutation-effects-observation-scope.md);
binding scope: [contract](../contracts/prg01-mutation-effects-observation-scope.md).

## Question

N5 found the parameter policies blocked on evidence. Is there a different axis
in PRG-01's Open list (read-only mutation-effects observation, another declared
download form, another identity/layout observation) that a simulator can verify
without encoding a guess?

## What a regular memory download already witnesses (verified in source/docs)

| Effect | Witness today | Source |
| --- | --- | --- |
| Data octets written | Written **and read back unchanged** inside the transfer, counted as `readBackOctets` | [download-failure-diagnostic](../contracts/download-failure-diagnostic.md) |
| Restart | `RestartOutcome` (sent/unconfirmed/`notInPlan`); never claimed as "sent" on unwitnessed ends | same |
| Reach, identity (mask, manufacturer, application, declared `CompareProperty`) | One read-only attempt after the transfer | [post-download-check](../contracts/post-download-check.md) |
| Load states (address/association/application/PEI) vs plan | Same attempt | same |
| Run state | Exact masks `0700h`/`0701h`/`0705h` (+`0702h` on its branch) property read | [run-state](../contracts/post-download-run-state.md) |

## Candidates examined

| Candidate | Finding | Verdict |
| --- | --- | --- |
| (a) Re-read the written memory ranges after restart and compare | `post_download.rs` states it deliberately reads no memory: after a restart the device may rewrite volatile octets, so a mismatch would mislead, and it costs a full re-read. No source in the repository says which written octets are stable across a restart. A simulator would only return what the simulator author chose. | **Blocked: needs a per-mask stability statement or observation.** |
| (a') Re-read only the load-control/property state beyond today's | Already covered by the load-state machines and run-state property. | Nothing new. |
| (b) Further declared download forms | Forms in ADR-0167/0168/0170/0171 are delivered; every remaining `ImageError::Unsupported` site in `knx-productdb` `image.rs` is a refusal whose lifting needs a statement of what ETS writes for that form, i.e. the same evidence class as N5. No refusal was found whose correct behaviour is derivable from a cited document alone. | **Blocked: no cited rule.** |
| (c) Other identity/object layouts | N4 (ADR-0182) pinned the unadmitted masks as refused; layouts beyond object 3 lack a type-and-plan binding source. | **Blocked: no layout source.** |
| Hardware survival of written state, installation function | Not simulator-observable. | **Needs hardware proof; not claimed.** |

## Conclusion

No simulator-verifiable increment exists on this axis without inventing
semantics. Delivering code would have required choosing which octets must
survive a restart, or what ETS writes for a refused form, and testing the choice
against a simulator built on that same choice. The named missing evidence is the
deliverable.

## Evidence that would unblock each item

1. **Post-restart memory comparison:** a primary-source statement or an observed
   (private, ETS-side or owner-authorised) before/after dump per mask naming the
   octet ranges that are stable across the plan's restart. Then a simulator RED
   whose device rewrites a *declared-volatile* range and must not mismatch.
2. **Further forms:** an ETS observation or Manufacturer Tool statement for the
   specific form, plus a RED failing on today's refusal.
3. **Layouts:** an original-source type/plan binding for the object, as for
   `0701h`/`0705h` ([research](2026-10-29-property-run-state-observation.md)).
