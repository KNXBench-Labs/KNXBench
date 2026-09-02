# IMPLEMENTATION_STATUS.md

Last updated: 2026-09-02

## Where the project stands

| Session | Scope | Status |
| --- | --- | --- |
| 0 | Technical research | **Done** — see [RESEARCH.md](RESEARCH.md) |
| 1 | Architecture | Not started |
| 2 | KNX core | Not started |
| 3 | ETS project import | Not started |
| 4 | Manufacturer database | Not started |
| 5 | UI / UX | Not started |
| 6 | KNXnet/IP | Not started |
| 7 | Integration & hardening | Not started |

**There is no application code yet.** The repository currently contains research
artifacts, sample data, and two read-only analysis scripts.

## What exists

| Path | Purpose |
| --- | --- |
| `docs/RESEARCH.md` | Session 0 result: verified findings on the `.knxproj` format, manufacturer data, master data, KNXnet/IP, KNX Secure, licensing, risks. |
| `tools/inspect_knxproj.py` | Stdlib-only inspector that reproduces every container/project number quoted in `RESEARCH.md`. |
| `monitor_bus.py` | Captures live telegrams from the KNXnet/IP gateway into `bus_traffic.jsonl`. |
| `Unser Zuhause ets4 - 2025-12-15.knxproj` | Real ETS 4.1.8 reference project (schema 11), unprotected. |
| `project_dump.json`, `group_addresses.json`, `devices.json` | `xknxproject` output for the same project — a cross-check baseline, known to be lossy (RESEARCH.md §7.1). |
| `bus_traffic.jsonl` | 280 captured live telegrams (gitignored). |

## Environment

* Python 3.14 venv at `.venv/`. Use `.venv/bin/python`.
* `xknx` 3.20.0 (MIT) — live bus access.
* `xknxproject` 3.10.0 (**GPL-2.0-only**) — reference/test use only; must never become a runtime dependency. See RESEARCH.md §10.
* KNXnet/IP gateway at `192.0.2.1:3671`, tunnelling verified working.

No build, lint, or test tooling is configured yet. The stack has not been chosen —
that is a Session 1 decision and belongs in `docs/ARCHITECTURE.md` plus an ADR.

## Next session

Session 1 (Architecture). Inputs: `RESEARCH.md` §12 (twelve carried-forward
recommendations) and its list of open questions. Deliverables per the development
strategy: `ARCHITECTURE.md`, `DATA_MODEL.md`, `IMPORT_EXPORT.md`,
`COMPATIBILITY.md`, `ROADMAP.md`, and the first ADRs — at minimum one recording
the own-parser decision and the `xknxproject` licensing boundary.

Blocking gap to close before Session 3: no ETS5 (schema 13/14/20) or ETS6
(schema 21+) sample project is available. Everything verified so far is schema 11.
