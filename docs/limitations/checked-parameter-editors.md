# Checked parameter editors

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-checked-parameter-editors)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Checked parameter editors in the device editor (2026-10-12)

- The **Checked editor** is opt-in per field and offers only what the bounded leaf
  admits (section *Bounded AP3 parameter controls* below): decimal Time counts,
  explicit integer sliders, and an unconverted original for Colour/Picture.
  Module sections always fall back; PDB-04 stays LATER.
- Writes still use the legacy, not revision-conditional parameter route. Ignoring a
  late reply does not cancel a sent write; the host publishes every completed
  write instead.
- The source is read once per opened field and revision; no bulk query or cache.
- A refresh no longer blanks the parameter rows for the same device and product
  language, so the previous values stay visible for the moment the new answer takes.
- Evidence is Chromium (intercepted production build and a loopback-only
  real-server smoke with a synthetic product). No physical device, ETS reimport,
  native WebKitGTK window, Orca session, release or deployment is claimed.
<!-- END retained catalogue record -->
