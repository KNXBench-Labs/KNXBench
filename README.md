# KNXBench

KNXBench is a modern, Linux-first KNX engineering application: an independent,
open alternative to ETS for working with KNX projects. It has its own Rust
domain core, `.knxproj` parser, SQLite project format, product database, and
KNXnet/IP implementation. No ETS internals were harmed—or imitated—in its
construction.

The project is **KNX-compatible**, not KNX-certified and not a claim of full
ETS compatibility. Correctness and preserving project data come before shiny
buttons; fortunately, it has some of those too.

## What it does

- Imports supported ETS `.knxproj` archives and reports warnings and errors.
- Saves and reopens projects in the native, versioned `.knxdb` SQLite format.
- Lets you inspect and edit topology, buildings, devices, individual and group
  addresses, group links, communication-object DPTs, and descriptions.
- Provides undo/redo, search, command palette, dashboard, project explorer,
  inspector, and a themed React UI.
- Ingests manufacturer data from supported project archives and enriches device
  communication objects from the local product database.
- Offers a CLI and KNXnet/IP discovery, tunnelling, routing, monitoring, and
  group-value sending for supported plain KNXnet/IP installations.

The current implementation status and compatibility evidence live in
[Implementation status](docs/IMPLEMENTATION_STATUS.md),
[Compatibility](docs/COMPATIBILITY.md), and
[Known limitations](docs/KNOWN_LIMITATIONS.md).

## Deliberate boundaries

KNXBench v1 is a project editor, not a replacement for every ETS workflow.
Device parameter editing, commissioning/download, KNX IP Secure, and direct
encrypted `.knxprod` imports are out of scope. `.knxproj` export is an
interoperability convenience; the supported lossless working format is native
`.knxdb`.

Some ETS schemas and vendor-specific data are only supported where real,
independent test material exists. Unsupported data is retained or reported
where technically possible—silence is not a compatibility strategy.

## Quick start: web/Docker

The web deployment serves both the API and frontend. The example deliberately
publishes **external port 8484** while the container listens on `8080`.

```bash
docker build -t knxbench-server -f apps/knx-server/Dockerfile .
docker run -d --name knxbench -p 8484:8080 \
  -v "$(pwd)/data:/data" knxbench-server
curl -sf http://127.0.0.1:8484/healthz
```

Open <http://127.0.0.1:8484>. The mounted `data/` directory keeps native
projects across container restarts—because losing an electrical installation
to an ephemeral container is a particularly expensive kind of automation.

The image accepts:

- `KNX_PORT` — internal listening port; defaults to `8080`.
- `KNX_DATA_DIR` — project storage directory; defaults to `/data`.
- `KNX_STATIC_DIR` — frontend bundle location; set by the image.

The server has **no authentication**. Run it only on a trusted network; use a
firewall or an authenticated reverse proxy before exposing it anywhere less
friendly than your LAN.

To run the Docker smoke test (build, boot, health check, and project import):

```bash
apps/knx-server/scripts/smoke-test.sh
```

### Update a running Docker installation

The commands above name the container `knxbench`. To update an existing
installation, build the new image, replace only that container, then start it
again with the same port and data mount:

```bash
docker build -t knxbench-server -f apps/knx-server/Dockerfile .
docker stop knxbench
docker rm knxbench
docker run -d --name knxbench -p 8484:8080 \
  -v "$(pwd)/data:/data" knxbench-server
curl -sf http://127.0.0.1:8484/healthz
```

`docker rm knxbench` removes the old container, not the `data/` directory or
its mounted projects. If the running container has a different name, find it
first with `docker ps` and substitute that name. A small ritual, but much less
exciting than discovering that the container was called `sleepy_babbage`.

## Native desktop app

Prerequisites: the Rust toolchain specified by `rust-toolchain.toml`, Node.js
22.12 or later, and the Linux packages required by Tauri/WebKit.

```bash
cargo install tauri-cli --version "^2" --locked  # once
cd apps/knx-desktop
npm ci
cargo tauri dev
```

Create a release bundle with:

```bash
cd apps/knx-desktop
cargo tauri build
```

The desktop shell starts the same `knx-server` core used by Docker; the React
frontend always talks HTTP rather than directly to storage or import code.

## CLI

The `knx` binary supports project import, product-database management, and
plain KNXnet/IP operations.

```bash
# Import a project and persist it as a native database.
cargo run -p knx-cli -- import path/to/project.knxproj --store project.knxdb

# Inspect available commands and options.
cargo run -p knx-cli -- --help
```

Live bus operations require a compatible, reachable gateway. Use them with the
same care you would use around a live distribution board: test first, then
send.

## Development and verification

The workspace contains Rust crates for the domain, storage, import/export,
product data, projections, networking, and applications. The architecture is
documented in [Architecture](docs/ARCHITECTURE.md), the domain in
[Data model](docs/DATA_MODEL.md), and durable choices in [ADRs](docs/adr/).

Run the relevant checks before contributing:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p xtask -- check-layering
cargo run -p xtask -- check-headers

cd apps/knx-web
npm ci
npm test
npm run build
```

`check-layering` protects the important seams: the domain remains independent
of UI, SQL, XML, and networking frameworks; project import, storage, and
product data remain separately owned.

`check-headers` keeps the first-line convention honest: a source file's
first line is one sentence saying what the file is for (`//! ...` in Rust,
`/** ... */` in TypeScript), checked wherever one exists; the number of
files without one is a ratchet that may only go down, so new files get a
header and old ones are not swept. Every program carries its own SemVer version;
`knx --version` and `knx-server --version` add the commit they were built
from. See [ADR-0018](docs/adr/0018-program-versions-and-file-headers.md).

## Further reading

- [Architecture](docs/ARCHITECTURE.md)
- [Data model](docs/DATA_MODEL.md)
- [Import and export](docs/IMPORT_EXPORT.md)
- [Compatibility](docs/COMPATIBILITY.md)
- [Known limitations](docs/KNOWN_LIMITATIONS.md)
- [Roadmap](docs/ROADMAP.md)
- [Architecture decision records](docs/adr/README.md)

## License

AGPL-3.0-or-later.
