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
KNX_AUTH_PASSWORD_HASH="$(docker run --rm -i knxbench-server --hash-password <<<'your password')"
docker run -d --name knxbench -p 8484:8080 \
  -e KNX_AUTH_PASSWORD_HASH="$KNX_AUTH_PASSWORD_HASH" \
  -v "$(pwd)/data:/data" knxbench-server
curl -sf http://127.0.0.1:8484/healthz
```

Open <http://127.0.0.1:8484> and log in with that password. The mounted
`data/` directory keeps native projects across container restarts—because
losing an electrical installation to an ephemeral container is a particularly
expensive kind of automation.

The image accepts:

- `KNX_PORT` — internal listening port; defaults to `8080`.
- `KNX_DATA_DIR` — project storage directory; defaults to `/data`.
- `KNX_STATIC_DIR` — frontend bundle location; set by the image.
- `KNX_AUTH_PASSWORD_HASH` — the login credential, as printed by
  `knx-server --hash-password`. **This is the one that makes the container
  reachable at all.**
- `KNX_AUTH_PASSWORD` — a plaintext password, hashed at startup. Convenient
  for a quick `docker run -e`, and weaker: the value is readable in
  `/proc/<pid>/environ`, in `docker inspect` and in your shell history. If
  both are set, the hash wins and the server says so.
- `KNX_AUTH_COOKIE_SECURE` — set it to `1` when the server is reached over
  HTTPS, so the session cookie is marked `Secure`. Leave it unset on plain
  HTTP, where a `Secure` cookie would never be sent back at all.

### Authentication, and what happens without it

`knx-server` will not serve an unauthenticated API to the network. With no
password configured it binds `127.0.0.1` instead of `0.0.0.0` and prints a
loud line saying why—which inside a container means `-p 8484:8080` publishes
a port nothing is listening on, and the health check above fails. That is the
intended failure: the alternative was handing your project, your `/api/fs/*`
file browser and your KNX bus routes to whoever found the port first.

`--hash-password` reads the password from standard input, never from an
argument, because `ps` shows every process's arguments to every user on the
machine:

```bash
knx-server --hash-password <<<'your password'
# $pbkdf2-sha256$i=600000$...$...
```

What the login does **not** give you: TLS, user accounts, roles, or an audit
trail. There is one shared password, and everyone who has it can do
everything, including writing to the bus. Put a TLS-terminating reverse proxy
in front of anything that matters—over plain HTTP the password and the session
cookie both cross the network in the clear. See
[ADR-0026](docs/adr/0026-server-authentication-or-loopback.md) and
[KNOWN_LIMITATIONS.md §22](docs/KNOWN_LIMITATIONS.md#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback)
for the full list of what is and is not defended.

To run the Docker smoke test (build, boot, health check, and native
save/reopen cycle):

```bash
apps/knx-server/scripts/smoke-test.sh
```

To additionally import a local ETS project through the running image:

```bash
KNXBENCH_REFERENCE_PROJECT="/path/to/reference.knxproj" \
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
  -e KNX_AUTH_PASSWORD_HASH="$KNX_AUTH_PASSWORD_HASH" \
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

Build the Linux AppImage locally with the same bundle command that the release
workflow uses:

```bash
cd /path/to/KNXBench
npm ci --prefix apps/knx-web
cd apps/knx-desktop
NO_STRIP=1 APPIMAGE_EXTRACT_AND_RUN=1 cargo tauri build --bundles appimage --ci
```

The desktop shell starts the same `knx-server` core used by Docker; the React
frontend always talks HTTP rather than directly to storage or import code.

### Linux AppImage

The AppImage is the first Linux desktop package ([ADR 0021](docs/adr/0021-appimage-is-the-first-linux-package.md)).
After downloading the current alpha artifact, make it executable and start it:

```bash
appimage='KNXBench_0.1.0-alpha.1_amd64.AppImage'
chmod +x "$appimage"
"./$appimage"
```

The GitHub Actions `Linux AppImage` workflow is configured to build and upload
an Actions artifact for a manual run. A pushed `v*` tag is configured to create
or update a GitHub release with that artifact. The workflow has not been run by
this project yet; use a published release only after it exists and verify its
file identity from that release.

One local artifact was built and launched on Arch Linux through XWayland. Its
tested boundary is x86_64 Linux with compatible glibc, GTK 3, and WebKitGTK
4.1; it is not a portability result for other distributions or display stacks.
The desktop stores its application data under
`$XDG_DATA_HOME/com.knxbench.knxbench-labs/projects` (usually
`~/.local/share/com.knxbench.knxbench-labs/projects`).

There is no automatic updater, package signature, ARM64 build, or native
package-manager integration in this alpha. To update, close KNXBench and
replace the AppImage file manually. To remove the application, delete that
file; delete the application-data directory separately only if its stored
projects are no longer needed. The AppImage does not install or update host
GTK/WebKitGTK dependencies.

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

`knx bus discover` needs IP multicast to reach the KNX gateway; the `knx`
CLI is not part of the `knx-server` Docker image above, but if you run it
inside any container of your own (a dev container, CI, or a custom image),
Docker's default bridge network will not carry that multicast traffic —
run the container with `--network host` (Linux-only) instead. See
[KNOWN_LIMITATIONS.md §79](docs/KNOWN_LIMITATIONS.md#79-discovery-needs-ip-multicast-which-dockers-default-bridge-network-does-not-carry).

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
cargo run -p xtask -- check-anchors

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

`check-anchors` walks `docs/` and the repo-root markdown files, slugs every
heading the way GitHub's renderer would, and fails loudly with `file:line`
and a nearest-match suggestion for any in-repo anchor link whose target
does not resolve — a heading rename needs a back-compat `<a id="…">`
alias for its old slug, and a moved file needs every link that points at
it updated.

## Further reading

- [Architecture](docs/ARCHITECTURE.md)
- [Data model](docs/DATA_MODEL.md)
- [Import and export](docs/IMPORT_EXPORT.md)
- [Compatibility](docs/COMPATIBILITY.md)
- [Known limitations](docs/KNOWN_LIMITATIONS.md)
- [Roadmap](docs/ROADMAP.md)
- [Architecture decision records](docs/adr/README.md)

## License

KNXBench is free software licensed under the
[GNU Affero General Public License version 3 or later](LICENSE).

The licence permits private and commercial use, modification, and
redistribution under its terms. Modified versions made available to users over
a network must also offer those users the corresponding source code as required
by the AGPL.
