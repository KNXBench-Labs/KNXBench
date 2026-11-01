# Alpha 19 — pre-release

Alpha 19 covers changes since `v0.1.0-alpha.18`: read-only OpenKNX/manufacturer
distribution inspection, one consented diagnostic-bundle workflow, narrower
regular-programming and restore rules, and Settings in the center workspace.
Better evidence is not broader hardware support. Project schema stays 16; the
product database moves 30 → 31. Keep a copy before upgrading.

## For reporters first

### Issue #30: System-2 download for UP 258D12 / `M-0001_A-9205-04-9270`

[Issue #30](https://github.com/KNXBench-Labs/KNXBench/issues/30) stays open.
The experimental scope remains **UP 258D12 application
`M-0001_A-9205-04-9270` only, software-tested**. The field-observed final
allocation mismatch is unresolved; no successful hardware download is claimed.

A final allocation mismatch keeps expected/readback bytes and differing ranges
in the private journal, including whether those bytes were written. Fresh reopen
retains the comparison; strict verification still fails. The public summary stays
minimized.
The diagnostic bundle carries only an admitted closed System-2 projection and
short report, **never raw System-2 recordings or comparison bytes**, even with
consent (`b28c14dcf`, `8f1f65afb`, integrated in `45036df13`).
[Comparison contract](../contracts/system2-download-result.md#allocation-mismatch-evidence-private).

### Issue #22: Siemens presence detector (`MV-0025`)

[Issue #22](https://github.com/KNXBench-Labs/KNXBench/issues/22) stays open.
**Nothing new since Alpha 18 for issue #22 itself:** no targeted programming fix
or new device admission. The System-2 evidence changes above also help preserve
facts about the shared #30/#22 allocation mismatch; they do not solve it.

### Issue #7: ETS 5.7 project import (schema 20)

[Issue #7](https://github.com/KNXBench-Labs/KNXBench/issues/7) stays open.
**Nothing new since Alpha 18:** no change to the project importer for this issue.
Manufacturer-distribution schema inspection below is a different reader, not a
fix for `.knxproj` import. The authentic ETS 5.7 schema-20 evidence gap remains
[KL-177](../KNOWN_LIMITATIONS.md#catalogue-kl-177).

## OpenKNX and manufacturer distributions

- **Inspect without installing** (`a0466e9ea`, merge `d606b3938`).
  `knx products inspect-distribution <source.xml|source.zip>` reads a manufacturer
  XML or distribution ZIP and prints JSON. It changes no product database, runs
  no scripts and uses no network. Catalogue, hardware and application facts,
  parameter-reference fallback values and the existing Dynamic evaluator's
  subset are reported with exact source coordinates. Duplicate identities,
  dangling or ambiguous references and unknown constructs stay visible.
  `complete` is always `false`; reading a distribution grants no installation,
  initialization, write or programming authority. Native package installation
  keeps its existing master/signature and safe-name checks.
- **Dependencies are declarations, not execution** (`2f0440c04`,
  `f3b895da4`, follow-up `128233100`). Scripts, calculations, validations,
  Button handlers and Assign declarations are inventoried with presence-exact
  attributes and scoped endpoints. Declared Assign cycles are distinguished
  from loops whose ambiguous references prevent a decision. Nothing is run,
  resolved to a function, applied or declared valid.
- **Source candidates, not automatic repairs** (`457295909`, `4c149d5e9`,
  merge `afb28da1f`). Source-candidate rows (introduced with report format 4,
  carried on in format 5) compare TypeNumber values and overrides
  exactly with declared bounds, and Union member bit intervals with declared
  extents. Findings get rows; all checked occurrences contribute to totals.
  Missing offsets or widths are not guessed. Overlap between alternatives is
  not itself an error. These are named KNXBench arithmetic models, not schema,
  ETS or device verdicts, and they never change the source.
- **Bounded real-input check.** The recorded format-4 run returned partial
  reports for all 37 public OpenKNX release ZIPs in its selection. The largest
  report was 202 MiB, the slowest took 9 s and peak RSS was about 1.6 GiB, not
  a performance promise. All 37 still refused installation. The NeoPixel 0.3
  inspection-budget regression fixed by `61e730abf` arose after Alpha 18;
  it was not shipped in that release.

[Inspection contract and limits](../contracts/distribution-inspection.md) ·
[CLI guide](../manual/user-guide/10-command-line.md) ·
[KL-187](../limitations/KL-187-distribution-inspection.md) ·
[OKX-07 capability reconciliation](../status/2026-11-01-openknx-capability-reconciliation.md).

- **Baggage finds its owners, PNGs keep their manners** (`ccfe4c610`, merge
  `8fbe945e8`). Report format 5 joins each manufacturer `Baggages/Baggage`
  declaration to its exact consumers (context help, icons, extension baggage)
  in the original namespace and manufacturer scope. Unique PNGs are decoded under
  separate byte, pixel and work budgets and re-encoded without source metadata;
  original and derived hashes are both reported. Help and other ZIP payloads stay
  metadata only, never extracted or rendered; animated PNGs and unsafe paths are
  named refusals. `--output-format html` is an explicit opt-in offline view on
  stdout: escaped text and derived PNG images only, no scripts, links or
  downloads. JSON stays the default. Native package baggage installation is not
  performed by inspection.
  [Resources and previews](../contracts/distribution-inspection.md#distribution-resources-and-passive-png-presentation).

## Diagnostics and support bundle

**Report problem…** in **File → Help & diagnostics** and the command palette
opens one wizard; contextual shortcuts select the device and exact run. Existing
live-read actions still require their separate read consent. Exporting a report
does not start another device read (`72a2eeddf`, `5cc4caf66`,
`7c71ffa2f`, later admission in `45036df13`).

- **Operation recordings.** Supported server operations that open a tunnel
  record bounded JSONL with build identity, timing, datagrams and tunnel events
  (`5cc4caf66`). Key octets in authorize/key-write services are zeroed before
  the recorder sees them. Recorder faults become gaps, not failures of the device
  operation. The free-running bus monitor is not recorded wholesale. Finished
  recordings are limited to 100 / 512 MiB, with successful runs evicted first;
  **Delete recordings** keeps active files.
- **Review binds the bytes** (`e5185f17e`, `80c0fdbfb`). Choose the problem type, sections and
  operation, inspect content and sizes, then explicitly consent. Text and ZIP
  export bind the reviewed content and server/project/revision; changed context
  or contents refuse export and require another review. A missing or expired
  retained result is reported, never replaced with an unrelated latest run.
- **A bundle is private, not anonymous** (`45036df13`, `1cc638f9e`).
  Non-System-2 recordings and declared-memory diagnostic results can leave after
  consent, with the warning **possibly passwords; keep private**. Recognized
  wire keys are redacted, but arbitrary configuration/memory is not classified
  credential-free. System-2 is different: only its admitted closed projection
  and short report leave; raw recordings and expected/readback arrays never do.
- **Project and import evidence have their own admission.** The optional whole
  project is **off by default** and copies the last saved native project, not
  unsaved edits. The shared sanitizers remove recognized credential fields and
  disclose withheld data; a recognized credential witness refuses that source
  instance. Device/catalogue extracts from a retained result need proven matching
  current-project attribution; a manual address is not guessed into a project.
- **Availability is explicit** (`c37437183`, `9ba431da2`). Specific,
  content-independent unavailability reasons and fixed notes are translated in
  English and German; content-sensitive refusals stay generic. At this source
  boundary, session log, bus monitor and client state remain withheld, not empty
  successful exports.
- **Save first, mail second.** After **Download requested**, check that the ZIP
  exists and acknowledge the saved file before opening the mail draft. It has no
  attachment; attach the ZIP yourself. There is no upload service, email is not
  encryption, and public GitHub issues get only minimized reports, never this ZIP.
- **CLI opt-in** (`bd67dc88f`, `236a8e9f2`).
  `--diagnostic-bundle <new.zip>` on device/bus commands is consent for that
  invocation. No flag means no recording. An existing output, duplicate flag,
  missing filename or use outside device/bus refuses before device contact.
  Non-System-2 recordings are included as recorded with capture-time key
  redaction; `system2-*` commands never contribute raw recordings.

DIAG03-CLOSEOUT-PENDING

Recorded acceptance uses synthetic producers, simulator/loopback, a real
server/production-web join and installed-CLI flag checks, not physical KNX hardware.
Keep private journals, memory dumps and full bundles out of public issues.
[Contract](../contracts/diagnostic-bundle.md) ·
[Guide](../manual/user-guide/08-reports-and-diff.md#report-problem) ·
[ADR-0156](../adr/0156-operation-recorder-and-diagnostic-bundle.md) ·
[Work and remaining limits](../open-work/DIAG-03-unified-diagnostic-bundle.md).

## Programming (PRG-01)

- **Reported run state after a regular download** (`2a353f43a`,
  merge `161010078`). On exact masks `0701h` / `0705h`, the existing read-only
  post-download check can observe PID 6 of Applicationprogram object 3 only after
  fresh type and exact plan-application binding and successful prior load reads.
  It reports the property's state, not whole-device health or installation
  function. Unsupported layouts, unavailable reads and undefined values remain
  explicit; transfer, load state, run state and functional verification stay
  separate. No write, authorization, restart or memory fallback is added.
  [Contract](../contracts/post-download-run-state.md) ·
  [Software acceptance](../status/2026-10-29-property-run-state-observation.md).
- **Exact `0700h` qualifies its optional property first** (`5f075cf99`,
  merge `0af68438f`). The same object binding is followed by a PID 6 property
  description read. Only qualified metadata permits the value read; one byte of
  value alone is insufficient. Unqualified metadata does not prove a missing
  property, authorization failure or faulty hardware. `0701h` / `0705h` keep
  their schedules. [Contract](../contracts/post-download-run-state-0700.md) ·
  [Software acceptance](../status/2026-10-30-descriptor-qualified-0700-run-state.md).
- **Customer-adjustable declarations block regular writes** (`d6918027b`,
  merge `e990fd956`). In the documented source contexts, the presence of
  `CustomerAdjustable` or root `CustomerAdjustableParameters` refuses preparation
  with `customer-adjustable-parameters`, even for empty, `false` or unknown
  values. Complete and concrete Partial/Changes scopes and restore inherit the
  refusal; status changes are no bypass. Read-only comparison and diagnostics
  keep their paths. This conservative rule does not implement ETS customer rights
  or calibration-preserving writes; excluded syntax is not proof of safety.
  [Contract](../contracts/customer-adjustable-write-admission.md) ·
  [Software acceptance](../status/2026-10-30-customer-adjustable-write-admission.md).
- **Byte-precise inactive selection** (`b84db2ad3`, `8d8277702`,
  merge `2bec7dd8a`). The regular Easy3/070n memory route omits only whole bytes
  positively proved exclusive to inactive supported branches, with matching
  current/source-default selection topology. Active Union data, necessary
  literals and source-protected bytes keep their rules. An unproved byte is
  written as before; a program-wide proof failure omits nothing, rather than
  refusing the download. This is not “skip everything hidden”. A changed scoped
  plan does not inherit historical hardware verification merely because the
  application matches. Omitting direct writes does **not** prove survival through
  unload, allocation, task controls or restart.
  [Contract](../contracts/inactive-byte-write-selection.md) ·
  [Focused proof and limits](../status/2026-10-30-inactive-byte-write-selection.md).
- **Restore must match the captured exemplar** (`d12a79f16`,
  merge `0ddfe4953`). Regular backed-up downloads observe Device Object PID 11
  before mutation. Restore requires bindable captured serial evidence and equal
  live answers before any write, including its own backup observation. Same
  product/application/address is no substitute. Missing, all-zero or all-FF serial
  evidence cannot restore; a normal download may still record it as unavailable.
  Existing application, procedure, shape and load-state checks remain. Matching
  serial observations are a necessary guard, not absolute physical identity or
  whole-device recovery. **Old format-1/2 backups no longer restore, including
  partial backups.** [Contract](../contracts/regular-restore-exemplar-binding.md) ·
  [Software acceptance](../status/2026-10-29-regular-restore-exemplar.md).

## Workbench and settings

- **Settings is a page, not a window** (`a6bcc0032`, merge `fd1dfa0e4`).
  Toolbar, sidebar and palette open one continuous center page with section
  jumps, even without a project. Properties is temporarily hidden without
  changing its preference. Back preserves selection and unapplied project-note /
  device-instructions drafts. Retained comparison dialogs release their hidden
  foreground isolation and resume their state and focus on return. Existing
  immediate-apply/save acknowledgements remain; there is no global Save button
  or Escape-to-dismiss rule. Global stale-programming-mark warnings and their
  reload retry remain usable in Settings (`92007aaf9`).
  [Contract](../contracts/settings-workspace.md) ·
  [Delivery and browser-only boundaries](../status/2026-10-30-settings-workspace-delivery.md).
- **Passive Button labels** (`2a4e43db9`, merge `6c731be2d`). Activated,
  source-bound program-owned grid Buttons can show literal text in their declared
  cells, marked **Source button label – action unsupported**. They are plain
  text: no click, focus stop, callback or online transfer. Button-only blocks can
  appear as areas. This does not implement manufacturer Button actions.
  [Contract](../contracts/alpha16-parameter-layout.md#passive-source-button-labels-button-literal-passive-v1).
- **Checked IP-address literals** (`e7a175ae4`, merge `484e946a8`). An exact
  source-checked `HostAddress` form offers a text draft for dotted IPv4 without
  leading zeroes or full eight-group IPv6. Explicit **Apply value** preserves
  spelling and hex case; opening, blur and Enter write nothing. No lookup,
  normalization, automatic repair or memory-image codec is added. Other source
  forms retain the plain-field fallback.
  [Guide and limits](../manual/user-guide/05-devices-and-products.md#checked-editor-since-alpha-10).

## Desktop and web

- **A support live read no longer disables activity history** (`37b5f164f`,
  merge `ff78e58bf`). Alpha 12 through Alpha 18 rejected the `supportDiagnostics`
  activity kind, latching history unavailable and blocking later write admission
  until restart although the read itself succeeded. Storage and web readers now
  admit that read-only kind and show its EN/DE label; no history migration is
  needed. [Regression record](../status/2026-10-29-support-diagnostics-activity-kind.md).
- **Diagnostic wizard controls stay above toasts** (`e8943dc37`). Notifications
  no longer cover Next/Close. Toasts and autosave countdowns remain dimmed behind
  the wizard and their timers keep running; errors remain in the notification log.

## Security updates

- **Dependency alerts closed** (`df503e2a4`, merge `be12e246e`). `zip` 8.6 → 9.2.1
  and `rustls` 0.23.45 → 0.23.46 address the open Dependabot advisories;
  `rustls-webpki` 0.103.16 and `tokio` 1.54.1 follow RustSec advisories. With
  `zip` 9 a member name that does not decode is now a visible import refusal
  instead of a trusted string. Unflagged UTF-8 names and their CP437 twins still
  collide as before; no real package identity in the private corpus changed.

## Changed defaults

- Server operation recording is on by default in the data directory's
  `diagnostics/` folder. Operators can disable it at startup with
  `KNX_DIAGNOSTIC_RECORDING=off`; existing recordings remain accessible.
- Settings uses the center workspace instead of the former modal window.
- Regular write preparation has the customer-adjustable refusal and byte-precise
  inactive selection above; failed selection proof keeps the regular write plan.
- New regular backups use format 4 and name the actual write-selection generator.
  Restore has no exemption for old backups without captured exemplar evidence.
- No port, data-directory or Docker tag rule changes; `latest` follows published
  releases, including alphas.

## Upgrade

**Native/store schema 16 and core model 16 stay unchanged: no `.knxdb` migration.**
The separate **product database moves 30 → 31** (`f4761e69a`, merge `5eaadab0b`).
The parser fixes UTF-8 BOM coordinates in `knx_master.xml`; a data-only migration
shifts affected master declaration ranges and their install diagnostics by three
bytes only when authenticated source reparsing matches the other facts. No DDL,
product/parameter rewrite or new winner policy is added. Damaged, diverging or
already-correct source records are left untouched, not guessed into shape.
[Migration contract](../contracts/catalogue-jobs/schema-admission.md).

**Before upgrading, close KNXBench and keep independent copies of the product
database and the complete server/desktop/Docker data directory**, native projects,
private operation journals and device backups, plus original import files.
Writable product-database opens migrate in place, including normal server startup;
read-only readers migrate an in-memory copy. **Alpha 18 refuses v31 with
`FutureVersion` and the server starts without products.** Going back means
restoring the closed pre-upgrade catalogue/data copy, not just the older binary.
[Catalogue rollback warning](../manual/known-issues.md#product-database-31--source-newer-than-alpha-18).

Other downgrade boundaries are separate from project schema 16:

- **Regular backups:** Alpha 18's format-2 reader rejects formats 3/4. Format 3
  adds `exemplar_serial`; current format 4 also requires `write_selection`.
  Alpha 19 reads formats 1–3 without rewriting them, but formats 1/2 cannot
  restore, and any restore still needs bindable exemplar/application evidence.
- **Post-download records:** new `runState`, known runtime values and the exact
  `0700` marker/description vocabulary extend existing report/history shapes
  without a format bump. Alpha 18's strict readers can reject them. No history
  rewrite or seamless downgrade is promised.
- **Activity history:** pages containing the new `supportDiagnostics` kind are
  refused by older readers, despite the storage/wire format staying 2.
- **System-2 journals:** a populated private allocation-comparison record uses
  checkpoint envelope 4. Alpha 18 admits only envelopes 1–3 and refuses it rather
  than dropping the evidence. Old journals reopen without fabricated comparisons;
  the minimized public report/status formats stay unchanged.

Keep old backups/data copies unchanged; in-file history is not an independent
backup or a downgrade guarantee. Update `knx-mcp` together with the app.

## Boundaries

- **Not verified:** new programming, restore and diagnostic behavior has no new
  physical-device or installation-function acceptance. Simulator, loopback,
  intercepted-browser and source-review evidence is not hardware verification.
  Returning an upgraded data directory to Alpha 18 is not a tested rollback;
  restore the pre-upgrade copy instead.
- PRG-01 / KL-007 remain open: other identity/object layouts, memory run-state
  encoding, wider Union/default/customer-write semantics, inactive-byte storage
  survival and whole-mutation recovery are not delivered by these increments.
- System-2 stays experimental and application-limited; #30/#22 remain unresolved.
  More diagnostic evidence is not an invitation to repeat an unapproved bus write.
- Distribution inspection is read-only and partial. No XSD/authenticity check,
  translation, registration, installation or programming authority follows.
- Settings evidence is mocked Chromium, not native WebKitGTK/Tauri acceptance;
  broader WP-11 desktop acceptance remains open.
- KNXBench is KNX-compatible. No KNX Secure, `.knxproj` export or broader
  device-family admission is added.

## Distribution

Release channels retain the Alpha 18 layout: GitHub pre-release
`v0.1.0-alpha.19`, Linux amd64 AppImage
`KNXBench_0.1.0-alpha.19_amd64.AppImage`, `knx-mcp-x86_64-linux`, and
`SHA256SUMS` covering both binaries. Verify a single download with
`sha256sum -c --ignore-missing SHA256SUMS`. The versioned Docker image is
`knxbench/knxbench-server:0.1.0-alpha.19` (without `v`), for `linux/amd64` and
`linux/arm64`; `latest` follows published releases, including alphas.
[Installation](../manual/getting-started/04-installation.md) ·
[Docker update](../manual/user-guide/11-web-and-docker.md#updating-a-docker-hub-installation) ·
[Known issues](../manual/known-issues.md).

## By the numbers
RELEASE-GATE-PENDING

## Acceptance and public verification
RELEASE-GATE-PENDING
