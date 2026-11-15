# Alpha 22 — pre-release

Alpha 22 brings a more orderly workbench: consistent spacing and field widths,
an optional technical light/dark appearance, a direct Settings entry and clearer
parameter navigation. Product captions get better language support, more server
waits move away from request-processing workers, and experimental TCP reaches
more explicitly selected device operations. The controls now line up; the bus
still requires permission.

**Before upgrading, close KNXBench and back up your projects and complete data
directory.** Project schema 21 and product database 33 are unchanged from Alpha 21;
this update adds no migration between those releases.

## At a glance

| Area | What is new |
|---|---|
| Settings | Its own sidebar entry alongside Diagnostics, also without an open project; the existing settings page stays familiar |
| Workspace | Shared spacing and aligned value fields across Devices, parameters and Properties; optional Technical Light and Technical Dark themes |
| Parameters | Supported display containers have their own navigation level, keeping their child sections, search context and last-download marks together |
| Product language | Literal grid row/column captions and supported installed-product folder captions follow the requested product language |
| Module objects | An unresolved module owner is reported as **Undetermined**, rather than accidentally **Inactive** |
| Keyboard | Notes and instructions return focus to the initiating button after Preview or explicit Cancel, unless you have moved elsewhere |
| Responsiveness | More project, bus, catalogue, diagnostic and System-2 waits leave the server’s asynchronous runtime while a save is working |
| Experimental TCP | More command-line device operations, plus device comparison, diagnostics and service control in the web app; UDP remains the default |
| Reporter evidence | Save/reopen and parameter-navigation feedback confirmed; additional successful Hager push-button reports retained |

## For reporters first

| Issue | Status in Alpha 22 |
|---|---|
| [#36](https://github.com/KNXBench-Labs/KNXBench/issues/36) Editor after saving | **Reporter confirmed save/reopen works; closed 14 November 2026.** The principal save/editor changes shipped in Alpha 21. Alpha 22 extends the off-runtime handling to more waits; it does not make every request nonblocking. |
| [#48](https://github.com/KNXBench-Labs/KNXBench/issues/48) Finding changed parameters | **Reporter confirmed; closed 14 November 2026.** Last-download section marks shipped in Alpha 21, not as a new Alpha 22 fix. The new display-container navigation retains descendant marks. |
| [#38](https://github.com/KNXBench-Labs/KNXBench/issues/38) Hager push-button | **Closed 13 November 2026.** Additional reporter runs cover complete programming and parameters plus group addresses for the reported application. Alpha 22 updates the retained readiness evidence; this is not a new programming algorithm or a claim for every Hager device. Group-address-only programming remains untested. |
| [#30](https://github.com/KNXBench-Labs/KNXBench/issues/30), [#22](https://github.com/KNXBench-Labs/KNXBench/issues/22) System-2 | **#30 is closed; #22 remains open.** Neither status establishes general System-2 compatibility. The experimental, application-specific admission and recovery limits still apply. |

[Reporter confirmations and their build context](../status/2026-11-14-issues36-48-reporter-closure.md) ·
[Device readiness evidence](../open-work/WP-33-hager-0701-download-evidence.md)

## A more consistent workspace

- **Settings** is a first-class sidebar action next to Diagnostics. No category
  needs unfolding, and no project needs opening. The toolbar gear and command
  palette remain available; this is a better entrance, not a second settings system.
- Devices, parameter fields and Properties use shared spacing and aligned value
  slots. Numbers and ordinary choices share a width; longer names and descriptions
  have a wider, bounded field. Compact and Comfortable remain distinct choices.
- **Technical Light** and **Technical Dark** add restrained blue-grey palettes and
  small corners on the same components. They are optional built-in themes, not
  imported code or a replacement for the other appearances. System theme selection
  still follows the operating system’s light/dark preference.

[Settings contract](../contracts/settings-workspace.md) ·
[Appearance guide](../manual/user-guide/09-settings-and-appearance.md) ·
[Workspace checks and limits](../status/2026-11-14-wp41-workspace-density.md)

## Parameters and product captions

Supported evaluated display containers now get their own navigation level above
their child parameter sections. Folding a container changes presentation only:
it does not deactivate parameters or alter the product’s source order. Search
paths identify the container, and its last-download marks include changed children.
This is a bounded container-to-section relationship, not arbitrary nesting.

Literal product-owned row and column captions in supported parameter grids use
the requested product language. Supported installed-product folder captions also
use that language, including the newer project-source namespace. Saved project
text, including explicitly blank text, still wins. Missing translations use the
documented literal fallback; a resource refusal withholds the optional folder
caption overlay rather than inventing translated content.

An imported module object whose own instance cannot be resolved is now
**Undetermined** after evaluation, with no invented channel. This fixes the
read-only activation overlay; it does not unlock module programming or infer
missing module identities.

[Parameter navigation and exclusions](../status/2026-11-14-parameter-navigation.md) ·
[Grid-caption contract](../contracts/alpha16-parameter-layout.md#literal-program-owned-axis-translations) ·
[Folder language and fallback rules](../status/2026-11-14-wp06-folder-caption-language.md) ·
[Module-owner boundary](../status/2026-11-14-wp07-unresolved-module-owner.md)

## Keyboard focus that comes back

In Notes and Instructions, a settled Preview returns focus to the exact button
that started it. Explicit Cancel returns there too. Moving focus elsewhere while
waiting withdraws automatic restoration; a completed request does not get to
interrupt your next task. Confirm and Apply are never activated automatically.
Preview and Cancel still do not apply an edit or write to a device.

The native observations remain bound to their recorded builds and execution
epochs. They do not establish acceptance of every desktop window in Alpha 22.

[Focus ownership contract](../contracts/editor-preview-focus.md) ·
[Bounded software and native evidence](../status/2026-11-14-wp11-editor-focus-candidate.md)

## Saving without occupying the request workers

Alpha 21 separated ordinary editor reads and edits from the save queue. Alpha 22
moves more project-lock and product-database waits away from the server’s
asynchronous runtime: other project edits and reads, bus-monitor start and scan
comparison, device comparison and diagnostics, support reads, serial-address
operations, catalogue reads and updates, and the System-2 worker’s final binding
wait. Reservations, validation and bus-operation safety rules stay in place.

This does **not** mean every operation runs in parallel. Commands that write the
project file themselves can still wait for a save, and the route audit is not
exhaustive. Keep saving and backing up; responsiveness is not crash protection.

[Save-queue scope and remaining waits](../open-work/WP-31-device-editor-unavailable-after-save.md)

## More explicit TCP choices — experimental

| Where | Alpha 22 adds explicit TCP selection for |
|---|---|
| Command line | Device diagnostics and read-only comparison; service-control reads and writes; ordinary download and backup restore; System-2 observation, execution and recovery |
| Server and web app | Device comparison, device diagnostics and device service-control reads and writes |

UDP stays the default. TCP must be selected for the operation; it never silently
falls back to UDP. The new web choices are local to the operation, not a command
to switch an existing monitor. Selecting a transport alone contacts nothing.
Older servers or inconsistent connection metadata are refused, not treated as a
successful TCP operation.

Existing write confirmations, access checks, recovery rules and target binding
still apply. A successful TCP socket write is not proof that a physical KNX device
executed it. Skipped stream frames are reported separately; they are not a count
of lost KNX telegrams.

**Experimental: simulator and loopback evidence only. No real-gateway TCP
interoperability was verified.** Plain TCP is not KNX Secure, and this extension
does not enable every server/web download or restore workflow over TCP.

[TCP contract and operation records](../contracts/tcp-tunnelling.md) ·
[Server/web comparison](../status/2026-11-14-server-web-tcp-comparison.md) ·
[Server/web diagnostics](../status/2026-11-14-server-web-tcp-diagnostics.md) ·
[Server/web service control](../status/2026-11-15-server-web-tcp-service-control.md)

## Upgrade

| What | Alpha 21 | Alpha 22 | What it means |
|---|---|---|---|
| Project file (`.knxdb`) | schema 21 | schema 21 | No project migration for this update. This does not establish a general downgrade guarantee. |
| Product database | version 33 | version 33 | No product-database migration for this update. Earlier Alpha 20 still refuses this version. |
| Appearance preference | Existing built-in themes and theme files | Adds Technical Light and Technical Dark | The choice uses the existing shared settings. Alpha 21 does not know these built-in themes. |
| Complete retained data | Keep independent backups | Keep independent backups | Project files alone do not include the whole product database, device backups or operation history. |

Close the app/server before copying the complete data directory or Docker volume,
projects, device backups and original import files. Update `knx-mcp` together with
the app. Ports, data-directory defaults and Docker channel policy are unchanged.

Alpha 21’s source treats an unavailable saved theme as **System**, displays an
unavailable-selection diagnostic and leaves the stored choice unchanged until
you choose another theme. That source-established fallback is not an executed
Alpha 22-to-Alpha 21 downgrade test. Selecting an older built-in theme before
returning avoids that particular unavailable choice; keep backups either way.
[Theme persistence and fallback](../manual/user-guide/09-settings-and-appearance.md#theme)

Upgrading from Alpha 20 or earlier still crosses the migrations introduced by
Alpha 21. Restore closed pre-upgrade copies to return to those older releases;
reinstalling the executable alone does not undo them.
[Earlier upgrade boundary](alpha21.md#upgrade)

## Boundaries

- **No new hardware verification:** this release’s software work does not verify
  real TCP gateways, USB/serial interfaces or physical programming. The Hager
  reports are retained third-party, application-specific evidence, not a new
  release hardware campaign.
- Native focus observations retain their original source/build limits; there is
  no blanket native desktop-window acceptance claim for Alpha 22.
- System-2 remains experimental and application-specific. Issue #30 is closed;
  #22 remains open. KNX Secure and `.knxproj` export are unsupported.
- Remaining nested, repeated and module parameter layouts and Button actions
  are not made compatible by new navigation or translated captions.
- Last-download marks describe retained software evidence, not live device
  readback. Untested programming still requires explicit acceptance.
- The AppImage needs glibc 2.39 or newer; that is not a guarantee for every Linux
  host or graphics driver. ARM runtime execution, FUSE launch and general
  downgrade compatibility are not established here.

## Distribution

| Download | Name |
|---|---|
| GitHub pre-release | `v0.1.0-alpha.22` |
| Linux AppImage (amd64) | `KNXBench_0.1.0-alpha.22_amd64.AppImage` |
| MCP server | `knx-mcp-x86_64-linux` |
| Checksums | `SHA256SUMS`; run `sha256sum -c --ignore-missing SHA256SUMS` |
| Docker (amd64 and arm64) | `knxbench/knxbench-server:0.1.0-alpha.22`; `latest` follows releases |

[Installation](../manual/getting-started/04-installation.md) ·
[Docker updates](../manual/user-guide/11-web-and-docker.md#updating-a-docker-hub-installation) ·
[Known issues](../manual/known-issues.md)

## By the numbers

| | Alpha 22 |
|---|---|
| Commits since Alpha 21 | **97** (**75** without merges) |
| Files changed | **250**, +21,750 / −2,379 lines |
| Estimated active development time | RELEASE-GATE-PENDING |
| Rust tests | RELEASE-GATE-PENDING |
| Frontend unit tests | RELEASE-GATE-PENDING |
| Browser test executions and their scope | RELEASE-GATE-PENDING |
| Corpus and predecessor comparison | RELEASE-GATE-PENDING |
| Release gate and packaging | RELEASE-GATE-PENDING |

## Acceptance and public verification

RELEASE-GATE-PENDING
