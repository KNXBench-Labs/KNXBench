# U16 — acknowledged settings and lossless theme files

## Scope and inspected state

Owned checkout/branch: `ui-theme-storage`, based on verified U15 receipt
5257b05f; reservation fe02deeb is published with exact handover/ref readback.
U15 worktree/branch/scratch were actually removed. Root and foreign trees stay
untouched. This goal explicitly owns narrowly required settings API changes;
no domain, project, product or settings-file schema changes are planned.

Read `settingsStore.ts`, `settings_routes.rs`, `settings.rs`, existing tests,
U16, ADR-0060 and THEME_PACKS. The existing queue serializes ordinary optimistic
patches and refresh generations, but ignores PUT bodies and has no conditional
multi-key acknowledgment. The server locks reads/writes and preserves unknown
preferences, but `PatchRequest` currently ignores `expectedSettings`. Reusing
this without implementation would falsely claim stale-write protection.

## Contract before implementation

- File import checks declared and actual bytes and uses fatal UTF-8 decoding;
  one leading UTF-8 BOM is accepted as an encoding marker, not pack content.
  JSON admission remains duplicate-aware U15 parsing. Export is recursively
  key-sorted two-space JSON plus newline, validating only the supported pack.
  Neither decoder nor export changes settings or original input.
- Conditional PUT compares supplied key values under the existing settings
  write lock, before applying/writing. Null means the key must be absent.
  Older nonconditional callers retain their existing patch semantics. Conditional
  writes must also refuse an immediately quarantined record. No multiprocess
  filesystem CAS is claimed.
- Add a small `conditionalPatchVersion: 1` response capability. File schema 1
  alone cannot distinguish an older server that silently ignores the new
  request field; new theme mutations must fail closed without the capability.
- Only the existing settings client owns the last authoritative snapshot and
  serialization queue. Deferred writes are not optimistic theme installation.
  Relevant concurrent updates conflict; unrelated local edits are retained.
  A rejected PUT leaves the last acknowledgment intact; transport/malformed-ack
  ambiguity triggers authoritative reread, never blind replay. Cache failure
  after server acknowledgment is a cache diagnostic, not durable rollback.
- Installation preserves raw foreign invalid entries whenever the map itself
  is bounded/recoverable. Corrupt non-map storage is not implicitly replaced.
  Replacement consent is tied to inspected content, not only ID/version.
  Removing the active valid pack selects System in the same conditional patch.
- All tests use synthetic files, mocked fetch or in-process HTTP with isolated
  fixture data and no product DB. No productive backend, discovery, gateway,
  tunnel, bus or hardware write; no independent/native/Orca/release claim.

## Encoding evidence

Retrieved WHATWG Encoding Standard on 2026-10-02, sections 7.1 and 7.2:
https://encoding.spec.whatwg.org/#interface-textdecoder
`fatal` selects error instead of replacement; default `ignoreBOM: false`
consumes an initial BOM. UTF-8 is explicit, with no legacy auto-detection.
This fact informs product policy; byte limits/BOM admission remain our contract.

## Acceptance status

Latest candidate4 (proc_f1cfb957471d) independently reconciled at controller:
16/16 commands passed, Web1665, Chromium61, ordinary Rust2925/0/164 over147 blocks,
eight conditional HTTP,17 bindings equal and621 frozen inputs unchanged. Complete
source/contract in-session pass plus added-line security scan has no open blocking
U16 finding. Delegation prohibition honored; no independent model review claimed.
Exact20 owned paths inventoried. Integration with current upstream, private offline
selection/matrix and publication/readback remain PENDING.


### Follow-up review disposition — U16-02 / U16-03

U16-02 CLOSED at focused boundary: new synchronous held-lease/real-worker case
fails with a named lock-bypass assertion when the writer lock is removed. That
compiled mutant failed the same named assertion in 20 additional executions;
original route restored byte-exactly and all eight conditional HTTP cases pass.
The original seven-test survivor remains in evidence. Six other compiled server
controls were killed separately; this is not a multiprocess filesystem-CAS proof.

U16-03 IMPORTANT, CLOSED at focused boundary: ordinary queue replies could leave
an earlier conditional capability snapshot valid and clear an unconfirmed intent.
RED tests observed stale Graphite after capability loss, and local Porcelain
reverting after an unrelated density acknowledgment. Legacy-server compatibility
and retired-owner tests also went RED before their minimal corrections. New
observations/confirmation/epoch handling preserves ordinary older-server sync
while conditional writes fail closed. Five behavioral guard-removal controls
are killed, sources restored, and five focused suites pass 144 with TypeScript.
ANSI framing hid one captured failed-test count; its existing receipt was
reclassified rather than replaying the completed mutation.

Previous 16-command candidate acceptance belongs to pre-review source only.
Current source requires a new full frozen gate and upstream integration.
No independent agent/model review is claimed: user forbids delegation, so this
is the distinct in-session source/contract review pass. Integrated acceptance,
publication/readback and U17/U18 remain PENDING.


### U16-02 — IMPORTANT: conditional writer-lock coverage

All 16 candidate commands are now verified green on the frozen candidate (the
invalid binding-count assertion was corrected with complete 17-binding inventory,
without replaying the ten successful commands). Compiled server controls kill six
mutants; removing only the writer mutex in `settings_routes.rs:244` survives all
seven conditional HTTP tests. `http_settings_conditional.rs:175` uses `tokio::join!`
with a synchronous handler, so it does not demonstrate overlapping critical
sections. Production still takes the lock; this is a blocking verification gap,
not proof of a production race. Add a real worker/held-file-lease test, require a
named behavioral failure with the lock removed, restore exactly, and rerun GREEN
before integrated acceptance. One mutation restoration used an ambiguous `false`
anchor; source was recovered byte-exactly and all common leases released.


Candidate attempt 3: all ten executed commands are green (2,924 ordinary Rust
passes / zero failed / 164 ignored / 147 blocks; Web 1,661; Chromium 61; seven
conditional HTTP cases). Runner failed only an unsupported `compared > 50`
assumption. The complete tracked/generated binding inventory is 17, all equal
under the exact CI whitespace policy, with 621 protected inputs unchanged.
Keep the failed parent receipt; do not replay successful commands merely to fix
a receipt-policy assertion. Six still-missing checks dispatched as continuation
proc_4d540a5bdb9e / PID 1093386, with parent log commitments and both source freezes.
Full candidate acceptance, integrated gates and publication remain **PENDING**.


Progress-fixture isolation: exact RED failed on unwanted default ProductDB admission;
explicit state/FakeConnector GREEN passed 100 repetitions of all nine cases (900),
with no default database created. Three import status assertions now include synthetic
error-body diagnostics without changing their success expectations. An accidental
fuzzy-patch rewrite of the native-open kind was restored and full-file expected-delta
verified; its rejected probe is not evidence of the original two HTTP 500 failures.
A renewed old-helper probe passed 800 cases; those transient causes remain unproven.
Production import/product code is unchanged. Third fresh 16-step candidate run is
**PENDING**, proc_5b5cb2af7826 / PID 961371. U16 is not delivered.


Renewed candidate attempt exited at workspace tests: 2,921 passed / two failed /
164 ignored / 147 result blocks; its 621 source fingerprints remain unchanged.
Both failures are legacy HTTP load-progress imports returning 500. Exact unchanged
compiled-target checks each pass eight cases in fresh parallel, fresh serial and
original-gate XDG environments. Root cause of the transient 500 is not yet proven.
The fixture's unrelated default ProductDB opener is directly observed in source;
a new behavioral fixture-isolation regression is queued as RED before any fix.
Dispatched 2026-10-02 23:05 CEST: proc_7c0a7d36b8ac / PID 905149, EXPECTED_RED_CONFIRMED, bounded exact test only,
both leases and own XDG data; completion and next full gates remain **PENDING**.


First full candidate attempt ended with fmt passing and exactly one full-Web
failure (1,660 passing / one failure of 1,661): the companion's pinned transitive
module inventory did not include the new pure canonicalJson dependency. Reviewed
the helper's complete no-import/no-IO body and settings import edge; added only
that exact module to the inventory with its capability rationale. No project/API
allowlist was relaxed. Focused DiagnosticsCompanion and TypeScript now pass.
The first receipt remains archived and is not credited as full acceptance.
Renewed runner prepared 2026-10-02 22:38 CEST with distinct candidate-gate-2/target-2 artifacts;
renewed completion remains **PENDING**.


Candidate gate dispatched 2026-10-02 22:34 CEST: `proc_3416e3a98ac2` / PID 795535, notify-on-complete.
Sixteen explicit steps, both common leases, fresh target, shadow bindings and
source freeze; current completion is **PENDING**, not acceptance. Read exact
summary/logs/result blocks and verify source identity before crediting the run.
No source edit while running. Compiled HTTP controls/integration/publication
remain separate subsequent boundaries.


Implementation, observed RED/GREEN and separate in-session review are complete
at candidate scope. IMPORTANT U16-01 (aggregate runtime refusal versus accepted
selection) was fixed by sharing readThemePackStore admission. Five-file focused
suite: 140 passed; TypeScript passes. Thirty-one unit controls are behaviorally
killed and byte-exactly restored; export re-admission and response status initially
survived and now fail after strengthened tests. An ambiguous short restore anchor
failed in the first runner; it was repaired, source restoration checked, and the
runner now uses exact bounded contexts. No mutated source remains. New-file
TS2322 canary caught and restored. Cold-restart roundtrip is synthetic file →
mocked acknowledged JSON persistence → fresh client hydration → export/reimport,
not a live server/native acceptance claim. Existing HTTP cases separately cover
actual isolated disk restart. HTTP mutation controls, complete candidate/integrated
gates and publication are **PENDING**. U17 and U18 remain open.

Recovery is an inert labelled browser-observed JSON scope, not an importable pack
or byte-exact server-file backup; oversized/nonserializable output refuses without
truncation. Stored null/absence and single-server-vs-multiprocess limits are
recorded in THEME_PACKS and KNOWN_LIMITATIONS.

Do not preserve secrets, tokens, credentials, passwords, API keys, or connection
strings; use `[REDACTED]`.
