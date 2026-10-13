# Group-address picker matching

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-178)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 178. The group-address picker filters and marks; it does not judge

**Issue #8 (2026-10-13).** The link row of a communication object offers its
linkable group addresses in a searchable picker instead of one long list.

- **Matching** is plain, case-insensitive substring matching per word: every
  word of the query must occur in the address (in the shown notation or the
  compatible spelling), the name, the DPT's label or number, or the stored DPT
  reference. No fuzzy matching and no ranking; the order is the caller's.
- **The DPT mark** compares the address's DPTs with the object's effective DPT.
  The same subtype, or a main-only reference on either side, is a match; the
  same main type with another subtype is marked *Same encoding, different
  subtype*; another main type *Different type*. With no DPT or an unreadable
  reference on either side nothing is marked. A mark never hides, moves or
  blocks an address; what linking does is unchanged.
- **At most 100 options** are drawn; beyond that *Showing N of M* asks for a
  narrower search. The list stays in the page flow (not an overlay), so a long
  list makes the panel longer.
- The linkable set is still MODEL-01's (per installation); drag-and-drop and
  the direction select are unchanged.

Checked by unit tests and production-build Chromium fixtures (EN/DE, 1440/400,
light/dark); the native WebKitGTK window was not exercised. Ledger `KL-178`.
<!-- END retained catalogue record -->
