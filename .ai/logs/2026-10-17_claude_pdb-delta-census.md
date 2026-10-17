# 2026-10-17 — Claude — delta census over two added product databases

Docs only. Same tools as the gap census (scanner, shape probe, installer probe
with the release build of `3e770524`), run over just the two added files; the
originals were byte-identical afterwards (SHA-256 before and after).

Findings are in the census dossier's addendum: both files are wrapper ZIPs, the
complete database has no install route as shipped, `TypeDate` appears for the
first time (G8), and property-placed parameters become common (G9).

Pitfalls:
- The install facts JSON uses camelCase (`unknownConstructs`); a snake_case
  lookup returns nothing and reads as "0 unknowns".
- The probe's large-profile retry looks for "size"/"limit" in the last output
  line; the current hint wording ("bounds") does not match, so the large
  profile was measured separately.
- The scanner skips dotfiles, so a file-list diff against the census shows the
  owner's password file as "new"; it is not product data and is not read.
- Verify printed page numbers per PDF page (`pdftotext -f N -l N`); the
  `-layout` full-text line number is not a page number.
