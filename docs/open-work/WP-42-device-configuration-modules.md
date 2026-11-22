# WP-42 — Device configuration modules

Owner request: 14–15 November 2026 (configurable menus, keys and LEDs for a
device with own firmware; KNXBench configures the structure). Owner decisions
15 November 2026: KNXBench stays modular and simple to extend; module data is
kept separate from projects; a device's own configuration is stored in the
project where sensible; independent codec with differential evidence; bindings
only as explicitly entered object numbers; structured preview first.
Decision: [ADR-0175](../adr/0175-device-configuration-modules-are-declarative-data.md).
Contract: [device configuration modules](../contracts/device-configuration-modules.md).

**Status (22 November 2026): WP-42.1–42.3 delivered on main; no open implementation
item.** WP-42.1 `1110bd4ef`; WP-42.2 storage and WP-42.3 editor `df896afe1` with review
fixes `af51c2983`, merged 17 November 2026 as PR !57 (`3a72996b4`, schema 22); the
editor/document corrections of WP-48.4 are PR #131 (`971653dda`). Not delivered and not
planned here: downloading a configuration to a device (needs its own qualification and
authorisation; the former "WP-42.4" label is not a package of this file). The row left
[OPEN_WORK](../OPEN_WORK.md) on 20 November 2026; a stale-base merge re-added it with
the pre-delivery text until WP-48.9 removed it again.

## Requested outcome

Edit, validate and export a device's own configuration (for the first device:
menu tree, key actions, LED policies) in KNXBench, from a declarative module
package instead of device-specific code. No bus, download or device action is
part of this work package.

## Bounded packages and actual status

1. **WP-42.1 — format, engine, CLI.** Crate `knx-devconf` (strict JSON,
   types, rules, exports), `knx devconf check|validate|export`, fictional demo
   module and tests. *Delivered* after independent review and the package gate;
   the first device module is differentially tested against its firmware
   outside this repository.
2. **WP-42.2 — storage.** Module packages installed with id, version and
   SHA-256 in the product-data layer; per-device configuration document in the
   project (native schema 22) with one undoable command per apply; a missing
   module keeps the stored document unchanged and read-only. *Delivered*
   (`df896afe1`, `af51c2983`; PR !57 `3a72996b4`).
3. **WP-42.3 — editor.** Device panel section: attach a module explicitly,
   structured editor (tree tables as trees, other tables as lists, typed field
   editors, reference pickers), live validation with field focus, export
   download. Object numbers are typed by the user; nothing is suggested from
   project data. *Delivered* with 42.2 (server routes
   `apps/knx-server/src/devconf_routes.rs`, web `DeviceConfigPanel`); load/refresh/
   strict-admission corrections in WP-48.4. Module/rule validation is a separate level
   the project edit routes do not run (contract).

Outside this repository: the first device module and its differential tests
against the firmware's own validator are maintained with that firmware.

## Open decisions

- **Owner/contract question (WP-48.4):** may a device-configuration document contain the
  JSON number `-0`? The strict parser refuses it (serde_json reads it as a float,
  "integers without fraction or exponent"); the knx-app test
  `negative_zero_is_refused_as_a_non_integer_number` pins the current behaviour and the
  fixture uses `-7`. Pre-existing engine behaviour, unchanged. Also listed in
  [OWN-01](OWN-01-owner-decisions.md).
- Pixel-accurate device preview (later, owner Q15).
- Downloading a configuration to a device: needs its own qualification and
  explicit authorisation; not planned here.

## Follow-up WP-48.4 (F04–F06)

Delivered in PR #131 (merge `971653dda`): the
editor loads and edits the stored document through a bounded single-device read, tracks
snapshot revisions under an explicit dirty-draft policy, and new documents must be strict
JSON at the `knx-app` admission boundary while stored historical bytes stay opaque. See the
contract section "Project-stored documents". Module/rule validation remains a separate
level that the project edit routes do not run.
