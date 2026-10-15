# ADR-0131: universal support evidence is not download admission

- Status: Accepted (owner's universal-reporting clarification and explicit Go, 2026-10-15)
- Date: 2026-10-15
- Scope: source-built web/server support reporting, independent of issue #22's programming prerequisites

## Context

A refusal before planning or connecting cannot produce the existing ended-download report. An unknown device may have no catalogue match, executable plan, reviewed diagnostic profile, gateway or project. Requiring any of these to report the problem would exclude precisely the devices that need investigation.

Existing facilities remain authoritative: product-database identity and program parsing, download-support classification, selected-project planning, contribution analysis, ended-download reports, authenticated server routes, contactable-address admission and read-only management sessions. This change does not replace those services or relax their safety checks.

## Decision

1. `knx-app::support_report` produces a separate, versioned, privacy-allowlisted evidence envelope. Missing evidence is a result, not a download failure. Known supported/untested/unsupported/excluded states are reused where they can actually be established; an unresolved source remains unknown.
2. Every fact states its origin and availability. User assertions, product declarations and live observations stay distinct. An optional ended-run projection carries only the typed outcome and exact scope; neither transport completion nor a user's functional answer changes the verification registry.
3. The default export contains allowlisted technical application identity, source hash/version where known, manually supplied firmware/mask, support/refusal category and missing/withheld evidence. It excludes installation names/addresses, gateways, credentials, parameter values, original files, binary/firmware/raw memory, source names and free-text errors. Manual fields must still be checked by the operator; this is an allowlist, not a promise to recognize every secret someone deliberately types.
4. Optional source context needs a separate rights assertion and is rebuilt from a reviewed projection: numeric memory ranges and known load-control fields only. Source bytes, inline comparison values, unknown XML/attributes and textual IDs are not attachments. Omitted/unsupported/truncated context stays visible. Neither a public URL nor a hash proves redistribution rights.
5. `knx-net::commissioning::support_diagnostics` owns fixed, versioned read-only recipes. The server admits one selected or manually addressed target, displays its exact read list locally, requires explicit acquisition consent and rechecks a fresh source/project snapshot. It reuses gateway exclusion/lease mechanisms; no arbitrary client property, memory address, script, key, setup, authorization, write or restart is accepted.
6. A device descriptor is read first. Family reads run only when an expected mask was explicitly previewed and matches the observed mask. Unknown masks or absent expected masks stay descriptor-only; they do not inherit another family's recipe. Diagnostics cannot enable programming.
7. Router-local snapshots are bounded to one replaceable preview, expire for use after fifteen minutes, and bind export to the displayed report digest. Responses are `no-store`. Live acquisition invalidates prior disclosure consent. Project/source/revision changes refuse stale acquisition/export. The ZIP contains only `report.json` and `README.txt`; save and later handoff are explicit user actions.
8. The UI offers this workflow from the File menu with or without a project and alongside device-download guidance. Existing product contribution and ended-run reports remain available. There is no receiving/upload service, automatic attachment, private-original intake promise or automatic support promotion.

## Verified diagnostic evidence

Authorized local KNX Standard PDFs were read directly; licensed pages/excerpts remain private, not repository assets:

- `03_05_01 Resources v01.10.01 AS.pdf`, mask/profile table, printed p.25: 0012h is BCU 1/System 1; 0020h/0021h/0025h are BCU 2/System 2.
- `06 Profiles v02.01.01.pdf`, §4.4.1.1, printed p.45: connection-oriented BCU 1 programming mode uses memory-mapped realisation type 2. Resources §4.26.3, printed pp.303–304, places the programming-mode bit at 0060h/bit 0. The new memory recipe is deliberately limited to mask0012h, one octet read, bit0 exported; other bits never leave the adapter. In particular 0701h's E-Mode caveat is not inferred from its mask.
- Resources §4.3.28, printed pp.78–79: Device Object hardware type is PID78 **decimal** (0x4e), six octets; manufacturer identity uses PID12. Resources §4.23.2 defines property-based load-state access. The retained MV-0025 master resource mapping supplies objects1/2/3, PID5 for the three machines; this recipe is limited to0025, rather than borrowing its mapping for0020/0021. No field is inferred from firmware bytes.

These are protocol/source-level recipe checks, not physical-device certification. A missing/refused/malformed value remains unavailable. No actual bus was contacted during implementation.

## Consequences and boundaries

The domain model and native schemas do not change; external source parsing stays in the product-database adapter. The database adds a small actual-blob-length query so report generation can refuse an unavailable or >8MiB source before parsing/planning. A separate metadata-only preflight reuses the existing parameter type loader to cap cumulative declared padded Text buffers at8MiB before the reporter calls plan assessment; inactive/duplicate references count conservatively. An unavailable/over-budget assessment stays unknown with an explicit gap; useful source context is retained. Neither helper changes the image encoder, programming policy or native model. This does not claim a whole-process resource bound. No dependency is added.

Reports can guide a maintainer toward the remaining image/table/LSM/recovery contracts and later target-specific verification. They cannot establish a complete System-2 programming plan or make issue #22's device programmable. Unsupported contexts and additional families require their own reviewed projection/profile and regression evidence. Existing report formats are not silently repurposed.

See the [behavioural contract](../contracts/universal-device-support.md).
