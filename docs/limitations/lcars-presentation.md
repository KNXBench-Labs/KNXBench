# LCARS presentation

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-lcars-presentation)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## LCARS presentation is Chromium-verified, not an executable imported pack

The optional built-in LCARS palette and workbench framing are implemented
([ADR-0092](../adr/0092-lcars-built-in-presentation.md), [guide](../DESIGN_LCARS.md)).
The palette applies application-wide; distinctive framing targets the main
workbench. Imported v1 packs cannot request this geometry or animations; LCARS
is not exportable as a lossless v1 pack. Density and motion remain independent.
No light variant or sounds. Native WebKitGTK/Orca, Firefox, browser-chrome zoom
and complete WCAG acceptance are unverified. Chromium proof uses only
intercepted synthetic API replies, including a deliberately refused save; it
does not establish live project persistence, bus operation or deployment.
<!-- END retained catalogue record -->
