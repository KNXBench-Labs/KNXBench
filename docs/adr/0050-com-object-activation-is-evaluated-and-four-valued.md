# ADR 0050: A communication object's evaluated activation is four-valued and separate from the stored `is_active`

Date: 2026-09-29
Status: Accepted
Session: ISSUE-08 data half, P2

## Context

ISSUE-08 reports that the device panel shows communication objects that the
device's parameters switch off, with no grouping by channel. Two sources can
answer "is this object active":

- **The project file's claim.** `ComObjectInstance::is_active` is read at
  import (IMPORT_EXPORT §9.3). A device KNXBench creates from the catalog
  stores `true` for every object, and nothing recomputes it when a
  parameter changes.
- **The product's `Dynamic` tree.** Evaluating it with the device's current
  values (`knx_productdb::dynamic::evaluate`) gives the activated
  `ComObjectRefRef`s. The parameter panel and the download planner already
  do this.

The evaluation is not always conclusive. A missing value, an unknown
parameter, a missing module definition or a budget limit can stop a branch
from being walked (`Diagnostic::may_hide_refs`). Two module instances with
one `RefId` cannot be given their own values (design D40). A stored value
that does not resolve (stale) leaves its parameter on the program default.

Measured on the three corpus projects after the P1 import fix: all 907, 867
and 75 objects evaluate as active and each is owned by exactly one channel
element (`apps/knx-server/tests/com_object_activation_corpus.rs`).

## Decision

1. `ComObjectNode` gets `activation: ComObjectActivation` and
   `channel: Option<ComObjectChannel>`, next to `is_active` and without
   replacing it. The projection alone sets `NotEvaluated`, and only the
   server with a product database evaluates.
2. There are four states:
   - `Active`: an activation names the object's lookup id. For a module
     object, the activation must also come from that object's own module
     instance (D39: `Module/@Id` ends with `_<RefId>`).
   - `Inactive`: no activation names it, and nothing made the evaluation
     uncertain (no stale value, no diagnostic that may hide refs, no scoped
     activation without an owning instance).
   - `Undetermined`: not activated, but the evaluation is uncertain, or
     the module instance cannot be told apart. It is never rounded to
     either side.
   - `NotEvaluated`: nothing was evaluated. There is no program reference,
     no product database, the program is not installed, or it has no
     `Dynamic` tree.
3. `device_detail` evaluates with the parameter panel's own
   `evaluate_device`, so the two views cannot disagree. The mapping is a
   pure module, `apps/knx-server/src/com_object_activation.rs`.
4. `channel` is the `Channel`/`ChannelIndependentBlock` the evaluator
   passed on the way to the activating ref (`ActiveRef::channel`). It
   carries:
   - an opaque `key`: the module node chain plus the node id, so that two
     expansions of one module definition are two channels;
   - the `kind`;
   - the element's `@Text` in the requested language, with module arguments
     substituted, or `None` when the element has no text;
   - `order`: the channel's position in the tree.
5. An unmatched `choose` (`NoBranchMatched`) is an expected product-data
   state, not a gap. Parameter diagnostics carry `severity`: it is `Info`
   for this diagnostic and `Warning` for all others. The evaluator, the
   download planner and their diagnostics stay unchanged.

## Alternatives considered

- **Overwrite `is_active` with the evaluation.** This was rejected. It
  loses the file's own statement (data integrity), and it would make an
  inconclusive evaluation look like a stored fact.
- **Two states, where anything not activated is inactive.** This was
  rejected. A missing module definition or a D40 ambiguity would then hide
  objects that the device actually uses.
- **Derive channels from object names or numbers.** This was rejected as a
  guess. The tree states ownership directly.
- **Evaluate in the frontend.** This was rejected. The product database is
  server-side, and the rule would be a second implementation next to the
  panel's.

## Consequences

- The UI can group objects by `channel.key`, sort them by `channel.order`,
  and show `Undetermined` distinctly. That is the ISSUE-08 UI half,
  goal-ui.md U12.
- Until then the fields are serialised on the wire but `#[ts(skip)]` in the
  generated bindings, because `apps/knx-web` is under the UI session's web
  lock. U12 removes the skips and regenerates the bindings.
- Most channel elements in the corpus have an empty `@Text`: 24 of 29
  across the three corpus projects' programs, including every one in the
  house exports. The UI
  needs a generic label for them. Their `@Name`/`@Number` attributes are not
  stored (KNOWN_LIMITATIONS §146).
- Each device-detail request now runs one evaluation. On the corpus this
  was not measured as a bottleneck, so it is not cached.
