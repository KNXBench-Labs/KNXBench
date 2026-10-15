# Universal device-support reporting

[ADR-0131](../adr/0131-universal-device-support.md). A support report is investigative evidence, **not** a download plan, unlock token or device certification. Available in source builds after Alpha11; not a released-binary claim.

## Offline envelope, format1

`kind: knxbench-support-report`, `format: 1`, `knxbenchVersion`, `support`, `written: false`, `facts`, optional `context`, `missingEvidence`, `withheld`, optional `priorRun` and `priorRunScope`.

A fact has `field`, `origin`, `status` and optional `value`. Origins distinguish `user`, `project`, `catalogue`, `product`, `offline-planner`, `product-default-planner`, `target-policy` and `live`. Known target exclusions are reported even without catalogue data or a project (manual targets); no installation address is copied into that policy fact. A user's firmware string or mask is not a live observation. A conflicting allowlisted manual application ID is retained separately as `assertedApplicationId`, with an explicit identity-conflict gap; it does not replace the selected source or satisfy an ended-run source match. `unknown` means insufficient source evidence, not a successful or failed hardware test. Established support levels come from the existing classifiers/planner; collection never writes the verified-download registry.

An absent project/device/catalogue/application/plan/gateway/profile is reportable. `missingEvidence` names actionable gaps (for example product identity, matching package, readable source, firmware, family diagnostics, recovery coverage and hardware verification). A failure to read source data does not become support. The general missing list is intentionally not an exhaustive per-device engineering checklist.

The default includes only allowlisted, source-witnessed application ID shapes (including the observed `-O` hash suffix), bounded firmware tokens, mask/application version, valid source SHA256 and stable refusal categories where available. Project name, device name, building/room, individual/group addresses, gateway, serial number, keys, configuration/parameter values, paths, raw errors and original files are not copied. Manual inputs remain the user's responsibility: do not enter customer information or secrets into technical identity fields.

Optional `context` requires both `includeContext` and `contextRights`. A reviewed projection exposes at most128 numeric segment ranges and256 known load-step summaries. Original segment IDs, application names, inline comparison values, unknown XML/attributes, memory data, firmware and full product/project/specification files remain withheld. Unknown steps have explicit withheld markers and an omission count; budget overflow sets `truncated`. Actual source blobs are limited to8MiB before parsing/planning, independently of a possibly stale metadata length. Before optional plan assessment, ProductDB also preflights a cumulative8MiB budget for declared padded Text buffers across parameter references, without encoding values; inactive/duplicate references count conservatively. Missing/invalid/oversized assessment metadata leaves support unknown with `plan-assessment-unavailable-or-over-budget`, while safe source identity/context stays available. These are investigation budgets, not new programming limits or a whole-process memory/time guarantee. Missing/legacy/oversized sources remain partial, never guessed.

An optional ended download reuses the existing typed outcome and exact `complete`/partial scope only. It does not copy that older report's parameter values, names, warnings, inferences or arbitrary text. `written: false` concerns **this report operation**; `priorRun.written` concerns the earlier run. Transport completion and a manually reported functional result stay separate. The original ended-download report remains an explicit, separate workflow with its existing parameter-value disclosure rules.

## HTTP admission and lifetime

The existing authenticated/session-guarded router owns three POST endpoints. No CLI/MCP protocol or native-store migration is introduced.

- `/api/support/preview`: `{deviceId?, address?, input?, downloadId?, context?}`. The two target forms are mutually exclusive. A selected device requires current server/project incarnation and revision in `context`; a manual individual address needs no project and stays local. All input objects reject unknown fields; the router body limit is8192 bytes.
- Response: `{token,digest,report,summary,localAddress,mask,profile,reads,deadlineSeconds}`. Only `report` and `summary` enter the ZIP; `localAddress`, target/preview tokens and gateway do not. `reads` is the exact logical recipe shown before optional contact.
- `/api/support/diagnose`: `{token,gateway,consent:true}`. One explicitly selected IPv4 KNXnet/IP endpoint; no DNS, sweep or gateway discovery by this endpoint. Excluded/noncontactable/ambiguous targets, stale snapshots, busy shared gateway users and false consent refuse before connection. Connection timeout10s, collection timeout30s, tunnel cleanup bounded to3s. The response advertises the30s collection budget, not a promised30s end-to-end transaction.
- `/api/support/export`: `{token,digest,consent:true}`. The exact current digest, unchanged project/source and explicit disclosure consent are mandatory. An earlier preview/digest cannot export a replacement/live-enriched report.

One router-local in-memory snapshot replaces the prior one; use expires after15minutes. There is no disk report history. All route responses have `Cache-Control: no-store`; another preview/tab can invalidate the current token. Consent is per action and cannot be carried across input/target/project changes. Each returned acquisition replaces the previous live projection rather than mixing unlabelled observations from different attempts/gateways; unavailable new fields cannot retain an old successful value. This is a current preview, not an acquisition history. Save a report deliberately before replacing it if needed.

The ZIP has fixed paths `report.json` and `README.txt`, no original assets or arbitrary filenames; JSON output is capped at512KiB. Save is local download; opening the fixed GitHub issue form or mail draft does not submit or attach anything. Email is not an encryption, privacy, delivery, retention or deletion guarantee. Unknown redistribution rights mean withholding content, not switching it to a private-mail workaround.

## Built-in read-only recipes v1

Each session uses free-access/read-only management admission, no key/authorize, VerifyMode, setup, writes or restart. A descriptor type0 read precedes and verifies any previewed family mask. A mismatch refuses; absent/unknown expected mask performs **only** the descriptor read, without automatically escalating to the newly discovered family.

- `system2-status-v1`, mask0025: Device Object0 PID12 (2octets) and PID78 decimal (6octets), then object1/2/3 PID5 (1octet each), index1/one element. Only expected lengths and known single-byte state values are exported; unavailable/refused/malformed fields are explicit. Five property reads plus descriptor, no fallback reads.
- `programming-mode-v1`, mask0012: descriptor plus one-byte MemoryRead at0060h; export only decoded bit0, never the shared raw octet or parity/other bits.
- `descriptor-only-v1`: all other families, or no expected mask. More detailed reporting stays possible offline; safe access is not inferred from a similar mask.

Source/spec citations and deliberately excluded0701h applicability are in ADR0131. These recipes have simulated read-only regression coverage, **not** a real-device support claim. A failed acquisition is represented as unavailable/partial rather than fabricated facts; a whole-session deadline can leave no collected facts in the returned report.

## Tests and remaining engineering boundary

Mandatory evidence covers wholly unknown/manual/project targets, catalogue refusal, source acquisition/output limits, unsupported/withheld context, exact ZIP contents, stale/replaced preview/export bindings, rights/consent and request grammar, excluded addresses and no bus access on refusal. Simulated transports assert bounded reads, expected-mask mismatch, unexpected property lengths and absence of write/setup/authorize/restart services. UI fixtures cover separate acquisition/disclosure consents, stale responses, context rights, input invalidation and deliberate manual handoff in EN/DE at1440/400px.

No real hardware operation, receiving backend, automatic upload, vendor contact, registry promotion, complete arbitrary-property/memory exploration, Secure/IP-Secure commissioning or System-2 download/recovery implementation is part of this feature. Issue22 stays open until the actual programming/recovery work has independent evidence.
