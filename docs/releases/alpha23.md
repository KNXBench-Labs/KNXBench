# Alpha 23 — pre-release

Alpha 23 is the release where the project file learns to remember things per module,
the Devices view gets a configuration editor, and group addresses can finally be
sent from where you are looking at them instead of being typed from memory. It also
carries a long run of programming safeguards that were tested against simulators
only. The bus is still not a test bench for hopeful guesses.

**Before upgrading, close KNXBench and back up your projects and the complete data
directory.** This update changes the project file (schema 21 → 22) and the product
database (version 33 → 34). Opening a project or the product database with Alpha 23
upgrades it. **Alpha 22 and older refuse the upgraded files.** Restore your backups
if you want to go back; reinstalling an older program does not undo the upgrade.

## At a glance

| Area | What is new |
|---|---|
| Module parameters | Parameter values of module-based devices are remembered per module instance, including their last-download marks |
| Group addresses | Optional marks for addresses whose links differ from the last download (a Settings switch, off by default); a send action opens the bus monitor's send form with the address filled in |
| Bus monitor | Searchable destination address in the send form, sticky details and header, sortable columns, session counters in the status bar |
| Device configuration | A new Configuration tab to edit a device's own configuration document (menus, keys, LEDs) for devices that have a module package; stored in the project |
| Programming | Complete programming can be prepared when nothing is pending; a CLI download report can update the server's pending marks when it matches exactly; more run-state and refusal checks (simulator only) |
| Parameter layout | More of the manufacturer's parameter layout is understood: captions from text parameters, module-aware renames and Buttons, nested and inline sections |
| Online catalogue | Optional manual Reko catalogue source beside local files |
| Workspace | Compact dropdowns that reveal long text on hover, calmer wording, configurable table columns |
| Correctness | A review of the new work caught and fixed a set of problems before release (see below; none of them was in Alpha 22) |
| Reporter evidence | A further successful push-button report retained; one more report registered for a presence detector |

## For reporters first

| Issue | Status in Alpha 23 |
|---|---|
| [#52](https://github.com/KNXBench-Labs/KNXBench/issues/52) “Programmieren vorbereiten” does nothing without pending changes | **Still open, waiting for your confirmation.** When no changes are pending, the dialog now says so and offers to prepare a complete programming explicitly. We could not prove from the source that this was the original cause, so we have not closed the issue ourselves. |
| [#51](https://github.com/KNXBench-Labs/KNXBench/issues/51) Searchable destination and send from the group-address workspace | **Closed.** The destination in the bus monitor’s send form is searchable by address, name and data type; free typing still works for addresses outside the project. The group-address workspace offers a send action that opens the form pre-filled. Selecting never sends. |
| [#50](https://github.com/KNXBench-Labs/KNXBench/issues/50) CLI download does not update the server’s pending marks | **Closed.** The server can now check a CLI download report against the open project and clear only the marks it covers. A stale, mismatched or incomplete report clears nothing. There is no button for this in the web app yet. See [the decision record](../adr/0179-external-cli-download-reconciliation.md). |
| [#53](https://github.com/KNXBench-Labs/KNXBench/issues/53) Hager STCC510S download report | **Closed.** All four download scopes for this presence detector are now recorded as verified reporter evidence. Restart confirmation stays an explicit limitation; two differing octets after the runs remain reporter observations with unknown cause. |
| [#49](https://github.com/KNXBench-Labs/KNXBench/issues/49) Device download report (group addresses only) | **Open.** The attached report was checked offline against the application data and the plan reproduced exactly (24 steps, 1020 data octets); the group-addresses-only scope is now recorded as verified evidence for that application. The unconfirmed restart remains a limitation. Nothing else changed for this issue since Alpha 22. |

Issues [#36](https://github.com/KNXBench-Labs/KNXBench/issues/36) and [#48](https://github.com/KNXBench-Labs/KNXBench/issues/48) were closed before Alpha 22 and are not new here. Nothing new since Alpha 22 for [#38](https://github.com/KNXBench-Labs/KNXBench/issues/38), [#30](https://github.com/KNXBench-Labs/KNXBench/issues/30) and [#22](https://github.com/KNXBench-Labs/KNXBench/issues/22).

[Issue batch #49–#52 and what was verified](../open-work/WP-43-programming-and-group-address-feedback.md) ·
[#52 bounded record](../status/2026-11-16-issue52-complete-preparation.md) ·
[Stored reporter evidence for #53](../evidence/tester-report-hager-stcc510s-2026-11-16.json)

## Module parameters and last-download marks

Devices built from modules (the same module used several times in one application)
used to share one pool of remembered values. Now each module instance keeps its own,
so “changed since the last download” can be shown for the right instance. The way a
repeated instance is identified follows the accepted
[module-instance decision record](../adr/0189-accepted-wp07-mapping-is-the-module-instance-authority.md)
(the owner accepted the counted comparison against exported projects it rests on,
[evidence](../research/2026-11-17-wp07-module-mapping-witness.md); the record itself is
still marked as a proposal).

- Module and ownership lookups are now exact. A module instance is matched by its
  full identity, not by whether one name happens to appear inside another; unclear
  or conflicting owners are refused instead of guessed.
- In the group-address views, an optional marker shows addresses whose links differ
  from the last download. It is a Settings switch and **off by default**; with it on,
  the app reads every device’s comparison (one request per device). The summary
  line only says “nothing differs” when every relevant device could be compared,
  and otherwise says how many were compared and why others were not.
- Last-download marks describe what KNXBench last wrote, not a live read of the device.

**Known limit:** a device whose module instances have stored, module-qualified
parameter values is refused at download (fail-closed). Showing and editing those
values works; writing them per instance into a download image is not implemented.

[Pending changes across the UI](../open-work/WP-37-pending-changes-across-ui.md) ·
[Module-instance authority](../open-work/WP-07-module-instance-authority.md) ·
[Known limitations](../KNOWN_LIMITATIONS.md)

## Device configuration editor

Devices with an installed module package get a **Configuration** tab. A module is a
declarative data file describing a device’s own configuration (for example menu
tree, key actions and LED policies); KNXBench does not hard-code any manufacturer’s
device. The document is stored in the project, each apply is one undoable step, and
a missing module keeps the stored document unchanged and read-only.

- The editor loads the stored document, so you can edit what you saved last week
  instead of retyping it.
- It follows the project revision: undo and redo refresh it, and your own apply does
  not raise a false “changed elsewhere” warning.
- New documents must be valid JSON. Older stored bytes are kept as they are, not
  silently rewritten.
- This is editing and exporting only. **Nothing is sent to a device**: downloading a
  configuration to hardware is not provided and not qualified.

[Contract](../contracts/device-configuration-modules.md) ·
[Decision](../adr/0175-device-configuration-modules-are-declarative-data.md) ·
[Work package](../open-work/WP-42-device-configuration-modules.md)

## Bus monitor and group addresses

- **Send from where you are.** Pick a group address in the group-address workspace
  and open the bus monitor’s send form with it filled in. In the form, the
  destination is searchable by address, name and data type. Opening the form or
  choosing an address never sends a telegram; sending is a separate explicit step.
- The telegram details pane stays in view while the table scrolls.
- Table headers are sticky and bold, and columns sort freely. Time sorting is exact:
  entries within the same second are ordered by their real time, and ties keep
  their arrival order.
- While connected, the status bar counts received and sent telegrams of the current
  session. A calm connection animation respects the reduced-motion setting.
- Tables in the Devices and catalogue views have configurable columns.

## Programming

All of this is **simulator and loopback evidence only**. It adds checks and clearer
refusals; it is not a new way to write to a device.

- **Prepare complete programming without a pending change** (#52, see above). The
  usual support checks, consent and confirmation still apply, and preparing never
  starts a device operation.
- **CLI reports and server marks** (#50). A report is accepted only if device,
  application data, planned content and the current project all match; edits made
  after the external run stay pending. Reports from before this check cannot clear
  marks by accident.
- **More run-state coverage.** The post-download check can observe the application
  run state for one more exact mask, only after an independent property check
  qualifies it; other masks are still refused rather than guessed.
- **Module-local options** that users may adjust are admitted in one more
  place, within the same limits.
- **Clearer refusals.** Every refusal of a regular download maps to one documented
  reason code, and the web app’s refusal categories are pinned to the server’s. The
  download report summary keeps the reason when a run state stays unknown.
- **Stricter backup reading.** A damaged device backup file with a `+` or a space
  inside a hexadecimal field was read as a number in Alpha 22; it is now refused as
  malformed and never rewritten. Valid backups are unchanged.
- Remaining programming gaps are scoped and parked at an evidence wall, not guessed.

[Programming gaps and increments](../open-work/PRG-01-programming-gaps.md) ·
[Refusal reason codes](../contracts/download-refusal-reason-codes.md) ·
[Run-state observation](../contracts/post-download-run-state-0702.md) ·
[Module-local options](../contracts/customer-adjustable-module-options.md)

## Parameter layout

Imported products show more of the manufacturer’s own layout. Each of these is a
bounded, evidence-based step, not “every product looks like ETS now”:

- Row and column captions can come from a text parameter, and module-scoped or
  repeated sections and Buttons use their own captions and translations.
- Conditional section renames work inside modules and across sibling sections.
- Nested block cells, inline sections and rows that collapse when empty are laid out.
- Long explanations moved behind the (?) help tip; the priority option reads
  “Default”; manufacturer-field hints appear once per section.

Products already in your database keep their earlier stored layout; sections of the
new inline style appear for products imported after this update. Unsupported layout
features are still reported, not silently dropped.

[Parameter layout contract](../contracts/alpha16-parameter-layout.md) ·
[Work package](../open-work/WP-05-parameter-layout-fidelity.md)

## Optional online catalogue (Reko)

You can add the Reko online catalogue beside local files. Sync is manual only, search
is local, downloads are size- and hash-verified against the entry you selected, and
one encrypted account per installation is stored separately from your projects.
Nothing contacts the service until you ask. Local-file installation stays available.
A catalogue entry whose content changed but whose size did not is not shown as
already downloaded; the verified digest decides. Back up the configuration directory
(in Docker, the retained `/config` volume) as well.

[Contract](../contracts/reko-online-catalogue.md) ·
[Decision](../adr/0176-manual-reko-catalogue-acquisition.md)

## Workspace polish

- Dropdowns have a bounded width. If the selected text does not fit, hovering or
  keyboard focus scrolls the text inside the field while the field itself stays
  still. Reduced motion shows an ellipsis and a tooltip instead.
- The redundant toolbar gear is gone (Settings keeps its sidebar entry and the
  command palette); settings fields are capped to a readable width.

## What the correctness review changed

After the feature work, a review of the new code looked for wrong behaviour. Every
finding was in work added **after** Alpha 22, so these are not fixes for a published
defect; they are the new features doing what the notes above say. The ones worth
knowing about: module ownership is exact, an old CLI report cannot clear newer
programming changes, translated captions are not described as stored values, the
configuration editor can reopen its document, time sorting is exact, and the
catalogue cache follows the selected digest. Several test suites were also made
able to fail when the behaviour breaks.

[Audit and per-finding outcome](../open-work/WP-48-recent-change-correctness-audit.md)

## Exporting stored device profiles from the command line

A device configuration that you saved in a project can now be turned into its export
file without opening the app:

```bash
knx devconf export-stored <project.knxdb> --device <id> --module <module.devconf.json> --export <id> --output <file>
```

The command reads one saved project **read-only** and writes the device's stored
configuration export to the file you name.

- The stored profile's module id, version and checksum must match the module you
  name. There is no fallback to another version; a mismatch writes nothing.
- The project is never migrated or written. An older project is upgraded only in a
  memory copy. SQLite may still maintain its usual companion files next to a project
  that is being read.
- An output file that is the project, the module or a database side file of the
  project is refused before anything is read.
- Exit codes: 0 success; 1 invalid document, module mismatch or refused export;
  2 unusable input (for example an unknown device or a device without a stored
  profile).
- Offline file work only: no bus, device or download. Behaviour against a project
  being written at the same moment, or a path swapped mid-run, is untested.

[Contract](../contracts/device-configuration-modules.md) ·
[Command-line chapter](../manual/user-guide/10-command-line.md#device-configuration-modules-and-stored-exports)

## Upgrade

| What | Alpha 22 | Alpha 23 | What it means |
|---|---|---|---|
| Project file (`.knxdb`) | schema 21 | schema 22 | Opening a project upgrades it (adds per-module parameter values and stored device configuration). **Alpha 22 and older refuse the upgraded file.** |
| Product database | version 33 | version 34 | Opening upgrades it. **Alpha 22 and older refuse the upgraded database.** Products already stored keep their earlier layout data. |
| Online catalogue account | none | optional | Only created if you use Reko; kept in the configuration directory, not in projects. |
| Complete retained data | Keep independent backups | Keep independent backups | Project files alone do not include the whole product database, device backups, configuration or operation history. |

Close the app/server before copying the complete data directory or Docker volume,
including projects, device backups and original import files. Update `knx-mcp`
together with the app. Ports and data-directory defaults are unchanged.

To return to Alpha 22, restore closed pre-upgrade copies of the project and the
product database; no downgrade path is provided or tested.

## Boundaries

- **Not verified:** no real-hardware or real-gateway verification was done for this
  release beyond the reporter evidence listed above. Programming work is simulator
  and loopback only; the Reko catalogue was tested with local fixtures, not the live
  provider.
- Reporter reports are third-party, application-specific evidence, not a release
  hardware campaign. KNX-compatible behaviour is claimed only for what is verified.
- Module-qualified stored values do not download yet; no module download to a device
  exists. Writing a device configuration to hardware is not provided.
- Last-download marks describe retained software evidence, not live device readback.
- #49 and #52 remain open; restart after download is still not confirmed in the
  reports.
- KNX Secure and `.knxproj` export remain unsupported. System-2 stays experimental.
- The AppImage needs glibc 2.39 or newer; that is not a guarantee for every Linux
  host or graphics driver. ARM runtime execution, FUSE launch and general downgrade
  compatibility are not established here.

## Distribution

| Download | Name |
|---|---|
| GitHub pre-release | `v0.1.0-alpha.23` |
| Linux AppImage (amd64) | `KNXBench_0.1.0-alpha.23_amd64.AppImage` |
| MCP server | `knx-mcp-x86_64-linux` |
| Checksums | `SHA256SUMS`; run `sha256sum -c --ignore-missing SHA256SUMS` |
| Docker (amd64 and arm64) | `knxbench/knxbench-server:0.1.0-alpha.23`; `latest` follows releases |

[Installation](../manual/getting-started/04-installation.md) ·
[Docker updates](../manual/user-guide/11-web-and-docker.md#updating-a-docker-hub-installation) ·
[Known issues](../manual/known-issues.md)

## By the numbers

| | Alpha 23 |
|---|---|
| Commits since Alpha 22 | **291** (**193** without merges), counted up to the version bump |
| Files changed | **469**, +39,608 / −1,373 lines |
| Estimated active development time | RELEASE-GATE-PENDING |
| Rust tests | RELEASE-GATE-PENDING |
| Frontend unit tests | RELEASE-GATE-PENDING |
| Browser test executions and their scope | RELEASE-GATE-PENDING |
| Corpus and predecessor comparison | RELEASE-GATE-PENDING |
| Release gate and packaging | RELEASE-GATE-PENDING |

## Acceptance and public verification

RELEASE-GATE-PENDING
