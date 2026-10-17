# Object-local manufacturer function dropdowns

Date: 17 October 2026. Agent: codex.

## Scope

Owner requested removal of the separate Device functions disclosure and function
selection in the expanded communication-object editor, below DPT as marked in
the supplied screenshot. Existing manufacturer parameters are reused, not free
FunctionText edits or building-function assignments.

## Implementation

- Extend the existing ProductDB structural layout with deduplicated exact
  object-reference sets and propagate them through its parameter dependencies.
- Reuse the server's declaration/alias identity resolver to map unscoped
  references to configured IDs. Ambiguous and module-local identities receive
  no inferred mapping. The new response metadata is additive, not persisted.
- Reuse ParameterFieldRow and its snapshot/expected-value write contract inside
  the communication-object mutation boundary. Render the choices under DPT;
  remove only the device-wide disclosure. Parameters remains the complete editor,
  including initial activation when a device currently has no objects.
- Keep immediate save, errors, conflicts, native persistence and Undo/Redo at
  their existing application/domain owners. No new bus/download behavior.

## Verification record

Observed named pre-fix failures: device-wide disclosure still present; structural
left-object target set empty; HTTP controlledComObjectIds absent. Separate
in-session self-review found missing object mutation admission and a premature
empty-state message; two named RED assertions preceded those corrections.
Initial focused UI188, ProductDB3 and HTTP47 passed before final review changes.
Final focused row/workspace/parameter UI73 passed; documentation checker reports
624 Markdown files, 5310 local targets, 59 images, 33 chapters and zero errors.
Final owning Rust1,916/0/82regularignored in158reports and all-target Clippy pass.
Full web63stages pass: components3,638 and browser911; additional four focused
production cases pass. Inputs match their initial fingerprint. The outer runner
incorrectly catches its successful SystemExit(0); the original receipt is kept,
and a separate reconciler checks every exit, complete stage order, frozen inputs,
nonempty raw reports and five negative controls. This is not a new runtime run.
DE narrow screenshot shows the dropdown beneath DPT without horizontal overflow;
wide original capture stops above that control, so a scrolled follow-up capture
is still needed before full visual approval. Integrated acceptance/publication
remains pending at this checkpoint.

## Boundaries

No dependency, persistence schema, Core or protocol changes. No module-instance
mapping, manufacturer-specific heuristic or universal manufacturer compatibility.
Older servers with no object mapping show an explicit notice. No original vendor
or customer data, physical bus, live-project edits, release or deployment.
Parallel collapsed-default work may change ParameterPanel grouping; preserve it
at integration. User-selected dirty root and unrelated research remain protected.
