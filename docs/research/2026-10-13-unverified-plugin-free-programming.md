# Programming unverified devices without manufacturer extensions

Date: 2026-10-13
Status: Local research / proposal only; no implementation or new write authorization.
Reviewed source: `origin/main` at `113d3ceaf03644b4bf03ab6172063402f955a8ea`.

## Question and terminology

An operator wants to program devices that KNXBench has not verified, initially excluding devices that require manufacturer DLLs. This note interprets “not approved” as missing **KNXBench hardware verification**, not missing KNX certification, device access credentials or permission to operate someone else's installation. Those are different questions.

## Existing behaviour — reuse, do not duplicate

- `knx_app::download_support` distinguishes `verified`, `untested` and `unsupported`. The evidence file records application programs and operation scopes; it is not a whitelist limiting downloads to tested products.
- A complete plan with no matching live evidence is `untested`. The server recomputes this status, requires the target-specific `acceptUntested` phrase and compares the newly prepared plan with the displayed plan before opening the write path. The web panel already collects this extra acknowledgement.
- A preparation failure is a technical refusal, not a request for approval. The current planner requires a supported memory-load mask and `ProductProcedure`. Parameter image, keying and execution constraints also apply.
- ADR-0086 permits support based on specifications, product databases and project evidence without owning a device of every family. Disclosed, source-consistent working inferences are permitted; pure guesses and contradictory evidence are not. Only an actual reviewed hardware run can promote a scope to `verified`.

Repository sources: [ADR-0049](../adr/0049-download-readiness-is-per-plan-and-backups-are-pre-write.md), [ADR-0086](../adr/0086-specs-product-data-and-projects-are-enough-evidence.md), `crates/knx-app/src/download_support.rs`, `crates/knx-app/src/device_download.rs`, `crates/knx-productdb/src/download_plan.rs`, `apps/knx-server/src/device_download_routes.rs` and `apps/knx-web/src/DeviceDownloadPanel.tsx`.

## Why excluding DLLs is necessary but insufficient

Absence of a DLL file must not be treated as proof that all required device behaviour is covered by KNXBench. Proposed scope must examine unresolved dependencies as well as the parameter image and load procedure.

External-source check: the configured extractor cannot retrieve URL content, and direct HTTP requests to the two KNX support article leads returned HTTP 403 on 2026-10-13. Their contents were **not retrieved or inspected**; this note makes no source-backed claim about their contents. Those failed leads are not evidence for an admission rule.

Proposed initial admission scope: no unresolved required manufacturer executable/plug-in/DCA behaviour; necessary product/application data present; image generation and every required load step supported for the concrete configuration. This is a proposal for scope, not a claim that a new dependency detector exists.

## Recommended route

1. Obtain the exact application/product version and the current preparation refusal. Inspect the user's `.knxprod` and the relevant project configuration offline.
2. If the concrete plan builds, use the existing **untested** path and its ordinary download confirmation plus additional acknowledgement. Do not invent a second override mechanism.
3. If preparation refuses, categorize the missing feature and extend the responsible adapter/image encoder/planner only where the available evidence supports it. Add fixture, malformed-input, plan/image regression and simulator tests, then update readiness and limitations. Missing endian semantics are an example of a real blocker: [KL-170](../limitations/KL-170-parameter-byte-order-download-boundary.md).
4. Keep device identity checks, valid access-key provenance, plan freshness, bus exclusivity, pre-write backup and readback active. A generic `force` flag that bypasses those checks is not the recommended design.
5. Expand coverage by the actual refusal distribution of the selected plug-in-free packages, not manufacturer names or unmeasured implementation-time estimates.
6. After a successful operator-authorized run, offer the existing voluntary tester report. Review it and functional checks before updating verification evidence for the exact application and scope; never promote automatically.

The backup contains the regions overwritten by the plan and affected load states, **not a full-device image or universal rollback**. Availability of a backup alone does not establish complete reversibility for an unsupported operation. See [backup boundary](../limitations/download-readiness-and-pre-write-backup.md) and [tester-report contract](../contracts/download-verification-reports.md).

## Required refusal UX — owner's clarification

The owner requires the reason for non-support to be visible, together with what can be done to obtain support and a link to the GitHub instructions for reporting by issue or mail.

Current source already translates stable readiness categories into plain reasons (`apps/knx-web/src/downloadReadiness.ts`). `DeviceDownloadPanel.tsx` displays that reason for the selected unsupported device. Its `DownloadVerification` component, however, is shown only for a non-running download session. Thus the existing post-run reporting flow alone does not fulfill the requirement for pre-plan refusals.

Required notice at the refusal point:

- Distinguish **untested but plannable** from **unsupported** and temporary/configuration failures; do not tell an unsupported user to bypass the guard.
- Show the concrete reason in plain language and retain the backend's technical detail in an expandable/copyable area. Keep unknown reasons visible with a useful generic next step.
- Give a category-appropriate next step: supply/install the missing application data; resolve a supported parameter/configuration error; or submit a support-gap report when an encoder/procedure feature is missing. Where more evidence is needed, state what kind, without demanding bus writes or promising a delivery date.
- Offer **Instructions**, **Report support gap**, and a documented **Email** path directly beside the reason, even if no plan or report file could be created.
- Reuse the existing German guide `docs/contribution-intake/DEUTSCH.md` and the `analysis.yml` issue template for unsupported cases, not the successful/attempted-download verification form.
- Optionally prefill a reviewed, minimal description with KNXBench version, product/application identification and refusal code. No project originals, addresses, customer/device names, paths, keys, credentials or parameter values are sent automatically. Reporting without attachments remains possible.
- Link targets must come from trusted application-owned mappings, not arbitrary product-data URLs. Opening a form or mail draft never submits anything automatically.

Published paths verified in the repository:

- Instructions: `https://github.com/KNXBench-Labs/KNXBench/blob/main/docs/contribution-intake/DEUTSCH.md`
- Support-gap form: `https://github.com/KNXBench-Labs/KNXBench/issues/new?template=analysis.yml`
- Existing download-report mailbox: `issue@knxbench.com`.

**Mail scope caveat:** the current German guide expressly limits `issue@knxbench.com` to download reports and says `contribute@knxbench.com` is not a confirmed private intake. The proposed implementation should explicitly document/admit support-gap inquiries to the existing mailbox (or a separately confirmed mailbox) before advertising that path. Do not promise private-original handling or use an unconfirmed mailbox as if it worked. Neither mailbox delivery nor GitHub form submission was tested here.

Acceptance for a future implementation: refusal reason and next action visible without a download session; known/unknown reasons covered; correct documentation and support-gap links; no silent uploads or hardware connection from reporting; original safety gates unchanged. EN/DE component and browser tests should cover the blocked-before-plan case as well as a failed completed run. Goal B owns the web surface, so this investigation does not implement a parallel UI change.

## Boundaries of this investigation

No application code, support policy, hardware evidence or compatibility promise changed. No bus access, manufacturer contact, vendor execution or hardware programming occurred. The user's specific device/package and refusal have not yet been supplied, so no device-specific compatibility conclusion is possible.
