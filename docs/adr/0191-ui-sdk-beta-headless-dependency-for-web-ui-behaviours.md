# ADR 0191: A beta headless dependency (ui-sdk) for web UI behaviours — evaluation and recommendation

Date: 2026-11-23
Status: Proposed
Session: WP-49.1 (owner request 2026-11-21, owner Go for the spike 2026-11-23)

Proposed only. Nothing here is accepted until the owner accepts this ADR;
pilots [WP-49](../open-work/WP-49-ui-sdk-behavior-pilot.md) 49.2 and 49.3 wait for
that. No dependency, `package.json`, lockfile or source change accompanies this
ADR.

## Context

The web UI (`apps/knx-web`) hand-writes the keyboard, focus, dismissal and ARIA
code of its comboboxes, menus, tabs and dialogs (inventory in WP-49). WP-49
asked whether the headless behaviour library
[ui-sdk](https://github.com/vercel-labs/ui) (`@ui-sdk/core` + `@ui-sdk/react`,
Apache-2.0, beta) can replace part of it without a visual or behavioural
regression. The owner decided on 2026-11-23: do the evaluation spike and this
ADR first; a beta dependency is acceptable in principle only if the spike
convinces and only under the policy below.

**Spike method (measured facts).** In a throwaway worktree on `origin/main`
(`b712774cd`), React 19.2.8, TypeScript 7.0.2, Vite 8.3.5, Vitest 5.0.5 with
happy-dom 21, `@ui-sdk/core@0.1.0-beta.14` and `@ui-sdk/react@0.1.0-beta.14` were
installed exactly. `GroupAddressPicker` was fully re-implemented on the library's
combobox (own filtering and 100-row window kept) and swapped into its two
production consumers so tree-shaking reflects real consumption (variant A); a
minimal `Menu` was added to price a second family (variant B). Evidence, every
number and the exact command for it: `<profile-scratch>/wp49-spike-20261123/`
(`SPIKE_RESULTS.md`, raw logs; spike commit local only, never pushed).
Byte sizes are deterministic; run times are single runs on a shared host.

**Measured results.**

| Item | Result |
| --- | --- |
| Install | `npm install --save-exact`: 2 packages added, 0 vulnerabilities, **no peer warnings**, **no transitive dependencies**; both Apache-2.0; ~11.7 MB on disk (dev/CI only) |
| Bundle, main chunk `index.js` (raw / Vite gzip) | baseline 2,379,912 B / 639.22 kB → A (combobox) 2,514,377 B / 687.82 kB (**+134,465 B raw, +5.65 %; +48.60 kB gzip, +7.6 %**) → B (+menu) 2,548,191 B / 698.22 kB (**+168,279 B raw, +7.07 %; +59.00 kB gzip**) |
| New lazy chunk | `sheet-presentation` 38.5 kB raw / 13.8 kB gzip, fetched from the local bundle only for the small-screen `sheet` presentation |
| CSS bundle | 0 B change (library CSS not imported) |
| Build | Vite 8: no new warnings (only the existing >500 kB chunk warning) |
| `tsc` 7.0.2 | `for c in tsconfig.json tsconfig.*.json`: 29/29 configs, no diagnostics |
| Vitest, picker test file on the new component | 9/9 pass **after three adaptations**: popup is portalled to `document.body` (tests scoped to the host element find nothing); highlight is `data-highlighted` and `aria-selected` means *chosen*; the combobox needed `loop={false}`, own open-on-focus and own clear-on-typing |
| Vitest, whole suite with the swap | 373 files / 6051 tests: **14 failed** in 3 consumer test files (`Inspector` 1, `DeviceWorkspace` 1, `FunctionsPanel` 12), 6037 passed; the same three files pass 165/165 on the hand-written picker. Causes: tests query options inside the row (popup portalled out) and the library renders all child items while closed (fixtures without `dpts` crash `dptText`) |
| happy-dom behaviours | focus stays in the input, `aria-expanded`, `aria-controls`, `aria-activedescendant`, arrow/Enter/Escape work without polyfills; identical under React 19 `StrictMode` |
| Network | static grep of both `dist`s: no `fetch`/XHR/WebSocket/beacon/`eval`, no `http(s)://` in JS; only 4 relative `import()` of local chunks; `web.css` has `http://www.w3.org/2000/svg` only as `data:` SVG xmlns. Not a runtime trace. |
| CSS | optional `web.css` is 86.6 kB raw / 12.6 kB gzip in a single `@layer ui-sdk`, with `--ui-*` tokens as `light-dark(oklch(...))` literals. Imported into `src/` it **fails** `themeTokens.test.ts` ("no literal colour outside the theme layer", ADR-0022, 8 literals); `motionGuard.test.ts` still passes. Behaviours work **without any CSS**. A `--ui-*`→`--knx-*` bridge was not built; real-browser appearance, ADR-0092 LCARS and ADR-0157 rail were **not** verified |
| Motion | the library reads only the OS `prefers-reduced-motion`, not KNXBench's in-app motion level |
| Escape ("a second Escape belongs to the page") | see below |

**Escape.** The hand-written picker closes the list on the first Escape, stops
propagation and leaves the second to the page. ui-sdk closes its top layer from
a **document-level** listener, calls `preventDefault` but **does not stop
propagation**: the first Escape also reaches the React parent (`Overlay.tsx`
`onKeyDown` → `onClose`) and window listeners (`multiSelection.ts`), so one
Escape would close a surrounding dialog too. `stopPropagation()` on the input
prevents the library from ever seeing the key (the list then does not close).
Parity needs an own handler on a controlled `open` (`if Escape && open:
preventDefault, stopPropagation, setOpen(false)`), verified in happy-dom; it
bypasses the library's Escape for that widget.

**Maturity (public sources, 2026-11-23).** Repository created 2026-10-26, 717
commits, 108 open issues, README: "in beta … subject to the Vercel beta terms".
npm shows 14 betas from 2026-11-12 (`0.1.0-beta.1`) to 2026-11-21
(`0.1.0-beta.14`), roughly one every 0.7 days; `latest` is beta.13, `beta` is
beta.14. The recent changesets are bug fixes; none is labelled breaking, but 0.x
prereleases carry no stability promise (assumption: breaking changes can land in
any beta). Sources: https://github.com/vercel-labs/ui,
https://github.com/vercel-labs/ui/releases,
https://registry.npmjs.org/@ui-sdk/core.

**Not measured / assumed.** No real-browser run (placement, top-layer stacking
against `ModalForeground`/`Overlay`, contrast, presentations); no run-time
benchmark; the menu family only as bundle cost, not behaviour; the whole
Vitest suite was not run on an unmodified tree inside the spike (only the three
consumer files); no `aria-haspopup="listbox"` on the library's input (the
hand-written one sets it; not asserted by tests).

## Decision

Proposed for owner acceptance. **Recommendation: take the fallback — apply
ui-sdk's behaviour and testing rules to the existing hand-written widgets,
without the dependency — and do not start pilots 49.2/49.3 as dependency
pilots.** Reasons, each from the measurements:

1. The gain is small where measured: the hand-written picker already passes a
   complete behaviour suite; the library adds arrow/Enter/ARIA parity that
   exists today, and the one behaviour the WP names (Escape) had to be
   re-implemented by hand on top of it.
2. The price is non-trivial and certain: +48.6 kB gzip (+7.6 %) for the first
   combobox, +59 kB with a menu, 14 consumer-test failures from the portal and
   eager-render model, and a CSS file that cannot be imported under ADR-0022
   without a bridge.
3. The risk is open-ended: a release roughly every 0.7 days with no 0.x
   stability promise, against a project whose priority order is correctness and
   compatibility first.

Nothing found blocks adoption outright (no peers, no transitive packages, no
network access in the code, TS 7 / Vite 8 / happy-dom / StrictMode clean). The
recommendation would flip to *adopt* if (a) the owner values the menu/roving
focus/submenu behaviour of WP-49.3 enough to pay the bundle cost, which the
spike did not measure behaviourally, or (b) a stable release plus an Escape
contract that respects `stopPropagation` ordering exists.

### If the owner accepts the dependency instead (policy as decided 2026-11-23)

A beta headless dependency is acceptable in principle only under all of:

1. **Exact pin**: `@ui-sdk/core` and `@ui-sdk/react` in `apps/knx-web/package.json`
   at the identical exact version (no `^`/`~`, `npm install --save-exact`), the
   lockfile committed.
2. **Deliberate updates only**: a version change is its own PR with the picker
   and menu behaviour tests, the three consumer test files, the Escape test
   above and a bundle-size re-measurement; never bundled with other work and
   never by automated range bumps.
3. **Documented revert path** (below).

Pilots 49.2/49.3 start only after acceptance and run on the spike's
adaptations (portal-aware tests, own Escape handler, `loop={false}`,
`autoHighlight`, no library CSS or a `--knx-*` bridge verified against
`themeTokens.test.ts`).

### Revert path

The dependency stays confined to the pilot widgets (no library type in a shared
interface; props of `GroupAddressPicker` and the `RenameWorkbench` menu stay as
they are). Reverting is a single PR: restore the hand-written component files
from `git` (the pilots keep them one commit back, and tests are written against
behaviour, not library internals), remove both packages from `package.json`
and regenerate the lockfile (`npm uninstall @ui-sdk/react @ui-sdk/core`), delete
any bridge CSS, then run the web gate. Nothing is persisted by the library, so
no data migration exists.

### Fallback (behaviour and testing rules only, no dependency)

Apply to the existing widgets, per widget family and each in its own package:
a keyboard/focus/ARIA test checklist modelled on the library's (focus stays in
the input, `aria-activedescendant` follows the highlight, arrows stop at the
ends, Enter chooses, first Escape closes and stops, second Escape belongs to the
page, Tab closes, focus returns to the trigger, pointer press does not blur);
`aria-selected` meaning chosen and a separate highlight attribute; and the
`aria-haspopup`/`aria-expanded`/`aria-controls` set on every combobox. No new
package, no bundle change.

## Alternatives considered

- **Adopt the dependency for pilots 49.2/49.3** — acceptable under the policy
  above if the owner values the menu behaviour; not recommended on the measured
  cost and behavioural gaps.
- **Other headless libraries** — not evaluated in this spike; out of WP-49 scope.
- **Vendor/copy the library's code** — rejected: it inherits the churn without
  the update path and adds licence notices to maintain.
- **Do nothing** — equals the fallback without the written rules.

## Consequences

If the fallback is accepted: no dependency, bundle or CSS change; the owner's
decision 3 applies; WP-49 49.4 becomes a plain checklist roll-out. If the
dependency is accepted: +48.6 kB gzip for the first widget family (+59 kB with
a menu) in the main chunk, a lazy 13.8 kB chunk for the sheet presentation,
three consumer test files to adapt, an own Escape handler on every library
widget, a bridge or no library CSS, and a standing update chore at the
library's release cadence. Either way, nothing is accepted by this document
until the owner records the decision in `docs/open-work/OWN-01-owner-decisions.md`.
