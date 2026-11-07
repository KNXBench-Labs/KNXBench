# ADR 0158: Object priority keeps source provenance separate from wire priority

Date: 2026-11-06
Status: Accepted

## Context

WP-06 requires raw/effective provenance before adding object priority across the
project model. Existing [override representation](0010-per-attribute-override-representation.md)
and [product-database separation](0005-separate-product-database.md) apply.
The [source investigation](../research/2026-11-06-object-priority-provenance.md)
records the original schema-23 PDF pages and the exact implementation boundary.

A project priority token is not a protocol priority bit pattern. The schema's
three tokens cannot simply be cast into the four-value transmission enum.
Existing image-construction policies, including a documented Alert inference
and refusal of High, are independent of read-only literal provenance.

The current product query already chooses the reference's priority when
present, otherwise the object's. It loses which of those layers supplied the
result. Identical strings at both layers must not conceal a reference override.
At the initial product-query admission, no core field or project editor supplied
a complete priority contract; the bounded captured-source successor is below.

## Decision

Maintain four distinct concepts:

1. **Raw declarations.** Preserve source presence and literal text at its
   owning layer. Absent, empty, known and unknown text remain distinguishable;
   no trimming, case folding or unknown-to-default replacement is allowed.
   Raw means the XML reader's decoded attribute value, not original XML bytes.
   Original bytes and source identities retain their separate storage role.
2. **Selected product literal.** An existing `ComObjectView.priority` remains
   an uninterpreted product-query result. Optional `priority_layer` identifies
   Program or ProgramRef only when a literal is present, including empty text.
   Neither layer is asserted when the selected value is absent. The reference
   wins by presence, not by comparing its value with the program value.
   This query result is not the device instance's effective priority.
3. **Project-effective priority.** A future implementation must preserve the
   instance declaration separately from product defaults, then report both the
   winning source and interpretation status. Source ownership, unresolved
   product/module identity, missing values and malformed values must be explicit.
   Empty/invalid instance text must not silently inherit a product value.
   Schema-23 omission inheritance is supported by the inspected source; other
   schema mappings require their own evidence. These rules do not yet define
   an effective/editable contract; the captured-source field below stays separate.
4. **Wire priority.** Image generation and bus services retain their existing
   explicit admission, inference and refusal policies. A read-only literal,
   recognised token or matching source layer does not authorise encoding,
   programming, state credit or a new device operation.

The first bounded implementation adds product-query provenance only. It changes
no selected literal, database schema, project state, import mapping, enrichment,
image request or wire byte. It does not present a new UI priority column or
claim that the broader WP-06 priority requirement is complete.

**Additional bounded admission, 7 November 2026:** the
[effective object priority contract](../contracts/effective-object-priority.md)
adds a read-only `effective_priority` per device-detail object. A Known saved
declaration wins; Empty/Unknown declarations and every unavailable saved fact
stay unresolved and never inherit. Both omission kinds take the selected product
literal with its layer (Project Schema23 §1.2.5.11 and §1.2.5.13, printed page
48). Missing product data, program or literal stays unresolved with a named
reason. No editing, wire, image or table-column change is admitted.

**Additional bounded admission, 7 November 2026 (owner decision):** the
[priority-edit contract](../contracts/object-priority-edit.md) adds a separate
user layer (`priority_edit`, native 18, validated undoable command) that wins in
the effective priority and never rewrites the captured fact. Wire priority keeps
its policy: the image request now states each object's priority, and the
builder refuses a statement that is not exactly the product literal instead of
writing the product's priority (KL-188). No project token is encoded.

### Future implementation requirements

**Additional bounded admission, 6 November 2026:** the
[captured saved-instance contract](../contracts/saved-instance-priority.md)
admits a separate immutable source observation, not effective resolution or
editing. Its additive native17 overlay leaves old files/history explicitly
NotCaptured rather than guessing a backfill. Captured payloads are boxed and
resource-bounded; original source retention remains unchanged. This is an
implemented contract with focused migration/history proofs and independent
review. The [delivery record](../status/2026-11-06-saved-instance-priority.md)
separates those executions from pending integrated acceptance; it does not close
effective priority, editing or whole WP-06.

Before promoting project priority, inspect current opaque retention and define
an atomic migration that cannot invent missing original declarations. Old files,
working history, save/reopen and undo/redo must preserve exact presence and
literal provenance. Read-only open must not migrate. User edits, if admitted,
use validated domain commands and must not overwrite retained source facts.

Every consumer must distinguish a raw literal from an interpreted effective
value. Diff/comparison must compare declared presence/provenance as required,
not confuse equal text from different layers, and never equate raw XML tokens
with live-device observations. API/UI/reports must expose unknown/unresolved
states rather than imply a default or a successful device read. Module scope
must reuse admitted module identity, never caption matching.

## Alternatives considered

- Reuse `TransmissionPriority`: rejected; it conflates source tokens and wire
  encoding and could silently extend existing programming support.
- Store only an effective enum: rejected; it loses malformed/empty input and
  cannot explain a selected value's origin.
- Guess origin by comparing strings: rejected; a reference can explicitly
  repeat the program's literal, and empty is a present override.
- Change image generation together with provenance: rejected; no new wire
  policy is needed to fix a read-only metadata gap.
- Add the complete project property in this increment: deferred; migration,
  command, refresh, compatibility and consumer contracts need explicit tests.

## Consequences

The small product-query change is backward compatible in selected value and
catalogue storage but adds a required Rust struct field. Explicit constructors
must be adapted; no optional fallback may fabricate an origin. Production
import/query/native-reopen tests verify the new metadata and unchanged literal
selection. Current image policy tests remain applicable without changing their
expected values.

This ADR resolves the raw/effective design prerequisite only. Broader project
priority, compatibility evidence and WP-06 remain open. No hardware, release,
CI, deployment or external contact is authorised by this decision.
