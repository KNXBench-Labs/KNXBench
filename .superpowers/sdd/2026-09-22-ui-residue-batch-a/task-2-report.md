# Task 2 — FsPicker drag-and-drop and multi-file upload report

## Scope

Closed `KNOWN_LIMITATIONS.md` §24 only. `openMountPicker()` retains its
public `Promise<string | null>` contract: browser multi-selection means a
local upload batch, after which the person explicitly chooses one listed
project. The server's `/api/fs/upload` route remains one file per request.

## TDD evidence

### RED

Command run before production edits:

```text
cd apps/knx-web && npx vitest run src/FsPicker.test.tsx

❯ src/FsPicker.test.tsx (7 tests | 3 failed) 46ms
× uploads each selected local file sequentially, then shows the uploaded directory without selecting one
× accepts only file drags in protected mode and consumes their files only at drop
× reports a failed filename without claiming a partially uploaded batch succeeded

Test Files  1 failed (1)
     Tests  3 failed | 4 passed (7)
```

The failures were intentional and attributable to the absent feature: the
input was not `multiple`, file-drag `dragover` did not prevent default or set
`dropEffect`, and no batch error/status existed.

### GREEN

After the minimal `FsPicker` implementation and localized messages:

```text
cd apps/knx-web && npx vitest run src/FsPicker.test.tsx src/motionGuard.test.ts src/i18n.test.tsx && npx tsc --noEmit

Test Files  3 passed (3)
     Tests  30 passed (30)
Duration  435ms
```

`npx tsc --noEmit` exited 0 with no output.

The first batch test holds the first upload response until it proves the
second `POST` has not begun, then verifies both one-file `POST`s, the
`uploads` list refresh, a localized status, and that the picker stays open.
The protected-mode fake throws if `files` is read at dragover; it is accepted
without throwing and consumed only at drop. The partial-failure test verifies
the failed filename and `1 of 2` count while asserting there is no success
status.

## Full frontend gate

```text
cd apps/knx-web && npm test

Test Files  63 passed (63)
     Tests  891 passed (891)
Duration  3.85s
```

`git diff --check` exited 0.

## Self-review

- The shared `uploadFiles()` loop awaits each existing `uploadFile()` before
  issuing the next request, so it preserves the endpoint's per-request size
  behavior and deterministic error attribution.
- A first failure stops the batch, preserves already-written files, reports
  the precise failed filename/error and completed count, and clears any prior
  batch-success announcement.
- Dragover consults only `DataTransfer.types`; drop alone calls
  `Array.from(dataTransfer.files)`. Accepted drags use `copy`, and visual
  readiness uses existing accent/surface tokens with no motion declaration.
- English and German plural success text plus both-language failure text are
  covered by the i18n gate. No native picker, route, server, or KNX/LAN/
  hardware behavior changed.

## Concerns

None found in the scoped review. The UI deliberately does not add a multipart
batch endpoint or infer which uploaded project should open; both would widen
the established singular-selection contract.

## Fix round 1 — review findings

### Root cause and RED

The review correctly traced three independent symptoms to one local event
flow: fire-and-forget input/drop handlers had no synchronous batch ownership
guard; the per-file loop advanced `refreshKey`; and the foreign-data type
guards returned before clearing `dropReady`.

Before changing production code, the expanded picker test ran:

```text
cd apps/knx-web && npx vitest run src/FsPicker.test.tsx

❯ src/FsPicker.test.tsx (8 tests | 4 failed) 58ms
× uploads each selected local file sequentially, then shows the uploaded directory without selecting one
× does not start a second input or drop batch while an upload is in flight
× accepts only file drags in protected mode and consumes their files only at drop
× reports a failed filename without claiming a partially uploaded batch succeeded

Test Files  1 failed (1)
     Tests  4 failed | 4 passed (8)
```

The exact observable failures were: an `uploads` listing began after the first
success (`expected 0, got 1`), a second batch sent a second POST while the
first awaited (`expected 1, got 2`), a foreign drag retained
`data-drop-ready="true"`, and a two-success partial batch made two uploads
listing requests where the regression requires one.

### GREEN and full gate

`uploadingRef` now takes immediate ownership before the first await; the input
is disabled while its batch is active. The loop sets directory/refresh once
after complete success, or once after a partial failure with prior successes.
Foreign dragover and every drop clear readiness before their type guard.

```text
cd apps/knx-web && npx vitest run src/FsPicker.test.tsx src/motionGuard.test.ts src/i18n.test.tsx && npx tsc --noEmit

Test Files  3 passed (3)
     Tests  31 passed (31)
Duration  450ms
```

`npx tsc --noEmit` exited 0 with no output.

```text
cd apps/knx-web && npm test

Test Files  63 passed (63)
     Tests  892 passed (892)
Duration  3.75s
```

`git diff --check` exited 0. Self-review confirms the ref is checked before
any asynchronous boundary, batches remain sequential one-file requests,
partial success has no success status, and ready-state feedback remains
token-only and motion-free.

## Fix round 2 — stale listing responses

### RED

The listing effect applied every completion unconditionally. A pending root
request could therefore finish after a newer `uploads` request and overwrite
the current entries or error state.

```text
cd apps/knx-web && npx vitest run src/FsPicker.test.tsx

❯ src/FsPicker.test.tsx (10 tests | 2 failed) 68ms
× keeps the newer uploads listing when the original root response arrives late
× ignores an older rejected listing after a newer uploads listing succeeds

Test Files  1 failed (1)
     Tests  2 failed | 8 passed (10)
```

The delayed root success replaced `fresh.knxproj` with `stale-root.knxproj`;
the delayed root refusal rendered `Error: stale root refusal` after the newer
uploads listing had succeeded.

### GREEN and full gate

The effect assigns each list request a monotonically increasing generation
and an effect-local cleanup flag. Its success and failure continuations update
state only while both remain current, so superseded requests cannot overwrite
entries or errors and unmounted pickers also ignore late completions.

```text
cd apps/knx-web && npx vitest run src/FsPicker.test.tsx src/motionGuard.test.ts src/i18n.test.tsx && npx tsc --noEmit

Test Files  3 passed (3)
     Tests  33 passed (33)
Duration  465ms
```

`npx tsc --noEmit` exited 0 with no output.

```text
cd apps/knx-web && npm test

Test Files  63 passed (63)
     Tests  894 passed (894)
Duration  3.74s
```

`git diff --check` exited 0. Self-review confirms that the guard covers both
the `then` and `catch` paths, preserves the newest request's normal success
and error behavior, and does not alter the sequential upload contract.
