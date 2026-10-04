# KNXBench — open items, blockers, research and decisions

**As of:** 2026-10-01 18:27 CEST

**Audited reference revision:** `origin/main` at `4ff56dcf5eccb7eece99df35ab35e80453badaa4`.

**Local checkout:** `60613033648364b375338de2c8e4e00b4410d30b`; 140 commits behind that reference revision, with pre-existing changes belonging to other work.

> **Post-snapshot additions (2026-10-03):** `KL-149`–`KL-153` from the public
> product-download test run are tracked outside this 180-entry snapshot, in
> [ALPHA_READINESS](../ALPHA_READINESS.md#post-snapshot-findings-outside-the-180-id-ledger)
> and `alpha-release-goal.md` §8 (package AR06P).

## Context and scope

This inventory combines Known Limitations, triage, research, the roadmap, all three goal files, ADRs, the product-data corpus, current implementation evidence and remaining manual/planning items. It also considers the **local, not yet fully versioned** research, statistics and Paperclip handover documents. Where sources conflict, current source code and tests take precedence over ADRs/technical documentation, followed by status/goal text; a historically unchecked box is not proof of a missing implementation.

The main table contains **180 entries**, including accepted boundaries and optional ideas; it is **not a list of 180 authorized implementation tasks**. All **110 numbered headings** in `docs/KNOWN_LIMITATIONS.md` are accounted for: remaining boundaries in the main table, and five already resolved or clarification-only entries in the exclusions table. §8/§26 are combined into one Secure topic. The two occurrences of §130 receive distinct IDs here: `KL-130-GATE` and `KL-130-ZOOM`. Unnumbered boundaries and additional research/decision items are included; cross-cutting questions and their concrete workflow boundaries may occupy separate rows without constituting separate implementation packages.

**Audit depth:** a documentation-based completeness and status audit with targeted source-code checks, not a new comprehensive code/security audit or a new test run. No KNX devices were contacted, no write authorizations were granted and no compatibility claims were extended. A download-error cleanup that landed during the audit was considered up to the reference commit stated above.

## Priorities and statuses

Entries are sorted first by **priority**, then alphabetically by **topic area** within each priority. The classification assesses risk and acceptance needs, **not** effort or an automatically authorized sequence; it does not replace the existing K1–K4 triage.

| Priority | Meaning | Entries |
| --- | --- | ---: |
| **P0 — safety prerequisite** | Must be resolved before existing hardware-write restrictions are lifted; currently fail-closed, not an instruction to write. | 4 |
| **P1 — high** | Correctness, data integrity, safety/hardware evidence or decisive completion evidence. | 29 |
| **P2 — medium** | Actual remaining functionality, compatibility, research or acceptance work with bounded scope. | 87 |
| **P3 — low / later** | Convenience, maintenance, deliberate boundaries, optional extensions or side tracks. | 60 |

**Reading statuses:** “Safety blocker” prevents an action; “externally blocked” requires, for example, real samples or hardware; “evidence/acceptance missing” does not necessarily mean missing code; “decision pending” requires a scope decision; “accepted/deliberate boundary” remains visible but is not an active v1 implementation TODO; “parked/optional” is not automatically scheduled. The selected independent U13 review has not yet delivered a verdict; the earlier service interruption is neither a review nor a request for new quota checks.

## Prioritized overview table

`KL-*` refers to the original limitation number. `GAP-T30-*` retains the canonical research IDs. Other IDs apply only to this audit. The source column names the repository path and section; `ADR-NNNN` can be located through `docs/adr/README.md` or the correspondingly numbered file in `docs/adr/`. “Local” marks additional uncommitted evidence.

| Priority | Topic area | ID | Status | What is it about? | Source |
| --- | --- | --- | --- | --- | --- |
| P0 | Commissioning / Recovery | KL-116 | Safety blocker | Confirmed public button-based address writes remain blocked until device-specific durable pre-write recovery is demonstrated. | docs/KNOWN_LIMITATIONS.md §116 |
| P0 | Commissioning / Recovery | KL-139 | Safety blocker | The test device ignored serial-number-based address writing, which remains blocked through public interfaces until complete durable recovery is available. | docs/KNOWN_LIMITATIONS.md §139 |
| P0 | Commissioning / Recovery | KL-140 | Safety blocker | Public address reset remains blocked before tunnel creation until durable complete backups of all affected devices are available. | docs/KNOWN_LIMITATIONS.md §140 |
| P0 | Commissioning / Recovery | SAFE-01 | Identity / recovery pending | The new K6 candidate requires verified model/application identity, affected memory regions and recovery evidence before any write authorization. | goal-commission.md status / docs/RESEARCH.md §22–24 |
| P1 | Bus / Discovery / Scan | KL-79 | Partial / acceptance pending | The host-firewall cause has been resolved, while native discovery acceptance and Docker multicast boundaries remain. | docs/KNOWN_LIMITATIONS.md §79 |
| P1 | Bus / DPT / Protocol | KL-61 | Partial / research pending | DPT main types 1–30 work, but several encoding rulings and missing subtype/catalog semantics limit the claim. | docs/KNOWN_LIMITATIONS.md §61 |
| P1 | Bus / DPT / Protocol | KL-99 | Research pending | The MemoryControlBlock read/write nibble order is inferred rather than explicitly established by normative evidence. | docs/KNOWN_LIMITATIONS.md §99 |
| P1 | Commissioning / Hardware | KL-112 | Functional blocker | Download plans requiring A_Key_Write are rejected in full rather than performing unsecured key changes. | docs/KNOWN_LIMITATIONS.md §112 |
| P1 | Commissioning / Hardware | KL-136 | Evidence missing | The final Basic Restart of the tested 0701h download remains explicitly unconfirmed. | docs/KNOWN_LIMITATIONS.md §136 |
| P1 | Commissioning / Hardware | KL-138 | Hardware evidence missing | Authorization using a project or file key is implemented but has not been tested on a real key-protected device. | docs/KNOWN_LIMITATIONS.md §138 |
| P1 | Commissioning / Hardware | KL-141 | Hardware boundary | Destructive Master Reset erases data only in the simulator; the tested device retains its configuration. | docs/KNOWN_LIMITATIONS.md §141 |
| P1 | Commissioning / Hardware | KL-142 | Partial / UI missing | All partial memory-download modes are demonstrated on one device, but web selection and additional AppliesTo/legacy semantics are missing. | docs/KNOWN_LIMITATIONS.md §142 |
| P1 | Commissioning / Hardware | KL-7 | Partial / evidence missing | Production downloading is limited to the demonstrated 070nh memory path and is not verified for arbitrary masks or devices. | docs/KNOWN_LIMITATIONS.md §7 |
| P1 | Commissioning / Hardware | KL-92 | Evidence missing | Most commissioning functionality is verified only against the project's own simulator, not independently on a range of devices. | docs/KNOWN_LIMITATIONS.md §92 |
| P1 | Commissioning / Recovery | DEBUG-01 | Deliberate safety boundary | The default-off debug-property action has only an offline-verified property backup, not device-wide recovery or new live evidence. | goal-commission.md status / ADR-0051 |
| P1 | Commissioning / Recovery | SAFE-02 | Partial / evidence missing | Download backups cover only plan-affected memory regions and load states; live evidence is missing for group-address partial restore and refusal on backup failure. | docs/KNOWN_LIMITATIONS.md: Commissioning readiness / ADR-0049 |
| P1 | Commissioning / Recovery | SAFE-03 | Robustness evidence pending | Returned download errors close the connection on a best-effort basis; this does not cover future cancellation, process termination or power loss. | docs/KNOWN_LIMITATIONS.md §7 / goal-commission.md §3 |
| P1 | Data Integrity / Storage | DATA-01 | Remaining boundary | Beyond catalog preflight validation, other mutation paths still use unchecked u32 ID increments at the maximum ID. | docs/KNOWN_LIMITATIONS.md: U11 catalog batch scope / crates/knx-core/src/project.rs |
| P1 | Data Integrity / Storage | KL-129 | Parked / structurally pending | Duplicate-ID data loss is resolved, but ADR-0039 phases 3–5 do not yet enforce the command mutation path through the API. | docs/KNOWN_LIMITATIONS.md §129 |
| P1 | Diagnostics / Audit | KL-106 | Privacy boundary | Debug reports redact only four pattern classes and still require manual privacy review before sharing. | docs/KNOWN_LIMITATIONS.md §106 |
| P1 | Documentation / Release | DOC-01 | Contradiction pending | Goals and triage still list some resolved store-path, streaming and file-selection issues as open and need status corrections backed by source code. | goal.md §3 / §12.2 / docs/LIMITATION_TRIAGE.md / apps/knx-server/src/domain.rs |
| P1 | Import / Compatibility | KL-1 | Evidence missing | Project imports are demonstrated for only two independent installations; further project schemas and an independent modular schema-23 sample are missing. | docs/KNOWN_LIMITATIONS.md §1 |
| P1 | Import / Compatibility | KL-13 | Partial / externally blocked | AES projects are rejected for lack of a real sample; ZipCrypto still needs production password integration and a real ETS export as evidence. | docs/KNOWN_LIMITATIONS.md §13 |
| P1 | Manufacturer Data / Parameters | PDB-09 | Reporting gap | Attributes in the master Languages branch are preserved without an allowlist but are not yet fully reported as unknown attributes. | docs/KNOWN_LIMITATIONS.md: PDB-3 report history and coverage boundary |
| P1 | Manufacturer Data / Parameters | R-MODULE-01 | Research blocked | Modular parameter memory placement is contradictory in the corpus and requires an authorized device readback rather than a guessed formula. | docs/RESEARCH.md §19.11 / goal-commission.md |
| P1 | Quality / Evidence | KL-130-GATE | Remaining gate boundary | An xtask built in a removed worktree can successfully check zero files and requires demonstrated target and coverage validation. | docs/KNOWN_LIMITATIONS.md §130 (Gate) |
| P1 | Quality / Evidence | RELEASE-01 | Final acceptance pending | The independent whole-goal review and remediation of its findings are missing after completion or explicit exclusion of all active remaining items. | goal.md §9–10 |
| P1 | Quality / Evidence | RELEASE-02 | Completion evidence pending | The final nine gates must be confirmed on the actual finished integration revision with fresh evidence that does not silently check nothing. | goal.md §1 / §10 |
| P1 | Security / Operations | KL-22 | Documented boundary | The server provides one shared password without user roles, built-in TLS or a guarantee of safe Internet operation. | docs/KNOWN_LIMITATIONS.md §22 |
| P1 | Security / Operations | KL-63 | Accepted boundary | Multiple clients share the project and undo stack without conflict detection; true multi-user editing is deferred. | docs/KNOWN_LIMITATIONS.md §63 |
| P1 | Security / Operations | KL-8 | Deferred / externally blocked | Data Secure, IP Secure and keyring processing are missing and require suitable key samples and a secured test installation. | docs/KNOWN_LIMITATIONS.md §8/26 |
| P1 | UI / Acceptance | UI-01 | Review blocked | U13 requires the selected independent whole-UI review and remediation of its findings; the earlier service interruption delivered no verdict. | goal-ui.md §0 / docs/IMPLEMENTATION_STATUS.md: U13 |
| P1 | UI / Acceptance | UI-02 | Evidence / decision pending | ISSUE-12 has two open discovery-evidence checkboxes that must be accepted or re-scoped without inventing a capture or loopback test. | goal-ui.md §0 / docs/superpowers/plans/2026-09-21-user-reported-issues.md |
| P2 | Bus / Discovery / Scan | KL-126 | Deliberate safety boundary | Scan reconciliation changes project data based on selected occupancy evidence without inferring device identity from it. | docs/KNOWN_LIMITATIONS.md §126 |
| P2 | Bus / Discovery / Scan | KL-29 | Documented boundary | The CLI monitor uses a fixed three-level group-address display and can overlay names from multiple installations. | docs/KNOWN_LIMITATIONS.md §29 |
| P2 | Bus / Discovery / Scan | KL-31 | Evidence missing | Custom routing multicast groups are implemented, but real traffic on custom groups and corresponding discovery behavior are unproven. | docs/KNOWN_LIMITATIONS.md §31 |
| P2 | Bus / Discovery / Scan | KL-62 | Partial / evidence missing | The group monitor remains limited to one tunneling session with client-side filters and has mainly passive gateway evidence. | docs/KNOWN_LIMITATIONS.md §62 |
| P2 | Bus / Discovery / Scan | KL-72 | Documented boundary | A complete unthrottled line scan occupies one tunnel and substantial real bus time. | docs/KNOWN_LIMITATIONS.md §72 |
| P2 | Bus / Discovery / Scan | KL-73 | Functional boundary | The line scan determines occupancy and, where possible, mask version, but not product, manufacturer or serial identity. | docs/KNOWN_LIMITATIONS.md §73 |
| P2 | Bus / Discovery / Scan | KL-74 | Protocol boundary | The current occupancy-probe procedure cannot reliably distinguish a busy device from a vacant address. | docs/KNOWN_LIMITATIONS.md §74 |
| P2 | Bus / Discovery / Scan | KL-75 | Deliberate safety boundary | Shorter scan timeouts are possible but increase the risk of apparently vacant addresses on slow devices. | docs/KNOWN_LIMITATIONS.md §75 |
| P2 | Bus / Discovery / Scan | KL-77 | Functional boundary | Scans examine one line at a time and do not automatically traverse line or area couplers. | docs/KNOWN_LIMITATIONS.md §77 |
| P2 | Bus / Discovery / Scan | KL-78 | Evidence boundary | Other tunnel endpoints can appear as occupied addresses without being identified as physical bus devices. | docs/KNOWN_LIMITATIONS.md §78 |
| P2 | Bus / DPT / Protocol | KL-105 | Wire evidence missing | SYSTEM priority is encoded, but its actual transmission through the gateway onto TP1 has not been independently measured. | docs/KNOWN_LIMITATIONS.md §105 |
| P2 | Bus / DPT / Protocol | KL-108 | Documented ruling | Where the standard text contradicts itself on an occupied destination address, KNXBench explicitly follows the exception text. | docs/KNOWN_LIMITATIONS.md §108 |
| P2 | Commissioning / Hardware | KL-101 | Documented boundary | The standards-compliant additional load-state attempt can significantly extend the worst-case waiting time. | docs/KNOWN_LIMITATIONS.md §101 |
| P2 | Commissioning / Hardware | KL-104 | Documented boundary | A device going offline in LoadCompleting causes a complete reconnect on each poll. | docs/KNOWN_LIMITATIONS.md §104 |
| P2 | Commissioning / Hardware | KL-109 | Deliberate safety boundary | Plans containing multiple loadable parts of the same PartKind without an established order are rejected. | docs/KNOWN_LIMITATIONS.md §109 |
| P2 | Commissioning / Hardware | KL-111 | Deliberate safety boundary | Unloading the individual address is not implemented because it could make the target itself unaddressable. | docs/KNOWN_LIMITATIONS.md §111 |
| P2 | Commissioning / Hardware | KL-113 | Planning boundary | Partial-download escalation reloads only segments present in the supplied plan. | docs/KNOWN_LIMITATIONS.md §113 |
| P2 | Commissioning / Hardware | KL-114 | Documented ruling | Requiring a download counter for partial downloads is a conservative KNXBench rule, not a general System B requirement. | docs/KNOWN_LIMITATIONS.md §114 |
| P2 | Commissioning / Hardware | KL-143 | Externally blocked | RF domain-address procedures are simulated, with no available RF device and no production CLI/HTTP route. | docs/KNOWN_LIMITATIONS.md §143 |
| P2 | Commissioning / Hardware | KL-144 | Externally blocked | RF device configuration works in the simulator but lacks real hardware evidence or product-facing access. | docs/KNOWN_LIMITATIONS.md §144 |
| P2 | Commissioning / Hardware | KL-145 | Behavioral decision | For active unconnected group objects, the Communication bit differs from ETS; download parity is not claimed. | docs/KNOWN_LIMITATIONS.md §145 |
| P2 | Commissioning / Hardware | KL-93 | Parked / no owner | The declarative procedure model still names PID_PROGRAM_VERSION unconditionally, although this does not represent every real procedure. | docs/KNOWN_LIMITATIONS.md §93 |
| P2 | Commissioning / Research | GAP-T30-01 | Research pending | The product-specific meaning of legacy download flags is not fully documented despite a reconstructable matrix. | docs/RESEARCH.md §8.7.15 / docs/superpowers/specs/2026-09-13-commissioning-download-design.md §12 |
| P2 | Commissioning / Research | GAP-T30-02 | Research pending | An established mapping to the load-control subtype is still missing for 13 of the 25 LdCtrl kinds. | docs/RESEARCH.md §8.7.15 / docs/superpowers/specs/2026-09-13-commissioning-download-design.md §12 |
| P2 | Commissioning / Research | GAP-T30-03 | Manufacturer knowledge missing | The actual contribution of EtsDownloadPlugin/Baggage DLLs to transformation or image generation is unknown. | docs/RESEARCH.md §8.6.7 / §8.7.15 |
| P2 | Commissioning / Research | GAP-T30-07 | Research pending | The device- and size-dependent memory-programming delay is not specified as a usable formula or concrete value. | docs/RESEARCH.md §8.7.15 / docs/superpowers/specs/2026-09-13-commissioning-download-design.md §12 |
| P2 | Commissioning / Research | GAP-T30-08 | Normative gap | Load-state-machine realization type 2 remains undescribed in the available standard itself. | docs/RESEARCH.md §8.7.15 / docs/superpowers/specs/2026-09-13-commissioning-download-design.md §12 |
| P2 | Commissioning / Research | GAP-T30-09 | Scoped to profiles | The ordering between different load-state machines is delegated to device-specific profiles that have not yet been evaluated for this purpose. | docs/RESEARCH.md §8.7.14–15 / docs/superpowers/specs/2026-09-13-commissioning-download-design.md §12 |
| P2 | Commissioning / Research | R-DL-01 | Research pending | Ten-octet LdCtrlCompareProp InlineData versus six-octet PID_HARDWARE_TYPE still requires an established framing interpretation. | docs/RESEARCH.md §8.6.7 |
| P2 | Commissioning / Research | R-DL-02 | Accepted safety boundary | Direct programming-mode writing to 60h remains an explicitly excluded risky variant, not missing standard documentation. | docs/RESEARCH.md §8.7.15 / commissioning-download-design.md R11 |
| P2 | Data Integrity / Storage | DATA-02 | Integration boundary | Several structural commands lack complete incremental store synchronization and still require the tested whole-project save. | docs/KNOWN_LIMITATIONS.md: U12 structure editor scope |
| P2 | Data Integrity / Storage | DATA-03 | Distributed-state boundary | If a catalog batch response is lost, the commit status is unknown and automatic retry remains unsafe without a revision/idempotency contract. | docs/KNOWN_LIMITATIONS.md: U11 catalog batch scope |
| P2 | Data Integrity / Storage | KL-87 | Maintenance requirement | Parser corrections reach already stored product rows only through explicit migration and re-derivation. | docs/KNOWN_LIMITATIONS.md §87 |
| P2 | Diagnostics / Audit | AUDIT-01 | Incomplete evidence | Bus-activity data is server-volatile and bounded; serial-number address writes and group writes lack a complete durable receipt trail. | docs/KNOWN_LIMITATIONS.md: Partial commissioning bus-activity snapshot / ADR-0055/0056 |
| P2 | Diagnostics / Audit | KL-137 | Documented boundary | Bus-monitor JSON exports only a bounded telegram window and cannot reconstruct discarded or missed data. | docs/KNOWN_LIMITATIONS.md §137 |
| P2 | Diagnostics / Audit | KL-36 | Documented boundary | The exportable session log is a bounded in-memory window, not a permanently complete audit trail. | docs/KNOWN_LIMITATIONS.md §36 |
| P2 | Diagnostics / Audit | KL-82 | Documented boundary | The diagnostics window's stale-state guard detects changes only within the same browser profile. | docs/KNOWN_LIMITATIONS.md §82 |
| P2 | Documentation / Release | DOC-03 | Outdated statements | Manual chapters contain outdated statements about commissioning, browser export and entry counts and must be reconciled with the finished code. | docs/manual/known-issues.md / docs/manual/ideas-and-roadmap.md |
| P2 | Documentation / Release | RELEASE-03 | Acceptance / decision pending | After U13, the existing user manual needs a location/screenshot decision and verification of every product claim. | goal.md §5 / docs/manual/README.md / ADR-0024 |
| P2 | Documentation / Release | RELEASE-04 | User decision | An alpha tag or public release is not authorized; the AppImage sample demonstrates only the tested Linux scope. | goal.md §5 / docs/ROADMAP.md Session 7 |
| P2 | Domain Model / Topology | KL-127 | Partial / evidence missing | Site/Ground structures are tested synthetically but lack an independent ETS export and installation renaming. | docs/KNOWN_LIMITATIONS.md §127 |
| P2 | Domain Model / Topology | MODEL-01 | Functional boundary | Structural mutation, device moves and new group links partly operate only in the first installation. | docs/KNOWN_LIMITATIONS.md: U11 device editor scope / U12 structure editor scope |
| P2 | Domain Model / Topology | MODEL-02 | Repair workflow missing | Ambiguous imported topology IDs or multiple placements block editing, without automatic UI repair. | docs/KNOWN_LIMITATIONS.md: U11 device editor scope / U12 structure editor scope |
| P2 | Domain Model / Topology | MODEL-03 | Deliberate safety boundary | New device addresses with device octet zero are refused for lack of a verified coupler indicator, while imported values are preserved. | docs/KNOWN_LIMITATIONS.md: U11 device editor scope |
| P2 | Import / Compatibility | IMPORT-05 | Accepted format boundary | Encrypted manufacturer packages are not a supported installation path and remain separate from encrypted project files. | docs/manual/ideas-and-roadmap.md / docs/COMPATIBILITY.md |
| P2 | Import / Compatibility | IMPORT-06 | Documented diagnostic boundary | A present but empty installation DefaultLine is reported as an unresolved reference, although the rest of the import may be usable. | docs/manual/known-issues.md / docs/KNOWN_LIMITATIONS.md §2 |
| P2 | Import / Compatibility | KL-11 | Partial / externally blocked | Further standalone product schemas are unproven, schema 21 is only partly interpreted semantically and VD2 remains explicitly excluded. | docs/KNOWN_LIMITATIONS.md §11 |
| P2 | Import / Compatibility | KL-125 | Evidence missing | The ETS6 interpretation of device-local communication-object IDs is demonstrated in only one project. | docs/KNOWN_LIMITATIONS.md §125 |
| P2 | Import / Compatibility | KL-128 | Functional boundary / decision pending | Legacy VD3–VD5 and PR3–PR5 are rejected atomically but misleadingly; the separate import design is not implemented. | docs/KNOWN_LIMITATIONS.md §128 |
| P2 | Import / Compatibility | KL-15 | Documented boundary | Unreadable attributes outside override fields remain in the report but are not preserved losslessly in the normalized field. | docs/KNOWN_LIMITATIONS.md §15 |
| P2 | Import / Compatibility | KL-2 | Accepted boundary | Without an authoritative XSD, validation is structural rather than a complete schema-conformance check. | docs/KNOWN_LIMITATIONS.md §2 |
| P2 | Localization | KL-14 | Remaining boundary | Imported projects retain an English placeholder as their default language rather than a language established from the project. | docs/KNOWN_LIMITATIONS.md §14 |
| P2 | Localization | KL-37 | Partly pending | Imported translations reach selected surfaces, but not every manufacturer text and output. | docs/KNOWN_LIMITATIONS.md §37 |
| P2 | Localization | KL-64 | Partly pending | Translations outside programs are stored but are not used by every master-data reader. | docs/KNOWN_LIMITATIONS.md §64 |
| P2 | Localization | KL-66 | Partly pending | Some structured diagnostics are localized, while other server-side prose and report details remain English. | docs/KNOWN_LIMITATIONS.md §66 |
| P2 | Manufacturer Data / Parameters | KL-12 | Accepted remaining boundary | Ambiguous DPT lists are disclosed but cannot be manually resolved to a single unambiguous choice. | docs/KNOWN_LIMITATIONS.md §12 |
| P2 | Manufacturer Data / Parameters | KL-135 | Decision pending | Package candidates and differences are visible, but version selection and a user-selected winner are missing. | docs/KNOWN_LIMITATIONS.md §135 |
| P2 | Manufacturer Data / Parameters | KL-146 | Partial / data missing | Channel labels are visible, but indeterminate activations and missing or ambiguous DPTs remain honestly unresolved. | docs/KNOWN_LIMITATIONS.md §146 |
| P2 | Manufacturer Data / Parameters | KL-3 | Partial / evidence missing | Parameter editing works within the demonstrated scope, while additional module and Dynamic semantics remain unclear. | docs/KNOWN_LIMITATIONS.md §3 |
| P2 | Manufacturer Data / Parameters | KL-6 | Accepted boundary | Devices requiring Windows manufacturer plug-in logic remain unconfigurable, and their binaries are not executed. | docs/KNOWN_LIMITATIONS.md §6 |
| P2 | Manufacturer Data / Parameters | KL-68 | Accepted / research pending | Repeated module instances sharing a RefId remain read-only until RepeatIndex mapping is established. | docs/KNOWN_LIMITATIONS.md §68 / docs/RESEARCH.md §4.4 (RepeatIndex) |
| P2 | Manufacturer Data / Parameters | KL-69 | Accepted boundary | A module without a source ID cannot be reliably assigned to a project instance and receives no invented identity. | docs/KNOWN_LIMITATIONS.md §69 |
| P2 | Manufacturer Data / Parameters | KL-71 | Accepted / reimport required | Projects imported before storage schema 6 require reimport so that module-instance IDs permit editing. | docs/KNOWN_LIMITATIONS.md §71 |
| P2 | Manufacturer Data / Parameters | KL-85 | Accepted boundary | Product signatures are stored unchanged but are not cryptographically verified. | docs/KNOWN_LIMITATIONS.md §85 |
| P2 | Manufacturer Data / Parameters | KL-86 | Partly pending | Duplicate normalized product IDs are documented, while DPT provenance for duplicates within a file remains limited. | docs/KNOWN_LIMITATIONS.md §86 |
| P2 | Manufacturer Data / Parameters | PDB-01 | Not implemented | Dynamic Repeat nodes are reported, but their repetitions and project instances are not expanded for editing. | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary |
| P2 | Manufacturer Data / Parameters | PDB-02 | Not implemented | ParameterCalculation and Allocator are preserved and reported, but their scripts and allocation ranges are not evaluated. | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary |
| P2 | Manufacturer Data / Parameters | PDB-05 | Encoding unproven | Color, Picture and Raw values receive only XML/non-empty checks rather than semantically established value validation. | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary |
| P2 | Manufacturer Data / Parameters | PDB-06 | Diagnostic boundary | Hidden or unexpanded Dynamic subtrees and activation budgets do not always produce an individual diagnostic for every subordinate reference. | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary |
| P2 | Manufacturer Data / Parameters | PDB-08 | Typing missing | DPT layouts, PublicKeys, OrderNumberFormattingScript and further master resources partly remain untyped bytes with diagnostics. | docs/KNOWN_LIMITATIONS.md: PDB-3 report history and coverage boundary |
| P2 | Manufacturer Data / Parameters | PDB-10 | Semantics / evidence missing | Secure capacities, MinEtsVersion and ReplacesVersions are raw strings, not verified capabilities or interpretable version relationships. | docs/KNOWN_LIMITATIONS.md: PDB-7 catalogue metadata are source strings |
| P2 | Manufacturer Data / Parameters | R-DYNAMIC-01 | Research pending | Dynamic structural grammar, the no-match choose rule and additional Access/Visible gates are not fully established by normative evidence. | docs/RESEARCH.md §4.3 / Open questions |
| P2 | Manufacturer Data / Parameters | R-MODULE-03 | Sample missing | AllocatorRef module arguments have no practical corpus sample or sufficient normative example. | docs/RESEARCH.md §4.4 / docs/KNOWN_LIMITATIONS.md: PDB-9 |
| P2 | Manufacturer Data / Parameters | R-MODULE-04 | Independent evidence missing | Nested module definitions are expanded within bounds but lack a real manufacturer example or a normative AP structural definition as evidence. | docs/RESEARCH.md §4.4 / docs/GAP_ANALYSIS_ETS.md A3 |
| P2 | Platform / Maintainability | KL-133 | Not reproduced | A dead webview might no longer be closable through the window button because of the JS close guard. | docs/KNOWN_LIMITATIONS.md §133 |
| P2 | Reports / Diff / CSV | KL-38 | Evidence missing | The custom group-address CSV format is not verified against ETS CSV or ESF/OPC exchange. | docs/KNOWN_LIMITATIONS.md §38 |
| P2 | Reports / Diff / CSV | KL-39 | Accepted boundary | CSV supports explicit readdressing and deletion but cannot create, rename or modify group ranges. | docs/KNOWN_LIMITATIONS.md §39 |
| P2 | Reports / Diff / CSV | KL-40 | Decision pending | Derived CSV columns remain validated read-only fields and Description/Comment columns are missing; the remaining boundary has not yet been explicitly accepted. | docs/KNOWN_LIMITATIONS.md §40 |
| P2 | Reports / Diff / CSV | KL-44 | Evidence missing | The HTML project report is a custom format with no demonstrated content or layout parity with ETS reports. | docs/KNOWN_LIMITATIONS.md §44 |
| P2 | Reports / Diff / CSV | KL-47 | Partial / research pending | Parameter values and module arguments are output, but unverified formats and AllocatorRef semantics remain raw and carry warnings. | docs/KNOWN_LIMITATIONS.md §47 |
| P2 | Reports / Diff / CSV | KL-51 | Evidence missing | Native project comparison has not been checked for matching or content parity against an ETS comparison result. | docs/KNOWN_LIMITATIONS.md §51 |
| P2 | Reports / Diff / CSV | KL-60 | Decision pending | Large diff tables are displayed page by page; the remaining paging boundary still requires an explicit decision. | docs/KNOWN_LIMITATIONS.md §60 |
| P2 | Security / Operations | R-SEC-01 | Research deferred | The source of usable Data Secure runtime keys in project files versus keyrings has not yet been practically verified. | docs/RESEARCH.md §9 / Open questions |
| P2 | UI / Acceptance | UI-03 | Live acceptance missing | Read-only device comparison and the readiness view are tested with mocks and the simulator, but not through real web operation on a device. | docs/KNOWN_LIMITATIONS.md: Device-checks UI boundary |
| P2 | UI / Acceptance | UI-04 | Operations display pending | The global bus-activity bar and an action history are missing despite existing partial server-side activity data. | docs/KNOWN_LIMITATIONS.md: Partial commissioning bus-activity snapshot / ADR-0055/0056 |
| P2 | UI / Accessibility | KL-130-ZOOM | Native acceptance pending | Zoom and pane geometry are verified in Chromium but not in the native WebKitGTK window. | docs/KNOWN_LIMITATIONS.md §130 (Zoom) |
| P2 | UI / Accessibility | KL-20 | Acceptance pending | Shared overlays and keyboard controls are implemented, but native WebKitGTK and screen-reader acceptance are missing. | docs/KNOWN_LIMITATIONS.md §20 |
| P3 | Bus / Discovery / Scan | KL-124 | Display boundary | Interface discovery displays only some of the discovery properties already decoded. | docs/KNOWN_LIMITATIONS.md §124 |
| P3 | Bus / Discovery / Scan | KL-76 | Deliberate boundary | Negative Layer 2 acknowledgements have no accelerated scan path, and indeterminate results have no automatic retry. | docs/KNOWN_LIMITATIONS.md §76 |
| P3 | Bus / DPT / Protocol | KL-102 | Test boundary | No actually reachable public trigger is known for the write-echo decode-error branch. | docs/KNOWN_LIMITATIONS.md §102 |
| P3 | Bus / DPT / Protocol | KL-110 | Accepted medium boundary | The PL110-specific Group-Responser table is not implemented for the TP1/RF/IP project scope. | docs/KNOWN_LIMITATIONS.md §110 |
| P3 | Commissioning / Hardware | KL-115 | Remaining integration work | Calculated restart and Master Reset delays do not yet consistently have a caller that waits for them. | docs/KNOWN_LIMITATIONS.md §115 |
| P3 | Commissioning / Research | GAP-T30-04 | Implementation decision | The selection strategy for differentially changed download chunks is implementation-dependent, not a missing mandatory normative algorithm. | docs/RESEARCH.md §8.7.15 / docs/spec-audits/2026-09-19-cp-3_5_3-partial-download.md |
| P3 | Data Integrity / Storage | HISTORY-01 | Deliberate boundary | Undo history is session-bound and is not persistently stored with the native project. | docs/GAP_ANALYSIS_ETS.md B11 |
| P3 | Data Integrity / Storage | HISTORY-02 | Functional boundary | Autosave does not replace versioned backups or project time travel; no finished workflow exists for these. | docs/GAP_ANALYSIS_ETS.md C1 / D8 |
| P3 | Documentation / Release | DOC-02 | Inventory boundary | Known Limitations uses number 130 twice and leaves further boundaries unnumbered, so follow-up audits need stable unique IDs. | docs/KNOWN_LIMITATIONS.md / docs/LIMITATION_TRIAGE.md |
| P3 | Domain Model / Topology | MODEL-04 | Deliberate boundary | Catalog batches do not automatically assign individual addresses or installation-wide unique names. | docs/KNOWN_LIMITATIONS.md: U11 catalog batch scope |
| P3 | Domain Model / Topology | MODEL-05 | New ADR required | Combining separate installations under a shared Site is not modeled. | docs/RESEARCH.md §17.3 |
| P3 | Domain Model / Topology | MODEL-06 | Accepted boundary | A spatial floor-plan or topology canvas is deliberately absent; later placements need a separate model and storage design. | goal.md §6 / ADR-0019 |
| P3 | Domain Model / Topology | MODEL-07 | Scope / ADR pending | Project-related KNX Functions are researched for schema 23 but need a domain model, mapping, migration, UI and samples from older schemas. | docs/RESEARCH.md §15 / docs/ROADMAP.md: Functions |
| P3 | Import / Compatibility | IMPORT-01 | Functional boundary | Selective import of individual lines or devices from a project is not implemented. | docs/GAP_ANALYSIS_ETS.md C3 |
| P3 | Import / Compatibility | IMPORT-02 | Optional / ETS access required | Different internal ETS restore-point revisions could serve as additional private import regressions only after authorized ETS export. | docs/RESEARCH.md: local installation data (uncommitted addition) |
| P3 | Import / Compatibility | IMPORT-03 | Optional implementation | An ETS CommunicationLog XML importer is missing; existing private decoder samples require explicit sanitization before versioned regression use. | docs/RESEARCH.md: telegram-capture audit (uncommitted addition) / goal-commission.md K19 |
| P3 | Import / Compatibility | IMPORT-04 | Research not evaluated | The additional knx_cvexc XML files are inventoried but not semantically evaluated and provide no new compatibility evidence. | docs/RESEARCH.md: local installation data (uncommitted addition) |
| P3 | Localization | KL-100 | Deliberate boundary | Help texts are plain catalog paragraphs without Markdown, rich text or separate topic files. | docs/KNOWN_LIMITATIONS.md §100 |
| P3 | Localization | KL-48 | Accepted remaining boundary | Reports have German/English basic elements but no complete prose catalog or dedicated language selector. | docs/KNOWN_LIMITATIONS.md §48 |
| P3 | Manufacturer Data / Parameters | KL-134 | Deliberate boundary | Baggage is inventoried, but media, manuals, installation instructions and nested archives are not used or executed. | docs/KNOWN_LIMITATIONS.md §134 |
| P3 | Manufacturer Data / Parameters | KL-70 | Deliberate safety boundary | Declared parameters that are not currently displayed are rejected on write. | docs/KNOWN_LIMITATIONS.md §70 |
| P3 | Manufacturer Data / Parameters | KL-88 | Deliberate boundary | The manufacturer display name follows the most recently read master rather than a historically selectable version. | docs/KNOWN_LIMITATIONS.md §88 |
| P3 | Manufacturer Data / Parameters | PDB-03 | Partial / safety boundary | Conditional Rename/ParameterBlockRename nodes do not change UI labels, and Button EventHandlers are not executed. | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary |
| P3 | Manufacturer Data / Parameters | PDB-04 | Display boundary | Additional Type attributes and UIHints are reported but are not yet used for specialized time, color, picture or slider editors. | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary |
| P3 | Manufacturer Data / Parameters | PDB-07 | Historical boundary | Older installation reports remain historical snapshots even when migrations correct current blob evaluations. | docs/KNOWN_LIMITATIONS.md: PDB-9 / PDB-3 report history and coverage boundary |
| P3 | Manufacturer Data / Parameters | PDB-11 | Evidence boundary | For installations predating the encounter ledger, individual installation metrics are deliberately unavailable rather than misleadingly zero. | docs/KNOWN_LIMITATIONS.md: PDB-3 report history and coverage boundary |
| P3 | Platform / Extensions | FUTURE-01 | New scope decision | MCP/LLM interaction is researched but needs a new product-feature decision only after a revision, permission, validation and audit contract is defined. | docs/RESEARCH.md §13 / goal.md §7 |
| P3 | Platform / Extensions | FUTURE-02 | New scope decision | Generic macros and automation need a named first operation with deterministic preview and atomic undo-capable apply. | docs/RESEARCH.md §14 / goal.md §7 |
| P3 | Platform / Extensions | FUTURE-03 | Not scheduled | Project notes have an accepted design in ADR-0031 but no scheduled end-to-end implementation yet. | ADR-0031 / goal.md §7 / docs/ROADMAP.md: In-application help |
| P3 | Platform / Extensions | FUTURE-04 | Design / scope pending | A who-talks-to-whom view needs frozen sender/receiver evidence and suitable stale-state rules, not just an animated graph. | docs/RESEARCH.md §16 / goal.md §7 |
| P3 | Platform / Extensions | FUTURE-05 | Not scheduled | Online manufacturer-catalog updates have no defined work assignment; existing local package installation does not replace an online service. | goal.md §6 / docs/GAP_ANALYSIS_ETS.md C6 |
| P3 | Platform / Extensions | FUTURE-06 | Accepted platform boundary | Mobile applications and non-Linux desktop platforms are outside the current Linux-first delivery scope. | goal.md §6 |
| P3 | Platform / Extensions | FUTURE-07 | User-owned | The project logo belongs to the user and is not an authorized implementation task for the agent. | goal.md §6 |
| P3 | Platform / Extensions | KL-107 | Accepted boundary | There is deliberately no code plug-in API; new code functionality requires a fork or a later architectural decision. | docs/KNOWN_LIMITATIONS.md §107 |
| P3 | Platform / Maintainability | KL-16 | Upstream dependency | The Linux desktop shell remains on GTK3; the earlier archiving advisories, however, have been resolved. | docs/KNOWN_LIMITATIONS.md §16 |
| P3 | Platform / Maintainability | KL-42 | Documentation pending | The command_sync module documentation overstates the incremental storage actually used. | docs/KNOWN_LIMITATIONS.md §42 |
| P3 | Platform / Maintainability | KL-65 | Documented boundary | Version output names the build commit but does not indicate an uncommitted working state. | docs/KNOWN_LIMITATIONS.md §65 |
| P3 | Reports / Diff / CSV | KL-41 | Accepted boundary | Commas and semicolons work, but further spreadsheet transformations are not verified. | docs/KNOWN_LIMITATIONS.md §41 |
| P3 | Reports / Diff / CSV | KL-45 | Accepted boundary | Native PDF generation is deliberately absent; the existing HTML report uses browser printing as a fallback. | docs/KNOWN_LIMITATIONS.md §45 |
| P3 | Reports / Diff / CSV | KL-46 | Data-dependent boundary | Readable product and program names in the report require matching installed product data; otherwise raw references appear with a warning. | docs/KNOWN_LIMITATIONS.md §46 |
| P3 | Reports / Diff / CSV | KL-52 | Accepted boundary | Devices without a matching individual address and ETS ID cannot be reliably correlated between projects. | docs/KNOWN_LIMITATIONS.md §52 |
| P3 | Reports / Diff / CSV | KL-53 | Accepted boundary | Same-named sibling building parts produce ambiguous comparison keys rather than a unique mapping. | docs/KNOWN_LIMITATIONS.md §53 |
| P3 | Reports / Diff / CSV | KL-54 | Accepted boundary | Reference IDs regenerated on ETS reimport can appear as new rather than identical objects. | docs/KNOWN_LIMITATIONS.md §54 |
| P3 | Reports / Diff / CSV | KL-55 | Accepted boundary | A displayed project diff cannot be applied to a project as a sequence of changes. | docs/KNOWN_LIMITATIONS.md §55 |
| P3 | Reports / Diff / CSV | KL-56 | Accepted boundary | Three-way comparison using a shared project base is not implemented. | docs/KNOWN_LIMITATIONS.md §56 |
| P3 | Reports / Diff / CSV | KL-9 | Documented boundary | SQLite project files cannot be compared textually in version control, although in-application project comparison exists. | docs/KNOWN_LIMITATIONS.md §9 |
| P3 | Tools / Handover | TOOLS-06 | Local only / historical | Paperclip shutdown handovers are unversioned, and their open historical board steps are valid only after reconciliation with work that has since landed. | docs/paperclip-shutdown/STATUS.md / goal.md §12 |
| P3 | Tools / Telemetry | TOOLS-01 | Versioning pending | The AI statistics generator, helper module and tests live outside the repository; their supported location and publication need a separate decision. | .ai/CURRENT_STATE.md (local statistics handover) / docs/AI_STATS_TELEMETRY_PLAN.md |
| P3 | Tools / Telemetry | TOOLS-02 | Measurement basis missing | Actual historical network/TLS and remote-job traffic remains unmeasurable without approved attributable instrumentation. | docs/AI_STATS_TELEMETRY_PLAN.md |
| P3 | Tools / Telemetry | TOOLS-03 | Evidence missing | Reliable execution end times, tool outcome rates and cache savings remain only partly measurable depending on the provider. | docs/AI_STATS_TELEMETRY_PLAN.md / .ai/CURRENT_STATE.md (local) |
| P3 | Tools / Telemetry | TOOLS-04 | Coverage boundary | Unknown-provider sessions, cloud work without unambiguous attribution and AI authorship are not filled in using estimates. | docs/AI_STATS_TELEMETRY_PLAN.md / .ai/CURRENT_STATE.md (local) |
| P3 | Tools / Telemetry | TOOLS-05 | Parser boundary | Source-code metrics have limited CSS/Shell structural coverage and version-dependent parsers rather than compiler validation. | docs/AI_STATS_TELEMETRY_PLAN.md |
| P3 | UI / Accessibility | FUTURE-08 | Optional idea | A larger catalog of humorous messages is desired but is neither prioritized feature work nor a data-integrity prerequisite. | docs/manual/ideas-and-roadmap.md |
| P3 | UI / Accessibility | KL-121 | Documented boundary | Changed settings reach other already open windows only after reload. | docs/KNOWN_LIMITATIONS.md §121 |
| P3 | UI / Accessibility | KL-43 | Partly pending | Motion controls and OS reduced-motion support exist but do not cover every animation surface or individual category. | docs/KNOWN_LIMITATIONS.md §43 |
| P3 | UI / Accessibility | KL-97 | Documented boundary | Most loading phases provide honest phase text but no measurable percentage. | docs/KNOWN_LIMITATIONS.md §97 |
| P3 | UI / Accessibility | KL-98 | Cosmetic boundary | Fast project loading usually leaves the later rotating humor lines unseen. | docs/KNOWN_LIMITATIONS.md §98 |
| P3 | UI / Accessibility | UX-01 | Accepted remaining scope | Drag-and-drop covers targeted device moves, not generic reordering, multi-drag or group-address linking without direction selection. | docs/GAP_ANALYSIS_ETS.md B10 / docs/IMPLEMENTATION_STATUS.md T11 |
| P3 | UI / Accessibility | UX-02 | Remaining UX inconsistency | The catalog picker still offers VD2 for selection, although the installer always explicitly rejects this format. | apps/knx-web/src/CatalogBrowser.tsx / docs/manual/known-issues.md |
| P3 | UI / Accessibility | UX-03 | Functional boundary | The group-address style is selected when creating a project, but there is currently no UI switch to change it afterward. | apps/knx-web/src/NewProjectDialog.tsx / apps/knx-web/src/messages/en.ts |

## Resolved, withdrawn or not an open task

These items are explicitly **not** scheduled again as open implementation work; old headings or goal text may need to be updated accordingly.

| ID / Topic | Classification | Brief explanation | Source |
| --- | --- | --- | --- |
| KL-18 | Already fixed / outdated documentation | ETS import sets the store path to None through replace_project_state rather than reusing the previous file. | docs/KNOWN_LIMITATIONS.md §18 / apps/knx-server/src/domain.rs |
| KL-23 | Already fixed / outdated documentation | Project files are already streamed in bounded chunks; only temporary SQLite serialization remains. | docs/KNOWN_LIMITATIONS.md §23 / apps/knx-server/src/fs_routes.rs |
| KL-24 | Already fixed / outdated documentation | Multiple-file upload and file drop are implemented; the project to open afterward is still selected individually. | docs/KNOWN_LIMITATIONS.md §24 / apps/knx-web/src/FsPicker.tsx |
| KL-90 | Not a defect / clarification | 46 denotes a count in the master, not a missing DPT main type. | docs/KNOWN_LIMITATIONS.md §90 |
| KL-95 | Not a defect / guidepost | The entry documents six standard-text rulings rather than six further open implementation issues. | docs/KNOWN_LIMITATIONS.md §95 |
| ETS project export / reimport | Withdrawn | KNXBench does not write `.knxproj` files; signing and ETS reimport of such an export are therefore not open delivery goals. | ADR-0028 / `goal.md` §6 |
| GAP-T30-05 / -06 / -10 | Withdrawn, reclassified or closed | Extended discovery is documented, programming-mode parity is a risk decision and Authorize parameters are clarified. | `docs/RESEARCH.md` §8.7.15 / Commissioning design §12 |
| ISSUE-04 acceptance | Demonstrated / closed | Saved-baseline and localized last-save acceptance are already verified and checked off. | `goal-ui.md` status / `goal.md` §12.2 |
| Earlier GTK3 advisories | Outdated issue description | The old archiving advisories are withdrawn; the GTK3 platform dependency remains as KL-16. | `docs/KNOWN_LIMITATIONS.md` §16 |
| CLA / general license choice | Decided | ADR-0054 supersedes the CLA/dual-licensing design; it does not leave an open CLA work assignment. | ADR-0053/0054 |
| Historical plan checkboxes / Paperclip packages | Not automatically open | Earlier implementation plans were reconciled with the current delivery state rather than redispatching every unchecked individual instruction. | `docs/IMPLEMENTATION_STATUS.md` / `goal.md` §12 / local shutdown handovers |
| Architectural decisions | No explicitly proposed ADR found | Accepted ADRs can still have outstanding implementation, particularly ADR-0039, without the architectural decision itself being pending again. | `docs/adr/` / `docs/adr/README.md` |

## Key decisions and authorization boundaries

- **Hardware:** Establish reliable identity and durable device-/action-specific recovery first; do not transfer an old authorization to a new candidate or attempt.
- **UI completion:** Review U13 independently and limit ISSUE-12 evidence checkboxes to the evidence actually delivered; native WebKitGTK/screen-reader samples remain separate.
- **Remaining scope:** Explicitly decide remaining CSV fields/paging, package-version selection and later features; do not silently reopen already accepted DIN-26 boundaries.
- **Delivery:** Accept the manual, obtain the alpha decision and only then complete the final whole-goal evidence; this audit does not authorize a release.
- **Status maintenance:** Reconcile demonstrated contradictions in the goal, triage and manual; this file documents them but changes neither other work's status sources nor product functionality.

## Audited source groups and boundaries

- `docs/PROJECT_CONTEXT.md`, `.agent-memory/PROJECT_MEMORY.md` as a discovery aid, `AGENTS.md` and `CLAUDE.md`.
- `docs/KNOWN_LIMITATIONS.md`, `docs/LIMITATION_TRIAGE.md`, `docs/IMPLEMENTATION_STATUS.md`, `docs/ROADMAP.md`, `docs/GAP_ANALYSIS_ETS.md`, `docs/COMPATIBILITY.md`, `docs/IMPORT_EXPORT.md`, `docs/ARCHITECTURE.md`, `docs/DATA_MODEL.md`.
- `docs/RESEARCH.md`, `docs/PRODUCT_DATABASE_CORPUS.md`, `docs/VD4_PRODUCT_DATABASE_IMPORT.md`, `docs/PLUGIN_FEASIBILITY.md`, `docs/PERFORMANCE.md`, `docs/LANGUAGE_PACKS.md` and relevant specification audits.
- `goal.md`, `goal-ui.md`, `goal-commission.md`, `docs/adr/`, current user-issue planning and technical designs under `docs/superpowers/`.
- `docs/manual/`, particularly Known Issues, ideas/roadmap and acceptance notes; `docs/Issues.md` is empty at the audited reference revision.
- Local additions in `docs/RESEARCH.md`, `.ai/CURRENT_STATE.md`, `docs/AI_STATS_TELEMETRY_PLAN.md` and `docs/paperclip-shutdown/`; historical side-project handovers are not automatically part of the current KNXBench product backlog.
- Targeted source-code checks included store-path publication, streaming, the file picker, catalog, ID allocation, browser project export and new-project texts.

**Validity boundary:** This is a dated snapshot of the stated commit plus explicitly marked local additions. New commits, external issues and undocumented defects are not implicitly covered. Historical test evidence was read as such, not rerun. Private corpus files, credentials and telegram payloads are not part of this file. The list may be long; at least no bug is secretly hiding behind the fuse box.
