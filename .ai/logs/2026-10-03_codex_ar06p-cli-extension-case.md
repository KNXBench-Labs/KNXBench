# AR06P KL-149 — case-independent CLI package admission

## Scope and verified source mechanism

The only production change is the existing dispatch in
`apps/knx-cli/src/main.rs`: ASCII case-independent, exact extension comparison
for `knxprod` and the pre-existing `vd2` arm. The pre-destination legacy filename
refusal still runs first; the `vd2` arm remains unreachable for legacy inputs.
No legacy parser, format admission, schema, dependency, UI, vendor logic or bus
operation was added. `.KNXPROJ` remains on the project-import branch.

## Evidence already executed

- Original unchanged producer compiled; named uppercase synthetic CLI test
  failed with Rust101 / 0 passed / 1 failed / 0 ignored because the standalone
  package was misrouted to the project importer. Baseline debug binary retained
  with its verified digest for the later unpinned measurement.
- Smallest producer correction passed the same original regression.
- Expanded public seven-command retry accepted: five compiled registrations,
  focused5/0/0, ordinary CLI154/0/20 ignored across14 blocks, strict Clippy,
  formatting and whitespace checks. All700 code/config input hashes exact.
- The first expanded attempt is retained as rejected. Its project control
  incorrectly expected one manufacturer file; the fixture has XML plus retained
  baggage, and the existing CLI correctly counts two. Only that test premise and
  comment were corrected after tracing the actual importer.
- Separate full in-session producer/test review found no blocking code finding.
  This is not an independent-model review.
- Two separate full700-input source snapshots each changed only `main.rs`.
  Case-sensitive reversion and overbroad-any-extension routing compiled and
  exposed their registered targets, then produced four exact named Rust101 /
  0-1-0 behavioral failures. Eight mutation commands accepted independently;
  canonical source never mutated, and both snapshots' other inputs stay exact.
- The first public broad attempt retained four successful prerequisites and an
  actual Chromium exit1: another process listens on127.0.0.1:4173. This was a
  fixture-server port conflict, not a code regression. No foreign process was
  stopped. The alternate-port retry proc_1903cbc819d4 was also rejected:
  52 fixtures passed and20 failed because the existing theme fixtures correctly
  abort origins other than127.0.0.1:4173. Those guards were not changed. A
  user/network namespace keeps the original origin and no-proxy Vite settings,
  with only an isolated output directory/config wrapper in owned scratch.
- Fresh public16 proc_16290e8385d9 independently accepted: Rust3000/0/165
  across151 blocks, compiled ignored inventory165, Web1702 and Chromium72,
  17 semantically equal shadow bindings,700 exact source/config inputs, current
  root-explicit nonempty audits (corpus scan341). All16 raw-log hashes verified.
  Both earlier broad rejections remain intact. Candidate acceptance is scoped
  to f15f7cd2-based source, not the later U18 upstream integration.
- Private unpinned853 original-filename before/after measurement dispatched as
  proc_497717610b71 in a network-isolated namespace. Exact preserved baseline
  debug binary and candidate; descriptor-relative nofollow reads, same original
  basenames in owned readonly copies, isolated fresh databases, retained blob
  checks and three-table atomic-refusal checks. Raw output is never saved;
  only aggregate results can be published. Public synthetic helper checks8
  classifications/4 invalid-output-status refusals/5 confinement refusals pass.
  This first debug attempt was later deliberately interrupted after56 completed
  input pairs to move both measurement sides to the faster release profile.
  Exact owned Python received SIGINT, cleanup removed all private temporary
  inputs/databases, and no final accepted result exists. The rejected
  KeyboardInterrupt receipt/coarse log remain; this is not a code regression
  or full853 evidence. Initial cleanup and notice-verifier premises (a metadata
  file mistaken for a directory; one extra blank line) were corrected without
  deleting records. Full authoritative committed handover suffix stays exact.
- Fresh release-pair verification proc_f27d1c77cbba independently accepted9
  commands: baseline producer is byte-exact committed f15f7cd2 `main.rs`, the
  other699 snapshot inputs equal the frozen candidate, both sides compile with
  release profile and exact five-test inventories. Two baseline named CLI REDs
  fail101/0-1-0; candidate five tests pass5/0/0. Both actual release binary
  hashes are bound in the receipt. No mixed debug/release comparison.
- Full offline original-filename853 release measurement dispatched as
  proc_1fcfbf5cdfa3. Raw discard, held nofollow descriptors, isolated catalogues,
  byte checks, refusal checks and complete input before/after verification are
  unchanged. Its final result and independent readonly reconciliation are
  accepted:853/853 pairs,644→687 installed, Hager/Berker2→45,43 newly
  admitted. Two uppercase ZIP-limit and one evidence-item refusal stay explicit;
  all other outcomes unchanged. Original manifest and every original digest/
  size rechecked; both release binaries and source700 exact, retained successful
  archive blobs checked, three refusal tables empty. No raw private or item
  records and no private temporary directories remain. This is unpinned private
  measurement, not CI, full parser semantics or compatibility certification.

## Remaining acceptance

Reviewed candidate1cebc275 committed the nine owned source/test/document/receipt
paths with author and committer github@knxbench.com and no co-author trailer.
Normal upstream merge dd5a350c incorporates U18 e7f9db8e. Only handover
conflicted; resolution retains the entire own prefix and byte-exact entire
upstream suffix. Expected-content and truncated-suffix negative controls pass.
All704 current code/config inputs equal committed blobs;285 CLI Rust/build/test
inputs equal the measured candidate. The private853 receipt remains evidence
about its own producer/run, with that source-identity reconciliation explicit.
Fresh actual16 proc_7c8c83880554 ran with a new target and isolated
original-origin browser namespace. NEW actual16 independently accepted at
2026-10-03 18:32 CEST: Rust3000/0/165/151 blocks, Web1702, Chromium82 inventory/pass,
17 bindings/704 committed-exact inputs, all log hashes and actual-root nonempty
audits checked. All285 CLI Rust/build/test inputs match measured producer;
private853 receipt retains its own run identity. Candidate72 was not reused
as integrated82 acceptance. Root/foreign/retro listener and
private originals remain untouched.

Delivery362fec24 published/live/fetched read back: refs0/0, source704/ten
owned artifacts byte-exact, full owner suffix retained. Acceptance doc5 passed.
16 completed own scratch build/snapshot/shadow/XDG directories removed after
process-reference checks; all existing public logs/plans/aggregates preserved.
The first staged-doc verifier expected an unsorted path list and was rejected;
corrected exact sorted seven-path check passes, without changing source or gates.
Closing metadata gates/publication/readback and final actual-target/checkout/
branch cleanup remain. Broader AR06P/AR07/Alpha remain open; KL150 next.

Fresh fetched owner e7f9db8e records U18 completion and release of its own UI
reservation. Candidate f15f7cd2 proof kept its own identity; NEW dd5a350c
actual16 evidence above proves integrated U18 acceptance for this CLI scope.
Broader AR06P, AR07 and Alpha remain open.
The crawled files are local, unpinned private evidence, not committed fixtures or
CI gates. Native, ETS, hardware and statistics-owner decisions remain separate.
