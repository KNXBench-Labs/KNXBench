# DPT document audit — local integration and self-review

## Scope and delivery boundary

User-authorized local DPT-AS audit and closure of confirmed defects. No
commit, push, deployment, container restart, release, subagents or live bus
operations. Source identity and normative findings are in
`docs/spec-audits/2026-10-07-dpt-document-audit.md` and its three CSV matrices.

## Executed evidence before the integrated gate

- Source: authorized original, SHA-256 `72f924a6c793314d1e20573d71f697f07ce76072525b80c53ee0f68e3b7190af`;
  251 pages extracted, 250 printed footers matched, 280 contents entries,
  454 numbered IDs across 103 main numbers. Official attachment independently
  retrieved with the same hash. No licensed page text/images committed.
- Numeric RED: three intended failures (V16 percentage sentinel, F16 bounds,
  scaled V32 bounds); one valid-input control passed. Runtime RED: two HTTP
  assertions saw 200 rather than 400; two CLI dry-run cases accepted a
  restricted explicit/project-resolved type.
- Inventory oracle correction: labelled Scene Display is not decimal write
  input. No scene wire behaviour changed for that test correction.
- Isolated core suite: 720 passed, 0 failed, 0 ignored in seven result blocks.
  Focused caller targets before final matrix strengthening: HTTP bus write
  18/18, CLI bus DPT 12/12. Explicit and legacy project-resolved cases were
  then both pinned; the final integrated gate must certify that newer tree.
- Local integration: 13 tracked owned deltas plus six new artifacts. No
  HEAD/index changes. Foreign-only files matched their before bytes; applying
  the inverse owned delta in scratch reproduced every shared tracked before
  file. Shared status/research entries were inserted additively; another
  owner's staged commissioning CLI work and KL additions were preserved.
- Integration prechecks rejected a wrong Git strip level, then differing
  shared document contexts. Corrected before any integration: `-p0` plus
  additive status/research patches. These were harness refusals, not product
  failures. The initial whole-file equality verifier was also too strict for
  the preserved foreign KL additions; inverse-delta equality proved retention.
- `cargo fmt --all -- --check` and `git diff --check` passed on the integrated
  root. Citation identity/evidence verification passed. No minimum prose
  citation percentage or full semantic conformance claim is inferred from it.

## Separate self-review pass

| Requirement | Evidence reviewed | Verdict |
| --- | --- | --- |
| UI-independent policy | `knx-core/src/dpt/codec.rs::validate_group_write_dpt`, both re-exports | Pure core function, no new dependency or UI coupling. |
| Parameter codecs remain usable | runtime core regression exercises encode/decode for all nine subtypes | Runtime admission is separate from serialization. |
| Both public encoder APIs | numeric regression calls explicit and compatibility encoders | Quantized sentinel plus pre-quantization engineering bounds pinned. |
| Explicit and project-resolved runtime DPT | both CLI branches and the common HTTP resolved path | Same core policy called before send, no fallback to raw mode. |
| Input-format compatibility | explicit/legacy direct and project-resolved caller assertions | Neither grammar bypasses the policy. |
| No hidden scope expansion | 149 explicit unsupported-ID regressions; partial semantic CSV status | Structured codecs, FB semantics and full ETS conformance not claimed. |
| Source completeness vs semantics | page/contents/type matrices, 249.600 body definition | Whole-document inventory; not every sentence/footnote visually proven. |
| Unknown/reserved data | existing raw format-level variants/rulings retained | No guessing of enum names, masks or unsupported widths. |
| Existing scene behaviour | existing scene width tests; corrected comment/oracle | No Display-as-input or six-bit-inline support claim. |
| Foreign root work | guarded transfer, additive docs and inverse-delta comparison | No stash, index change, ref movement or foreign replacement. |

Cheap review corrections applied: scene Display comment, obsolete scene-control
inline explanation, unsupported-charset error documentation; project-resolved
caller tests now cover explicit and omitted input-format selection.

Final self-review severity: **Critical 0 / Important 0 / Minor 0** within the
owned package; **approve for local delivery**, conditional on the integrated
execution below. This is self-review, not an independent review, whole-release
acceptance or hardware proof. Retained semantic/FB/structured boundaries are
findings, not hidden claims of conformance.

## Integrated gate

**PASS — local delivery, no publication.** Final integrated gate ran on
`a033b3fc8222dd15e32e3ab20034b947e5726f6a` with 433 frozen source/config/fixture
inputs, UTC timestamps `2026-10-07T13:04:15.919649+00:00` through
`2026-10-07T13:08:48.949648+00:00`. Rust result: **3418 passed / 0 failed /
178 ignored, in 191 result blocks**. Changed `knx-core`, `knx-cli` and
`knx-server` were actually compiled. No totals from earlier overlapping runs
are added to this result.

| Executed command | Exit |
| --- | --- |
| `cargo test --offline --workspace --exclude knx-desktop --no-fail-fast -j 4` | 0 |
| `cargo clippy --offline --workspace --exclude knx-desktop --all-targets -j 4 -- -D warnings` | 0 |
| `cargo build --offline -p knx-cli -p knx-server -j 4` | 0 |
| `cargo run --offline -p xtask -- check-layering` | 0 |
| `cargo run --offline -p xtask -- check-headers` | 0 |
| `cargo run --offline -p xtask -- check-anchors` | 0 |
| `cargo run --offline -p xtask -- check-ledger` | 0 |
| `cargo run --offline -p xtask -- check-corpus-gates` | 0 |

The audits reported real scope: layering 449 resolved packages; headers 585
well-formed, 155 without headers at the existing ceiling, 42 generated skipped;
anchors 661 links in 312 Markdown files; ledger 191 rows; corpus guard 397 Rust
files. These are audit scopes, not test counts or corpus execution. Both
CLI/server debug binaries were built. The built CLI also refused the verified
`DPST-7-13` explicit-format dry run with the runtime-policy diagnostic and empty
stdout, without an external network interface.

The gate held both shared advisory leases, in the established order, and ran
with **loopback as the sole network interface**. Canonical inputs, HEAD and
staged index stayed byte/object-exact during this successful execution. No
Web/native UI acceptance, ignored/private-corpus sweep, real multicast or
hardware acceptance is inferred. Final formatting and whitespace checks pass.

### Retained unsuccessful attempts and verifier corrections

- First dispatch: refused before any test stage because a foreign Cargo corpus
  worker appeared; infrastructure refusal, not a detected defect. That session
  later delivered `a033b3fc`; its work and newest handovers were retained.
- Next attempt: workspace tests passed 3418/0/178, then Clippy exited 101 on
  two `inconsistent_digit_grouping` diagnostics in the new V32 bounds. Build
  and all five xtask commands were **not started**. Only literal separators
  were corrected; both f64 bit patterns were checked unchanged. The complete
  integrated chain above then passed on the corrected source.
- Extra built-binary probes initially used unsupported `--help`/dotted-DPT
  command grammars. Usage/malformed-reference refusal is not runtime-policy
  evidence. A separate dry run with the actual `DPST-7-13` grammar established
  that policy. No parser aliases or wire behaviour were changed to fit a probe.

### Outcome identity

Final receipt SHA-256: `ed6c8d0e809a53ff15e20acd4e4ef7f4d7a2661208c474940dc58fa2e16eff49`.
Executed stage-log digests:

| Stage | SHA-256 |
| --- | --- |
| `tests` | `5ffe9210dab8baa11bf8acbaa986bce89279e0169415b70ba42f9bd1577e58b7` |
| `clippy` | `1996f67a6722d4d0529a4ccdd630d62dea9b675ef1d7cd2a51f01f3a5f5325ec` |
| `build` | `8b57e6d48a0fd3ff15ab536a114fec1a8e6063944a5c0cf10ccc66bc93fba182` |
| `check-layering` | `a4857343425fc6d2dab453a4d3973822ac4d943b96a61d720c041c198d07bb7a` |
| `check-headers` | `17f9e64c2f3a4f7c553d1010afa8c69c9975b70912c8d7b9d58405272b7b70eb` |
| `check-anchors` | `0518039618a4be06c88fe624088505e6d11f4bdf5df5ca95ace264260f1d4534` |
| `check-ledger` | `8957141691a22765f1c0e51d9bf9c25fe19ea79ba5a01bf5664bb897ddb55e08` |
| `check-corpus-gates` | `46f4464548a179000ae2c49e965f3007abc058723d39bbabf04a9b88eb4ab3ba` |

The raw task-only runs/targets and private PDF extraction are removed after
verification; this durable receipt records the source, commands, individual
exits, totals, negative classifications and final scoped verdict. No independent
review or whole-KNX conformance claim replaces the self-review above.

## Final documentation and owned-artifact closure

Final documentation commands all exit 0:

| Command | Exit |
| --- | --- |
| `cargo fmt --all -- --check` | 0 |
| `git diff --check` | 0 |
| `python3 /home/knxbench/.hermes/profiles/knxbench/skills/research/grounded-citations/scripts/sources.py --ledger <owned citation ledger> verify ROOT/docs/spec-audits/2026-10-07-dpt-document-audit.md --evidence` | 0 |
| `cargo run --offline -p xtask -- check-anchors` | 0 |
| `cargo run --offline -p xtask -- check-ledger` | 0 |

The extra documentation helper initially refused before command execution on a
missing Python import; its two acquired leases were released and the corrected
helper executed these checks. This was not a product or citation failure.
The cleanup guard initially mistook an ignored local log for an absent root
artifact; `git check-ignore` and physical-file checks corrected that verifier,
without staging anything or removing data.

Root contains all 21 package artifacts (13 tracked deltas + six original new
artifacts + handover + this ignored log). Eight non-shared owned Rust files
match the isolated checkout exactly; the two CLI admission calls survive in
the current foreign CLI code. Twelve code/CSV files were hashed before cleanup;
canonical sorted path/digest-map SHA-256 is `47e79f9e86fd6db8433f4995906b7f832591083d52f2ee2067a44feebd256934`.
No staged paths, root ref movement, generated binding churn or discarded
foreign handover were observed at final verification. The log is gitignored:
include it with an explicit `git add -f` only if later publication is requested.
No ledger status/owner/route changes, extra codec support or new hardware
permission follow from this closure.

Cleanup completed: the owned `audit/dpt-spec-20261007` checkout/branch and
`dpt-audit-20261007` scratch namespace (targets, extraction, renders, raw logs)
were removed. All twelve root code/CSV digests remained identical. Foreign
`fix/kl-158` and `u21-fix` checkouts were left registered and untouched. Final
handover preserves every inherited byte and leaves publication unrequested.
All four DPT session tasks are completed, not the wider KNX/ETS feature scope.

## Publication follow-up — 2026-10-07 15:48 CEST

The user now explicitly requests commit and push of this package. The prior
local-only receipt remains historical, not a claim that publication happened
at the original gate time. Fetched main equals the accepted source base; all
twelve code/CSV hashes still match. The existing Rust/clippy/build receipt and
separate self-review carry over without new product edits. Outgoing added-line
security scan is clean. Only closure Markdown and this log's tracking status
change. Recheck staged whitespace and all five repository audits before
commit; actual remote verification remains pending. The separate untracked
`2026-10-07-dpt-scope-precheck.md` is excluded and preserved.

### Verified publication — 2026-10-07 15:56 CEST

Feature commit `eb0abc6a8e75c614a489af82ab9ac810b5ffb740` was pushed to
`origin/main` from the task-owned detached checkout. Fetched and live remote
main both matched that exact object. Author and committer are
`KNXBench-Labs <github@knxbench.com>`; no co-author trailer. The exact 21-path
commit excludes the separate scope-precheck file, whose bytes remain intact.
All five repository audits, fmt and staged whitespace checks passed before
commit. The staged check first caught an extra EOF blank line in this log;
only that metadata whitespace was corrected. Existing code/CSV hashes remain
identical to the accepted integrated Rust/clippy/build gate; no new full-suite
execution or expanded conformance claim is implied. This closure-only
bookkeeping is committed/pushed separately without product changes.
