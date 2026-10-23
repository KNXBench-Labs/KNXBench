# WP-04.3 fixed managed-writer SQL safety

Bounded implementation of the five fixed W settings in
[managed-binding §3](managed-binding.md#3-persistent-w-observations-and-ordinary-external-aba).
This is not a complete managed configuration receipt or B10 completion.

## Admission

Controlled preview and apply observe the actual W before guard/BEGIN, then
independently again as the first SQL work inside the original IMMEDIATE
reservation. Required native integer values are foreign_keys=1,
ignore_check_constraints=0, writable_schema=0, defer_foreign_keys=0 and
query_only=0. Only fixed trusted pragma identifiers are queried. Unsupported,
unreadable, missing or wrongly typed values refuse through private typed errors;
no setter, migration, default or silent repair obtains admission.

Initial refusal leaves a healthy owner healthy and autocommit, without generation
advance, copy allocation, growth-cap setter or live DML. Reserved refusal uses
the existing uncancellable inspected rollback. A denied rollback preserves the
configuration error separately from cleanup errors, quarantines W and cannot
invent an unallocated B. Existing preview NORMAL synchronous policy is unchanged.

## Native observation ordering

The pinned native runtime demonstrates that reading max_page_count can clear
defer_foreign_keys, while the five fixed getters and synchronous getter preserve
it. Capture cap diagnostics before injecting an unsafe flag, inspect flags
immediately after admission, and only then run cap/stamp diagnostics. Production
checks must not erase the state they are meant to refuse. Native authorizer
controls distinguish first reads from reserved second reads, with actual
native autocommit observations rather than transaction-name assumptions.

This is a pinned-runtime observation, not a universal SQLite-version promise.
Primary source provenance, exact native probe and execution receipts are retained
privately with the package evidence.

## Finite acceptance and limits

Twenty-three focused methods include 60 refusal scenario iterations: ten invalid
value cases and fifty native denied/no-row/reserved/rollback combinations. These
counts overlap owning tests and are not additional unique tests. Positive actual
preview/apply and unchanged preview synchronous policy remain covered. Eleven
compiled wrong variants discriminate initial/reserved omissions, each skipped
setting, wrong-value acceptance and repair; a parameterized mutant is credited
only for the failing row actually reached.

Original runtime allowed four unsupported initial configurations; query_only
already refused natively at attempted BEGIN but quarantined the owner. Neither
flag differences nor original success establish catalogue corruption or that
a deferred flag remained enabled throughout later DML. Wrong native scalar types
are rejected by strict extraction, not claimed as an injected native witness.

Independent source/evidence review accepts this slice without blockers. Maintained
branch/integrated gates and publication remain delivery requirements. Full B08,
variable-setting binding/configuration receipts, sticky unwind-safe commit witness
and composed same-gate B10, durable artifacts/sidecars, routes/lifecycle/UI,
real large-database and aggregate quotas remain open. No quiet API, schema, codec,
public capability, bus/device operation or ETS compatibility expansion.
