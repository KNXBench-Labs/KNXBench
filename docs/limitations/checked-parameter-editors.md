# Checked parameter editors

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-checked-parameter-editors)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## Update 2026-10-30: bounded IP-address literal editor (OKX-06)

Source after Alpha 18 adds an opt-in checked text editor for the exact direct
`TypeIPAddress AddressType="HostAddress"` declaration, without bounds/width or
additional presentation hints. Existing SHA/program/ref/type/snapshot/authority
checks remain. It preserves dotted IPv4 and exactly eight four-hex-digit IPv6
lexemes, including case, through explicit Apply and the existing undo/save path.
Other source roles, extra hints and module scopes retain specialized fallback;
this does not newly forbid the plain editor. Historical absent/empty/malformed
values are not silently repaired. No network lookup, address-role suitability,
parameter memory codec, release, hardware or wider PDB-04 closure follows.

The current [host contract](../contracts/parameter-editor-host.md) and
[user guide](../manual/user-guide/05-devices-and-products.md#checked-editor-since-alpha-10)
define the bounded form. The historical catalogue record below remains unchanged.

## Update 2026-10-17: the web sends revision-bound writes

The note below ("the web editor host still sends the unbound legacy form") was
overtaken the same day: Goal B1a (`d705a2b8`, 13 October 2026) binds every
parameter write from the Parameters rows and the Checked editor to the snapshot
and value the panel showed; a `409 parameterWriteConflict` keeps the draft and
resends only through **Apply my value** (`ParameterPanel.binding.test.tsx`,
`e2e/parameter-editors.e2e.ts`). Colour/Picture codecs and module mapping remain
the open part of PDB-04 ([review](../status/2026-10-17-ledger-residue-review.md#phase-0--stale-evidence-reconciled)).

## Update 2026-10-13 (Goal A2): the server accepts revision-bound writes

`POST /api/device/{id}/parameters` now also takes a binding — server
incarnation, project incarnation, snapshot revision and the panel `value` it
expects to replace — checked under the lock that applies, persists and
publishes the write; a stale or changed value is `409 parameterWriteConflict`
with no effect ([contract](../contracts/revision-bound-writes.md)). The web
editor host still sends the unbound legacy form until Goal B1 migrates it, so
the retained bullet below about legacy writes still describes the workbench.
Fixed on the way, for both forms: the panel showed only values stored in the
*first* installation (a device in another installation saw its defaults), and
an overwrite in another installation spent a parameter id. Evidence:
`apps/knx-server/tests/http_revision_bound_writes.rs`.

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
