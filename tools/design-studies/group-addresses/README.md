# Gruppenadress-Labor

Four clickable, **isolated read-only design studies**, not a production feature.
All views use 36 original fictional addresses and 12 fictional communication
objects. No project import, KNX connection, bus access, API call, telemetry,
external font or third-party runtime is used.

## Open

Open `index.html` directly in Chromium/Firefox, or from this directory run:

```sh
python3 -m http.server 8769 --bind 127.0.0.1
```

Then visit `http://127.0.0.1:8769/`. No installation or build is required.
Direct file opening and loopback HTTP were exercised in Chromium. Firefox is
an intended modern-browser target, **not an executed acceptance target**.

## Compare

- **Baumtabelle:** expand/collapse hierarchical rows with DPT and link columns.
- **Spaltenbrowser:** browse the numeric address levels through independently
  scrollable columns; two-level notation omits the middle column.
- **Adressatlas:** semantic zoom by opening labeled group areas, then individual
  address tiles. It is **not** a geographical map, animated canvas or pinch-zoom
  implementation. Breadcrumbs navigate back.
- **Verknüpfungsmatrix:** addresses against communication objects; click a cell
  for the exact modeled direction. Filter object columns without filtering the
  address rows. `S` means Send, `E` Receive, `S/E` both, `—` known absence, and
  `?` unavailable link data. This is not observed hardware state.

The view switch affects only the central workspace. The fictional Project
Explorer and shared Inspector stay stable. Search, numeric scope, DPT filter and
selected address IDs survive switching. Checkboxes or Ctrl/Cmd-click add/remove
addresses in the first two views; Ctrl/Cmd-click is supported in the atlas and
matrix as well. An out-of-filter selection remains visible in the Inspector and
is reported in the selection bar. Only the preferred view is stored under the
study-specific localStorage key `knxbench.ga-study.view.v1`; storage refusal
falls back to memory. Selection, project data and filters are not persisted.

The display switch is **not** a real project's address-style command. The raw
fixture values and stored example range paths are unchanged. Study layout uses
numeric bins with **explicit fictional labels**; importing arbitrary GroupRange
hierarchies or deriving room/function names from real address numbers is out of
scope. Source bit splits match `crates/knx-core/src/address.rs`:
three-level 5+3+8 bits and two-level 5+11 bits. Free-style notation is not studied.

Use **Vergleichsaufgaben** for the same tasks in all four views. This is a
comparison artifact, not a decision to ship every view. There are no editing,
save, link/unlink or hardware controls. Mobile layout uses internal horizontal
scrolling for wide tables/columns, and places the Inspector below the workspace.

## Reproduce checks

From the repository root:

```sh
node --test tools/design-studies/group-addresses/model.test.mjs
node --check tools/design-studies/group-addresses/model.js
node --check tools/design-studies/group-addresses/app.js
node --check tools/design-studies/group-addresses/browser-check.js
```

`browser-check.js` is a Playwright CLI **recipe function**, not application code
or an automatically registered production test. With an installed Playwright
CLI and a private Chromium session already opened to the study page:

```sh
playwright-cli --session ga-study run-code --filename tools/design-studies/group-addresses/browser-check.js
```

Use a task-owned CLI config selecting the installed Chromium executable if the
CLI's default Chrome executable is unavailable. The recipe returns its named
assertions and captures frames under `output/playwright/` in the current working
directory; it also uses that isolated browser origin's study preference key.
Run it in an owned output directory to avoid overwriting earlier captures.

## Evidence boundary

Local acceptance on 2026-10-11: nine model tests, 52 named final direct-file
Chromium assertions, 13 captured frames; desktop 1440×1000, compact 400×900,
light/dark and 2-/3-level cases. The earlier HTTP recipe passed 50 assertions;
the final direct-file recipe adds compact Inspector reachability checks.
No JavaScript/console errors or external requests were recorded in the final
recipe. Screenshots were inspected for actual geometry/readability, including
full dark matrix, atlas overview and scrolled compact Inspector frames. Lower
Inspector fields on compact screens require further normal page scrolling.

Self-review, not independent approval. The personal-browser-profile lock and
one malformed CLI invocation were tooling failures, excluded from acceptance.
The dotted-DPT search regression was reproduced with a failing ninth test and
fixed without rewriting address or DPT strings. No full application build,
production test suite, ETS compatibility, real project performance or hardware
acceptance is claimed.

See [the design record](../../../docs/design-studies/group-addresses.md).
