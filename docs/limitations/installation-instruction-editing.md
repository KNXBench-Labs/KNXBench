# Installation instruction editing

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-installation-instruction-editing)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->



## Installation instruction editor integration boundary (2026-10-11)

AP2 exposes a feature module and HTTP/application workflow; the device-editor tab
was added later (section *Installation instructions tab* above). [Host handoff](../contracts/instruction-editor-host.md).
The exact-source reader currently admits Schema23 UTF-8 XML1.0 unprefixed
installation/device originals only. Other encodings/schemas, ambiguous owners,
fragment-only originals and natively authored devices without retained imported
source are explicit refusals. They do not lose original bytes. Editing supports
plain text/Unicode/paragraphs/bold/italic/underline; unsupported conversion needs
explicit consent and never overwrites the imported original. Native working
history is recoverable but not implicit Save. There is no edited-override ETS
export or full ETS compatibility claim. A later combined-source/browser receipt
does not certify shell integration or hardware.
<!-- END retained catalogue record -->

## Update 2026-10-14: devices made in KNXBench (Goal B4)

The record above lists "natively authored devices without retained imported
source" among the refusals. Since Goal B4 a device KNXBench created itself
(`KB-DEV-…`/`KB-SCAN-…`, nothing retained from that path) gets instructions through
the same preview, apply, undo and native history, with no invented original or
digest ([contract](../contracts/instruction-editing.md#devices-made-in-knxbench-goal-b4-14-october)).
An imported device whose retained source is missing is still refused. There is
still no ETS export of these instructions. Evidence: `knx-app` and HTTP tests,
Vitest and Chromium with synthetic data; no shell, hardware or ETS.
