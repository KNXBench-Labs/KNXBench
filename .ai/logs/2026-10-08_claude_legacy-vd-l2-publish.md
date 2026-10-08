# 2026-10-08 — Claude — Legacy VD L2: publish programs into the product database

## Scope

Package L2 of ADR-0094 (decisions Q1–Q18 in
`2026-10-08_claude_legacy-vd-grilling.md`): import every application
program of a legacy `.vd3`–`.vd5` product database for offline engineering.
Download (L4) and the server/web upload (L3) stay out.

## Decisions taken while implementing

- **Ids (revises Q9/Q16):** `M-<hex>_A-LX<sha8>-<PROGRAM_ID>`. The marker
  sits in ETS's own segment, so every existing id parser sees the usual
  segment count. A digest-prefix collision is refused by name.
- **Shape:** pure mapping (`LegacyMapping`) plus one publishing
  transaction. Idempotent per payload digest; a renamed or re-encrypted copy
  only adds a `legacy_source_file` row.
- **Translations of group members:** left out only where they add nothing
  (no text override and equal to the representative's). Found through the
  synthetic fixture; `EIBMARKT.VD3` has one such case (1,365 → 1,366).
- **Write authority** is recorded for legacy programs (no calculations in
  EX-IM), so the parameter editor accepts edits.
- **Skipped rows** are reported per table and reason (`skipped-rows`),
  never dropped silently.
- **CLI order:** decrypt, then open the product database.

## Evidence

- Oracle N000520 (`knx-app/tests/legacy_oracle.rs`): 260 parameter refs,
  28 object refs, 3,535 translations, 36 visibility cases; three named
  deviations (5008 access, en-US program-name translation, 5008 placement,
  each checked against the ETS XML structure).
- Corpus (`legacy_corpus.rs`): both real files publish with pinned counts
  and evaluate with only `NoBranchMatched`. ETS's conversion shows the same
  kind (6 against 1 under defaults).
- Mutation sweep 20/20 with named failing tests.
- Gate: on `d110de30` (rebased on `8e8aa6b6`), under both gate
locks, inputs frozen (empty diff at start and end):
- Web build, fmt and clippy `-D warnings` (workspace) pass; all five xtask
  gates pass; `git diff --check` is clean.
- Workspace tests: **3,663 passed, 0 failed, 182 ignored** (217 result
  blocks).
- Corpus: legacy corpus 3/3 and oracle 1/1; `standalone_packages` ignored
  3/3; `legacy_member_names_corpus` 1/1.
- Product matrix (release): the first run was red on the aggregate
  commitment only. It now counts the four new, empty v22 tables. With them
  left out, the v16-shaped projection proved unchanged, so the commitment
  was re-pinned (`7b558cdd…` → `541d0afc…`) and each table pinned at 0.
  The rerun passes 1/1, and clippy for `knx-productdb` passes again.

## Found on the way

- Rewind fixtures (`v16`–`v20_rewind`, `write_authority.rs`,
  `parameter_attribute_unknowns.rs`, `read_only_open.rs`) did not drop the
  v22 tables, so 36 migration tests failed with `table legacy_source already
  exists`. `v20_rewind::drop_v22_objects` now runs in every rewind.
  `master_language_evidence.rs` pins the schema version (now 22, comment
  extended).
- The first real `.vd5` (Siemens, Nov 2016) is refused by size (173 MB
  payload) and has four members (an installer tree with mask images). This
  is recorded in research/KNOWN_LIMITATIONS; it needs its own package.

## Next

- L3: server upload with a password dialog and the remembered password
  (0600 file under `$XDG_CONFIG_HOME/knx/`); visual web check of a legacy
  device's catalog entry and parameter panel.
- `.vd5` layout + measured bounds (Siemens sample).
- L4: download (`s19_block`, mask 0701 untested, BCU1 refused).
