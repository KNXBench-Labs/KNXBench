# Read-only retained project payload catalogue

Local implementation contract, 2026-10-11. No domain/schema migration.
Accepted locally on frozen sources: owning Rust1303/0/100 ignored, whole
Web2689/0/175 files, built Chromium12/0/0 skipped, strict quality/build/type/
repository/docs gates pass. Additional private production/native byte and report
preservation probe passes with unchanged original; no private data published.
Self-review only; integration/publication remains separate.

## Contract

- Reuse native opaque/context bytes already preserved. Do not parse nested vendor
  packages, execute scripts/DLLs/ETS-Apps, replace attachments or claim plug-in
  compatibility. Source-derived paths are display-only, never filesystem targets.
- Catalogue whole payload rows only (empty xpath): native `Baggage`, project
  UserFiles/AddinData paths and .etsapp-named candidates. Keep exact source kind,
  path, byte length and declared source hash. Distinguish path-derived candidate
  role from existing content sniffing; neither is authenticity verification.
- Exclude primary/source report metadata, raw attribute fragments and unrelated
  source XML. Manufacturer product-database baggage remains in its separate
  catalogue, not invented as missing project bytes. Scope is the bytes actually
  retained in the current native project.
- Return a deterministic bounded catalogue (max 10,000 payload rows and 4 MiB
  cumulative source-path/kind/hash text before cloning), never a
  silent truncated success. Download is explicit, authenticated and bound to
  current server/load/revision plus exact integer row selector. Replacement or
  changed revision returns stale refusal, not another project's bytes.
- A selected local download verifies actual bytes against the original hash and
  is limited to 64 MiB; the client has the same independent upper cap. Larger payloads remain retained and listed with explicit
  download refusal; this limit is not a discarded-data count. No arbitrary path,
  archive extraction or automatic network/bus action.
- Reuse the existing temporary streamed-download body adapter and cleanup behavior;
  use application/octet-stream, fixed safe Content-Disposition attachment,
  X-Content-Type-Options:nosniff and no-store. Never serve foreign HTML as active
  inline content. The browser UI saves a local .bin file without rendering it.
- EN/DE UI loads catalogue only on deliberate action, separately shows exact source
  name/path, inferred role, content class, size and retained-only/unverified state.
  Warn that attachments may contain private data or executable content. No silent
  copy into public repository. Reset stale results on source revision replacement.

## Evidence required

Synthetic app catalogue/selection/hash/bounds cases; actual HTTP auth/current-
revision stale refusal/attachment headers/exact streamed bytes and temporary
cleanup; client/components stale/duplicate pending guards; built EN/DE400/1440
actual local download with independent byte equality. Preserve original model,
opaque bytes and manufacturer manifest across read/refusal/download. No private
project payload, vendor or hardware execution needed for generic acceptance.

## Not included

Attachment editing/replacement, dependency-aware UserFile declaration editing,
preview rendering, nested extraction, ETS-App runtime, product DB asset export,
authenticity verification, anonymous sharing or encrypted export are separate
researched features. Downloads do not make unknown data safe to execute.
