# Manual acceptance checklist (AR16)

Dated record of how the user manual (`docs/manual/`) was checked against the
finished application for the Alpha. Source of the task: `RELEASE-03`,
[alpha-release-goal AR16](../alpha-release-goal.md).

## Policy (user decision, 2026-10-06)

- The manual stays in the repository and is read on GitHub. It is not bundled
  into the application; the in-app help stays the short F1 panel
  ([ADR-0024](adr/0024-in-application-help.md)).
- The manual has screenshots. They show the finished application at its tested
  scope and use fictional data only.

## How the screenshots are made

All 21 pictures in `docs/assets/screenshots/` were regenerated on 2026-10-06
from the real application: release `knx-server` and production frontend at
`origin/main` `f7e98896`, Chromium, 1440×900 at 2× scale, Porcelain theme,
English interface, in a loopback-only network namespace. The project is the
fictional "Sample house" from `tools/manual_sample_project.py` (manufacturer
`M-7FF0` "Example Devices (fictional)", 8 devices, 15 group addresses, 39
communication objects); three unit tests pin its content and determinism. The
run is `apps/knx-web/e2e/manual-screenshots.shots.ts` with
`playwright.manual.config.ts`. After the UI owner's *Live activity* tab
(`892b9948`) the spec ran again on the merged tree (debug server, same
frontend build steps): only the bus-monitor picture changed and was replaced.
The regeneration recipe is in
[Contributing](manual/development/01-contributing.md#screenshots-in-this-manual).

Not shown, by design: any live KNX bus, gateway or device (the bus-monitor
picture shows the offline search result), the desktop shell's native dialogs,
and any theme other than Porcelain.

## Checklist — 2026-10-06

| # | Check | Result |
| --- | --- | --- |
| 1 | UI owner's closure | Done. Receipt [`84bc32c3`](UI_ALPHA_READINESS.md#ui-owner-closure-receipt--2026-10-06) after the two residues (`UI-04` Web half, `KL-61` wording) shipped in `892b9948`. Checked independently of the receipt's text: 26 UI ledger rows = 15 `DONE`, 10 `ACCEPTED_BOUNDARY`, 1 `LATER`; issue plan 68 ticked, 0 open; the regenerated `GroupAddressNode` binding on `origin/main` says *effective* type. The receipt is the owner's self-review; AR18 reviews independently. The new *Live activity* tab made the bus-monitor screenshot stale; it was re-shot. |
| 2 | Location and screenshot policy | Recorded above. |
| 3 | Every screenshot reference (26 places in README and 10 manual chapters) | Picture replaced; each alt text and its surrounding sentence re-read against the new picture and corrected. |
| 4 | Claims found stale while doing 3 | Fixed: the bus monitor's button is **Search**, not "Discover gateways" (7 places); the start-up gateway search is now stated; the welcome screen has three cards (New project…, Open KNXBench project, Import ETS project); help has eleven topics; the catalog row shows `name (number) — description`; the Parameters example has no modules. |
| 5 | Counts re-measured in the run | Command palette: 13 commands with the Ctrl+Z, Ctrl+Shift+Z, Ctrl+K and F1 hints. Help: 11 topics. |
| 6 | Remaining chapters, claim by claim | Done 2026-10-06 (slice 2), method below. |

## Row 6 — how the remaining chapters were checked (2026-10-06)

Against `origin/main` `6a9204a4`: the debug `knx` binary, a debug `knx-server`
with the production frontend, and the fictional sample project, offline.

| Check | Scope | Result |
| --- | --- | --- |
| Every `knx …` invocation in the manual and README against `knx --help` | 95 invocations, 28 commands | All commands and flags exist; the only unknown command is the removed `knx export`, always described as removed |
| Bold interface labels against the English message catalogue and components | all chapters except development | 39 labels not found verbatim; each read in context — prose terms, German labels on purpose, or templates, plus the stale ones below |
| `KNX_*` variables and `/api/…` routes | all chapters | All exist (the removed export route is described as removed) |
| Version strings, counts ("thirteen", "eleven", "seventeen", crates, ADRs) | all chapters | Corrected where stale |
| Every "not yet / not implemented / no way" sentence | all chapters | Each checked; stale ones corrected |
| Live probes | style change, catalog create, Settings, documentation export | Behaviour recorded below |

Stale claims found and corrected:

- **Group-address style:** the Project node changes it, undoably (probe: ThreeLevel →
  TwoLevel → Undo). Four chapters said it could not. The New project dialog's hint
  still says so; that string belongs to the UI owner (handed over).
- **Autosave** exists (on, five minutes, only with a file, countdown with Cancel); two
  chapters said there was none. Settings also gained **Autosave** and **Programming
  confirmation** descriptions.
- **Drag and drop:** works within any installation, and a group address can be dropped on
  a link row; four places said otherwise.
- **Documentation export:** the dialog has section checkboxes and a preview; one
  paragraph said they had not landed.
- **`knx diff --exit-code`** exists; the status page listed it as missing and the CLI
  chapter's exit-code table denied any other `2`. The CLI chapter now lists the device
  commands it did not mention.
- **Product-data translations and language packs** are used; the status page said not.
- **Versions:** programs carry their own versions (CLI, desktop, web `alpha.4`, server
  `alpha.1`); several pages said `alpha.1` everywhere.
- **Workflow chapter** now walks the fictional sample instead of a project the
  screenshots no longer show, and the address step matches the device-number field.
- **Device writes:** the architecture tour and the FAQ still said nothing writes to a real
  device; the download is verified on one device.
- **Group-address DPT:** schema-21+ declarations (ADR-0078) added to the KNX basics.
- **Developer pages:** thirteen crates (with `knx-build-stamp`), 83 ADRs, all six `xtask`
  checks.

Late corrections (2026-10-06, with the `KL-61` display): the group-address
chapter still said a group address carries no type of its own, and known
issues still said the codec infers the input format (both stale since ADR-0078
and T07). Both corrected; the chapter now describes the new Inspector rows.

Not done: a sentence-by-sentence reading of the five KNX-basics chapters for KNX
theory; only their statements about KNXBench were checked.

## Acceptance

**Accepted 2026-10-06 by the Alpha release owner**, at the scope above: the
manual on GitHub, with screenshots of the finished application and fictional
data, describes what the application does on `origin/main` today.

Exceptions the manual states and that only the user can accept (AR19): native
WebKitGTK/Tauri workflows, real screen readers, native file choosers, real
multicast discovery, a dead web view and any live KNX bus are not shown or
verified by these pictures (the UI owner's accepted boundaries of 2026-10-04);
screenshots show only the Porcelain theme in English; the KNX-basics chapters
were checked only for their statements about KNXBench. AR18's independent
review may still reopen any line.
