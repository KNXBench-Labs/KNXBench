# PRG-01-N5: evidence inventory for the remaining parameter-policy gaps

Date: 21 November 2026. Baseline: `origin/main` `95e4284d7`. Software and
document inspection only; no bus, device, ETS VM or manufacturer contact, and
KNX Secure is excluded. Decision: [ADR-0183](../adr/0183-prg01-remaining-parameter-policies-need-evidence.md);
binding scope: [contract](../contracts/prg01-remaining-parameter-policy-scope.md).

## Question

Which entry of the PRG-01 Open list can be implemented now with simulator or
loopback proof alone, without inventing ETS semantics?

## Findings (verified in source on the baseline)

| Candidate | What the repository already does | Independent evidence for more | Verdict |
| --- | --- | --- | --- |
| Default-token semantics of `DownloadInvisibleParameters` | Only `Background` is acted on (ADR-0161/0171); any other present value writes no invisible parameter, and System2 refuses any present value (`image.rs`, "requires a separate source interpretation"). | The inactive-byte contract states `None`, `DefaultValue` and an absent option have no observed ETS algorithm. `Background` was *observed* on an ETS 6.4 download (ADR-0161). No equivalent observation exists for the other tokens. | **Blocked: needs observation.** A simulator can only confirm what we assume. |
| Customer-adjustable markers in `Dynamic`, `Extension` or wrappers | Matcher covers root/Union/module parameters and own root Options only; excluded contexts are named limits (contract). | The three public KNX articles describe intent only; no XML grammar says which elements carry the attribute. | **Blocked: no grammar evidence.** Widening is a guess about where a spelling is legal. |
| Editable `Union` cases | Existing winner/conflict rules stand; an active alternative is written at the shared address; direct Union markers are refused. | Manufacturer Tool §2.2 describes shared storage, not a customer-edit rule. | **Blocked: no rule to implement.** |
| Physical inactive-byte survival (`regular-070n-exclusive-inactive-storage-survival`) | Transfer omission only. | Management Procedures/Resources specify no retention promise. | **Needs hardware proof; not claimed.** |
| Other object layouts for run-state | Exact masks `0700h`/`0701h`/`0705h` (plus `0702h` queued) with strict plan/type identity. | No further layout is specified in the reviewed sources. | Not attempted: no new source. |
| Bounded read-only whole-mutation-effects observation | Post-download check reads memory/state after a download. | A baseline *before* mutation is what a whole-effects claim needs; the backup covers plan regions only. | Larger than one increment; needs its own contract. |

## What evidence would unblock each item

1. **Default-token semantics.** A recorded ETS download (private evidence) of
   one program per token (`None`, `DefaultValue`, absent) with a visible and an
   invisible parameter, showing which octets ETS writes. Without it KNXBench
   keeps writing nothing for invisible parameters.
2. **Other customer-adjustable contexts.** A primary-source schema statement
   (or a catalogue census, in private evidence) naming the elements that carry
   the attribute.
3. **Union editing.** A normative statement of precedence between Union members
   on download, or an ETS observation of one.
4. **Storage survival.** A hardware run on dedicated test hardware with explicit
   operation-specific authorisation (WP-12).

## Non-claims

No behaviour changed; no hardware support, ETS-algorithm equivalence or
"certified" status follows. The conservative refusals and fallbacks above stay
the policy.
