# Plug-in data adoption (data only)

Date: 2026-10-17. Agent: Claude. Owner choice: "Datenübernahme bauen" — take the
edited SmartSensor plug-in bytes (EG blob + `.rbg`) from an import into an
existing KNXBench project, data only, no plug-in code (KL-055 / ADR-0025).

## What was built

- `knx_app::plugin_data` (plan / apply / candidates / confirmation token), CLI
  `knx adopt-plugin-data`, `knx_testsupport::PluginFixture` (synthetic archive
  with a plug-in device). ADR-0136, contract `docs/contracts/plugin-data-adoption.md`, KL-186.
- Design facts worth keeping: the opaque store is outside the command undo stack
  (every undo state is bound to one retained opaque context), so a byte
  replacement cannot be a plain `Command`; rollback is the version that
  `save_editor(..., save_root = true)` records as "Previous saved state".
  Selective import only ever adds opaque entries; this replaces bytes of
  existing ones and keeps the superseded bytes plus a digest-only record.
- The ExtraData association (parameter value = file number) is *observed* in one
  real project, not documented ETS behaviour; both sides must agree on it.

## Evidence

- 17 application tests (mutation controls: removing the group-link guard, the
  stale check, the retention or the import-prefix lookup each fails a named test), 4 CLI tests on the
  real binary.
- Private ETS6 export pair (ignored test): exactly one device adoptable, exactly
  its two entries replaced by the edited archive's bytes; same file as both
  sides → the test fails as it should. Real CLI run on that pair: listing, preview
  (88330→88336 and 88245→88251 bytes), confirmed apply, listing afterwards shows
  every device "nothing to adopt".

## Not done

No server route or UI (KL-186). No claim that the plug-in editor, ETS or a device
accepts adopted bytes. The offline host and any plug-in execution stay parked.
