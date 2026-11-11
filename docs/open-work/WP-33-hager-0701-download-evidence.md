# WP-33 — Record the Hager 0701 download reports (#38)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **open.**

## Basis

Four real-device runs on `M-0006_A-E052-12-3406` (mask 0701, application
version 18) with `0.1.0-alpha.20+g7e5612b`: complete, parameters only,
parameters only with an uncovered group-address change, parameters plus group
addresses; the reporter confirmed device function each time. Only the
`partial-parameters` report file is attached.

## Work

Follow `docs/contracts/download-verification-reports.md`: check the program
hash, add an evidence file under `docs/evidence/` and a
`verified_downloads.json` entry for `partial-parameters`. The other scopes
need their own report files (requested on #38); “group addresses only” stays
untested unless a report arrives.
