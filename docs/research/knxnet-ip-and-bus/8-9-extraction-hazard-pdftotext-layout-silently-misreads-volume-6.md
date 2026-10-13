# 8.9 Extraction hazard: `pdftotext -layout` silently misreads Volume 6 Annex A's tables (2026-09-20, task C18 fix round)

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="89-extraction-hazard-pdftotext--layout-silently-misreads-volume-6-annex-as-tables-2026-09-20-task-c18-fix-round"></a>
<!-- BEGIN preserved record-1-038 -->
### 8.9 Extraction hazard: `pdftotext -layout` silently misreads Volume 6 Annex A's tables (2026-09-20, task C18 fix round)

**Standing caution for anyone auditing Volume 6 (`06 Profiles`) Annex A's
per-object Property tables (A.2.3 through A.2.10, pp. 138-152): do not trust
`pdftotext -layout` column alignment on these tables. Render the page as an
image and read it visually instead.**

Found while re-verifying a task C18 finding: `PID_MCB_TABLE` was first read
via `pdftotext -layout` as carrying a mandatory-existence symbol for System B
(masks 07B0h/17B0h) in A.2.4 (p. 143) and A.2.5 (p. 145). It does not — the
printed symbol is `(3/3)`, the parenthesised *optional*-existence family per
Table 3's legend (p. 134). Rendering the pages at 250 DPI
(`pdftoppm -f 143 -l 143 -r 250 -png "06 Profiles v02.01.01.pdf" out`), then
cropping/upscaling the right-hand columns for legibility, showed why:
each system group (System 2, System 300, System 7, System B, ...) carries
**one extra data column with no mask number** — it inherits only the
rotated group-name header — in addition to the explicitly mask-labelled
columns (07B0h, 17B0h, 57B0h for System B). `pdftotext -layout` does not
know this unlabelled column exists as a distinct field; it silently merges
or shifts it against the neighbouring mask column, so a value read by
counting columns out from the mask-number headers lands one column off.
Text extraction gave a false mandatory reading; the rendered image gave the
correct optional one.

Practical rule: for any Annex A table read that matters (i.e. feeds a
compatibility or compliance conclusion, not just a sanity check), verify by
rendering the page as an image and counting columns visually against the
table's own header row on that same page — never against a remembered
column count from `pdftotext` output on a different page or a different
system group.

---


<!-- END preserved record-1-038 -->
