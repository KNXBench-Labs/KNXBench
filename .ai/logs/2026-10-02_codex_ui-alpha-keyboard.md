# UI alpha keyboard/modal/help-tip package — 2026-10-02

## Ownership and scope

Isolated `ui-alpha-keyboard`, reservation/base `8656ffa18396d387547be952c58bf97b92a53c95`. UI session owns the Web lock. No root synchronization, foreign worktree edit, real backend/proxy, gateway/tunnel/discovery, bus/device write, package/system/native setting change, quota probe, subagent, release tag or reserved ADR activation.

All 180 parent inventory identities and all 24 UI routes remain preserved; other owners and domain/sample dependencies are not reclassified. KL-20 implementation acceptance is scoped, never whole-source accessibility or alpha-release consent.

## Implemented

- Shared active-descendant hook integrates Search, Command Palette and Catalog Browser. Nearest-edge scrolling keeps the active row visible without changing combobox focus. Catalog highlight is separate from selection/creation; inactive/empty lists are safe.
- Document-local modal DOM lease retains previous inert/aria-hidden values, excludes background branches and late DOM, supports nested/independent/out-of-order roots, and restores original connected non-inert focus only when the final modal closes.
- Shared focus filtering excludes controls below hidden/inert/aria-hidden ancestors.
- HelpTip retains a permanent local clipped description plus an aria-hidden painted body portal. Fixed viewport bounds/placement account for application zoom, escape clipped/transformed ancestors, reposition on resize/captured scroll and clean up listeners/portal.
- Exact companion transitive graph includes the DOM-only helper; its API/project-mutation assertions remain unchanged. No Core/API/storage/protocol/schema/manufacturer change or dependency added.

## TDD and review

Observed REDs for Search, Palette, Catalog, modal background, out-of-order close, dynamic DOM, final focus, tooltip CSS/portal/position and hidden-ancestor focus. Focused GREENs followed the implementation slices. The first complete Web run failed its exact companion graph expectation (one failure / 1,352 passing); this failure remains a failure, corrected only after reading the helper and graph.

Separate in-session full-diff review found R1 companion graph, R2 hidden-ancestor focus and R3 stale tooltip CSS/accessibility comments/guards. All corrected; no independent external approval is claimed. Post-fix review finds no remaining package-blocking issue.

Eighteen runnable behavioral controls were rejected by test assertions and original source bytes restored. The early-background-release mutant initially survived because MutationObserver repaired the state before awaited assertions; a synchronous assertion immediately after lower-root unmount now detects it. This initial survivor is retained, not relabelled caught. A deliberate new-hook TS2322 error was caught, restored exactly, and clean type checking repeated.

Restored focused verification: nine files / 117 tests and TypeScript pass. Seventeen new Chromium cases pass: actual list visibility/focus, Chromium inert/AX background exclusion, nested close, exact accessible tooltip description/geometry in DE/EN at 360/640/1440 px and 100/150% application zoom, plus a tooltip inside a modal. Every API request is intercepted; no KNX backend is started. Initial browser fixture test errors (unscoped native select option and Node-side CSS.escape) are retained as failures, corrected in the test without weakening geometry/AX assertions.

## Public technical basis

Consulted 2026-10-02:

- [WHATWG inert subtrees](https://html.spec.whatwg.org/multipage/interaction.html#inert-subtrees): inert nodes are generally not focusable and are excluded from accessibility exposure; our custom modal must deliberately isolate its background, unlike automatic native showModal behavior.
- [React createPortal](https://react.dev/reference/react-dom/createPortal): portal placement can escape clipping while React ancestry/context/event propagation remain; the painted tooltip is decorative, not a second accessible description.
- [WAI-ARIA tooltip pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tooltip/): describes tooltip/aria-describedby, retained trigger focus and Escape dismissal. Using the pattern as interaction guidance is not a conformance claim.

## Complete acceptance

First run `proc_034266a906ce` exited 1 at headers after fmt/type/full Web/build/all 52 intercepted Chromium/workspace tests/strict Clippy/layering passed. The SPDX-first prefix hid the required purpose header; that failure stays failed. Corrected purpose-first/SPDX-second order, added headers to edited modal files, lowered measured absence ceiling 160 to 157 and bumped only frontend/root-lock version to alpha.2 under ADR-0018, without dependency changes. Renewed `proc_870fe2d19835` repeats all twelve coordinated offline gates with both shared Git-common-dir leases held and a worktree-owned target. Renewed final verdict is passed: all twelve steps exit 0, Web 84 files / 1,357 tests, 52 intercepted Chromium cases, Rust 146 result blocks / 2,890 passed / zero failed / 163 ignored, 576-source freeze. Integrated proc_490a156044df repeats all twelve gates on 2e57f8e556932a80afb9c496f33ee247f01113a9, with identical counts and 576-source freeze. The failed predecessor is not relabelled successful. Do not infer ignored/private corpus execution from corpus configuration checks.

## Publication and cleanup

Source 2e57f8e556932a80afb9c496f33ee247f01113a9 is published and fetched back; exact ref/tree/all 28 owned artifacts, zero outgoing commits, author/committer email and no-co-author verified. Rebase preserved upstream compare.md and the complete authoritative handover suffix. Scoped KL-20 owner implementation receipt releases only this holder's Web lock; remaining boundaries below stay open. Narrow receipt gates/publication/readback and actual owned checkout/branch/scratch cleanup follow; no cleanup proof is claimed ahead of execution.

## Explicitly separate open boundaries

- Actual native Tauri/WebKitGTK and Orca interaction, full-theme/whole-app visual audit, native chooser and dead-WebView behavior.
- Real multicast/firewall/network acceptance; no live discovery/tunnel or write authorization.
- Independent ETS/installation/schema samples; synthetic fixtures prove only their own data.
- Domain/application contracts for general multi-installation editing, coupler discrimination, address allocation/repair, transactional collaboration/history/replay and additional structural/group-address drag gestures.

Those are open requirements, not accepted alpha exceptions or duplicated U0–U13 tasks. Root statistics and foreign root state remain owner-controlled and untouched.
