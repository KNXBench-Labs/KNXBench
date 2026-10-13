# Add-device preview freshness

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-167)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 167. The add-device wizard confirms names and addresses, not the whole project

**Limitation.** The wizard's review ([ADR-0093](../adr/0093-wizards-are-views-over-existing-commands.md))
is computed by `POST /api/devices/preview` on a copy of the project. The create
sends back only the previewed names and addresses (`expected`); the server
refuses with `409 catalogPreviewStale` when it would now produce different
ones. Other changes in between are not detected as staleness: a line or
building part deleted meanwhile is refused as an ordinary `400`, and a
product package replaced in the database between preview and create yields
the new package's diagnostics in the result. The wizard sets no parameters
or group links. In the explorer only rooms offer an add-device row among
building parts; other kinds are chosen in the wizard's placement step or
through the palette with the part selected. The wizard keeps its state in
memory only; closing it keeps no draft.

**Why.** User decisions of 2026-10-08 (grill-me Q4, Q8, Q13, Q16): the
expectation is tied to what the user saw, not to a revision counter whose
meaning (`snapshot_revision` only orders responses) does not fit.

**Impact.** A create can still be refused after a successful preview; the
wizard says why and nothing is created. Parameters and links are edited in
the device panel afterwards, as for catalog devices.
<!-- END retained catalogue record -->
