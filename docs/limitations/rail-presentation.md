# Rail presentation

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-rail-presentation)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Rail presentation is Chromium-verified, not an executable imported pack

The built-in Rail and Rail Dark palettes share one trusted presentation
([ADR-0157](../adr/0157-rail-built-in-presentation.md)): square geometry,
Geist type, a decorative toolbar rail and a few action-bound effects behind the
`theme` motion category. Imported v1 packs cannot request this geometry, the
Geist fonts or the effects; neither palette exports as a lossless v1 pack.
Geometry targets shared controls, the workbench shell and the common dialog and
menu surfaces; components with their own hard-coded paint keep it. Native
WebKitGTK/Orca, Firefox and complete WCAG acceptance are unverified. Chromium
proof uses intercepted synthetic API replies only; it establishes no live
project persistence, bus operation or deployment.
<!-- END retained catalogue record -->
