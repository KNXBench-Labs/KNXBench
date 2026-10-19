# WP-17 — master identity provenance

## Scope and source

Owner request: execute the next package in `docs/OPEN_WORK.md`. The release
handover selects WP-17 as the next offline-ready Alpha 16 candidate. Started
from `5faed038`; claim published in `041bbd1d`. The selected checkout contains
foreign handover/research work and is deliberately not synchronized.

Contract: [ADR-0147](../../docs/adr/0147-master-identity-provenance.md).
Normalized first-winner values must remain unchanged. Retained bytes determine
current declarations, not historical winning-source/install facts. No bus,
hardware, release, deployment or third-party contact is authorized by this task.

## Evidence obtained

- Core implementation committed as `18d02546` (worker). Controller inspected
  the source, new regression assertions and raw logs: ProductDB 457 library,
  586 integration and 2 doc tests pass, zero failed; 30 integration and one
  doc test remain ignored. ProductDB all-target warning-denied Clippy passed.
  New direct-import, built CLI and HTTP projections each passed one regression.
- The independent pinned-corpus baseline on `041bbd1d` passed 11/11, with
  frozen source and 115 instances / 113 unique packages. Two independent raw
  XML counters predict 67 master sources, 26,110 declarations, 459 winners and
  43,631 added diagnostic rows (43,508 collisions, 123 malformed declarations).
  Candidate matrix execution and migration comparison remain pending.
- Web-only independent review accepted with one LOW coverage finding: explanatory
  description mappings were not asserted per diagnostic row. Exact EN/DE row
  assertions now catch a description-swap mutant at the named semantic assertion;
  original bytes were restored and the focused tests passed again.

- Existing master-parser baseline passed using an isolated data/config home
  and a task-specific Cargo target.
- Web regression: the two new diagnostic kinds lacked readable labels/source
  details. EN/DE component cases failed on their intended missing label, then
  passed with translated descriptions and escaped technical disclosures.
- Focused component/German-copy checks: 38 passed; production TypeScript check
  passed. Full Vitest: 4,136 passed, zero failed, 284 files.
- Initial four Chromium cases passed text/scroll-width assertions, but actual
  screenshots exposed left clipping at 400 px. Stronger bounding-box assertions
  failed in EN and DE at x = -113 px. `min-width: 0; overflow-wrap: anywhere`
  on the report fixes the intrinsic-width expansion; all four EN/DE ×
  400/1440 cases pass and every corrected screenshot was inspected.
- Neutral report heading: EN/DE cases failed against the old unsupported-data
  heading, then 38 focused cases passed with `Source diagnostics` /
  `Quelldiagnosen`. The original heading is inappropriate for an ordinary
  restated-identity collision.
- Documentation local-target/chapter checks passed after the source-build
  architecture/model/upgrade/manual additions. Heading-anchor acceptance remains
  part of the final package gate.

## Pending acceptance

Core implementation, replay/migration regression evidence, source-integrity
review, package gate, integration, publication and owned cleanup are pending.
Earlier green web results are not whole-package acceptance.
