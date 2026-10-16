# ADR-0133: Parameter edits reconcile the configured object inventory

Status: accepted for offline project editing.
Date: 2026-10-16.

## Context

Reloading device detail can reevaluate only objects already in the project.
Imported projects may retain only the communication-object instances used by
that configuration. Enabling a previously unused button or selecting another
operating mode can therefore activate a product reference with no project
instance. Refreshing the browser cannot create that missing domain entity.

Product files also carry parameter-block structure and parameters controlling
object branches. Flattening those structures into one device-level list makes
otherwise supported operating modes difficult to find.

## Decision

Reuse the existing product evaluator and enrichment mapper. After validating a
parameter write, prepare its prospective project on a copy. Append missing,
positively activated, unscoped product references with new project IDs. Commit
ID reservations, the parameter and the appended objects as one ordinary Core
command batch through the existing durable-before-live path. Recheck the source
project and revision binding at publication. Never replace existing instances,
links, overrides or inactive objects. Resolve existing raw references against
actual program declarations before deciding whether an object is missing: a
literal catalogue ID and a project-local expansion are different candidates.
If both exist, report uncertainty on reads and refuse the entire write rather
than guessing identity. This is bounded application policy, not a schema rule.
Core owns reversible append/remove
commands, not product parsing or evaluation.

Keep optional parameter presentation metadata in the product-data adapter:
nearest declared block and structural `choose` dependencies, including controls
that gate other object-controlling parameters. Use declared IDs, not translated
names or manufacturer-specific identifiers. Multiple possible block owners have
no claimed single group. Recover older catalogues' optional block-caption
references from bounded, hash-checked retained source, never the audit `extra`
string. Missing captions use a numbered UI label; all parameters remain present.

The web client displays the same revision-bound parameter editor beside the
object table. Enum selections commit immediately; text/numeric edits retain
blur/Enter behavior. Folding retains mounted drafts and does not mutate projects.
No schema, dependency, bus protocol, device-admission or download change.

## Consequences and boundaries

This mechanism is manufacturer-independent, not a promise that all manufacturer
scripts or formats are evaluated. New module-scoped instances require an exact
instance/number mapping that this change does not invent: if a positively active
module reference is missing, the write is refused without changing the project.
Existing module objects and supported module parameter edits remain supported.
Unknown/skipped branches retain the existing evaluator diagnostics; no hidden
branch is manufactured. New objects use the catalogue mapper's existing DPT
ambiguity policy, not an inferred DPT choice. Its pure declaration resolver is
also used to reconstruct warnings on panel reads for configured objects without
a resolved DPT; an immediate follow-up GET must not erase a write's warning.
Warnings are current derived metadata, not a new persisted import report.

The native save/load and undo contracts are tested on synthetic data; an
owner-authorized offline project copy additionally reproduced a previously
unused button's object growth. Neither result establishes physical programming
or ETS reimport compatibility. See the
[editing contract](../contracts/parameter-object-refresh.md).
