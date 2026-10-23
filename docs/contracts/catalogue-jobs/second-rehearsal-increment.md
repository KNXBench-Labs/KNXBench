# WP-04.3 second apply rehearsal

Bounded implementation admitted 23 October 2026 under
[managed-binding §6](managed-binding.md#6-wsd-recipes). No new architecture or
public capability: apply must use its already validated D before W DML, rather
than discarding D's capability and comparing only after writing W.

## Required behaviour

Retain D's validated capability while B is copied/validated from unchanged S.
Explicitly end/close S with existing inspected primary/cleanup preservation.
Under the same original W IMMEDIATE reservation, reopen D with the accepted
owned RW lifecycle and invoke the shared controlled **noncommitting** installer
with the exact admitted source/name/limits. Compute complete PreviewIdentity
and compare the whole value to retained shown.preview before any W DML.

Every obtained D rehearsal receipt receives an explicit uncancellable close,
including installer failure, Stop, identity refusal and mismatch. Preserve both
primary and native close errors using the existing private typed failure payload;
a close failure alone does not invent uncertain W COMMIT. Existing baseline,
rollback/quarantine and inventory-based B disposition still decide W settlement.
Require a final checkpoint and unchanged W baseline before the live installer.
Keep the live result comparison and single publication gate; D never claims it.

## Allowed write surface

Only the local apply body in managed_catalogue.rs, one focused second_rehearsal
test module/registration, and if necessary one cfg(test)-only pre-close native
observation seam in the existing destination module. No lifecycle redesign,
codec/binding-version/schema/quiet API change, server or UI wiring.

## Finite acceptance

- Actual D installer writes precede all W writes; original W reservation stays
  held, B remains validated pre-install data, and matching apply commits once.
- Native D installer refusal prevents W DML, measured via native observations
  and W total_changes rather than final row count alone.
- Every complete preview-identity field and a same-summary raw-result change
  refuse before W writes; D executes and explicitly closes.
- Cancellation after actual D insertion prevents W work, closes D despite Stop
  and reaches inspected W rollback.
- Real outstanding native D statement makes close Busy; matching, mismatching
  and cancelled work retain exact primary/close facts and owner-held handle.
- D refusal plus denied W rollback remains NotCommitted/Unresolved/quarantined
  with allocated B retained. Existing prestate/source/policy/reservation and
  original recovery regressions remain unchanged.

Observe semantic original RED before each behaviour, retain exact source and
executed binaries, then corrected GREEN and bounded critical wrong variants.
Focused/owning tests overlap. Independent code review, maintained dependent
package/merged gates and publication are required before delivery.

Implementation is in progress; no new runtime test or source acceptance is
claimed by this contract. Growth/max_page_count, B10, production artifacts,
routes/lifecycle/UI and WP-19 stay separately open. No bus/device operation or
ETS compatibility expansion.
