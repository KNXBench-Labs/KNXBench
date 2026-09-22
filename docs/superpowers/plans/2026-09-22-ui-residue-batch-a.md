# UI Residue Batch A Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close T12's six available UI/file-workflow limitations with independently tested, independently revertible changes.

**Architecture:** Extend the existing owners rather than adding a framework: `App` issues one-shot reveal and load-recovery intents, `ProjectExplorer` owns branch expansion, `FsPicker` coordinates browser files over the existing single-file route, and `knx-server` owns current-tree projection and file-backed HTTP bodies. Existing toasts, catalogues, auth layering, and full-tree responses remain authoritative.

**Tech Stack:** React 19, TypeScript, Vitest/happy-dom, Rust 2021, Axum, tower-http `ServeFile`, Tokio, Cargo workspace gates.

**Spec:** `docs/superpowers/specs/2026-09-22-ui-residue-batch-a-design.md`

## Global Constraints

- Implement only `KNOWN_LIMITATIONS.md` §§19, 23, 24, 30, 96, and 118. Do not implement §§49/50 because `.superpowers/sdd/goal/task-14-report.md` is absent.
- Do not generalize adjacent UI or refactor neighboring components.
- Do not initiate KNX, multicast, LAN, gateway, or hardware traffic.
- Do not introduce private-network literals or the forbidden individual-address fixture named in T12.
- Preserve slash-only group-address formatting and the project's selected notation.
- Every new user-facing string must exist in `messages/en.ts` and `messages/de.ts`.
- New visual feedback uses existing `--knx-*` tokens and no unguarded motion.
- Every new `.rs`, `.ts`, or `.tsx` file needs a purpose-sentence header without a version number. Prefer no new source files in this batch.
- Commit as `KNXBench <github@knxbench.com>`, without a co-author trailer, with a short gloomy/funny subject and explanatory body.
- Keep the repository buildable after every task and push each completed task.

## Review Focus

- Repeating the same search hit after manually collapsing it again must reveal it again; Task 1's generation-based test covers this.
- A device rendered in topology and building structures must scroll only its canonical topology occurrence; Task 1 asserts one `scrollIntoView` call.
- A non-file drag over `FsPicker` must not activate or accept the drop; Task 2 models protected drag mode and checks `defaultPrevented`.
- A batch whose later upload fails must preserve earlier successes but never announce full success; Task 2 asserts request order and error text.
- A successful snapshot carrying a foreign or empty client token must never recover the current tree; Task 5 keeps the existing foreign-token probes and adds an empty-token recovery assertion.

---

### Task 1: Reveal search hits in collapsed Project Explorer branches (§19)

**Files:**
- Modify: `apps/knx-web/src/App.tsx`
- Modify: `apps/knx-web/src/App.test.tsx`
- Modify: `apps/knx-web/src/ProjectExplorer.tsx`
- Modify: `apps/knx-web/src/ProjectExplorer.test.tsx`
- Modify: `docs/KNOWN_LIMITATIONS.md` (stable heading §19)

**Interfaces:**
- Produces: `ProjectExplorer` optional `revealRequest: { selection: Selection; generation: number } | null` prop.
- Preserves: `selectEntity(sel)` as canonical view/Inspector selection and all ordinary tree-click collapse behavior.

- [ ] **Step 1: Write failing component tests**

In `ProjectExplorer.test.tsx`, render nested topology, building-part, and group-range fixtures with a `revealRequest`. Collapse their ancestor toggles, rerender with a larger generation, and assert the selected row returns. Stub `HTMLElement.prototype.scrollIntoView` and assert one canonical call for a duplicated device. Rerender the same selection with another generation after collapsing again to prove repeated reveals work.

In `App.test.tsx`, open search, choose a result, and assert `ProjectExplorer` receives the external reveal while an ordinary explorer selection does not create a new reveal generation.

- [ ] **Step 2: Run RED tests**

Run: `cd apps/knx-web && npx vitest run src/ProjectExplorer.test.tsx src/App.test.tsx`

Expected: FAIL because `revealRequest` is not accepted and collapsed ancestors stay closed.

- [ ] **Step 3: Implement the smallest reveal path**

Add to `TreeNode`:

```tsx
const labelRef = useRef<HTMLButtonElement>(null);
useEffect(() => {
  if (props.revealGeneration !== undefined) setOpen(true);
}, [props.revealGeneration]);
useEffect(() => {
  if (props.scrollOnReveal && props.revealGeneration !== undefined) {
    labelRef.current?.scrollIntoView({ block: "nearest" });
  }
}, [props.revealGeneration, props.scrollOnReveal]);
```

Pass the generation only through ancestors that contain the requested selection. For devices, use the topology/unassigned occurrence and do not mark the building copy as the scroll target. In `App`, keep a monotonic generation and route `Search.onSelect` through a wrapper that records the request before calling `selectEntity`; pass the request to `ProjectExplorer`.

- [ ] **Step 4: Run GREEN tests and static checks**

Run: `cd apps/knx-web && npx vitest run src/ProjectExplorer.test.tsx src/App.test.tsx && npx tsc --noEmit`

Expected: PASS.

- [ ] **Step 5: Resolve and commit §19**

Rewrite the §19 entry to record external-selection expansion, nearest scrolling, repeated-generation behavior, and canonical device occurrence.

Commit subject: `feat(search): branches reluctantly reveal answers`

---

### Task 2: Add drag-and-drop and multi-file upload to `FsPicker` (§24)

**Files:**
- Modify: `apps/knx-web/src/FsPicker.tsx`
- Modify: `apps/knx-web/src/FsPicker.test.tsx`
- Modify: `apps/knx-web/src/messages/en.ts`
- Modify: `apps/knx-web/src/messages/de.ts`
- Modify: `apps/knx-web/src/styles.css`
- Modify: `docs/KNOWN_LIMITATIONS.md` (stable heading §24)

**Interfaces:**
- Consumes: existing `uploadFile(file): Promise<string>` and `/api/fs/upload` one-file contract.
- Preserves: `openMountPicker(filters): Promise<string | null>`; batch upload never auto-selects one file.

- [ ] **Step 1: Write failing upload tests**

Add tests which provide two `File` objects through a `multiple` input and through a protected-mode `DataTransfer` fake. Assert two sequential `POST /api/fs/upload` calls, navigation/list refresh under `uploads`, a localized `role="status"` count, `dropEffect="copy"`, token-backed `data-drop-ready`, and rejection of drags whose types omit `Files`. Add a partial-failure test: first POST succeeds, second fails, no batch-success status appears, and the error names the failed file.

- [ ] **Step 2: Run RED test**

Run: `cd apps/knx-web && npx vitest run src/FsPicker.test.tsx`

Expected: FAIL because the input is single-select and no drop handlers or batch status exist.

- [ ] **Step 3: Implement sequential batch coordination**

Inside `Modal`, add one shared routine:

```tsx
async function uploadFiles(files: readonly File[]) {
  setError(null);
  setUploadSummary(null);
  for (const file of files) {
    try { await uploadFile(file); }
    catch (error) {
      setError(t("fsPicker.uploadFailed", { file: file.name, error: String(error) }));
      return;
    }
  }
  setDir("uploads");
  setRefreshKey((key) => key + 1);
  setUploadSummary(t("fsPicker.uploaded", { count: files.length }));
}
```

Make listing depend on `refreshKey`, add `multiple`, and forward `Array.from(input.files ?? [])`. The upload label handles file-only dragover/drop, exposes `data-drop-ready`, and resets state on leave/drop. Add English/German count and failure messages. Style ready state with existing accent/border/surface tokens only.

- [ ] **Step 4: Run GREEN and guard tests**

Run: `cd apps/knx-web && npx vitest run src/FsPicker.test.tsx src/motionGuard.test.ts src/i18n.test.tsx && npx tsc --noEmit`

Expected: PASS.

- [ ] **Step 5: Resolve and commit §24**

Rewrite §24 to state exactly that multi-selection covers local upload batches while project selection remains singular.

Commit subject: `feat(files): let uploads arrive in a small herd`

---

### Task 3: Stream `/api/project/download` from its temporary file (§23)

**Files:**
- Modify: `apps/knx-server/Cargo.toml`
- Modify: `apps/knx-server/src/fs_routes.rs`
- Modify: `apps/knx-server/tests/http_fs_routes.rs`
- Modify: `docs/KNOWN_LIMITATIONS.md` (stable heading §23)

**Interfaces:**
- Consumes: installed `tower_http::services::ServeFile` and `tower::ServiceExt`.
- Adds: test-only direct `http-body-util = "0.1"` dependency (already locked transitively) so the regression test can inspect individual body frames.
- Preserves: live in-memory serialization with opaque entries and manufacturer refs, `application/octet-stream`, and attachment filename `project.knxdb`.

- [ ] **Step 1: Write a RED streaming proof**

Add `http-body-util = "0.1"` under `[dev-dependencies]`, then add a `#[cfg(test)]` unit around a private `stream_temp_file` helper. Create a temporary file larger than twice a deliberately small test chunk, consume body frames individually with `http_body_util::BodyExt::frame`, and assert at least two non-empty data frames plus byte equality. Add an integration success test that creates a new project, downloads it, checks disposition/content type, and verifies the body opens as a KNX store rather than merely being non-empty.

- [ ] **Step 2: Run RED tests**

Run: `cargo test -p knx-server fs_routes::tests::stream_temp_file_emits_bounded_frames -- --exact && cargo test -p knx-server --test http_fs_routes`

Expected: FAIL because the helper does not exist and the route currently builds one `Vec<u8>` body.

- [ ] **Step 3: Replace whole-file buffering**

Create a response with `ServeFile::new(temp_path).with_buf_chunk_size(DOWNLOAD_CHUNK_BYTES)`, call it with an empty GET request, map its body into `axum::body::Body`, attach an `Arc<TempPath>` response extension so cleanup occurs only after the response is dropped, override content type/disposition, and return it. Delete `std::fs::read(tmp.path())` entirely.

- [ ] **Step 4: Run GREEN tests and Rust hygiene**

Run: `cargo test -p knx-server fs_routes::tests::stream_temp_file_emits_bounded_frames -- --exact && cargo test -p knx-server --test http_fs_routes && cargo fmt --all --check && cargo clippy -p knx-server --all-targets -- -D warnings`

Expected: PASS with multiple frames proved.

- [ ] **Step 5: Resolve and commit §23**

Rewrite §23 with the chunked file-body mechanism and proof; retain the note that serialization still creates one temporary SQLite file.

Commit subject: `fix(download): stop swallowing the whole database`

---

### Task 4: Expose browser project download in the File menu (§30)

**Files:**
- Modify: `apps/knx-web/src/App.tsx`
- Modify: `apps/knx-web/src/App.test.tsx`
- Modify: `apps/knx-web/src/messages/en.ts`
- Modify: `apps/knx-web/src/messages/de.ts`
- Modify: `docs/KNOWN_LIMITATIONS.md` (stable heading §30)

**Interfaces:**
- Consumes: `filePicker.isTauri()` and `GET /api/project/download`.
- Produces: browser-native anchor download; no `Blob`, object URL, or frontend buffer.

- [ ] **Step 1: Write RED menu tests**

Assert that the plain web File menu shows localized Download, disables it without a tree, and on activation clicks a temporary anchor whose `href` ends in `/api/project/download` and whose `download` is `project.knxdb`. Mock `isTauri()` true and assert the command is absent.

- [ ] **Step 2: Run RED test**

Run: `cd apps/knx-web && npx vitest run src/App.test.tsx`

Expected: FAIL because no download command exists.

- [ ] **Step 3: Implement native browser navigation**

Add a small local `downloadProject()` helper in `App.tsx`:

```ts
const anchor = document.createElement("a");
anchor.href = "/api/project/download";
anchor.download = "project.knxdb";
anchor.click();
```

Render its localized File-menu button only when `!isTauri()`, disabled when `tree` is null.

- [ ] **Step 4: Run GREEN and type checks**

Run: `cd apps/knx-web && npx vitest run src/App.test.tsx src/i18n.test.tsx && npx tsc --noEmit`

Expected: PASS.

- [ ] **Step 5: Resolve and commit §30**

Rewrite §30 to distinguish browser Download from mounted-volume Save As and native Tauri Save As.

Commit subject: `feat(download): give browser projects an exit`

---

### Task 5: Recover owned successful loads through current-project retrieval (§96)

**Files:**
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/src/routes.rs`
- Modify: `apps/knx-server/tests/http_project_routes.rs`
- Modify: `apps/knx-server/tests/http_auth.rs`
- Modify: `apps/knx-web/src/api.ts`
- Modify: `apps/knx-web/src/api.test.ts`
- Modify: `apps/knx-web/src/App.tsx`
- Modify: `apps/knx-web/src/App.test.tsx`
- Modify: `docs/KNOWN_LIMITATIONS.md` (stable heading §96)

**Interfaces:**
- Produces: `domain::current_project_tree(&AppState) -> Result<ProjectTree, String>`.
- Produces: authenticated `GET /api/project` and `api.currentProject(): Promise<ProjectTree>`.
- Consumes: exact `ownsOperation({ clientToken }, snapshot)` and only `snapshot.status === "succeeded"`.

- [ ] **Step 1: Write RED server and client tests**

Server: `GET /api/project` before a project exists returns a client error; after `POST /api/project/new`, it returns a full tree with current undo/redo and import counts. Add it to the guarded-route inventory.

Client API: assert `currentProject()` sends GET to `/api/project`.

App: replace the old §96 “still reports failure” expectation with recovery. Mock an owned succeeded snapshot and `currentProject` tree; assert tree renders, banner clears, and no alert toast exists. Add cases for foreign/empty token and a failed recovery GET, both of which remain failures.

- [ ] **Step 2: Run RED tests**

Run: `cargo test -p knx-server --test http_project_routes && cargo test -p knx-server --test http_auth && cd apps/knx-web && npx vitest run src/api.test.ts src/App.test.tsx`

Expected: FAIL because the route/client do not exist and `runLoad` still converts succeeded snapshots to local failures.

- [ ] **Step 3: Implement current-tree read and recovery**

Implement `current_project_tree` by locking project, command stack, and import counts and calling existing `tree_with_state`. Register `GET /api/project` in `project_routes` and map “no project open” to `400 Bad Request`.

In `runLoad`, defer `reportError(e)` until after the final snapshot decision. For an owned succeeded snapshot, await `api.currentProject()`, call `resetTree`, set store-path state from `final.kind === "open"`, clear banner state, and return. If that fetch fails, use its error as the local failure; all other snapshots preserve the current error path.

- [ ] **Step 4: Run GREEN tests and focused checks**

Run: `cargo test -p knx-server --test http_project_routes && cargo test -p knx-server --test http_auth && cd apps/knx-web && npx vitest run src/api.test.ts src/App.test.tsx && npx tsc --noEmit`

Expected: PASS.

- [ ] **Step 5: Resolve and commit §96**

Rewrite §96 to record exact-token recovery and current-server-truth semantics; retain the rationale that foreign/missing tokens never recover.

Commit subject: `fix(load): retrieve the project that already arrived`

---

### Task 6: Announce successful project loads (§118)

**Files:**
- Modify: `apps/knx-web/src/App.tsx`
- Modify: `apps/knx-web/src/App.test.tsx`
- Modify: `apps/knx-web/src/messages/en.ts`
- Modify: `apps/knx-web/src/messages/de.ts`
- Modify: `docs/KNOWN_LIMITATIONS.md` (stable heading §118)

**Interfaces:**
- Consumes: existing `pushFun` and `ToastStack` non-error `role="status"` behavior.
- Produces: localized `loadProgress.succeeded` message for direct and recovered loads.

- [ ] **Step 1: Write RED accessibility assertions**

For direct import, direct native open, and §96 recovery, assert exactly one `.toast--fun[role="status"]` appears with the active-locale success message and source filename. Assert no `role="alert"` for recovery.

- [ ] **Step 2: Run RED test**

Run: `cd apps/knx-web && npx vitest run src/App.test.tsx`

Expected: FAIL because success currently only unmounts the banner.

- [ ] **Step 3: Add one shared success exit**

Add English/German `loadProgress.succeeded`. Factor the common successful tail of `runLoad` into a local helper that resets the tree, sets `hasStorePath`, clears banner state, and calls:

```ts
pushFun(t("loadProgress.succeeded", { source: fileNameOf(path) }));
```

Use it for the direct POST response and §96 recovery so neither can drift.

- [ ] **Step 4: Run GREEN accessibility, catalogue, and type checks**

Run: `cd apps/knx-web && npx vitest run src/App.test.tsx src/Toast.test.tsx src/i18n.test.tsx && npx tsc --noEmit`

Expected: PASS.

- [ ] **Step 5: Resolve and commit §118**

Rewrite §118 to identify the existing toast live region and the direct/recovered success coverage.

Commit subject: `fix(a11y): let successful loads say so`

---

### Task 7: Reconcile documentation and run whole-repository gates

**Files:**
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Create: `.ai/logs/2026-09-22_codex__t12_ui_residue_batch_a.md`
- Modify: `.ai/CURRENT_STATE.md`

**Interfaces:**
- Records: six closed limitations, §§49/50 deferred because T14 report was absent, exact gate evidence, and absence of KNX/network traffic.

- [ ] **Step 1: Update durable status and handover**

Add a compact T12 entry to `IMPLEMENTATION_STATUS.md`, a detailed session log with commit/gate evidence, and a latest handover entry naming the next goal task. Do not edit `LIMITATION_TRIAGE.md`.

- [ ] **Step 2: Run frontend gates**

Run in `apps/knx-web`:

```bash
npx tsc --noEmit
npx vitest run
```

Expected: exit 0.

- [ ] **Step 3: Run Rust and repository gates**

Run from repository root with the established low-quota target settings if needed:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo run -p xtask -- check-layering
cargo run -p xtask -- check-headers
cargo run -p xtask -- check-anchors
cargo deny check
```

Expected: every command exits 0; header absence does not exceed the documented ceiling.

- [ ] **Step 4: Audit scope and forbidden literals**

Run focused `git diff`/`rg` audits for the six stable headings, message keys, no report-preview edits, no forbidden address/private-network literal, and no KNX/network command. Run `git diff --check`.

- [ ] **Step 5: Commit and push evidence**

Commit subject: `docs(ui): six loose ends file their paperwork`

Push, fetch, and prove `HEAD == origin/main` only after final review and integration.
