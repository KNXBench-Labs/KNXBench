# Settings Surface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the existing settings surface with grouped controls, a consumed preferred gateway, shared lossless line-scan exclusions, and localized typed diagnostics.

**Architecture:** Keep ADR-0029's server-owned `settings.json` and synchronous frontend cache. Small preference modules own values; the existing panel composes them. A typed server diagnostic is shared by settings responses and log entries. Workflow panels consume a gateway preference once and then retain local ownership.

**Tech Stack:** Rust, Axum, Serde, React 19, TypeScript, Vitest, happy-dom, CSS tokens, Cargo.

**Spec:** [`docs/superpowers/specs/2026-09-22-settings-surface-design.md`](../specs/2026-09-22-settings-surface-design.md)

## Global Constraints

- Keep the single server settings record and its one browser cache.
- Keep the historical eight-key adoption table closed.
- Preserve unknown keys, including `groupAddressNotation`; expose no notation selector. Group addresses stay slash-formatted.
- Reading or writing a preferred gateway must cause no discovery, connection, scan, multicast, or KNX traffic.
- Use documentation addresses only, such as `192.0.2.10:3671`.
- Preserve every legacy exclusion string byte-for-byte until explicit removal. New entries must be unique dotted individual addresses.
- Use existing `--knx-*` CSS tokens and typed English/German catalogues.
- Give every new TypeScript file a purpose-sentence header.
- Never hand-edit `docs/LIMITATION_TRIAGE.md`.

## Review Focus

- Pre-hydration local edits survive while untouched server keys hydrate: Task 2.
- Typing or discovery before hydration wins; active workflows never reseed: Task 3.
- Duplicate and invalid legacy exclusions remain individually removable but block scan work: Task 4.
- Every settings-file outcome remains distinct and localizable without matching English prose: Tasks 1 and 5.
- No settings read/write path performs network or hardware activity: Tasks 3 and 6.

---

### Task 1: Typed server settings diagnostics

**Files:**

- Modify: `apps/knx-server/src/settings.rs`
- Modify: `apps/knx-server/src/settings_routes.rs`
- Modify: `apps/knx-server/src/session_log.rs`
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/tests/http_settings.rs`
- Modify: `apps/knx-server/tests/http_log_route.rs`
- Modify: `apps/knx-server/tests/http_debug_report.rs`

**Interfaces:**

- `SettingsQuarantineReason`: `Unreadable`, `InvalidJson`, `NotObject`, `MissingSchemaVersion`, `SettingsNotObject`.
- `SettingsDiagnostic`: tagged camel-case variants `Migrated`, `Adopted`, `RefusedNewer`, `Quarantined`.
- `SettingsDto` replaces `notice` with optional `diagnostic` and optional English `message`.
- `LogEntry` gains `diagnostic: Option<SettingsDiagnostic>`; unrelated records use `None`.

- [x] **Step 1: Write failing HTTP contract tests**

  Exercise migration, adoption, refusal, and all five quarantine inputs through real routes. Assert exact diagnostic fields, an English fallback `message`, and no `notice`.

  ```rust
  assert_eq!(body["diagnostic"]["kind"], "quarantined");
  assert_eq!(body["diagnostic"]["reason"], "invalidJson");
  assert!(body["message"].as_str().is_some());
  assert!(body.get("notice").is_none());
  ```

- [x] **Step 2: Confirm RED**

  ```bash
  CARGO_TARGET_DIR=/tmp/knxbench-t10-target cargo test -p knx-server --test http_settings --no-fail-fast
  ```

  Expected: FAIL because `notice` is still the only diagnostic field.

- [x] **Step 3: Type quarantine causes without losing details**

  Change `SettingsLoad::Quarantined` to carry `reason`, `detail: String`, and `moved_to`. Pass stable reason codes through `quarantine` while retaining today's dynamic OS error in `detail`. Keep collision-safe rename behavior unchanged.

- [x] **Step 4: Define the transport diagnostic**

  Define the following in `session_log.rs`, plus a five-variant `SettingsQuarantineReasonDto`:

  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
  #[serde(tag = "kind", rename_all = "camelCase")]
  pub enum SettingsDiagnostic {
      Migrated { from_version: u32, to_version: u32 },
      Adopted { from_version: u32, to_version: u32 },
      RefusedNewer { file_version: u32, current_version: u32 },
      Quarantined { reason: SettingsQuarantineReasonDto, moved_to: String },
  }
  ```

- [x] **Step 5: Replace `notice` with typed fields**

  Build `diagnostic` directly from `SettingsLoad`, and build `message` from the same outcome plus retained `detail`. Successful browser adoption emits `Adopted { from_version: 0, to_version: 1 }`, never `Migrated`.

- [x] **Step 6: Carry diagnostics through the log**

  Add the optional field to `LogEntry`. Settings records receive `Some`; every unrelated literal in `session_log.rs`, `domain.rs`, and `http_debug_report.rs` receives `None`. Extend `http_log_route.rs` to prove `/api/log` returns both diagnostic and fallback message.

- [x] **Step 7: Verify the server slice**

  ```bash
  CARGO_TARGET_DIR=/tmp/knxbench-t10-target cargo test -p knx-server --test http_settings --test http_log_route --test http_debug_report --no-fail-fast
  CARGO_TARGET_DIR=/tmp/knxbench-t10-target cargo test -p knx-server settings --no-fail-fast
  ```

  Expected: PASS; all outcomes remain distinct, unreadable detail survives, and unrelated log constructors compile.

- [x] **Step 8: Commit the contract**

  ```bash
  git add apps/knx-server/src/settings.rs apps/knx-server/src/settings_routes.rs apps/knx-server/src/session_log.rs apps/knx-server/src/domain.rs apps/knx-server/tests/http_settings.rs apps/knx-server/tests/http_log_route.rs apps/knx-server/tests/http_debug_report.rs
  git commit -m "feat(settings): give mishaps proper name tags" -m "Emit typed settings and log diagnostics while retaining detailed English fallback text."
  ```

### Task 2: Lossless frontend hydration journal and diagnostic state

**Files:**

- Modify: `apps/knx-web/src/settingsStore.ts`
- Modify: `apps/knx-web/src/settingsStore.test.ts`

**Interfaces:**

- Consumes Task 1's `SettingsDiagnostic` plus `message` response.
- Produces `SettingsHydrationState = "cached" | "hydrated" | "failed"`.
- Produces `getSettingsState()`, `subscribeToSettingsState()`, and `useSettingsState()` returning `{ hydration, diagnostic, fallbackMessage }`.
- Preserves synchronous `getSetting`, `settingsStorage`, serialized PUTs, one bootstrap sequence, and the closed adoption map.

- [x] **Step 1: Write failing bootstrap-journal tests**

  Against a deferred GET, write `preferredGateway` and overwrite cached `theme`; then resolve the GET with another theme plus untouched `density`. Assert local edits win, density survives, and one coalesced PUT carries the journal. Add cases for `null` deletion, GET failure, and adoption-409/re-read. Assert typed state and fallback transport without matching English contents.

- [x] **Step 2: Confirm RED**

  ```bash
  cd apps/knx-web
  npx vitest run src/settingsStore.test.ts
  ```

  Expected: FAIL because authoritative apply currently erases in-flight local edits.

- [x] **Step 3: Implement the per-key journal**

  Add `pendingBeforeHydration = new Map<string, unknown>()`. Before synchronization, record each final patch value, including `null`, after cache mutation. On successful GET/adoption, merge server settings first and journal second; then mark hydrated and enqueue exactly one journal patch through the existing PUT queue. Clear only after enqueue. Failed GET retains cache and journal.

- [x] **Step 4: Publish hydration and diagnostic state**

  Replace `SettingsResponse.notice` with the Task 1 union and `message`. Notify state subscribers on `hydrated` and `failed`. Reset all new state in `resetSettingsForTests()`.

- [x] **Step 5: Verify and commit**

  ```bash
  cd apps/knx-web
  npx vitest run src/settingsStore.test.ts
  npx tsc --noEmit
  git add src/settingsStore.ts src/settingsStore.test.ts
  git commit -m "fix(settings): stop hydration eating homework" -m "Journal pre-hydration edits and retain typed load diagnostics for subscribed consumers."
  ```

  Expected: PASS, including the existing eight-key adoption tests.

### Task 3: Preferred gateway and one-time workflow seeding

**Files:**

- Create: `apps/knx-web/src/gatewayPreference.ts`
- Create: `apps/knx-web/src/gatewayPreference.test.tsx`
- Modify: `apps/knx-web/src/BusMonitorPanel.tsx`
- Modify: `apps/knx-web/src/BusMonitorPanel.test.tsx`
- Modify: `apps/knx-web/src/LineScanPanel.tsx`
- Modify: `apps/knx-web/src/LineScanPanel.test.tsx`

**Interfaces:**

- Consumes Task 2's `useSettingsState()`.
- Produces `PREFERRED_GATEWAY_KEY`, `loadPreferredGateway()`, `savePreferredGateway(value)`, and `usePreferredGateway()`.
- Leaves endpoint validation at the existing server trust boundary. Discovery never persists the preference.

- [x] **Step 1: Write failing preference tests**

  Cover absent/blank values as `""`, trimming on read/save, persistence, explicit unset, and storage inside the single settings-cache document rather than a loose localStorage key.

- [x] **Step 2: Confirm RED**

  ```bash
  cd apps/knx-web
  npx vitest run src/gatewayPreference.test.tsx
  ```

  Expected: FAIL because the module does not exist.

- [x] **Step 3: Implement the preference owner**

  Use `settingsStorage` and `useSettingsRevision()` like existing preference modules. Blank saves call `removeItem`. Do not parse endpoints or call fetch/discovery/network code.

- [x] **Step 4: Write failing Bus Monitor seed tests**

  Cover cached first render, delayed authoritative replacement while untouched, typing before hydration, discovery before hydration, later preference changes, and a successfully attached/started session. Add a normal 404 reattach-before-hydration case proving no-session does not freeze seeding. Assert typing and discovery never write the preference.

- [x] **Step 5: Write failing Line Scan seed tests**

  Cover the same cache, authoritative, and manual rules. Add a 404 initial-poll case. Prove a successful existing scan, user-requested estimate, or started scan resolves ownership and blocks later reseeding.

- [x] **Step 6: Confirm panel RED**

  ```bash
  cd apps/knx-web
  npx vitest run src/BusMonitorPanel.test.tsx src/LineScanPanel.test.tsx
  ```

  Expected: FAIL because both gateway fields always start empty.

- [x] **Step 7: Implement one-time seeding**

  Initialize local fields from `loadPreferredGateway()`. Track touched/resolved refs. First authoritative hydration may replace the cached seed only while untouched and unused. Typing, discovery, and user-initiated connect/estimate/start resolve immediately. Background reattach/poll resolves only after a successful existing-session/scan response, never on 404. Workflow panels never call `savePreferredGateway`.

- [x] **Step 8: Verify and commit**

  ```bash
  cd apps/knx-web
  npx vitest run src/gatewayPreference.test.tsx src/BusMonitorPanel.test.tsx src/LineScanPanel.test.tsx
  npx tsc --noEmit
  git add src/gatewayPreference.ts src/gatewayPreference.test.tsx src/BusMonitorPanel.tsx src/BusMonitorPanel.test.tsx src/LineScanPanel.tsx src/LineScanPanel.test.tsx
  git commit -m "feat(settings): remember the gateway without calling it" -m "Seed fresh monitor and scan workflows once while preserving manual choices and active sessions."
  ```

  Expected: PASS with no request caused by reading the preference.

### Task 4: Shared lossless line-scan exclusion editor

**Files:**

- Create: `apps/knx-web/src/lineScanExclusions.ts`
- Create: `apps/knx-web/src/lineScanExclusions.test.ts`
- Create: `apps/knx-web/src/LineScanExclusionsEditor.tsx`
- Create: `apps/knx-web/src/LineScanExclusionsEditor.test.tsx`
- Modify: `apps/knx-web/src/LineScanPanel.tsx`
- Modify: `apps/knx-web/src/LineScanPanel.test.tsx`

**Interfaces:**

- Produces `LINE_SCAN_EXCLUSIONS_KEY`, load/save functions, `validateIndividualAddress(value)`, and `useLineScanExclusions()`.
- Produces `<LineScanExclusionsEditor disabled={boolean} />` over that shared key.
- Preserves the frozen `activeExclusions` request snapshot.

- [x] **Step 1: Write failing losslessness tests**

  Seed ordered valid text, two equal occurrences, whitespace-bearing text, out-of-range octets, and arbitrary text. Assert the loader returns every string byte-for-byte in order. Assert new-value validation accepts only complete `0..15.0..15.0..255` dotted addresses and rejects a duplicate.

- [x] **Step 2: Confirm RED**

  ```bash
  cd apps/knx-web
  npx vitest run src/lineScanExclusions.test.ts
  ```

  Expected: FAIL because the owner module does not exist.

- [x] **Step 3: Implement lossless storage and validation**

  Keep every array string unchanged; never filter, trim, deduplicate, or rewrite legacy data. Return validity separately. Save the complete array through `setSetting`. Keep dotted individual-address grammar separate from group-address parsing.

- [x] **Step 4: Write failing editor tests**

  Prove invalid legacy entries are visibly marked and removable; invalid/duplicate additions are rejected; removal needs two clicks; disabled blocks mutation; and two mounted editors synchronize. Identify rows by occurrence index or stable occurrence id, never value alone. Prove removing one equal occurrence preserves the other exactly.

- [x] **Step 5: Extract the editor**

  Move the heading, list, confirmation, input, and handlers out of `LineScanPanel.tsx`. Preserve accessible names and translate invalid-state copy. Do not use raw value alone as React key or removal identity.

- [x] **Step 6: Block unsafe scan work**

  Derive `hasInvalidExclusions`; disable estimate and start while true and render the translated reason. On start, copy the exact valid configured list into `activeExclusions`. Keep request-fingerprint and cancellation behavior unchanged.

- [x] **Step 7: Verify and commit**

  ```bash
  cd apps/knx-web
  npx vitest run src/lineScanExclusions.test.ts src/LineScanExclusionsEditor.test.tsx src/LineScanPanel.test.tsx
  npx tsc --noEmit
  git add src/lineScanExclusions.ts src/lineScanExclusions.test.ts src/LineScanExclusionsEditor.tsx src/LineScanExclusionsEditor.test.tsx src/LineScanPanel.tsx src/LineScanPanel.test.tsx
  git commit -m "feat(settings): give scan exclusions one clipboard" -m "Preserve legacy entries losslessly, share one editor, and refuse scans until invalid entries are removed."
  ```

  Expected: PASS; old invalid data remains visible and cannot authorize a scan.

### Task 5: Grouped settings surface and localized diagnostics

**Files:**

- Create: `apps/knx-web/src/settingsDiagnostic.ts`
- Create: `apps/knx-web/src/settingsDiagnostic.test.ts`
- Modify: `apps/knx-web/src/SettingsPanel.tsx`
- Modify: `apps/knx-web/src/SettingsPanel.test.tsx`
- Modify: `apps/knx-web/src/LogPanel.tsx`
- Modify: `apps/knx-web/src/LogPanel.test.tsx`
- Modify: `apps/knx-web/src/api.ts`
- Modify: `apps/knx-web/src/messages/en.ts`
- Modify: `apps/knx-web/src/messages/de.ts`
- Modify: `apps/knx-web/src/styles.css`
- Modify: `apps/knx-web/src/motionGuard.test.ts`

**Interfaces:**

- Consumes Tasks 1/2's diagnostic state, Task 3's preference, and Task 4's editor.
- Produces `formatSettingsDiagnostic(t, diagnostic, fallback)`, the only frontend mapping.
- Unknown/missing diagnostic variants display the server fallback; ordinary log entries remain unchanged.

- [x] **Step 1: Write failing formatter tests**

  Cover all four kinds and all five quarantine reasons in English and German without using server prose as expected output. Unknown-kind and absent-diagnostic cases return fallback unchanged.

- [x] **Step 2: Confirm RED**

  ```bash
  cd apps/knx-web
  npx vitest run src/settingsDiagnostic.test.ts
  ```

  Expected: FAIL because the formatter does not exist.

- [x] **Step 3: Implement frontend contracts and formatter**

  Export the discriminated union from `settingsStore.ts`; reuse it for `api.ts`'s optional `LogEntry.diagnostic`. Map known cases to typed catalogue keys. Keep fallback handling outside key selection so future server variants remain readable.

- [x] **Step 4: Write failing Settings composition tests**

  Assert translated Appearance, Language & data, and Bus & diagnostics headings; gateway persistence/unset; shared exclusions; and retained hydration diagnostic in German. Preserve all current appearance/language tests.

- [x] **Step 5: Write failing Log translation tests**

  Give a known typed diagnostic an unrelated English fallback and assert German catalogue output. Give an unknown diagnostic and assert fallback. Keep ordinary disclosure behavior.

- [x] **Step 6: Build and style the grouped panel**

  Group existing controls semantically. Add the gateway control via `usePreferredGateway()` and the shared editor. Render the latest diagnostic with `role="status"`. Add no Save button, registry, or second overlay. Style with existing tokens; if motion is added, guard it and update `motionGuard.test.ts`.

- [x] **Step 7: Verify the frontend**

  ```bash
  cd apps/knx-web
  npx vitest run src/settingsDiagnostic.test.ts src/settingsStore.test.ts src/SettingsPanel.test.tsx src/LogPanel.test.tsx src/gatewayPreference.test.tsx src/BusMonitorPanel.test.tsx src/lineScanExclusions.test.ts src/LineScanExclusionsEditor.test.tsx src/LineScanPanel.test.tsx src/motionGuard.test.ts
  npx tsc --noEmit
  npx vitest run
  ```

  Expected: PASS with all visible new copy in both catalogues.

- [x] **Step 8: Commit the surface**

  ```bash
  git add src/settingsDiagnostic.ts src/settingsDiagnostic.test.ts src/SettingsPanel.tsx src/SettingsPanel.test.tsx src/LogPanel.tsx src/LogPanel.test.tsx src/api.ts src/messages/en.ts src/messages/de.ts src/styles.css src/motionGuard.test.ts
  git commit -m "feat(settings): put every knob in the labelled drawer" -m "Group the settings surface and localize typed diagnostics in Settings and the session log."
  ```

### Task 6: Documentation and integrated verification

**Files:**

- Modify: `docs/adr/0029-application-settings-file.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `docs/KNOWN_LIMITATIONS.md`
- Create: `.ai/logs/2026-09-22_codex__t10_settings_implementation.md`
- Modify: `.ai/CURRENT_STATE.md`

**Interfaces:**

- Produces durable D8 status and resolves §122.
- Records `preferredGateway` as unencrypted installation-network metadata in `settings.json`.
- Leaves §121's multi-window last-writer limitation open.

- [x] **Step 1: Reconcile documentation**

  Amend ADR-0029 with the gateway metadata and backup consequence. Record implemented consumers and explicit non-goals in `IMPLEMENTATION_STATUS.md`. Mark §122 resolved with typed-diagnostic evidence. Leave §121 open; do not edit `LIMITATION_TRIAGE.md`.

- [x] **Step 2: Run focused source audits**

  ```bash
  rg -n "groupAddressNotation|Group address notation" apps/knx-web/src/SettingsPanel.tsx apps/knx-web/src/messages || true
  git diff --unified=0 main...HEAD -- apps/knx-web/src apps/knx-server/src docs/adr/0029-application-settings-file.md | rg '^\+' | rg "192\.168\.|10\.[0-9]|172\.(1[6-9]|2[0-9]|3[01])\." || true
  git diff --check
  ```

  Expected: no notation control, no private-LAN literal in added lines, and no whitespace error. Documentation-range examples remain allowed.

- [x] **Step 3: Run every repository gate**

  ```bash
  export CARGO_TARGET_DIR=/tmp/knxbench-t10-target
  export CARGO_INCREMENTAL=0
  export CARGO_PROFILE_DEV_DEBUG=0
  export CARGO_PROFILE_TEST_DEBUG=0
  cargo fmt --all --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace --no-fail-fast
  cargo run -p xtask -- check-layering
  cargo run -p xtask -- check-headers
  cargo run -p xtask -- check-anchors
  cargo deny check
  cd apps/knx-web
  npx tsc --noEmit
  npx vitest run
  ```

  Expected: every command exits 0. If stale artifacts are suspected, clean only `/tmp/knxbench-t10-target` and rerun the missing gate.

- [x] **Step 4: Write handover evidence**

  Record exact commands, test counts, no-network proof, rulings, and open §121 in the dated `.ai` log. Prepend `.ai/CURRENT_STATE.md` with the final commit range and next goal task.

- [x] **Step 5: Commit documentation**

  ```bash
  git add docs/adr/0029-application-settings-file.md docs/IMPLEMENTATION_STATUS.md docs/KNOWN_LIMITATIONS.md .ai/logs/2026-09-22_codex__t10_settings_implementation.md .ai/CURRENT_STATE.md
  git commit -m "docs(settings): explain what the knobs remember" -m "Record the settings surface, gateway metadata, localized diagnostics, verification, and open multi-window limitation."
  ```

- [ ] **Step 6: Review, integrate, and push**

  Run a fresh whole-branch review. Fix every Critical/Important finding with RED→GREEN proof, rerun affected gates, merge into `main`, rerun full gates on the merge commit, push, and verify `HEAD` equals `origin/main`.
