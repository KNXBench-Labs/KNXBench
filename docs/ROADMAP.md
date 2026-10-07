# Roadmap

## Post-Alpha Flow usability follow-up (2026-10-07)

Implemented on the user's explicit go: larger/maximized area, closable Inspector,
dedicated source-bound Flow window, stronger readability-first rearrangement,
project links and switchable auto zoom. See [ADR-0085](adr/0085-readable-flow-and-source-windows.md)
and [Flow follow-up](TELEGRAM_FLOW_VISUALIZATION.md#23-readable-area-source-windows-and-project-links-2026-10-07).
Browser/offline acceptance is recorded there; native WebKitGTK/screen-reader
validation and a new packaged release remain separate. This does not move the
existing Alpha tag or authorize house-bus operations.

## Cross-cutting — Telegram-flow nervous system (Alpha addition, 2026-10-04)

Approved and authorized for the responsible Goal sessions, **not implemented**.
The [contract/research](TELEGRAM_FLOW_VISUALIZATION.md) and
[ADR-0077](adr/0077-session-local-telegram-flow-view.md) define a session-local,
read-only dynamic communication map: directional/group-labelled pulses, immediate
per-group values with 7-second expiry, an observed-sender activity center,
freezable layout and resting edges that never silently vanish. Theme/motion
preferences apply; inferred endpoints stay visibly distinct from receipt proof.
Execution order: UI U19 → alpha AR20 → UI U20/U21 → alpha AR21, then finished
AR15–AR18 acceptance; AR19 still needs a separate exact release decision.
No new commissioning work, floor-plan editor, permanent coordinates or traffic
history. Existing historical milestones and owner work are not reopened.
**Done 2026-10-05:** U19–U21 and AR20 delivered, AR21 accepted the feature for
the Alpha on a recorded envelope (Motion Off for large maps; Chromium only);
see [TELEGRAM_FLOW_VISUALIZATION §22](TELEGRAM_FLOW_VISUALIZATION.md#22-ar21-rerun-of-findings-6-and-7-and-acceptance-alpha-2026-10-05).


## Integrated CRT animation follow-up (2026-10-03)

Application-owned CRT feedback is implemented separately from the completed
declarative theme-pack delivery: real tree/address-table fills, bounded light
and manual Save activation. Existing keyboard/selection paths and the v1 palette
contract are retained. See [CRT guide](DESIGN_RETRO_GREEN_CRT.md) and ADR-0022.
Authorized main integration is3d03aea5 (feature16c9d774) with repeated merged-result
gates; current refs/handover establish publication. Native WebKitGTK/Orca evidence,
Save-only semantic color roles and broader component redesign remain separate; this is not a
reopening of the completed theme-pack format milestone or Alpha/ETS acceptance.

The original Session 0–7 delivery sequence is complete at its evidenced
scope: Sessions 0–6 shipped, and Session 7 includes a first, safety-gated
`070nh` download and a historically verified button-address path on one MDT
device (`1.1.67`). Confirmed public K6/serial/K13 address writes currently
fail closed before any tunnel until action- and device-specific durable
pre-write recovery exists (ADRs 0057–0059). This is **not** full ETS,
manufacturer or KNX hardware coverage.
Remaining v1 decisions and UI/manual/release work are tracked in
[`goal.md`](archive/goal.md), [`goal-ui.md`](../goal-ui.md) and
[`goal-commission.md`](../goal-commission.md). Historical milestone detail
remains in [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md).

## Session 0 — Technical research

Done. Sources and format boundaries: [RESEARCH](RESEARCH.md).

## Session 1 — Architecture

Done. Layering, domain contracts, migrations and ADRs are in
[ARCHITECTURE](ARCHITECTURE.md) and [DATA_MODEL](DATA_MODEL.md). The historical
`.knxproj` export contract was withdrawn (ADR-0028).

## Session 2 — KNX core

Done. Typed entities and addresses, validation, command/undo model and
versioned storage shipped. ADR-0039 phases 3–5 still leave a public mutation
invariant to be enforced (KNOWN_LIMITATIONS §129).

## Session 3 — ETS project import

Done at the evidenced schema-11/21/23 import boundaries, with reports,
retained opaque data, tests and native `.knxdb` persistence. `.knxproj`
export is withdrawn (ADR-0028). An independent module-using schema-23
project and projects with schemas 12–14, 20 and 22 are still missing;
this is an evidence limit, not an importer success claim (limitations §1).
See [COMPATIBILITY](COMPATIBILITY.md) and [IMPORT_EXPORT](IMPORT_EXPORT.md).

## Session 4 — Manufacturer databases

Done for the implemented versioned product database and the evidenced
`.knxprod` master-data schemas 11, 12, 13, 14, 20 and exact-namespace 21
(since 2026-10-04/05 also exact 23 and exact 10, ADR-0072/ADR-0083).
PDB-1–PDB-11 landed; source blobs and unknowns are retained/reported, not
claimed to be fully interpreted. Legacy `.vd`/`.pr` uses a separate design;
unknown vendor semantics and encrypted packages stay refused. Evidence:
[PRODUCT_DATABASE_CORPUS](PRODUCT_DATABASE_CORPUS.md),
[COMPATIBILITY](COMPATIBILITY.md), [KNOWN_LIMITATIONS](KNOWN_LIMITATIONS.md).

## Session 5 — UI and UX

Tauri/WebKitGTK shell and one React Web UI over the HTTP API shipped with
project explorer, inspector, search, command palette, save/open, Settings,
structure editing, product catalog, diagnostics and scoped device actions.
U0–U12 UI work is delivered within its tested scope: read-only readiness
and device comparison, Site/Property creation, and a default-off Debug
property action with an offline-tested, property-only backup (ADR-0051).
K6's Web tab shows fail-closed availability rather than a working write.
[`goal-ui.md`](../goal-ui.md) is complete through U13 (2026-10-01).
The operator accepted the independent GPT-6.1-Sol review instead of unavailable
Claude; all three P1 findings are fixed and mutation/regression-tested.
ISSUE-12's two evidence boxes reconcile the host-firewall correction and actual
offline unicast-loopback transport tests, without claiming multicast proof.
Final integrated Rust/Web/Chromium/repository gates passed; see the U13 log
and [IMPLEMENTATION_STATUS](IMPLEMENTATION_STATUS.md).
Native WebKitGTK and screen-reader coverage remain bounded (§20, §130).

## Cross-cutting — Theme packs

U14 resolves the declarative, versioned theme-pack contract and security/
negative-fixture matrix in [THEME_PACKS](THEME_PACKS.md) and
[ADR-0060](adr/0060-versioned-declarative-theme-packs.md). This is contract
research, not shipped import/export. U15 runtime admission, cache revalidation
and reversible theme application are delivered as 9d1ae19d with complete
candidate/combined gates and exact remote readback. U16 delivers acknowledged
storage/file roundtrip as 1f94808d (144 focused passes, restored
controls, complete 16-check candidate and 22-check actual-merged gates), with
exact source publication/artifact readback verified. U17 management/preview
passes actual-merged23-command acceptance on f16f1e40: Web1702, Chromium72,
Rust2984/0/165,27 selected private offline cases plus the115-instance matrix,
17 equal bindings and697 source/config inputs/420 private files unchanged.
Publication/readback is recorded in the current handover. Eleven manager browser
flows and31 actual parent/five root cases cover the management contract.
U18 in [goal-ui](../goal-ui.md) passes actual24-command extension acceptance
on1660911b: Web1702, Chromium82, representative10/10, Rust2995/0/165,28
offline cases including the115-instance matrix;703 source/config inputs and421
private files unchanged. Closing self-review settles U18-R1 with real component
states and detected rendered-style controls, not native/Orca/general WCAG proof.
Publication/readback and owned cleanup are tracked in the current handover.
Existing built-ins/System stay
available; no arbitrary CSS, external assets or new alpha release blocker.

## Cross-cutting — Internationalization

English/German UI chrome and importable language packs shipped (T25).
Product-data translation is available at evidenced surfaces, not universally;
project default language, master-scope translation readers and remaining
server prose are bounded by §14, §37, §64 and §66. Do not call this full
localization. See [LANGUAGE_PACKS](LANGUAGE_PACKS.md).

## Cross-cutting — Motion and animation

T27 shipped user-selectable level and smooth/glitch styles. The OS
`prefers-reduced-motion` setting wins, and a style guard covers the
`styles.css` transitions it can measure. Per-category controls and other
CSS surfaces are not claimed (KNOWN_LIMITATIONS §43).

## Cross-cutting — Web/Docker deployment target

One frontend/API serves desktop and Docker. The server now authenticates
LAN clients with one password or refuses non-loopback without one
(ADR-0026); it is **not** an unauthenticated-LAN design. There is no
per-user authorization or concurrent project editing (limitations §22/§63).
Docker's bridge cannot carry discovery multicast by default; host-network
permissions are a deployment decision (limitations §79).

## Cross-cutting — LLM / natural-language interaction

Research complete (RESEARCH §13); no mutation-capable LLM or MCP surface
is scheduled. Require a mature command/authorization/revision-bound audit
contract before proposing one. Bus and device writes remain excluded.

## Cross-cutting — Repetitive-task automation

Research complete (RESEARCH §14); no generic macro/script API is scheduled.
A future operation template needs a deterministic revision-bound preview
and one atomic, undoable command batch.

## Cross-cutting — KNX `Functions` domain concept

Schema-23 prerequisite researched (RESEARCH §15), but project entity,
import, storage and UI remain deferred pending a dedicated ADR and samples
for older project schemas. Do not confuse master `FunctionType` data with
project Function instances.

## Cross-cutting — Third-party extension and plugins

No plugin API is planned (ADR-0025). Supported extension surfaces remain
product data, language packs, CSV and the CLI. A code plugin host needs a
proven second implementation and a safe authorization boundary, not a
speculative interface (KNOWN_LIMITATIONS §107).

## Session 6 — KNXnet/IP

Discovery, tunnelling, routing, monitor and scoped group communication
shipped. On this host the gateway's unicast reply was blocked by `ufw`;
after the user's UDP source-port 3671 rule, CLI and HTTP discovery both
found it (RESEARCH §20.1, limitations §79). A native WebKitGTK **Search**
click remains unobserved. Docker bridge multicast remains a separate
deployment boundary. KNX IP Secure is out of scope (§26). Bus traffic and
live-device support are not implied by simulator tests.

## Session 7 — Integration and hardening

Roundtrip/migration suites, large-project performance measurements and the
first x86_64 Linux AppImage are delivered at their tested boundaries.
Commissioning has a safety-gated `070nh` memory download and historical
individual-address evidence on MDT `1.1.67` (ADR-0048; RESEARCH §19).
Public confirmed address writes are now pre-tunnel refused pending verified
durable recovery (ADRs 0057–0059); no prior go transfers to another device.
Other masks, device families and RF hardware remain refused or simulator-only.
The application-download backup is not a universal rollback (ADR-0049).
ADR-0064 adds a separate durable one-shot activity-metadata backend and bounded
history API; complete long-session journalling and the global Web consumer
remain open. See [COMMISSIONING_ALPHA_LEDGER](COMMISSIONING_ALPHA_LEDGER.md)
for all 42 commissioning-routed source IDs, safe fallbacks and exact unblocks.
The remaining v1 closeout is manual acceptance, the user's alpha-tag decision
and the final review (`goal.md` §5/§10). No public release or blanket ETS
compatibility follows from this milestone.

## Cross-cutting — In-application help and user documentation

T28 shipped translated in-app tips and F1 help. A separate
[user manual](manual/README.md) exists but still needs its location,
screenshot and claim-by-claim acceptance after UI U13. In-app *project*
notes are another, unscheduled domain feature requiring an ADR (ADR-0024).

## Open questions and where they land

- Independent `.knxproj` samples (especially module-using schema 23) and an
  AES-protected ETS6 sample are prerequisites for broader claims (§1/§13).
- The user decides whether and when to publish an alpha tag; CI configuration
  and one local AppImage run are not proof of a distributable release.
- KNX Secure, legacy formats, `Functions`, multi-user support and RF hardware
  each need their own evidence/scope decision. See `goal.md` §6 and
  `goal-commission.md` §3c; none is silently scheduled here.
