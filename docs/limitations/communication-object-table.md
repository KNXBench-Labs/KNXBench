# Communication-object table

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-communication-object-table)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## Measured table fitting (18 October 2026)

The shared column adapter measures mounted single-column header/cell words and
caps each automatic floor at 320 px. Tokens wider than that budget and explicit
user shrinking can still wrap at character level. Spanning headings/editors and
fixed matrix object columns retain their own layouts. Browser proof covers
grouped/flat objects at 360 px, status at 640 px, a long German address DPT and
monitor decoded label at 400 px. Production EN/DE fit-budget checks at 1440/1920 px
use synthetic data, not arbitrary imported projects. With real density/theme
padding, the 1440 px device panes cannot fit their measured floors (EN 919 px,
DE 955 px, each in 780 px); independent intrinsic budgets are 924/961 px and
text columns alone already exceed the pane. German flat objects need 800 px in
738 px. These cases
scroll locally. Grouped objects fit at 1440 px, and all tested tables fit at
1920 px. Sorting reserves a fixed icon slot; mounted floors grow but never shrink
until table reset/remount. History instances share preferences, not measured
maxima. Ancestor class/style changes and colspan remapping invalidate the affected
table; dynamic DOM reparenting requires remounting. Different content can still produce different widths. The stored DPT preference
minimum is 100 px; the rendered width follows the fitted-baseline pixel delta
and can be lower. [Contract and exact verification](../contracts/table-column-widths.md).

UI-05 supplies column resizing; the older no-resizing statement below is
historical. Reordering remains outside that package.

<!-- BEGIN retained catalogue record -->

## Communication-object table scope (2026-10-09)

The [local 2026-10-11 DPT selection extension](../contracts/dpt-selection.md) names
454 standard IDs in EN/DE without needing installed master data. This is a
versioned display vocabulary, **not** additional codec/device support or a
manufacturer database. Main-only/unknown/imported values remain exact selectable
values; no subtype/name is invented for them. Additional UI language packs use
English labels. Raw master-text language/fallback remains separately disclosed;
these authored labels are not claimed as official KNX translations. Chromium
checks do not establish native WebKitGTK/Orca, ETS or hardware compatibility.
The owner-authorized main integration is accepted on `e57e3a09ff34` and
published at `ab7db1ae7991`; the
[source/publication receipt](../evidence/dpt-selection-publication-2026-10-11.json)
records the separately verified merged-source and remote-readback evidence. The extension stays
outside the Alpha.8 release snapshot and existing live container.

The local UI table keeps all object summaries mounted for stable editor identity;
editing forms mount lazily when first opened. The synthetic 1,000-object measurement
is one production-browser action-to-visible sample, not a universal latency bound
or proof with all editing forms expanded. Large devices may still take noticeable
time to switch to the flat view. No virtualization is introduced without evidence.
View state is intentionally lost when leaving the editor/changing device/project;
there is no project-wide CO list, bulk editing, export, persistent view cache or
resizable/reorderable columns. A UI mutation guard cannot cancel an already admitted
server edit or replace server concurrency rules. Native WebKitGTK/Orca, real hardware
and complete ETS compatibility remain untested by this offline UI package. Local
source is not in alpha.6; authorized main publication is separate from release/deployment.
[Behavior and acceptance](../COMMUNICATION_OBJECT_TABLE.md).

A diagnostic real-server attempt found one existing local schema-v22 product
catalogue without `source_identity_scan.scanner`; package installation refused
with HTTP 500. The fresh isolated catalogue used by acceptance has the expected
column and installs the fictional package. No catalogue repair/migration change
is included in this UI package. The initial harness had isolated `KNX_DATA_DIR`
but not XDG, so it opened the existing catalogue before refusal. Its main-file
mtime still predates the run and the test package is absent; this is not a
before/after whole-file byte audit. The corrected accepted harness isolates HOME,
XDG data/config/state and the project directory; the source demo remains unchanged.
<!-- END retained catalogue record -->
