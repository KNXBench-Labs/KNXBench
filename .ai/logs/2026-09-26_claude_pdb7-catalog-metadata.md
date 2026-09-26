# PDB-7: source-value catalogue metadata, and one XML validator that grew up

**Date:** 2026-09-26
**Agent:** Claude
**Branch:** `pdb7-catalog-metadata` (base `3fb910a`)

## What shipped

ProductDB schema **v13**. Eight `ApplicationProgram` attributes are persisted
as nullable, parser-decoded source strings on the winning program row:

`IsSecureEnabled`, `MaxSecurityGroupKeyTableEntries`,
`MaxSecurityIndividualAddressEntries`, `MaxSecurityP2PKeyTableEntries`,
`MaxTunnelingUserEntries`, `MaxUserEntries`, `MinEtsVersion`,
`ReplacesVersions`.

They are exposed by `query::ProgramRow` and `knx products show`, and nothing
interprets them. `IsSecureEnabled="true"` is a vendor catalogue claim, not a
verified device property — see ADR-0037 and the new KNOWN_LIMITATIONS entry.

Semantics that are actually tested, not merely intended:

- absence stays `NULL`; an explicit empty attribute stays `Some("")`
- first writer wins for duplicate program IDs, in one blob and across blobs
- namespace-qualified lookalikes remain unknown evidence, never catalogue values
- v12→v13 backfill rederives only from the *matching winning* source record
- a package rollback takes the catalogue columns down with the program row

## The interesting part: what the reviews caught

Two independent read-only reviews produced three Important findings. All three
were real, and the fix ended up larger than the feature.

**1. The migration's XML validation was not actually validating.** The backfill
walked events and stopped at `Eof`, which quick-xml happily produces for an
unclosed root. A truncated retained blob could contribute catalogue values.

**2. Direct `ingest_file` bypassed validation entirely.** Package admission
validated; the `.knxproj` import path did not. Same bytes, two verdicts — the
exact asymmetry that makes data-integrity claims worthless.

**3. Even the extracted validator was incomplete.** `quick_xml::Reader`
defaults `check_comments` to **false**, and nothing enforced declaration or
DOCTYPE placement. A comment containing `--` sailed straight through.

Fix: `package::validate_xml`'s body moved into
`knx_productdb::xml::validate_complete_document`, gained `check_comments =
true` plus prolog/root-phase tracking, and is now called from **all three**
paths. One validator, one verdict.

## Three fixtures that were quietly wrong

Tightening the check turned up pre-existing test debt — none of it in product
code:

| Fixture | Problem |
| --- | --- |
| `cli_documentation_export.rs` | hard-coded "version 13 / supports up to 12"; now derived from `CURRENT_PRODUCTDB_VERSION` |
| `http_documentation_export.rs` | positional `INSERT INTO application_program VALUES (...)` with 14 values against a 22-column table; now named columns |
| `http_parameter_panel.rs` | an XML comment containing `--`, which XML 1.0 forbids |

The first two were *passing for the wrong reason* before v13 and would have
broken on every future migration. Fixed at the root, not at the symptom.

## Evidence

Full workspace gates, exit 0:

- **2,110 tests** across 104 blocks, 0 failures
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`
- layering, headers (211 with / 162 without, ceiling 162)
- anchors: 376 links across 196 markdown files, none dead
- `cargo deny check`: advisories, bans, licenses, sources all ok
- `git diff --check`

Private read-only corpus matrix (115 instances, 113 unique hashes), **passing
and byte-identical before and after the stricter validation**:

- isolated attribute presence: `[34, 34, 32, 27, 6, 6, 310, 140]`
- shared attribute presence: `[33, 33, 31, 27, 5, 5, 273, 130]`
- isolated unknown constructs: 22,758 (589 fewer than PDB-6)

That byte-identity is the load-bearing result: the tightening cost the real
corpus exactly nothing.

## Boundaries

No ETS parity. No runtime KNX Data Secure support. No commissioning claim. No
hardware, bus or network access occurred. A vendor file that ETS tolerates but
XML forbids would now be rejected rather than half-read — documented, not
hidden.

PDB-8 is explicitly **not** started.
