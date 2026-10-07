# 2026-10-04 — Claude — AR08 password-protected import entry paths

## What changed
- `knx-etsproj`: `ProjectPassword` (redacting `Debug`, no Display/Clone/serde),
  `import_knxproj_with` / `import_knxproj_bytes_with`; the old entry points
  delegate with `None`. `Container::decrypted_payload()`; a decrypted import
  adds an `unsupported` report entry for the lost protection.
- Container fix: inflate failure of a decrypted Deflated entry →
  `WrongPassword` (was `Read { cause: "corrupt deflate stream" }`); Display
  now says a damaged encrypted entry reads the same way.
- `knx-app`: `import_ets_project_with_password`.
- CLI: `knx import --password-stdin` (first line, one trailing CR/LF
  stripped, empty refused); `--password`/`--password=` refused without echo.
- Server: `ImportBody { path, clientToken?, password? }` (no Debug, empty =
  none); `domain::open_project_with_password` → `LoadFailure { message, kind }`;
  `ApiError::refused(kind, msg)` → `422 { error, kind }`; `tracked_load`
  takes a closure.
- Fixture: `crates/knx-testsupport/fixtures/zipcrypto-minimal.knxproj`
  (Info-ZIP `zip -P`, commands in `fixtures/README.md`).

## Evidence
RED first for every layer (library 3 fail, app 2, CLI 5, server 4, report 1,
false-accept 1); empty-string mutant killed. fmt, clippy (workspace minus
knx-desktop, whose build script needs the Web dist locally), tests of
knx-etsproj/-app/-cli/-testsupport/-secure and full knx-server green;
check-anchors 445/274, check-ledger 186, check-headers 474/157/17.

## Handoff to UI (Web lock not touched)
`POST /api/project/import` accepts `password`; on `422` read `kind`:
`projectPasswordRequired` → ask, `projectPasswordWrong` → ask again. Never
store the password in client state beyond the request, never log it.

## Not covered
Real ETS4/ETS5 sample, AES, `knx diff`/`POST /api/project/diff`/`products
ingest` with passwords.
