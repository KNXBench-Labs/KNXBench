# KNXBench

A modern, Linux-first KNX engineering application — an independent
alternative to ETS, built as a native Rust core with a Tauri/React desktop
shell. No ETS internals imitated; own `.knxproj` parser, own project format,
own product database.

Priorities, in order: **Correctness → Data Integrity → Compatibility →
Maintainability → UX → Performance**.

## Status

Session 5 (UI/UX) of a seven-session roadmap is in progress. See
[docs/IMPLEMENTATION_STATUS.md](docs/IMPLEMENTATION_STATUS.md) for the
detailed, per-cycle state and [docs/ROADMAP.md](docs/ROADMAP.md) for what's
next.

## Repository layout

```text
crates/          KNX domain core, product DB, ETS import/export, storage, net
apps/knx-cli/    Command-line tool
apps/knx-desktop/ Tauri + React desktop application
docs/            Architecture, data model, compatibility, ADRs — source of truth
```

## Documentation

Start with [`CLAUDE.md`](CLAUDE.md) for the project's guiding principles,
then [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) and
[`docs/DATA_MODEL.md`](docs/DATA_MODEL.md). Architecture decisions are
recorded in [`docs/adr/`](docs/adr/).

## Building

```bash
cargo build --workspace
cd apps/knx-desktop && npm install && npm run dev
```

## License

AGPL-3.0-or-later.
