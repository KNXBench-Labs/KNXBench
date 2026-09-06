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

## Testing the app

Two shells share the same `knx-server`/`knx-web` core: a Docker web
deployment and a native Tauri desktop app.

### Docker (web)

```bash
docker build -t knxbench-server -f apps/knx-server/Dockerfile .
docker run -d -p 8080:8080 -v "$(pwd)/data:/data" knxbench-server
curl -sf http://127.0.0.1:8080/healthz
```

Open `http://127.0.0.1:8080` in a browser. `/data` is where
`KNX_DATA_DIR` persists projects — mount a host directory so work
survives a container restart. Env vars the image respects:
`KNX_PORT` (default `8080`), `KNX_DATA_DIR` (default `/data`),
`KNX_STATIC_DIR` (frontend bundle location, pre-set in the image).

An automated end-to-end check (build, run, `/healthz`, import a
`.knxproj`) lives at `apps/knx-server/scripts/smoke-test.sh` — run it
from the repo root:

```bash
apps/knx-server/scripts/smoke-test.sh
```

### Native desktop app (Tauri)

```bash
cargo install tauri-cli --version "^2" --locked   # once
cd apps/knx-desktop
npm install
cargo tauri dev
```

`cargo tauri build` produces a release bundle. On Hyprland/Wayland, an
agent-friendly driver script (build, launch, screenshot, keyboard
input) is documented in
[`apps/knx-desktop/.claude/skills/run-knx-desktop/SKILL.md`](apps/knx-desktop/.claude/skills/run-knx-desktop/SKILL.md).

Fast, headless checks (prefer these over launching the GUI for
logic-only changes):

```bash
cd apps/knx-desktop
npm run test                # frontend: vitest
cargo test -p knx-desktop   # backend: command dispatch, roundtrip, etc.
```

## License

AGPL-3.0-or-later.
