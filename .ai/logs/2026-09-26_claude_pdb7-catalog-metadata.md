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

---

## Addendum, 2026-09-26 11:10 — gating on merged `main`

### Pre-existing defect surfaced by the full workspace run (not caused by PDB-7)

`cargo test --workspace` fails on **seven** tests that assume a *flat*
`OriginalData/ProductDatabases`: the five `dynamic_tree` `corpus_*` tests,
`parameter_views_corpus::parameter_views_and_parameter_ref_ids_match_the_ap_level_count_on_prod3`,
and `standalone_packages::installs_the_readable_corpus`. The local corpus now
has per-manufacturer subdirectories, so every referenced fixture still exists —
one directory deeper.

Verified pre-existing by running the identical tests at base commit `3fb910a`
in a detached worktree with `KNXBENCH_PRODUCT_CORPUS` pointed at the real
corpus: the failing set is **exactly equal** before and after the merge
(`only in merged: NONE`). Documented in `docs/KNOWN_LIMITATIONS.md`
(commits `182b54f`, `2162fff`); not fixed here, as it is unrelated to
catalogue metadata.

> **Trap worth remembering.** The first baseline attempt reported 54/54 passing
> — because an isolated worktree has no `OriginalData/` and the tests silently
> took their skip path. A corpus test that skips quietly is worse than one that
> fails loudly. Always pass `KNXBENCH_PRODUCT_CORPUS` explicitly when judging
> corpus-gated results.

### Two portable lessons from fixing the gate run

**Positional INSERTs are a schema tripwire.** Two tests broke on v13 purely
because they used `INSERT INTO application_program VALUES (...)` with a
hand-counted value list. Both were converted to named column lists.

**Our own fixtures must be valid XML.** `http_parameter_panel.rs` carried a
comment containing `--`, which XML 1.0 forbids. The new validator was right and
the fixture was wrong; the comment was reworded rather than the rule relaxed.
All 115 corpus instances pass the stricter check, so the hardening cost nothing
in real-world compatibility.

### Gate results on merged `main`

Excluding only the seven flat-layout tests: **1399 passed / 0 failed**.
`cargo fmt --check`, `clippy --workspace --all-targets -D warnings`,
`check-layering`, `check-headers` (215 headers), `check-anchors` (376 links /
202 files), `cargo deny check`, `git diff --check` — all exit 0.

### Pushed — but not by me (correction, 11:16)

`origin/main` moved to `a64edc8` at **11:13:06** (reflog: `update by push`). My
last commit was 11:07:39 and I issued no push. Presumably the parallel
commissioning agent pushed, which carried all seven commits public — including
`15f704a` and `03e358f`, the commissioning strand **this work did not review**.

The earlier intent recorded here ("not pushed, deliberately") was overtaken by
events rather than reversed by judgement. Consequence to act on: that strand is
now public with no review of mine behind it, and it should be gated on its own
terms retroactively. The root worktree's foreign local edits were protected
across the merge with a targeted stash and confirmed byte-identical afterwards.
The PDB-7 worktree and branch were removed after confirming the branch is an
ancestor of `HEAD`.
