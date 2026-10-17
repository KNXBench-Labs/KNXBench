# Plug-in data adoption is CLI-only and not undoable

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-186)

<!-- BEGIN retained catalogue record -->

## Data-only adoption of plug-in bytes (17 October 2026)

`knx adopt-plugin-data` and `knx_app::plugin_data` replace the destination
project's `BinaryData` blob and numbered `ExtraData` table of one device with
the bytes of the same entries in a source archive of the same lineage. The
bytes stay opaque ([ADR-0025](../adr/0025-extension-is-data-not-code.md)); the
rules, refusals and retained evidence are in the
[contract](../contracts/plugin-data-adoption.md) and
[ADR-0136](../adr/0136-plugin-data-adoption-is-audited-opaque-replacement.md).

Limits:

- **No server route, no web UI.** The operation is reachable from the CLI and
  the application layer only.
- **Not part of command Undo.** Undo restores the model; the opaque store is
  outside it. The way back is the project version recorded on save, plus the
  superseded bytes retained in the project.
- **Same lineage only.** Product and application references, blob ids, ExtraData
  associations and group-address links must match. Different ids or file
  numbers are refused, never remapped, because that would be a guess about the
  plug-in's format.
- **The ExtraData association is observed, not documented.** A device parameter
  whose value is the number of an `ExtraData` file selected it in the one real
  ETS6 project examined (each of its three `ExtraData` files is named by a
  parameter value of a SmartSensor device). A coincidental
  numeric value is harmless only because both sides must agree and an
  unchanged file is never touched.
- **Acceptance is unverified.** That the plug-in editor, ETS or a device
  accepts the adopted bytes has not been tested. KNXBench writes no
  `.knxproj` ([ADR-0028](../adr/0028-no-knxproj-export.md)).
- **One device per operation.**

Lifted when: a server route and UI with the same preview/confirm contract exist,
and acceptance of adopted bytes by the originating plug-in is verified by the
owner on their own installation.
<!-- END retained catalogue record -->
