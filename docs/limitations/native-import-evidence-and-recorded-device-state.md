# Native import evidence and recorded device state

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-native-import-evidence-and-recorded-device-state)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Ported native import evidence (2026-10-12)

The five boundaries dated 2026-10-11 further down (recorded device state, modeled
project information, native-export privacy, retained-payload access, durable
import report) now describe code on main; their "local" and "integration remains
separate" remarks are historical ([evidence](../evidence/native-evidence-port-2026-10-12.json)). Still true:

- Stored BusAccess/IP configuration is not shown. The device view covers recorded
  load state only (project belief and reports held in the model), never a bus query.
- Reports absent from older native files are not reconstructed; selective-import
  reports stay source-scoped and do not replace the main project's counts.
- `/api/project/download` without a binding stays available to existing direct
  clients; only the web dialog sends the reviewed snapshot binding.
- Real-server browser evidence uses two fictional schema-23 projects in Chromium. No
  WebKitGTK window, private corpus run, hardware, release or deployment is claimed.
<!-- END retained catalogue record -->
