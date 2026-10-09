# RESEARCH.md — Session 0: Technical Research

Status: **complete for Session 0 scope**
Date: 2026-09-02
Method: primary-source inspection of a real ETS4 project plus a live KNX installation, supplemented by public documentation. No application code written in this session.

Every statement below is tagged:

* **[V]** — verified in this repository against real data or installed source code. Reproducible.
* **[D]** — documented by a public, citable source, not verified here.
* **[A]** — assumption or inference. Must be validated before it drives an irreversible design decision.

## Where the research lives

Split by topic on 2026-10-04 (AR14D D4). Section numbers stay global and
stable, so `RESEARCH §8` still means section 8; follow the table to its
file. New findings go into the topic file they belong to: a dated entry at
its top, or the next free section number (§26 next) at its end. Sources
stay at the end of this file.

| Section | Where |
| --- | --- |
| [2026-10-09 — A second program verified live: the presence detector 1.1.8](research/commissioning.md#2026-10-09--a-second-program-verified-live-the-presence-detector-118) | Commissioning and device download |
| [2026-10-08 — Public intake, disclosure and manual delivery boundaries](research/community-intake.md) | Community evidence |
| [2026-10-07 — DPT source inventory and generic runtime admission](research/knxnet-ip-and-bus.md#2026-10-07--dpt-source-inventory-and-generic-runtime-admission) | KNXnet/IP and bus access |
| [2026-10-07 — Inferences under ADR-0086: the house plans 32 of 35 devices](research/commissioning.md#2026-10-07--inferences-under-adr-0086-the-house-plans-32-of-35-devices) | Commissioning and device download |
| [2026-10-04 — Product scheme23: project documentation is not manufacturer grammar](research/product-database.md#2026-10-04--product-scheme23-project-documentation-is-not-manufacturer-grammar) | Manufacturer and product data |
| [2026-10-04 — Telegram-flow Alpha design](research/features-and-ui.md#2026-10-04--telegram-flow-alpha-design) | Features and UI research |
| [2026-10-01 — Backup directory chains, not only the final directory](research/commissioning.md#2026-10-01--backup-directory-chains-not-only-the-final-directory) | Commissioning and device download |
| [2026-09-29 — Read-only inventory of local ETS installation data](research/project-format.md#2026-09-29--read-only-inventory-of-local-ets-installation-data) | Project file format |
| [2026-09-29 — Parameter fields across an octet boundary; module instances measured](research/commissioning.md#2026-09-29--parameter-fields-across-an-octet-boundary-module-instances-measured) | Commissioning and device download |
| [2026-09-29 — Rename leaves no longer block a download image](research/commissioning.md#2026-09-29--rename-leaves-no-longer-block-a-download-image) | Commissioning and device download |
| [2026-09-29 — Download coverage re-measured: signed and text values, honest mask refusals](research/commissioning.md#2026-09-29--download-coverage-re-measured-signed-and-text-values-honest-mask-refusals) | Commissioning and device download |
| [2026-09-29 — Commissioning readiness: offline coverage of the installed product corpus](research/commissioning.md#2026-09-29--commissioning-readiness-offline-coverage-of-the-installed-product-corpus) | Commissioning and device download |
| [2026-09-29 — U6 root zoom and persisted pane geometry](research/features-and-ui.md#2026-09-29--u6-root-zoom-and-persisted-pane-geometry) | Features and UI research |
| [2026-09-29 — U5 validation and help routing research](research/features-and-ui.md#2026-09-29--u5-validation-and-help-routing-research) | Features and UI research |
| [1. Evidence base](#1-evidence-base) | this file |
| [2. `.knxproj` container format](research/project-format.md#2-knxproj-container-format) | Project file format |
| [3. Project content model (`0.xml`)](research/project-format.md#3-project-content-model-0xml) | Project file format |
| [4. Manufacturer data model](research/product-database.md#4-manufacturer-data-model) | Manufacturer and product data |
| [5. `knx_master.xml`](research/project-format.md#5-knx_masterxml) | Project file format |
| [6. Group addresses and DPT resolution](research/project-format.md#6-group-addresses-and-dpt-resolution) | Project file format |
| [7. Opaque and unsupported data](research/project-format.md#7-opaque-and-unsupported-data) | Project file format |
| [8. KNXnet/IP and bus access](research/knxnet-ip-and-bus.md#8-knxnetip-and-bus-access) | KNXnet/IP and bus access |
| [9. KNX Secure](#9-knx-secure) | this file |
| [10. Legal and licensing constraints](#10-legal-and-licensing-constraints) | this file |
| [11. Risks](#11-risks) | this file |
| [12. Conclusions for Session 1 (Architecture)](#12-conclusions-for-session-1-architecture) | this file |
| [13. Natural-language interaction and MCP prerequisite audit (2026-09-22, T19)](research/features-and-ui.md#13-natural-language-interaction-and-mcp-prerequisite-audit-2026-09-22-t19) | Features and UI research |
| [14. Repetitive-task automation and macro-layer decision (2026-09-22, T20)](research/features-and-ui.md#14-repetitive-task-automation-and-macro-layer-decision-2026-09-22-t20) | Features and UI research |
| [15. KNX `Function` project semantics feasibility (2026-09-22)](research/features-and-ui.md#15-knx-function-project-semantics-feasibility-2026-09-22) | Features and UI research |
| [16. “Who talks to whom?” flow-view decision (2026-09-22)](research/features-and-ui.md#16-who-talks-to-whom-flow-view-decision-2026-09-22) | Features and UI research |
| [17. Site/property above buildings (ISSUE-06, 2026-09-26)](research/features-and-ui.md#17-siteproperty-above-buildings-issue-06-2026-09-26) | Features and UI research |
| [18. Legacy VD/PR (`EX-IM`) product files (DIN-9, 2026-09-26)](research/product-database.md#18-legacy-vdpr-ex-im-product-files-din-9-2026-09-26) | Manufacturer and product data |
| [19. Application download to a mask `0701h` (BIM M112) device (2026-09-27)](research/commissioning.md#19-application-download-to-a-mask-0701h-bim-m112-device-2026-09-27) | Commissioning and device download |
| [20. UI issue U2: AppImage interface discovery and line-relative addresses (2026-09-28)](research/features-and-ui.md#20-ui-issue-u2-appimage-interface-discovery-and-line-relative-addresses-2026-09-28) | Features and UI research |
| [21. U7 bus-monitor decoding and bounded snapshots (2026-09-29)](research/features-and-ui.md#21-u7-bus-monitor-decoding-and-bounded-snapshots-2026-09-29) | Features and UI research |
| [22. Serial-address write recovery scope (2026-09-30)](research/commissioning.md#22-serial-address-write-recovery-scope-2026-09-30) | Commissioning and device download |
| [23. K13 reset recovery scope (2026-09-30)](research/commissioning.md#23-k13-reset-recovery-scope-2026-09-30) | Commissioning and device download |
| [24. K6 button-address programming recovery scope (2026-10-01)](research/commissioning.md#24-k6-button-address-programming-recovery-scope-2026-10-01) | Commissioning and device download |
| [25. UA1: coupler `.0` evidence and Site/Ground samples (2026-10-04)](research/features-and-ui.md#25-ua1-coupler-0-evidence-and-siteground-samples-2026-10-04) | Features and UI research |

---

## 1. Evidence base

| Artifact | What it is | Notes |
| --- | --- | --- |
| `Unser Zuhause ets4 - 2025-12-15.knxproj` | Real ETS 4.1.8 project, 1.7 MB packed / 22 MB unpacked, 38 archive entries | Not password protected. 36 devices, 514 group addresses, 4 manufacturers. |
| `Unser Zuhause ets 6.3.0 - 2026-09-02.knxproj` | The **same** installation, re-exported unchanged from ETS 6.3.7959.0, 1.7 MB packed / 22 MB unpacked, 48 archive entries | Not password protected. Schema 23. Same 4 manufacturers, same 514 group addresses. Gives a same-content, cross-schema diff instead of a second independent sample — see §2.4/§3.3. |
| `KV v2.5 - demo.knxproj` | KNX Association/manufacturer demo project (ETS 5.7), 111 KB packed | Not password protected. Schema 21. A **genuinely independent second installation** — different devices (4), different group addresses (13), different manufacturer (`M-00FA`) — not a re-export of the reference project. Session 7 evidence (2026-09-06); see §2.5/§3.4. Sourced from `OriginalData/DemoProjects/` (gitignored there; committed to the repo root as a test fixture instead, same as the other two `.knxproj` files). |
| `project_dump.json`, `group_addresses.json`, `devices.json` | `xknxproject` 3.10.0 output of the ETS4 project | Used as a *reference implementation baseline*, not as ground truth. |
| `bus_traffic.jsonl` | 280 live telegrams captured from the real bus | Via `monitor_bus.py`, KNXnet/IP tunnelling to gateway `192.0.2.1`. |
| `.venv/` | `xknx` 3.20.0 (MIT), `xknxproject` 3.10.0 (GPL-2.0-only) | Source read directly for format details. |
| KNX Standard v3.0.0, full text | 180 documents (all 10 volumes + Application Notes), local at `/home/knxbench/knx-ai/extracted_clean/` on the dev machine | Licensed KNX Association material — machine-local, not committed to this repo, not portable. Query it via the `knx-spec` Claude Code skill (`~/.claude/skills/knx-spec/`) rather than grepping by hand. Primary source for protocol/DPT correctness questions; cite as `<filename> §<section>`, never quote verbatim into `docs/`. |

All container- and project-level numbers quoted below are reproducible with:

```bash
python3 tools/inspect_knxproj.py "Unser Zuhause ets4 - 2025-12-15.knxproj"
python3 tools/inspect_knxproj.py "Unser Zuhause ets 6.3.0 - 2026-09-02.knxproj"
```

(stdlib only — deliberately no `xknxproject`, see §10.)

This gives one complete, non-trivial, real-world reference installation, exported twice: once from ETS 4.1.8 (schema 11) and once — unchanged, by the same user — from ETS 6.3.7959.0 (schema 23). It is still a **single installation**: the two exports let us diff schema 11 against schema 23 with confidence (same devices, same group addresses, same manufacturers), but they say nothing about schema 12, 13, 14, 20 or 22, and — until Session 7 — nothing about a second, differently-structured project. Sections marked as such must be re-verified against other ETS5/ETS6 projects before being treated as general.

Session 7 (2026-09-06) added a second, genuinely independent sample for schema 21 (§2.5/§3.4) — still leaves schema 12, 13, 14, 20 and 22 unsampled.

---

## 9. KNX Secure

Not present in our sample installation (`data_secure=false` on all 514 group addresses, no keyring) — everything here is **[D]**.

* **Data Secure**: each secured group address has a 128-bit runtime key. Each secure device has a tool key. ETS stores them in the `.knxproj` in protected form and can export them to a password-protected `.knxkeys` **keyring** file, which is the supported route for non-ETS clients.
* **FDSK**: factory key printed on the device label / QR code as a 36-character device certificate (FDSK + KNX serial number). On first commissioning ETS replaces it with a per-tool Device Key. Without the project password, stored FDSKs are inaccessible and secure devices cannot be reprogrammed.
* **Keyring format**: KNX publishes an official article; FDSK inside is separately encrypted, and the XML is signed using the same length-prefixed serialization scheme as product data. `xknx` already implements keyring decryption.
* **ETS6 tunnel clients**: the keyring is exported per IP tunnel via *Export Interface Information*, not from the project security page.

Design consequence: **key material must be a separate, isolated subsystem** with its own storage and access rules from day one. It must never be flattened into the general project model, never written to logs, exports, or reports, and must be omitted by default from any diagnostic dump. Retrofitting this is how secrets leak.

Open question for a later session: can we read secured runtime keys directly out of a `.knxproj`, or only from a `.knxkeys` keyring? Untested.

---

## 10. Legal and licensing constraints

**Not legal advice.** These are the constraints as best established from public sources; anything with commercial consequences needs a lawyer.

* **Protocol vs. document.** Copyright covers the specification text, not the protocol. Implementing from a lawfully obtained specification is standard practice; redistributing the specification text is not. [D]
* **Trademark.** "KNX" is a registered trademark. An independent tool can interoperate but cannot call itself KNX-certified without membership and conformance testing. Existing projects consistently use "KNX-compatible". **We should adopt the same wording in all user-facing text.** [D]
* **ETS is proprietary.** Reading `.knxproj` we own is fine. We must not bundle ETS binaries, DLLs, or converters (`KnxCvNext.exe` requires an installed ETS). [D]
* **Manufacturer product data.** `.knxprod` files are distributed through the KNX online catalog under KNX/manufacturer terms. Users importing their own downloaded product data is one thing; **us redistributing a product database is another**. Product data must stay strictly separable from application code and must not be committed to this repository. [D] — matches the rule already in `CLAUDE.md`.
* **`knx_master.xml` extracted from a project** is KNX Association content. Read it from the user's own file at import time; do not vendor a copy. [A — conservative default]
* **Dependency licensing — hard architectural constraint [V]:**

  | Package | License | Consequence |
  | --- | --- | --- |
  | `xknx` 3.20.0 | MIT | Safe to depend on under any license. |
  | `xknxproject` 3.10.0 | **GPL-2.0-only** | Linking it forces the whole application to GPL-2.0. |

  Since §7.1 already establishes that we need our own parser, the clean resolution is: **`xknxproject` is a development/test-only dependency, never a runtime dependency.** This must be enforced mechanically (separate dependency group + a test that the runtime import graph never reaches it), and recorded as an ADR in Session 1.

* **The `.knxprod` container** is the same XML family as `.knxproj` (master data scheme 11 vs. 12+); newer files cannot be read by older ETS. Its encryption/obfuscation layer for newer schemes was **not** established by this research and remains an open question. [D/open]

---

## 11. Risks

| # | Risk | Severity | Mitigation |
| --- | --- | --- | --- |
| R1 | Single-sample bias: everything verified here is schema 11 / ETS 4.1 | High | Acquire ETS5 (13/14, 20) and ETS6 (21+) sample projects before Session 3. Treat §3 as version-specific until then. |
| R2 | No authoritative XSD available | High | Tolerant parser + exhaustive unknown-element/attribute inventory, reported to the user (§12). |
| R3 | `Dynamic` tree (`choose`/`when`) evaluation is the real complexity | High | **Done (2026-09-11).** The dedicated Session 4 research spike ran — see §4.3. The Standard normatively specifies the `@test` value grammar; the surrounding structural grammar remains corpus-observed only, not Standard-normative. This closes the research risk; it does not build the evaluator. T18 (the parameter editor, [GAP_ANALYSIS_ETS.md](https://github.com/KNXBench-Labs/KNXBench/blob/a584007fc05a/docs/GAP_ANALYSIS_ETS.md)) is no longer blocked on research — it now needs a no-match design decision and a defensive parser, both implementation work. |
| R4 | Round-trip cannot be byte-exact (signatures, attribute ordering, ETS-internal ids) | Medium | Define round-trip fidelity as *semantic* equality over a declared model + verbatim passthrough of opaque parts. Never claim byte-exactness. |
| R5 | Vendor plug-in DLLs make some devices unconfigurable by us | Medium | Detect `Baggages`, mark affected devices read-only, report clearly. |
| R6 | GPL-2.0 contamination via `xknxproject` | Medium | Test-only dependency, enforced by CI. |
| R7 | Product database size/performance (22 MB for 12 programs) | Medium | Indexed, cached, versioned product DB layer; never re-parse per open. |
| R8 | Secret leakage once KNX Secure is supported | High | Isolated key subsystem, excluded from exports/logs by default. |
| R9 | Writing an unsigned `.knxproj` may be rejected by ETS on re-import | Medium | **Closed as not applicable (2026-09-20).** The test was never run, and now has no subject: [ADR-0028](adr/0028-no-knxproj-export.md) withdrew `.knxproj` writing entirely, so KNXBench produces no file for ETS to reject. |
| R10 | Commissioning/device-download procedures might be undocumented outside ETS internals | High | **Partly done (2026-09-11).** The R5 research spike ran — see §8.4. The generic download/unload/reset/memory-write procedures and the Load State Machine are Standard-normative and now cited in full; "Differential Download" is a formal Glossary term. This closes the research risk on the *generic* procedure; it does not build a downloader, it does not verify anything against real hardware, and the product-specific `Legacy*` compatibility-flag matrix and vendor `Baggage` DLL involvement (R5 above) remain genuinely undocumented in this corpus — see §8.4 Q4/Q9. |

---

## 12. Conclusions for Session 1 (Architecture)

Recommendations carried forward, each traceable to a finding above:

1. **Write our own `.knxproj` reader.** §7.1. `xknxproject` stays as a test oracle only, and as a test-only dependency for license reasons (§10).
2. **Model the three-layer override chain explicitly** (ComObject → ComObjectRef → ComObjectInstanceRef), keeping provenance per resolved value. §3.2 — this affects 758 of 907 objects and cannot be bolted on later.
3. **Two orthogonal hierarchies over one device set**: topology (Area/Line, plus unassigned) and building (recursive typed `BuildingPart`). Neither owns the device. §3.1.
4. **Directional group-address links** (send / receive), not undirected associations. §3.1.
5. **Commissioning state is domain data**, not import metadata: `*Loaded` flags, `LastDownload`, `CompletionStatus`, `Broken`. §3.1.
6. **Language-aware strings from the start.** Translations are a side table with 5919 entries in a single application program. §4.1.
7. **Every import produces a report**: unknown elements/attributes encountered, opaque data preserved, values inferred (e.g. DPT from linked objects), conflicts, and unsupported features. This is a core deliverable of the importer, not a logging afterthought. §6.1, §7, R2.
8. **Opaque-passthrough store** keyed by source path, so binaries, signatures and legacy plugin data survive a round trip untouched. §7.
9. **Product database is a separate, versioned, cached layer**, keyed by (manufacturer, application program, version), never bundled with the application. §4.1, §10.
10. **Key material is an isolated subsystem** even before KNX Secure is implemented. §9.
11. **Retain low-level programming data** (`Memory`, `AbsoluteSegment`, `LoadProcedures`, mask/resource data) at import even though commissioning has not started, so that path stays open. §8.3.
12. **User-facing wording is "KNX-compatible", never "KNX certified" or "full ETS compatibility".** §7, §10.

### Open questions to resolve before or during Session 3

* ETS5/ETS6 schema deltas (13, 14, 20, 21+) — needs sample projects. (R1)
* `Functions` element semantics — absent from our sample. (§3.1)
* ~~`when/@test` expression grammar in the `Dynamic` tree. (R3)~~ — **answered, §4.3 (2026-09-11).** What is still open, carried forward from that section: the `Dynamic`/`Channel`/`ParameterBlock`/`choose`/`When_t`/`ChannelIndependentBlock` *structural* (complexType) grammar, which no schema document available here defines; the no-match evaluation rule for a no-default `choose`; and `Access`/`Visible` as gating mechanisms independent of `choose`/`when`.
* Whether ETS re-imports an unsigned `.knxproj` written by a third-party tool. (R9)
* Whether Data Secure runtime keys are readable from `.knxproj` or only from `.knxkeys`. (§9)
* `.knxprod` encryption for master data scheme 12+. (§10)

---

## Sources

* [Project schema description – KNX Association](https://support.knx.org/hc/en-us/articles/4408207190674-Project-schema-description)
* [Unexpected XML namespace "http://knx.org/xml/project/14" – KNX Association](https://support.knx.org/hc/en-us/articles/360007208400-Unexpected-XML-namespace-http-knx-org-xml-project-14)
* [calimero-project/import-ets-xml (archived)](https://github.com/calimero-project/import-ets-xml)
* [calimero-project/calimero-core](https://github.com/calimero-project/calimero-core)
* [XKNX/xknxproject](https://github.com/XKNX/xknxproject)
* [KNX Data Secure – KNX Association](https://support.knx.org/hc/en-us/articles/360012689639-KNX-Data-Secure)
* [Keyring File Format – KNX Association](https://support.knx.org/hc/en-us/articles/18968368409874-Keyring-File-Format)
* [Manufacturer Product Databases – KNX Association](https://www2.knx.org/ie/software/ets/manufacturer-product-databases/index.php)
* [File formats used during registration/certification – KNX Association](https://support.knx.org/hc/en-us/articles/4659247971346-File-formats-used-during-registration-certification)
* [ISO 22510:2019 — KNXnet/IP communication](https://www.iso.org/standard/73364.html)
* [thelsing/CreateKnxProd](https://github.com/thelsing/CreateKnxProd)
* [XKNX/xknxproject test resources](https://github.com/XKNX/xknxproject/tree/main/test/resources) (GPL-2.0; inspected, not copied)
