# Catalogue-authored project portability — 20 October 2026

## Outcome, not a repair

The KL-183 case previously traced only in source is now runtime-reproduced.
A newly authored project with a catalogue-created device saves successfully and
retains its complete normalized model, but reopening it with an empty product
database cannot resolve its application. This package adds one synthetic HTTP
characterization and current documentation; **production is unchanged**.

Baseline: published `93b1a2d5`, following WP-13's documentation reconciliation.
No real project, product corpus, device, network service, credential or vendor
code is used. The original imported-project tests are retained.

## Executed boundary matrix

The fixture installs a fictional `.knxprod` through `POST /api/catalog/install`
with a master, catalogue, hardware and tiny parameter program, then uses the
real `/api/project/new`, `/api/devices`, save and open routes. No API response is
mocked; each expected successful HTTP response is checked.

| Boundary | Observed/asserted outcome |
| --- | --- |
| Install and catalogue create | One exact catalogue item; one device; parameter program resolves as `M-0001_A-1`. |
| Source availability | Authoring ProductDB retains exact hardware, program and catalogue source hashes. |
| Save As, then Save | Both return success and produce an existing native file. |
| Native source inventory | Saved manufacturer manifest and carried-file table are empty. |
| Reopen with original ProductDB | Complete project equality and resolving program. |
| Reopen with a fresh empty ProductDB | No installed applications before open; complete project equality afterward, but `programId: null`. |
| Missing-data diagnostic | Exact `programRef`, `productDatabase: true`, `missingFiles: 0`. That zero measures absent manifest entries, not complete source availability. |
| Reinstall exact synthetic package | Program resolves again; complete project and saved native bytes unchanged; no carried files silently appear. |

The desired-portability test first failed at the intended final assertion after
successful create/save/open and project-equality checks (`null` versus the exact
program ID). Its test source, digest, command, raw log and real exit101 are
retained privately. The committed test is explicitly named for the **current
limitation**, not presented as a fixed portability regression. Its positive
characterization and the owning 12-test suite pass. Those scopes overlap and
are not summed; the later complete scoped package gate is recorded separately.

## Meaning and limits

- Model persistence and product-source portability are independent outcomes.
  The witnessed gap does not mean the device or project model was discarded.
- Reinstalling the matching package is demonstrated only on this synthetic
  source. It is not universal device/vendor, native desktop, Docker rebuild or
  whole-recovery acceptance.
- Imported projects with already-populated manifests retain their existing
  positive tests. No general catalogue dependency closure is inferred from them.
- This test intentionally pins the present limitation. A future manifest repair
  must replace its expected missing-source outcome with a real portability
  assertion, while retaining the imported-project positives and model checks.

## Package acceptance

Delivered on `main` as main-first `0dbef5cf` (feature `4b7b8db0`); immediate
local/tracking/live readback matched. Maintained scoped gate13/13 passed:
1,057 Rust tests, zero failures, 53 ignored in the ordinary run; 51/51 selected
corpus cases separately executed; 323 registered tooling tests with one existing
optional skip. All proofs valid, source inputs frozen and user-data guards clear.
Review is a separate in-session self-review, not independent approval. Counts
from overlapping focused/ordinary/corpus scopes are not added. This accepts the
characterization package, not a production fix or full-workspace/release gate.

## Next implementation gate

Define which exact installed source identities/dependencies belong in a project
when catalogue devices are authored, including shared hardware, program versions,
resource/baggage needs, conflicting candidates and unavailable bytes. Specify
how manifest acquisition joins revision-bound create/history/undo/save behavior
without exporting unrelated catalogue content. Then implement that separately
and require empty-product-database reopen, stale/refusal atomicity and source
identity regressions. Do not repair this by guessing filenames, embedding the
whole global database or counting `missingFiles: 0` as success.

[Current limitation](../limitations/KL-183-knxdb-manufacturer-files.md) ·
[Manufacturer-file ADR](../adr/0128-native-project-files-carry-their-manufacturer-files.md) ·
[HTTP characterization](../../apps/knx-server/tests/it/http_knxdb_product_data.rs).
