# Partial-download evidence

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-142)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="142-the-mask-070nh-partial-download-is-tested-in-the-simulator-only"></a>
## §142 Partial download: one verified device, product and UI boundaries remain

**User decision 2026-10-05.** The Web scope selector is handed to the UI
owner; the backend contract is in the
[commissioning handoff](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/COMMISSIONING_ALPHA_LEDGER.md#handoff-to-the-ui-owner-2026-10-05).

**Verified scope (2026-09-29/30; RESEARCH §19.15/§19.17).** On the MDT
`1.1.67`, the `parameters`, `group-addresses` and `both` partial scopes
completed with the same option-C image: respectively 394, 1022 and 1416
octets read back, with the relevant load states `Loaded` and byte-identical
post-run dumps. One earlier group-address run was interrupted by an external
shell timeout; the complete download restored it before a successful retry.
The history and recovery evidence are retained in RESEARCH, not an open
"simulator-only" limitation. Its closing restart remains unconfirmed (§136).

`knx device download --partial
parameters|group-addresses|both` and the `partial` field of
`POST /api/device-download/plan` derive CP §3.9.2.4's partial download
from the complete plan (`knx_core::commissioning::partial_memory_download`).
The partial plan is a subset of the complete one: no application unload, no
application allocation, tables only when group addresses are selected.

- **Hardware evidence is narrow.** All three partial scopes ran only on
  `1.1.67` with the same option-C image; other programs and changed target
  configurations remain unverified. `WriteScope::Download` still requires
  the device-specific phrase, a pre-run baseline and a restoration plan.
- **Two checks are KNXBench's, not the Standard's.** Before the first
  write, the device must report the plan's application in
  `PID_PROGRAM_VERSION` (object 3), and every part the complete plan loads
  must be `Loaded`. Otherwise nothing is written and the message says to
  run the complete download. Object index 3 for the application program is
  the documented default (`ObjectIndex::APPLICATION_PROGRAM`), and it is
  what `1.1.67` answered to.
- **Non-EEPROM application data is not written.** CP rule 3 ignores it. The
  plan lists every such write as "not written"; the MDT products in the
  corpus have none.
- **`AppliesTo` and `LegacyAllowPartialDownloadIfAp2Mismatch` are not
  interpreted.** Only two of 203 `070n` programs carry `AppliesTo`, both
  `full,par`, and no PDF read defines the option.
- **Web UI (delivered 2026-10-05, UI owner).** The Download to device tab
  offers *What to write* — complete, parameters, group addresses, both —
  sends exactly that `partial` body (absent for complete), and shows the
  partial scope and every `notWritten` write before consent; a refusal is
  shown with no plan. Its only evidence is mocked/intercepted (Vitest and
  Chromium), no live device. The device *comparison* view still compares the
  complete plan only; its route's optional partial selection is not offered.
<!-- END retained catalogue record -->
