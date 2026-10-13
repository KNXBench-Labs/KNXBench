# Resolved product filename case handling

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-149)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->
## §149 `knx products ingest` matches the `.knxprod` extension case-sensitively

**Scoped resolution published362fec24 (2026-10-03):** The scoped AR06P CLI correction
uses exact ASCII case-independent extension comparison, with five synthetic CLI
regressions, four named compiled mutation failures, public workspace/build
gates and same-release-profile baseline RED/candidate GREEN verification.
`.KNXPROJ` remains project import; case-independent pre-destination legacy
refusal remains unchanged. Full unpinned853 original-filename measurement
is independently accepted on the same release profile:644→687 installs,
Hager/Berker2→45;43 upper-case packages admitted, two ZIP-limit and one
evidence-item refusal remain explicit/atomic. Originals and retained successful
blobs verified unchanged; no private raw/item logs. Normal U18 merge dd5a350c
is independently actual16 accepted: Rust3000/0/165, Web1702, Chromium82,
17 bindings/704 committed-exact inputs. All285 CLI build/test inputs equal
the measured producer. Delivery362fec24 live/fetched/local refs equal0/0,
source704 and ten owned artifacts byte-exact; five acceptance Markdown gates
pass. This lifts only the case-sensitive CLI dispatch limitation, not §150–153
or full product/ETS/commissioning compatibility.

**Historical baseline observed 2026-10-03** (PRODUCT_DATABASE_CORPUS §Public crawler corpus run).
`run_products_ingest` (`apps/knx-cli/src/main.rs`) routes a file to
`knx_productdb::install_package` only when its extension is exactly
`knxprod` or `vd2`. Hager and Berker publish their product databases as
`*.KNXPROD`. Those 46 files fall through to the `.knxproj` project importer
and fail with the misleading `no P-*.signature entry; cannot determine the
project part`. The same bytes under a lowercase name install 43 of 46 (the
other 3 hit §151/§152).

- Only the CLI is affected. The server install route calls
  `install_package` directly with no extension switch, and the web file
  picker's `accept=".knxprod"` matches case-insensitively per HTML.
- `install_package` itself already compares legacy extensions
  case-insensitively (`.vd3`–`.vd5`, `.pr3`–`.pr5`), so the two entry
  points disagree.
- **Historical workaround (older CLI builds):** rename or symlink to lowercase `.knxprod`.
- **Lifted in the scoped delivery above:** exact ASCII case-independent routing
  with uppercase/mixed-case, project negative-control, legacy-refusal, retained
  byte/idempotence regressions and independently compiled routing mutants.
<!-- END retained catalogue record -->
