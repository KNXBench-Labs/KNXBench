# 2026-10-04 — AR08 Web half: project-password dialog

Agent: Claude, goal-ui.md owner session; Web lock `ccdb9038` (held on for KL-60).

- Contract (`apps/knx-server/src/routes.rs:665-695`, `errors.rs:100-150`,
  `domain.rs:396-407`): `POST /api/project/import` takes an optional
  `password` (empty counts as none); a refusal is `422` with body
  `{ error, kind }`, where `kind` is `projectPasswordRequired` or
  `projectPasswordWrong`. Nothing is written on refusal.
- Change: `api.importProject(path, clientToken, password?)` sends the field
  only when non-empty. `projectPassword.ts` classifies the refusal (kept out
  of `api.ts` so App tests run the real logic). `ProjectPasswordDialog.tsx` is
  built on the shared `Overlay`. In `App.tsx`, `runLoad` has a `kind`; an
  import refusal opens the dialog instead of `reportError` and the failure
  banner; submit retries with the password in a closure.
- RED: 3 failing cases plus two missing modules. A slip in the message
  script (seven entries joined by a literal `\n`) was caught before any
  test run and repaired.
- Controls: the e2e fails 2/2 against the previous app. 7 mutants caught.
  `refusal-reported-as-error` survived at first: the unit test looked for a
  toast class that does not exist. It now asserts that the error text and the
  "Could not load" banner are absent.
- Full gate (first attempt): web build, fmt, clippy -D warnings, workspace tests 3,178 passed / 0 failed / 177 ignored in 174 blocks with 0 skip markers, five repository gates (headers 490 ok; anchors 450 ok; ledger 186 rows), tsc, `check:flow-study`, Vitest 1,869/106 files, complete Chromium suite 114/114, whitespace; source frozen.
