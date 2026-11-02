# Selective System-2 settlement on an unchanged source

## Delivery (2 November 2026)

Feature `3f19ebe65` is published main-first as `0d6299187`. Local, tracking and
live remote references were read back equal. This is main-only source delivery,
not a release, deployment or physical-device acceptance.

A verified configured run now separates strict proof of the original product
source from comparison with the current project's request. A rename or an edit
elsewhere no longer loses every programming mark. A link or flag change leaves
Group addresses pending; a parameter change leaves Parameters and Group addresses
pending. A missing, changed or refused source and a changed device reference
still earn no new memory credit. Start admission, Net verification, recovery,
journal/bundle formats and status/UI schemas are unchanged.

The [result contract](../contracts/system2-download-result.md#settlement) owns the
precise dependency mask, identity checks and timing disclosure. This is **not**
per-part equivalence across changed product sources; that remains open under
[S2-05](../open-work/S2-05-result-and-settlement.md).

## Software acceptance

Independent whole-feature review initially blocked on an unguarded worker
refusal mapping. Test-only corrections closed that gap and two smaller guard
coverage gaps; independent re-review approved the final candidate. Production
code did not change during that correction.

- Parent repeated six App cases, fourteen server-settlement cases and three
  mounted-worker cases. The mounted cases finish an actual simulated verified
  run after a rename, parameter edit or refused product-reference switch.
- Executed negative controls reject the former all-or-nothing behavior, forced
  full credit, refusal mapped to full credit, omission of original-source
  admission, and omission of the device-source comparison. Forged source hashes
  and plans are refused while the request remains intact.
- Undo/redo, saved-project reopen and failed persistence retain their conservative
  programming-belief behavior. Mounted reopen is not claimed; it is covered at
  the server settlement seam.
- The opt-in large synthetic-project debug measurement is disclosed in the
  contract. It measures helper cost, not a release-build or real-project bound.

Both the feature and main-first merged candidates passed all **14 selected
package stages**, with frozen inputs and no touched or unchecked user data.
Each run passed **3,382 Rust tests**, with **143 ignored**, and **91/91 selected
corpus tests**. These are overlapping executions, not additive unique coverage.
The gates include formatting, documentation, browser-lane declarations, layering,
headers, anchors, ledger/corpus declarations, tool tests, desktop clean/build
prerequisites, Clippy, Rust tests, selected corpus and diff checks. No web/browser
suite or bindings stage was selected; the web bundle was built as a desktop
prerequisite.

## Retained limitations

- No real bus, gateway, device or VM was contacted. The mounted workers use the
  digest-pinned public package with simulated hardware-kind transport.
- The optional existing
  `pinned_public_http_worker_cancellation_reopens_original_once_only_recovery`
  case remained red in focused investigation and also failed with the original
  server files. It is not credited as passed and is separate from the selected
  91-case corpus gate. Its wait budget was not relaxed.
- Links, flags and changed-source outcomes are tested at App/settlement level,
  not each through a mounted public-package worker. The source-switch loop's
  path cell kills the omitted whole-source comparison; separate mutant kills
  for its later id/combined cells are not claimed.
- Field outcomes for #30/#22, the firmware flag's exact clear time, and source
  equivalence across changed product data remain open. No new device retry is
  implied or authorized.
