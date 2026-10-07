# DPT scope precheck (2026-10-07)

> Historical precheck, retained during the user-authorized repository closeout.
> Its implementation observations describe the earlier snapshot, not current
> `main`. The subsequent [document-wide audit](2026-10-07-dpt-document-audit.md)
> and its committed fixes supersede the proposed follow-up below.

This is a bounded evidence check supporting the user's question whether a
closer DPT analysis would help close gaps. It is **not** a complete audit of
DPT-AS, a new implementation task, or permission to contact the bus.

## GAP-DPT-SCOPE.1 — the unsupported 200-series are classified too broadly

**Primary evidence.** `03_07_02 Datapoint Types v02.02.01 AS.pdf`:

- §1.2, printed page 13, classifies main numbers 200–299 as structured;
  the LTE-only label belongs to the HVAC subnumber range, not that entire
  main-number range.
- §6.6.1, printed page 182, defines `232.600 DPT_Colour_RGB`, with use G.
- §6.18, printed page 194, defines `251.600 DPT_Colour_RGBW` for colour control.

These tables were visually checked by rendering the user's local primary PDF
under `/mnt/daten-i/Sourcecode/knx-spec-kb/sources/The KNX Standard v3.0.0/`.
Its printed footers/identifiers were independently checked with `pdftotext`;
physical pages 13/182/194 match those printed pages. This is evidence against
a blanket LTE/system classification, **not** a claim that every structured DPT
can be used in every communication context. In particular, §6.6.1 NOTE 27 limits
that RGB coding to one receiver because interpretation is device-dependent.

**Current repository.** [Known Limitations §61](../KNOWN_LIMITATIONS.md)
labels the eighteen 200-series main types in its reference master catalogue
LTE/system types and gives absent LTE addressing as the reason standalone
codecs would not be usable. That general rationale needs correction: the
catalogue named in the same entry includes 232 and 251. The current public
codec's decode dispatch was recounted and contains main types 1–30 only.
The examples above are therefore outside its present support boundary.

**Disposition.** Documentation/classification finding. The recorded decision
to leave those types unsupported remains unchanged; this precheck neither
reopens that decision automatically nor implements new codecs. A full audit
should classify each omitted type by actual use and dependencies, then measure
which of the user's devices need it before prioritizing an extension.

## Recommended audit boundary

Build a complete section/type/rule inventory from the PDF, not one ETS master
catalogue. Map each rule to implementation, independently sourced test evidence
and the existing limitation or a new finding. Separate implementation defects,
missing semantics, permitted scope exclusions, Standard contradictions and
hardware evidence gaps. Audit first; fix confirmed defects in focused follow-ups.

Prioritize supported types and the user's actual device needs. Include subtype
ranges/enumerations, reserved bits, invalid values, receive-versus-send policy,
parameter-only restrictions, variable strings and cross-layer exposure. Keep
raw evidence when semantic interpretation is unsupported. The existing
[format-width audit](2026-10-05-dpt-format-widths.md) and
[numeric-range audit](2026-10-05-dpt-numeric-ranges.md) remain scoped evidence,
not full-document closure. No product changes, test reruns or bus operations
were made in this precheck.
