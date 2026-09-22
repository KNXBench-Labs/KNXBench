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
