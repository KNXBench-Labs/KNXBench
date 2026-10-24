# KNXBench Project Instructions

## Mission

KNXBench is a modern, Linux-first KNX engineering application: an independent alternative to ETS. It imports ETS projects (`.knxproj`, one-way), edits them in its own `.knxdb` format and talks to KNX installations via KNXnet/IP.

Priority order: **Correctness → Data Integrity → Compatibility → Maintainability → UX → Performance**

Do not imitate ETS internals blindly. Build a clean, independent architecture.

## Before Coding

- Read the relevant `docs/` and the existing code first. Do not duplicate existing functionality.
- The repository and its maintained docs are the source of truth for project state, not chat history.
- Do not assume proprietary formats or undocumented behavior. Research architecture- and format-critical facts first and record the evidence in `docs/research/`.
- Agent handover: read `.ai/CURRENT_STATE.md` (newest entry on top) before project work and update it afterwards.

Entry points: `docs/OPEN_WORK.md` (everything not done), `docs/status/LEDGER.md` (per-ID status of record), `docs/IMPLEMENTATION_STATUS.md`, `docs/ROADMAP.md`, `docs/KNOWN_LIMITATIONS.md`, `docs/DEVELOPER_PITFALLS.md`, `docs/adr/` (architecture decisions), `docs/VERIFICATION.md` (gates).

## Architecture

```text
UI → Application / Services → KNX Domain Core → Infrastructure
                                  (Project Storage, Import/Export, Device Database, KNXnet/IP)
```

- The KNX core must not depend on the UI. External formats and protocols never dictate the domain model. `cargo run -p xtask -- check-layering` enforces it.
- External data flows: Parser → Validation → Normalized KNX Model → Application → Exporter. Use adapters for formats and protocols.
- Project state changes only through validated, undoable domain commands (ADR-0039).
- Manufacturer and product data live in a separate, versioned database layer. Never hard-code manufacturer products. The domain model is versioned and migratable.
- Keep KNXnet/IP behind a dedicated service/adapter layer, separate from the UI. Implement only technically verified protocol behavior.
- Do not solve domain problems with UI workarounds. UX aims at a modern professional engineering tool (explorer, tabs, inspector, search, command palette, dark/light mode, clear validation feedback).
- Avoid unnecessary frameworks, dependencies and abstraction layers.

Layout: `crates/` (domain, app, store, import, net, productdb, ...), `apps/` (`knx-cli`, `knx-server`, `knx-web`, `knx-desktop`, `knx-mcp`), `xtask/`, `tools/` (Python gates), `docs/`.

## Data Integrity & Compatibility

- Never silently discard information. Preserve unknown data where possible, otherwise mark it unsupported and report it in diagnostics and import/compatibility reports.
- Imports provide format/version detection, validation, warnings/errors, mapping information and loss reporting.
- Never claim compatibility that is not verified. Say "KNX-compatible", never "KNX-certified" or "full ETS compatibility".
- Live bus: tests use loopback or simulated transports. Real-installation access is read-only and only with explicit authorization. Writes need dedicated test hardware and explicit authorization naming device and operation. State how bus code was tested.

## Build, Test, Run

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
(cd apps/knx-web && npm ci && npm test && npm run build)   # build the web bundle before workspace-wide Rust steps
cargo run -p xtask -- check-layering     # also: check-headers, check-anchors, check-ledger, check-corpus-gates
python3 tools/check_documentation.py
python3 tools/check_local_info.py
python3 tools/run_package_gate.py --base origin/main     # routine package gate; --full for releases
cargo build -p knx-server                # web deployment; desktop shell: apps/knx-desktop/src-tauri
```

- Private corpus tests (need `OriginalData/`): `unshare --user --map-root-user --net sh -c 'ip link set lo up && exec "$@"' sh python3 tools/run_corpus_tests.py`. See `docs/VERIFICATION.md`.
- A local gate that skips a check is partial. Say which part ran.
- Direct `cargo test` inherits the real home. Export scratch `XDG_DATA_HOME` and `XDG_CONFIG_HOME` first (see `docs/DEVELOPER_PITFALLS.md`).

## Development

- Work incrementally with the smallest clean solution. No unrelated refactors.
- Tests are mandatory for core functionality: unit, integration, import/export, roundtrip, migration, compatibility and regression tests. Cover malformed input, unsupported data, duplicates, invalid addresses, incompatible versions and large projects.
- Prefer simple, explicit, strongly typed, deterministic code in small modules. Avoid speculative abstractions, global state, duplicated logic, magic constants and UI/domain coupling. Optimize only measured bottlenecks.
- New or materially edited source files start with a one-sentence purpose header, at most 100 characters (ADR-0018). Never raise the header ceiling in `xtask/src/headers.rs`.
- Distinguish verified facts from assumptions and expose uncertainty. Prefer maintainability over cleverness and data integrity over convenience.
- Document important discoveries (formats, compatibility, protocol, limitations) in `docs/`, architecture decisions in `docs/adr/`.

## Completion Criteria

Work is complete when relevant tests pass, the build works, implementation status, limitations and roadmap are updated where affected, and `.ai/CURRENT_STATE.md` is current.

## No Local Machine Details

The repository is public. Never write local machine details into tracked files, commit messages, tags or release notes:

- absolute local paths (mount points, home directories, checkouts, worktrees, agent profile, scratch or evidence directories)
- host names, user names, exact kernel releases, CPU models, distribution or desktop names
- network addresses of a real installation (use a documentation address such as `203.0.113.250`)

Use placeholders: `<sources>` for the directory holding checkouts and worktrees, `<profile>` / `<profile-scratch>` for agent profile data, `~` for a home directory, `<host>` or "the dev machine" for the machine. Tests needing a home directory use a placeholder like `/home/alice`.

`tools/check_local_info.py` enforces this for tracked files (part of `tools/check_documentation.py`, so CI and the web gate run it) and checks commit messages with `--message-file` (usable as a `commit-msg` hook) or `--commits RANGE` before a push. A line that must show such a detail as an example carries the marker `local-info: allow`. Tokens no machine can report go into the untracked, clone-local `.git/info/local-info-deny`.

## Git

- Focused commits, no unrelated changes. Concise messages describing the actual change; a little humor is welcome.
- Author: `KNXBench <github@knxbench.com>`. Never add `Co-Authored-By:`.
- Do not run plain `git pull` on `main` (`pull.rebase=true` has replayed merge history). Do not rewrite published history without explicit authorization.

Agent identity, model selection, task assignment and cross-agent memory are managed externally by Hermes and must not be duplicated here.
