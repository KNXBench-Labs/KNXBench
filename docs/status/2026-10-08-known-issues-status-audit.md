# Known issues and implementation status audit — 8 October 2026

## Scope

Source `608a204bf28aa9df83413aa5ccb65241c32478aa`, with the existing local
README/manual/media refresh preserved. This extends the [ideas audit](2026-10-08-ideas-roadmap-audit.md),
not an authorization to fix application behavior, deploy or contact hardware.

Read all **40 original user-facing issue topics**, the complete implementation
status chapter, the engineering log's current entries, the complete limitation
heading/anchor inventory and relevant limitation bodies. Trace contradictions
through actual source/consumers and focused offline tests. The [receipt](../evidence/known-issues-status-audit-2026-10-08.json)
retains the original topic inventory, current chapter inventory and executed
results. This is not a fresh revalidation of all historical protocol, private
corpus, hardware, native-platform or security-advisory evidence.

The technical catalogue has 129 numbered headings but 128 distinct numbers:
the inherited number 130 covers gate-root and application-zoom entries.
Retired/resolved numbers are also preserved as explicit anchors. Neither
heading totals nor the 47 current manual topics are an open-bug count.
No number is reassigned and the formal [owner ledger](LEDGER.md) is unchanged.

## Corrections with source and remaining boundary

| Claim / area | Checked evidence | Current reading |
| --- | --- | --- |
| Empty `DefaultLine` import error | [Mapper resolution](../../crates/knx-etsproj/src/map.rs), [CLI comparison](../../apps/knx-cli/src/main.rs) | Import can produce a project plus an unresolved-reference error; comparison rejects error-bearing ETS inputs. “Fully usable” must not mean ignoring all diagnostics. |
| One-way import and opaque preservation | [Import/export contract](../IMPORT_EXPORT.md), [native migration](../../crates/knx-store/src/migration.rs) | Native persistence is not ETS roundtrip; retained bytes are not universal editable semantics or lossless normalized mapping. |
| Product namespace admission | [Package gate](../../crates/knx-productdb/src/package.rs), ADR-0072/0083 | 10/11/12/13/14/20 and exact 21/23 are admitted at stated parser/storage scope. No full manufacturer/runtime claim. |
| `.vd2` picker and all-legacy refusal | [Shared picker](../../apps/knx-web/src/ProductInstallControl.tsx), [legacy route tests](../../apps/knx-server/tests/http_legacy_install.rs), ADR-0094 | Web picker no longer offers VD2; L1–L3 VD3/VD4 offline import exists. CLI ingest help/refusal copy, VD5 bounds/layout, DPT/download and spacer presentation retain their separate limits. |
| Three uniformly missing manufacturer-resolution features | [Object projection](../../crates/knx-projection/src/lib.rs), [Inspector](../../apps/knx-web/src/Inspector.tsx), [report composition](../COMPATIBILITY.md) | Names depend on matching installed data; a program DPT fallback is displayed beside an empty instance slot. Dedicated ambiguity handling is not the same as the ordinary typed DPT override. |
| Whole-project wizard revision protection | [Expectation DTO](../../apps/knx-server/src/routes.rs), [HTTP tests](../../apps/knx-server/tests/http_device_wizard.rs), [wizard contract](../adr/0093-wizards-are-views-over-existing-commands.md) | Names/addresses are rechecked, not every project/product revision. Creation/placement are one undo step; the new-project seed is initial state. |
| Every attribute has layered provenance | [Core provenance](../../crates/knx-core/src/provenance.rs), [data model](../DATA_MODEL.md) | Modelled layered values have `Override`/`Resolved`; ordinary aggregate attributes are not all layered. |
| Pending lifecycle/Web/history adoption | [Live view](../../apps/knx-web/src/BusActivityLive.tsx), [history view](../../apps/knx-web/src/BusActivityHistory.tsx), [HTTP history tests](../../apps/knx-server/tests/http_activity_history.rs), [history contract](../contracts/COMMISSIONING_ACTIVITY_HISTORY.md) | Consumers and scoped offline contracts exist. Partial coverage is disclosed; no complete journal, universal recovery or idle-bus proof. |
| “KNX Secure absent” means every secure-metadata archive is refused | [Compatibility](../COMPATIBILITY.md), [AES refusal](../KNOWN_LIMITATIONS.md#13-password-protected-projects-zipcrypto-ets4ets5-is-decrypted-aes-ets6-is-still-refused) | Secured bus runtime is absent; retained unsupported metadata and AES archive protection are separate boundaries. |
| DPT format always guessed / complete DPT conformance | [Codec scope](../spec-audits/2026-10-07-dpt-document-audit.md), [CLI](../../apps/knx-cli/src/main.rs) | New Web grammar is explicit; omitted legacy fields use compatibility parsing. Main-family formats are not complete subtype semantics; higher types are not all LTE. |
| Global meaning of CLI exit code 2 / no pipeline diff | [CLI `run_diff`](../../apps/knx-cli/src/main.rs), [diff tests](../../apps/knx-cli/tests/cli_project_diff.rs) | `diff --exit-code`: 0 equal / 1 different / 2 failure. Import report errors use their own contract. Web panels do not have a process exit code. |
| Non-program Languages discarded / interface only EN/DE | [Master parser](../../crates/knx-productdb/src/parse/master.rs), [language badges](../../apps/knx-web/src/languageFallback.tsx), [product language tests](../../apps/knx-server/tests/http_product_language.rs), [bundled packs](../../apps/knx-web/src/bundledLanguagePacks.ts) | Program/catalog/hardware/master ingestion and supported overlays exist, plus playful/importable UI packs. Remaining prose/consumer/project-language boundaries are not blanket data loss. |
| Every old-project reader mutates its file | [Native reader](../../crates/knx-store/src/migration.rs), [read-only tests](../../crates/knx-store/tests/read_only_open.rs), ADR-0090 | Normal migration is in-place/transactional without an automatic independent copy; MCP migrates an in-memory copy and sees saved state only. |
| All AppImages force X11 | [Launcher contract](../APPIMAGE_LAUNCHER.md), [owned policy](../../tools/appimage/display-backend.sh), current and alpha.5-tagged `tauri.conf.json` | Old alpha.4 is separate; current/tagged source has the owned hook. Source inspection is not a new native run of a downloaded image/GPU. |
| Desktop has no network endpoint | [Native wrapper](../../apps/knx-desktop/src-tauri/src/lib.rs) | Embedded HTTP binds 127.0.0.1 without the standalone password guard. No LAN bind is not isolation from other local software. |
| Current upstream GTK/security claims | [Dated dependency reviews](../KNOWN_LIMITATIONS.md#16-tauri-v2-remains-on-gtk3-former-maintenance-advisories-are-resolved) | Repository remains on its pinned GTK3 stack; old advisory/upstream reviews keep their dates, not a new security certification. |
| Everything in the limitations catalogue is inevitable / opposite to verified capability | [Current reading guide](../KNOWN_LIMITATIONS.md#current-reading-guide--8-october-2026) | Actual defects, intentional boundaries and evidence gaps differ. A verified narrow implementation can retain a wider limitation. |

## Remaining boundaries retained or made easier to find

The rest of the original topic inventory was reconciled with its documented
contract, without silently closing it:

- Small project corpus/no authoritative XSD; AES refusal and bounded synthetic
  ZipCrypto evidence; whole-file import; deliberately withdrawn ETS export.
- Vendor plug-ins, repeated/id-less/old-schema modules and limited drag/drop.
- Native CSV without ETS interoperability or group-range editing.
- One verified device/operation scope, refused public address/reset operations,
  unconfirmed restart and bounded backup; no new hardware go.
- Single tunnelling monitor, one-gateway live evidence, one-line occupancy scans,
  source-bound Flow performance measurements and retained-window limits.
- Volatile authentication sessions, shared password/project/undo, no per-user
  role/conflict model; bridge discovery and opt-in Route Back boundaries.
- Server-side Save As versus browser export; one-host native coverage, no native
  signing/updater/package matrix, incomplete screen-reader evidence.
- HTML/EN-DE report with native print/ETS parity/formatting limits; comparison
  without merge/apply/three-way or cross-table search; OS reduced motion wins.
- Monitor style refresh and old DPT documentation defect are already resolved;
  older auth/triage analyses are historical, not current task queues.

Seven existing technical boundaries are now explicit user-facing topics:
legacy offline/password limits; wizard expectation scope; unsafe interruption
of a device write; native alpha.4→alpha.5 data-folder separation; shared
preferences/achievement counters; saved-snapshot MCP/access/privacy; and manual,
non-anonymous support-gap evidence. These are **not seven newly discovered bugs**.

## Implementation-status evidence policy

The chapter keeps **implementation** and **evidence type** separate. A green
marker needs reachable code and named verification at its stated scope, but
may be synthetic/offline. Real-file, live-device and native/assistive-technology
claims need their own evidence. Existing real-data counts are scoped receipts,
not new corpus runs or universal support. Broader live-send/native-dialog rows
are marked bounded rather than using a green marker to borrow passive/browser
proof. Formal owner statuses are not copied or changed.

The engineering `IMPLEMENTATION_STATUS.md` is a newest-first history. Add the
current audit entry and reader guidance; do not turn older results into newly
executed tests, erase failures or rewrite old compatibility milestones.

## Verification and publication

Focused runtime tests cover the source contracts named above; documentation
checks cover local targets, preserved anchors, images and all 33 chapter links.
Final results and source-preservation assertions are in [the receipt](../evidence/known-issues-status-audit-2026-10-08.json).
In-session self-review only, not independent or whole-platform acceptance.

Local edits remain in `docs-refresh-20261008`, uncommitted/unpublished. No
application behavior, owner-ledger row, real-device state or deployment changes.
Prior README drift and its retained sync backups are preserved, not overwritten.
