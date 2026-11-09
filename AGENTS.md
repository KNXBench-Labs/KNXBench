# KNXBench Project Instructions

## Mission

KNXBench is a modern, Linux-first KNX engineering application: an independent alternative to ETS. It imports ETS projects (`.knxproj`, one-way), edits them in its own `.knxdb` format and talks to KNX installations via KNXnet/IP. Build a clean, independent architecture; do not imitate ETS internals.

Priority order: **Correctness → Data Integrity → Compatibility → Maintainability → UX → Performance**

## Before Coding

- The repository and its maintained docs are the source of truth for project state, not chat history. Read the relevant `docs/` and the existing code first; do not duplicate existing functionality.
- Before non-trivial project work, read the restart snapshot `.ai/CURRENT_STATE.md`.
- Do not assume proprietary formats or undocumented behavior. Research architecture- and format-critical facts first and record the evidence in `docs/research/`.

Entry points: `docs/OPEN_WORK.md` (everything not done), `docs/status/LEDGER.md` (per-ID status of record), `docs/IMPLEMENTATION_STATUS.md`, `docs/ROADMAP.md`, `docs/KNOWN_LIMITATIONS.md`, `docs/DEVELOPER_PITFALLS.md`, `docs/adr/` (architecture decisions), `docs/VERIFICATION.md` (gates and commands).

## Architecture

```text
UI → Application / Services → KNX Domain Core → Infrastructure
                                  (Project Storage, Import/Export, Device Database, KNXnet/IP)
```

- The KNX core must not depend on the UI. External formats and protocols never dictate the domain model. `cargo run -p xtask -- check-layering` enforces it.
- External data flows through adapters: Parser → Validation → Normalized KNX Model → Application → Exporter.
- Project state changes only through validated, undoable domain commands (ADR-0039).
- Manufacturer and product data live in a separate, versioned database layer; never hard-code manufacturer products. The domain model is versioned and migratable.
- KNXnet/IP stays behind a dedicated service/adapter layer. Implement only technically verified protocol behavior.
- Do not solve domain problems with UI workarounds. UX aims at a modern professional engineering tool (explorer, tabs, inspector, search, command palette, dark/light mode, clear validation feedback).

Layout: `crates/` (domain, app, store, import, net, productdb, ...), `apps/` (`knx-cli`, `knx-server`, `knx-web`, `knx-desktop`, `knx-mcp`), `xtask/`, `tools/` (Python gates), `docs/`.

## Data Integrity & Compatibility

- Never silently discard information. Preserve unknown data where possible, otherwise mark it unsupported and report it in diagnostics and import/compatibility reports.
- Imports provide format/version detection, validation, warnings/errors, mapping information and loss reporting.
- Never claim compatibility that is not verified. Say "KNX-compatible", never "KNX-certified" or "full ETS compatibility".
- Live bus: tests use loopback or simulated transports. Real-installation access is read-only and only with explicit authorization. Writes need dedicated test hardware and explicit authorization naming device and operation. State how bus code was tested.

## Development

- Work incrementally with the smallest clean solution: simple, explicit, strongly typed, deterministic code in small modules. No unrelated refactors, speculative abstractions, unnecessary dependencies, global state or magic constants. Optimize only measured bottlenecks.
- Tests are mandatory for core functionality: unit, integration, import/export, roundtrip, migration, compatibility and regression tests. Cover malformed input, unsupported data, duplicates, invalid addresses, incompatible versions and large projects.
- New or materially edited source files start with a one-sentence purpose header, at most 100 characters (ADR-0018). Never raise the header ceiling in `xtask/src/headers.rs`.
- Distinguish verified facts from assumptions and expose uncertainty.
- Document important discoveries (formats, compatibility, protocol, limitations) in `docs/`, architecture decisions in `docs/adr/`.

## Verification

- Routine acceptance is the change-scoped package gate `python3 tools/run_package_gate.py --base origin/main` (`--full` for releases). Individual commands, build order and private corpus tests: `docs/VERIFICATION.md`.
- Direct `cargo test` inherits the real home. Export scratch `XDG_DATA_HOME` and `XDG_CONFIG_HOME` first (see `docs/DEVELOPER_PITFALLS.md`).
- A local gate that skips a check is partial. Say which part ran.

## Completion Criteria

Work is complete when relevant tests pass, the build works, implementation status, limitations and roadmap are updated where affected, and `.ai/CURRENT_STATE.md` is current.

## Restart Snapshot and Coordination

- `.ai/CURRENT_STATE.md` is a compact, overwrite-in-place restart snapshot. Its shape, contents, status vocabulary and compaction are defined in `.ai/archive/README.md`.
- Update it at meaningful operational changes and before pausing, handing off or finishing non-trivial work. Read-only questions and trivial tasks need no update. Do not infer current activity from an old entry.
- Before writing, re-read it and preserve other sessions' active or parked tracks. An isolated worktree does not grant ownership of another session's files or frozen gate inputs.

## No Local Machine Details

The repository is public. Never write local machine details into tracked files, commit messages, tags or release notes:

- absolute local paths (mount points, home directories, checkouts, worktrees, agent profile, scratch or evidence directories)
- host names, user names, exact kernel releases, CPU models, distribution or desktop names
- network addresses of a real installation (use a documentation address such as `203.0.113.250`)

Use placeholders: `<sources>` for the directory holding checkouts and worktrees, `<profile>` / `<profile-scratch>` for agent profile data, `~` for a home directory, `<host>` or "the dev machine" for the machine. Tests needing a home directory use a placeholder like `/home/alice`. A line that must show such a detail as an example carries the marker `local-info: allow`. `tools/check_local_info.py` enforces this for tracked files and commit messages (details: ADR-0135).

## Git

- Focused commits, no unrelated changes. Concise messages describing the actual change; a little humor is welcome.
- Author: `KNXBench <github@knxbench.com>`. Never add `Co-Authored-By:`.
- Do not run plain `git pull` on `main` (`pull.rebase=true` has replayed merge history). Do not rewrite published history without explicit authorization.

## Agent skills

Configuration for the engineering skills from [mattpocock/skills](https://github.com/mattpocock/skills).

### Issue tracker

Own work lives in `docs/OPEN_WORK.md` with one detail file per ID under `docs/open-work/`; GitHub Issues take external reports. See `docs/agents/issue-tracker.md`.

### Triage labels

The five default role labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `docs/GLOSSARY.md` plus `docs/adr/`. See `docs/agents/domain.md`.
