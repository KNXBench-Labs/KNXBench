# AP1 resolver bounds and parser disposition (WP-04.2)

## Decision

Retain the existing namespace-aware parser, resolution algorithm and limits.
The [accepted synthetic baseline](productdb-baseline-2026-10-20.md) does not
justify replacing them: the admitted 11398-byte and 8384512-byte sources each
produce the same five declarative controls, with median absolute target peaks
of 6120 and 22544 KiB. The 8392704-byte source explicitly reports unavailable
at the independent 8388608-byte resolver ceiling. These are historical
source-bound measurements, not newly executed WP-04.2 benchmarks.

Install/database growth is not evidence of a resolver allocation bottleneck.
Those measurements neither isolate parser allocations nor demonstrate that a
selective parser would improve them. A rewrite would also need to preserve
whole-source validation, refusal precedence, unknown subtrees, ordering and
source offsets. There is no measured benefit here to justify that extra risk.
This is the permitted no-optimisation disposition in the
[ordered goal](../goals/alpha16-successor-sequence-2026-10-19.md), not a claim
that future workloads can never justify optimisation.

This is implementation-policy evidence, not a KNX format limit or broader
execution/ETS compatibility statement. [ADR-0098](../adr/0098-offline-ap1-procedure-resolution.md)
and the [offline resolver contract](../OFFLINE_PROCEDURE_RESOLUTION.md) still
apply: package-owned sources, explicit unqualified AP1 selection, preserved
unknowns and `executable=false`. No schema, installer profile, parser algorithm,
budget, device admission or UI change is part of this disposition.

## Implemented resource boundaries

Source references are `crates/knx-productdb/src/procedure_resolution.rs` and
its `procedure_resolution/xml.rs` child; constants below describe the existing
implementation, not new allowances.

| Boundary | Inclusive allowance | Enforcement and meaning |
| --- | ---: | --- |
| Program or master source | 8388608 bytes per document | Stored declared length must first match actual BLOB length; oversized sources refuse before loading, then hash integrity is checked. XML parsing also checks source bytes before UTF-8 validation. |
| XML elements | 65536 per document | Start/Empty elements including root; End events do not add nodes. |
| XML depth | 64 levels including root | Start and Empty elements both count; a level-65 Empty leaf refuses. |
| XML work | 200000 event reads per document | EOF itself needs a slot; comments, PI, declarations, End and text/reference events count even when not retained. |
| Raw element attributes | 256 per element | Namespace declarations count before being excluded from retained attribute values. Existing exact-boundary tests already cover this. |
| Retained-tree policy charge | 67108864 bytes per document | Each node charges 256 plus expanded namespace/local-name bytes; retained attributes charge 96 plus expanded-key/value bytes; retained text/CDATA/decoded references add their bytes. |
| Resolution policy charge | 1048576 bytes per resolution | Initial metadata and retained subtree charges are checked before cloning. This is not serialized output size or measured RSS. |
| Effective and unplaced roots | 2048 combined per resolution | Both append paths share the count and charge allowances; descendants and merge records are not extra top-level roots. |
| Retained issues | 2048 records per resolution | Further issues increment the omitted count, preserving the retained prefix rather than silently disappearing. |
| Measurement-probe projection | 32 issue codes, each at most 96 bytes | A reduced observer view, not the resolver's issue limit; projection exclusions add to its reported omissions. |

Program and master each have independent input/tree counters and are both
retained during expansion. The policy charges are not process-memory limits:
reader/container/path/issue/origin/serialization allocations are not exact
allocator-accounted by them. Partial results may preserve an already emitted
prefix after refusal, but cannot become executable or complete by truncation.
Nested merge content is retained/reported, not recursively executed.

## Boundary-test acceptance scope

The frozen characterization selection passes 15 library tests (11 XML and four
resolver-helper tests), 25 bundled integration tests and 13 measurement-probe
tests. The complete owning ProductDB run passes 1226 tests with 31 ignored,
including both 13-test examples and two doc tests; all-target warning-denied
Clippy also passes. These selections overlap and are not added together.
Production parser prefixes and existing XML tests remain byte-exact. This is
coverage for already-correct behavior, not an original production-defect RED.
Independent whole-change review accepts this bounded scope; its minor prose
spacing correction is closed. Twelve compiled isolated wrong variants fail
their intended semantic assertions; the same 15 library and 25 integration
tests pass before and after byte/mode-exact snapshot restoration. The canonical
tree was never mutated. Source-byte discrimination changes both serial admission
layers as one policy case; its first failure does not separately exercise the
master-source row. The order variant proves reversal sensitivity only. All 6919
private artifact identities and 16 retained compiled binary hashes are verified.
This is characterization of unchanged production, not a new bugfix or speedup.
Maintained package gates and publication remain pending.

The executed boundary selection distinguishes these mechanisms:

- Exact source-byte admission for installed program and master separately:
  8388608 bytes retains the exact five-control expansion; 8388609 returns
  unavailable with exactly `source-byte-limit` at the appropriate source.
  Exact installed package/source bytes and hashes, every table/schema and
  database/sidecar bytes remain unchanged across resolution.
- XML node, depth and EOF-event boundaries, with admitted structure asserted
  and other ceilings kept below their respective allowances.
- A reachable exact retained-tree charge of 67108864 and its next retained
  byte: 1000 shallow namespace-amplified children and retained text isolate it
  from node, depth and input limits. Actual source sizes are 1383176/1383177.
- Exact combined step/unplaced count, independently of output charging: the
  2048/2049-root requests charge 1005034/1005524, both below the byte allowance.
  The refused public result retains 2048 exact roots with a named partial
  outcome. Direct helper tests cover both mixed append directions. The older
  attributed 2049-control test remains only a broad output-refusal safety net.
- Exact output charge 1048576 and its next reachable value 1048582: a three-root
  fixture admits Connect/Load/RelSegment at the limit and preserves exactly
  Connect/Load when the final clone refuses. Putting the heavy node last is
  essential; the ordinary five-control fixture would refuse a later Restart.
  Direct helper tests check both append paths and unchanged state on refusal;
  initial metadata cases use nearest reachable charges 1048574/1048580. A
  whole-resolution charge of 1048577 is not promised merely for symmetry.
- Exact issue-prefix retention and omitted counts at helper level. The probe's
  existing 32-code test is separate and does not establish the 2048-issue cap.
- The existing five-control oracle strengthened to exact order, attributes,
  original source slices, offsets, hashes and origin locations. Preserve its
  merge dispositions and deterministic repeat, plus all existing unknown,
  ambiguity, malformed, unplaced-order and read-only refusal tests.

Direct helper-state tests establish the helper boundary, not an unexecuted
source-to-database path. Coverage-only tests may correctly pass the unchanged
production baseline; discriminate important assertions using isolated wrong
variants, then verify the original source again. No fabricated original parser
defect or unmeasured speedup is required to justify this no-change decision.

The bundled commands are `cargo test --offline --locked -p knx-productdb --lib
procedure_resolution::`, `cargo test --offline --locked -p knx-productdb --test
it procedure_resolution::`, and `cargo test --offline --locked -p knx-productdb
--example productdb_measure`. Run them with disposable test homes and a
worktree-specific Cargo target directory. Independent review and the maintained
change-scoped gate remain separate acceptance requirements. WP-04.3's web
workflow and WP-19 are not implemented or accepted by this disposition.
