# Alpha scope, risk and decision matrix (AR15)

Status of record for **what the Alpha candidate does, how well that is proven,
and what still stands between it and a release**. Built by AR15 on 2026-10-06
from `origin/main` `717d07c0`. Per-ID status stays in the
[ledger](status/LEDGER.md); this page groups it and names a release
disposition for everything not yet accepted. It does not accept anything on
the user's behalf.

Evidence levels used below:

- **Verified** — automated tests on real or synthetic data, or a documented
  run against a real file, gateway or device, named in the linked entry.
- **Simulator-only** — exercised only against KNXBench's own simulator.
- **Externally blocked** — needs a sample, source or hardware run the project
  does not have.
- **Accepted boundary** — a named, dated decision or accepted ADR keeps the
  limitation for the Alpha (ledger `ACCEPTED_BOUNDARY`).
- **Later** — explicitly out of the Alpha (ledger `LATER`).

## 1. Capabilities

| Area | Level | What exactly | Evidence |
| --- | --- | --- | --- |
| `.knxproj` import, ETS schema 11 and 21 | Verified | Two real reference projects with measured counts; unknown data preserved and reported | [COMPATIBILITY §2](COMPATIBILITY.md#2-verified-today) |
| `.knxproj` schema 23 | Verified, narrow | One ETS6 re-export of the reference project; module handling inferred from schema 21 | [COMPATIBILITY](COMPATIBILITY.md) |
| ZipCrypto-protected `.knxproj` (ETS4/5) | Verified (synthetic) | Library, CLI, server and Web password dialog | `KL-13`, [ALPHA_READINESS AR08](ALPHA_READINESS.md#ar08-password-import-entry-paths-2026-10-04) |
| AES-protected `.knxproj` (ETS6) | Externally blocked | Refused; no genuine sample | [KL §13](KNOWN_LIMITATIONS.md#13-password-protected-projects-zipcrypto-ets4ets5-is-decrypted-aes-ets6-is-still-refused) |
| Other ETS project schemas (12–14, 20, 22) | Externally blocked | Parser family exists, no real file tested | `KL-1`, `KL-11` |
| Native `.knxdb` store | Verified | Save/open/save-as; schema 10; upgrade of older files atomic, in place | [STORAGE_COMMAND_CONTRACT](STORAGE_COMMAND_CONTRACT.md), [KL §157](KNOWN_LIMITATIONS.md#157-opening-an-older-project-upgrades-it-in-place) |
| Editing topology, group addresses, building, devices, parameters | Verified | Undo/redo; parameter writes need recorded write authority (ADR-0080) | [IMPLEMENTATION_STATUS](IMPLEMENTATION_STATUS.md), [ADR-0080](adr/0080-parameter-write-authority.md) |
| Product database from `.knxprod` | Verified | Schemes 10–14, 20, exact 21/23; 852 of 853 public downloads install (large ones via CLI opt-in) | [PRODUCT_DATABASE_CORPUS](PRODUCT_DATABASE_CORPUS.md), ADR-0082/0083 |
| Parameter semantics | Verified for the evaluated subset | Unknown vendor logic is never executed; calculations not written | [PARAMETER_SEMANTICS_BOUNDARY](PARAMETER_SEMANTICS_BOUNDARY.md) |
| Group-address CSV export/import | Verified | Destructive changes need a confirmation token | [KL §39–41](KNOWN_LIMITATIONS.md) |
| HTML documentation export | Verified | Names need installed product data; translated texts follow the device-detail rule | [KL §46](KNOWN_LIMITATIONS.md#46-project-documentation-export-resolves-names-only-with-installed-product-data--partially-resolved-2026-09-23-t14) |
| Project diff (CLI, Web) | Verified | KNXBench diff, not an ETS comparison; Git external-diff recipe | [manual 08](manual/user-guide/08-reports-and-diff.md) |
| KNXnet/IP discovery, tunnelling monitor, group write | Verified on one gateway | 34-minute live session without drops | [manual implementation status](manual/implementation-status.md) |
| Routing monitor/send, custom multicast group | Verified offline; live custom-group router traffic externally blocked | `KL-31` | [KL §31](KNOWN_LIMITATIONS.md) |
| Telegram-flow view | Verified in Chromium | Motion Off for large maps; no WebKitGTK/Orca/real-bus evidence | [TELEGRAM_FLOW_VISUALIZATION §22](TELEGRAM_FLOW_VISUALIZATION.md#22-ar21-rerun-of-findings-6-and-7-and-acceptance-alpha-2026-10-05) |
| Device download (`070nh` memory path) | Verified on one device | CLI and Web; plan, per-device phrase, pre-write backup; others refused by name | [KL §7](KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked) |
| Property-based downloader, master reset, RF configuration | Simulator-only | No hardware route | [KL §92](KNOWN_LIMITATIONS.md#92-commissioning-phase-2-is-verified-against-a-simulator-this-project-wrote-and-has-never-addressed-a-device), §141, §143–§144 |
| Individual-address programming, reset, serial writes | Refused (fail closed) | Commands exist; refused before a tunnel until durable recovery exists | ADR-0057/0058/0059 |
| KNX Secure (Data/IP) | Accepted boundary | Not implemented; deferred 2026-09-11 | `KL-8` |
| Multi-user server | Accepted boundary | One shared password, no roles/audit/TLS of its own | `KL-63`, [ALPHA_READINESS AR13](ALPHA_READINESS.md#deployment-and-privacy-checklist-for-ar15ar17-and-release-notes) |

## 2. Deployment, import and hardware boundaries

- **Deployment.** Linux desktop (Tauri 2 on GTK3/WebKitGTK, `KL-16`) and a
  container/web server. The server binds loopback unless a password is set; a
  TLS proxy is required for networked use; release builds must use
  `KNX_REQUIRE_CLEAN_TREE=1` (AR13 checklist). Docker bridge tunnelling needs a
  gateway that honours Route Back (`KL-155`); discovery needs host networking.
- **Import.** Project data is imported, never written back as `.knxproj`
  (ADR-0028). Unknown or unsupported content is preserved or reported, never
  dropped silently (import reports, ADR-0081). Product packages above 64 MiB
  per member / 256 MiB total install only through the CLI opt-in (ADR-0082).
- **Hardware.** All live evidence comes from one installation: one gateway,
  one written device (`1.1.67`, mask `0701h`). Nothing extends to other
  vendors, masks, RF or Secure devices.
- **Storage.** A `.knxdb` is SQLite; review under Git via `knx diff`
  (`KL-9`). Opening an older file upgrades it in place (`KL-157`).

## 3. Ledger totals (189 rows, 2026-10-06)

| Status | Rows | Meaning for the release |
| --- | --- | --- |
| `DONE` | 42 | Delivered with evidence in the row |
| `ACCEPTED_BOUNDARY` | 111 | Kept for the Alpha by a named decision or ADR |
| `LATER` | 23 | Out of the Alpha by decision |
| `BLOCKED_EXTERNAL` | 8 | Needs a sample/source/run the project lacks — see §4 |
| `WAITING_OWNER` | 3 | Release gates — see §4 |
| `WAITING_DECISION` | 1 | Release decision — see §4 |
| `IN_PROGRESS` | 1 | Commissioning owner — see §4 |

Recount: `cargo run -p xtask -- check-ledger` (counts are enforced there).
Limitations: 108 numbered boundaries, 107 rated
([LIMITATION_TRIAGE](LIMITATION_TRIAGE.md)).

## 4. Not yet accepted — release disposition

Nothing below is waived. Each row says what has to happen before AR18/AR19.

| ID | Owner | What is missing | Disposition |
| --- | --- | --- | --- |
| `RELEASE-01`, `RELEASE-02` | alpha (AR18) | Final integrated gates on a frozen candidate and an independent whole-product review | **Blocks release.** Cannot be produced from historical receipts. |
| `RELEASE-03` | alpha (AR16) | UI owner's closure receipt, the user's manual location/screenshot policy, a dated manual checklist | **Blocks release.** Manual content was corrected in AR15 (`DOC-03`), acceptance is still open. |
| `RELEASE-04` | user (AR19) | Explicit tag/version/publication decision on the reviewed candidate | **Blocks release** by design; no automatic tag. |
| `UI-04` | commissioning | Owner receipt for the partial bus-activity snapshot (ADR-0055/0056) | **Owner must close or the user must accept it as a boundary before AR18.** |
| `KL-1`, `KL-11`, `KL-125` | alpha | Independent project samples (other schemas, second ETS6 project) | **Proposed: ship as disclosed boundary** (no broader compatibility claim). Needs the user's acceptance at AR19. |
| `KL-31` | alpha | Live router traffic on a custom multicast group | **Proposed: ship as disclosed boundary** (offline-tested only). Needs acceptance at AR19. |
| `PDB-01`, `R-DYNAMIC-01`, `R-MODULE-03`, `R-MODULE-04` | alpha | Primary semantics/samples for unevaluated parameter logic and nested modules | **Proposed: ship as disclosed boundary** (raw values preserved, never guessed). Needs acceptance at AR19. |

Also open outside the ledger: AR14D D5 (the `goal-commission.md` status
section still needs its owner's link-over); documentation hygiene only, not a
product blocker.

## 5. Risks to watch

| Risk | Why it matters | Current mitigation |
| --- | --- | --- |
| Single-installation evidence (`KL-1`) | Every compatibility statement rests on two projects and one bus | Claims scoped to tested inputs; unknown data preserved |
| DPT encoding rulings (`KL-61`, K1) | A wrong ruling sends a valid-looking telegram | Explicit input formats, disclosed rulings |
| Device writes on one device only (`KL-92`, K1) | A second device could behave differently | Plans refuse unknown shapes; backup before writes; per-device phrase |
| Motion cost on large maps (`KL-154`) | The flow view can saturate the UI thread | Manual and known issues say Motion Off for large installations |
| In-place upgrade (`KL-157`) | Older KNXBench cannot reopen an upgraded file | Atomic upgrade; manual tells users to copy first |
