# WP-42 — Device configuration modules

Owner request: 14–15 November 2026 (configurable menus, keys and LEDs for a
device with own firmware; KNXBench configures the structure). Owner decisions
15 November 2026: KNXBench stays modular and simple to extend; module data is
kept separate from projects; a device's own configuration is stored in the
project where sensible; independent codec with differential evidence; bindings
only as explicitly entered object numbers; structured preview first.
Decision: [ADR-0175](../adr/0175-device-configuration-modules-are-declarative-data.md).
Contract: [device configuration modules](../contracts/device-configuration-modules.md).

**Priority:** WP-42.1, WP-42.2 and WP-42.3 delivered 17 Nov 2026 (branch work/wp42-storage-20261117, schema 22); WP-42.4 download/bus integration continues as WP-43.

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
   module keeps the stored document unchanged and read-only. *Storage delivered*
   (schema 22 `device_config_profile`, undoable commands; PR 57). `knx devconf export-stored` exports a
   stored document through a project-read-only CLI path.
3. **WP-42.3 — editor.** Device panel section: attach a module explicitly,
   structured editor (tree tables as trees, other tables as lists, typed field
   editors, reference pickers), live validation with field focus, export
   download. Object numbers are typed by the user; nothing is suggested from
   project data. *Editor delivered* (editor fixes PR 131); a physical device
   download is not delivered and remains unqualified.

Outside this repository: the first device module and its differential tests
against the firmware's own validator are maintained with that firmware.

## Open decisions

- Pixel-accurate device preview (later, owner Q15).
- Downloading a configuration to a device: needs its own qualification and
  explicit authorisation; not planned here.

## Follow-up WP-48.4 (F04–F06)

Implemented on branch `work/wp48-4-devconf-editor-20261122`, pending review/gate: the
editor loads and edits the stored document through a bounded single-device read, tracks
snapshot revisions under an explicit dirty-draft policy, and new documents must be strict
JSON at the `knx-app` admission boundary while stored historical bytes stay opaque. See the
contract section "Project-stored documents". Module/rule validation remains a separate
level that the project edit routes do not run.
