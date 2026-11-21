# WP-49 — Pilot: headless UI behaviours from ui-sdk for menus and comboboxes

Owner request: 21 November 2026 (WebUI session). Recorded only; not claimed,
no implementation Go and no dependency decision yet.

## Requested outcome

Find out with a small, reversible pilot whether the headless behaviour
library ui-sdk ([vercel-labs/ui](https://github.com/vercel-labs/ui),
Apache-2.0, npm `@ui-sdk/core` + `@ui-sdk/react`) can replace part of the
web UI's hand-written keyboard, focus, dismissal and ARIA code without a
visual or behavioural regression. If the pilot convinces, record the adoption
as an ADR; if not, remove it again and keep only the behaviour/test lessons.

The library ships behaviours, not styles: KNXBench keeps its own theme tokens,
presentations and CSS.

## Current basis (source anchors on `origin/main` `ed47bcad4`)

The web UI (`apps/knx-web`, React 19.2, Vite, Vitest with happy-dom,
Playwright lanes) builds every interactive widget itself. Production `.tsx`
files (tests excluded) carry 6 `role="dialog"`, 4 `role="menu"`,
4 `role="combobox"`/`role="listbox"` pairs, 2 `role="tablist"` and
2 `role="toolbar"`:

- Comboboxes with `aria-activedescendant` and own arrow/Escape handling:
  `GroupAddressPicker.tsx:49-73`, `CommandPalette.tsx:79-123`,
  `Search.tsx:65-92`, `BusComposeForm.tsx:362-379`.
- Menus: `FileMenu.tsx:39,296-299,358,676` (roving-tabindex menu with flyout
  submenus on `<details>`), `RenameWorkbench.tsx:56` (context menu).
- Tabs: `Inspector.tsx:1081-1084` (device tabs), `BusMonitorPanel.tsx:1222`.
- Toolbars: `TelegramFlowView.tsx:599`, `features/rtf/InstructionEditor.tsx:118`.
- Dialogs are individual components (for example `ContributionButton.tsx:26`,
  `StoredBusAccess.tsx:118`, `DiagnosticExportButton.tsx:28`) layered by
  `ModalForeground.tsx:7`; there is no shared dialog primitive.
- Motion and theme rules that a pilot must keep: motion levels and the
  reduced-motion guard (`motionGuard.test.ts`, `themeTokens.test.ts`), the
  built-in presentations (ADR-0092 LCARS, ADR-0157 rail).

## Bounded packages and actual status

| Package | Outcome | Status |
| --- | --- | --- |
| 49.1 | Evaluation spike in a throwaway branch: install the exact beta version, measure bundle-size delta, check React 19 / TypeScript 7 / Vite 8 / happy-dom compatibility, offline and Tauri operation (no runtime fetch), coexistence of its optional `@layer` CSS with the theme tokens and presentations; write an ADR draft with the measured results | Not started |
| 49.2 | Pilot 1: `GroupAddressPicker` on the ui-sdk combobox/listbox behaviour, keeping its labels, filtering, Escape semantics ("a second Escape belongs to the page") and existing tests; add keyboard, focus and ARIA tests per the library's testing checklist | Not started; needs 49.1 and owner Go on the dependency |
| 49.3 | Pilot 2: `RenameWorkbench` context menu on the ui-sdk menu behaviour (dismissal, focus return, roving focus) | Not started; needs 49.1 and owner Go |
| 49.4 | Decision: roll out to the other comboboxes/menus/tabs/dialogs (one package per widget family) or revert the pilot; record the result in the ADR | Not started |

## Boundaries

- Web UI only; no domain, server, storage or bus changes.
- No runtime dependency lands on `main` before the owner accepts the 49.1 ADR
  (`AGENTS.md`: no unnecessary dependencies). Pin the exact version; the
  packages are beta (`0.1.0-beta.14` on 21 November 2026).
- No visual change beyond what the owner accepts; keep theme tokens,
  presentations, motion levels and dark/light behaviour.
- Its agent-tool and WebMCP features are out of scope for this pilot.
- Tests through the existing Vitest and browser lanes; colour contrast is
  checked in a real browser, not in happy-dom.

## Open decisions

1. Is a beta headless dependency acceptable at all for the web UI, and under
   which pinning and update policy?
2. If 49.1 shows a problem (size, compatibility, styling conflicts), should the
   fallback be to apply only its behaviour and test rules to the existing
   hand-written widgets, without the dependency?
