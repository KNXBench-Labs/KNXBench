# Writing inactive parameters

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-070)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 70. Writing a declared-but-not-currently-shown parameter is now refused

**Limitation.** Before T18 slice 4, a `POST` naming a parameter's declared
`ets_id` was accepted even when that field was not currently active/shown
(old D24 check 2: "this check does not require the parameter to be
currently active"). This slice narrows that: a write is now accepted only
if the named id is the `ets_id` of an unscoped field the just-assembled
panel currently shows, or the `write_ets_id` of an editable module-scoped
field in that same panel (design D43). Naming a bare declared id that the
program has but the current panel does not currently show is now rejected
with `"is declared by this program but not currently active"`
(`apps/knx-server/src/domain.rs:2578`).

**Cause.** D43 replaces the old two-check validation with one rule: "the
panel is the single authority on what is writable." Accepting a bare
declared id unconditionally would mean guessing whether that id names a
top-level field or a module-scoped one — `knx-productdb`'s `parameter_ref`
table carries no `module_def_id` column
(`crates/knx-productdb/src/migration.rs:201-210`) to answer that question
without inference, and inferring scope is exactly what this slice's own
constraint forbids (D38's rationale, applied here to the read side of the
same question).

**Impact.** Narrow: a client that wrote to a hidden-but-declared top-level
field (one sitting behind a currently-unmatched `choose` branch) before
this slice can no longer do so directly — it must wait until that field is
shown, i.e. until the `choose` that gates it resolves to a matching
branch. No corpus-observed workflow depends on writing a hidden field
sight-unseen; `Access` itself has no attested write-gating correlation
either ([§3](../KNOWN_LIMITATIONS.md#3-device-parameters-are-preserved-but-not-interpreted)).

**Task 12 (2026-09-14): unaffected.** The task added a
`module_def_argument` table, not a scope marker on `parameter_ref`. A
declared parameter id still carries no scope of its own, so the panel
remains the single authority on what is writable, exactly as D43 has it.

**Lifted when.** Would need `parameter_ref` (or a sibling table) to carry
a `module_def_id` or equivalent scope marker, so the server could resolve
a bare id's scope without first evaluating the tree it belongs to. Not
scheduled.

**Alpha decision (user, 2026-10-04).** Accepted as an Alpha boundary.
<!-- END retained catalogue record -->
