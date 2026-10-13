# Maintaining the limitations catalogue

[Short overview](../KNOWN_LIMITATIONS.md) · [Canonical ledger](../status/LEDGER.md)

## Roles

- `KNOWN_LIMITATIONS.md` is the generated, topic-grouped reading index: a current
  title, short boundary summary, ledger-derived status and link per entry.
- Each `KL-NNN-*.md` file contains that limitation's technical detail and dated
  history. Unnumbered notices use descriptive names; these are not invented KL IDs.
- `catalogue.json` holds authored titles, summaries, topics, filenames, legacy
  aliases and explicit ledger references. It contains **no stored statuses**.
- `status/LEDGER.md` remains the only formal per-ID status record. A row marked
  `DONE` can still have compatibility/evidence residue; do not equate it with
  universal support. Broad feature references on unnumbered entries are related
  ledger dispositions, not newly created limitation IDs.
- `manual/known-issues.md` remains the practical user selection; `OPEN_WORK.md`
  remains the work queue. Neither is replaced by this catalogue.

## Update an existing entry

1. Edit its title/short current boundary in `catalogue.json`. Change formal status
   only in its existing ledger row, following the owner rules.
2. Add a dated detail/update **above** the retained historical record in its
   individual dossier. Do not silently rewrite an earlier claim or test result.
   Old "above/below" references refer to the former combined catalogue: navigate
   through the overview and original numbered aliases instead.
3. Regenerate and check:

   ```sh
   python3 tools/limitations_catalogue.py --write
   python3 tools/check_documentation.py
   python3 -m unittest discover -s tools/tests -p test_limitations_catalogue.py
   cargo run -p xtask -- check-anchors
   cargo run -p xtask -- check-ledger
   ```

   `check_documentation` checks that the overview exactly matches its authored
   summaries and the current ledger rows. It does not obtain release information,
   run vendor code, contact hardware or certify compatibility.

## Add an entry

Create one detail file and one metadata entry. Allocate numbers only after
checking fresh upstream; never reuse an old number. A new formal source ID needs
its own owner-authorized ledger row. Until then use `ledger_ids: []`, which renders
**Not separately tracked**, not an invented `TODO`/`DONE` status.

Preserve existing aliases and the separate `KL-130-GATE` / `KL-130-ZOOM`
identities: their inherited number is shared, their evidence and statuses are not.
Unknown catalogue versions, duplicate identities/files/aliases, unlisted dossiers,
missing files, path traversal and stale generated output are refused.

## Preservation evidence

The [migration receipt](../evidence/limitations-reorganization-2026-10-13.json)
binds the 189 original sections, all original heading/alias identities and the
five descriptions previously kept in `history/KNOWN_LIMITATIONS_resolved.md`.
Each original body was preserved, with only reversible local link destinations
adapted for its new directory. Historical record markers and retained-body hashes
make accidental deletions detectable by the regression suite. Do not change these
frozen blocks or their receipt just to make a failing test pass; append a dated
correction outside them. Git also retains the original monolithic source.

The old resolved-history path remains a short link index for compatibility.
No limitation, ledger disposition, user decision or product support claim was
closed or broadened by the reorganization. Root-only, unpublished vendor research
was not incorporated into this migration; its owner must publish it separately
into the appropriate dossier without restoring the old monolithic file.
