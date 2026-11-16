# WP-42 — Device configuration modules

Owner request: 14–15 November 2026 (configurable menus, keys and LEDs for a
device with own firmware; KNXBench configures the structure). Owner decisions
15 November 2026: KNXBench stays modular and simple to extend; module data is
kept separate from projects; a device's own configuration is stored in the
project where sensible; independent codec with differential evidence; bindings
only as explicitly entered object numbers; structured preview first.
Decision: [ADR-0175](../adr/0175-device-configuration-modules-are-declarative-data.md).
Contract: [device configuration modules](../contracts/device-configuration-modules.md).

**Priority:** In progress.

## Requested outcome

Edit, validate and export a device's own configuration (for the first device:
menu tree, key actions, LED policies) in KNXBench, from a declarative module
package instead of device-specific code. No bus, download or device action is
part of this work package.

## Bounded packages and actual status

1. **WP-42.1 — format, engine, CLI.** Crate `knx-devconf` (strict JSON,
   types, rules, exports), `knx devconf check|validate|export`, fictional demo
   module and tests. *In progress.*
2. **WP-42.2 — storage.** Module packages installed with id, version and
   SHA-256 in the product-data layer; per-device configuration document in the
   project (native schema 22) with one undoable command per apply; a missing
   module keeps the stored document unchanged and read-only. *Not started.*
3. **WP-42.3 — editor.** Device panel section: attach a module explicitly,
   structured editor (tree tables as trees, other tables as lists, typed field
   editors, reference pickers), live validation with field focus, export
   download. Object numbers are typed by the user; nothing is suggested from
   project data. *Not started.*

Outside this repository: the first device module and its differential tests
against the firmware's own validator are maintained with that firmware.

## Open decisions

- Pixel-accurate device preview (later, owner Q15).
- Downloading a configuration to a device: needs its own qualification and
  explicit authorisation; not planned here.
