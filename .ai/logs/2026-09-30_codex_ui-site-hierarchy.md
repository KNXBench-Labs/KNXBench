# ISSUE-06 — Ground-root Site/Property UI

Timestamp: 2026-09-30 23:11 CEST

## Scope and evidence

- Accepted ADR-0038 chooses an existing Project Schema23 `Ground` root inside one installation, with multiple `Building` children. It is not an ETS `Site` kind, an IoT `loc:Site` equivalence or a new installation. The three reference projects contain no Ground root. Existing synthetic `knx-etsproj/tests/site_hierarchy.rs` proves two uniquely owned devices and one shared topology; native store roundtrip covers persistence. The first-installation-only editor, absence of installation rename and lack of a real ETS Ground export remain explicit.
- `StructureWorkspace.tsx` adds **Add site / property** in the Buildings overview for the first installation. `NewBuildingPartRow` is not a separate route: the existing component in `ProjectExplorer.tsx` takes a fixed `Ground` kind, shows no selectable kind and calls the same validated `createBuildingPart`. Existing `Inspector.tsx` parent select calls `moveBuildingPart`; no model, storage, importer or protocol changed.
- The hint and manual explain how to place existing buildings under a site, that device ownership and line stay unchanged, and why no second installation is created. EN/DE labels and responsive token-based styles use the existing design system. The site action is keyboard reachable through a disclosure and native form controls.

## Regression and gates

- RED→GREEN: `StructureWorkspace.test.tsx` initially failed because the site disclosure was missing; it then checked fixed `Ground` root creation, two explicit reparent commands, one installation's unchanged topology and exactly one building-view device projection for each building. A deliberate wrong-kind mutation failed the focused test and was restored.
- Local Playwright/Vite fixture intercepts **every** `/api/**` request. `site.e2e.ts` passed 4/4 EN/DE at 360/1440 px; it captures exact create/move payloads, requires two device tiles and no horizontal overflow, and never connects to a KNX gateway. Existing local browser suites passed 4/4 Debug, 4/4 Device checks, 4/4 monitor and 10/10 ISSUE-09.
- Full Web: 82 files / 1,296 tests; TypeScript no diagnostics and Vite build green. Corpus-backed Rust before the parallel commissioning changes: 137 suites / 2,776 passed / 0 failed / 161 ignored / 0 `SKIP:`. On the October 1 rebased tree: 139 Rust suites / 2,791 passed / 0 failed / 161 ignored / 0 `SKIP:`, Web 82 / 1,296, TypeScript/build, mocked Chromium 4/4 new + 4/4 Debug + 4/4 Device checks + 4/4 monitor + 10/10 ISSUE-09, strict workspace Clippy, fmt, headers, anchors, layering, corpus gates and diff check passed. The upstream K6/K13/serial-address safety refusals and ADR-0056 telemetry were preserved; this UI package changes none of them. All bus-related tests were mocked or simulator-based; no live tunnel, device write or production discovery.

## Pending delivery

The scoped diff and six initial plus two later commissioning commits were reviewed and rebased without dropping their `.ai` and status entries. Publish the integrated candidate with exact remote SHA readback, release the Web lock in a separate handover commit and clean only this package's ignored corpus link, worktree and scratch. U13 closing review remains a user decision; ISSUE-12 discovery requires wire/gateway evidence before any protocol change.
