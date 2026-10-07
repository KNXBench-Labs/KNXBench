← Previous: [Contributing](01-contributing.md) · [Manual index](../README.md)

# Building from source

[Installation](../getting-started/04-installation.md) covers the short version: build it,
run it, done. This chapter is the longer one, for somebody who intends to change the code
and wants to know what each part is and how to build it on its own.

## Prerequisites

| What | Version | Where it is pinned |
| --- | --- | --- |
| Rust | **1.98.0** | `rust-toolchain.toml`, and `rust-version` in `Cargo.toml` |
| Node.js | **22.12.0 or later** | `engines.node` in `apps/knx-web/package.json` |
| Tauri CLI | **2.11.4** | pinned by the AppImage workflow and launcher contract |
| Python | **3** | standard library only; required when bundling the AppImage |

If you install Rust with `rustup`, `rust-toolchain.toml` selects 1.98.0 for you the first
time you run `cargo` in the repository, including `rustfmt` and `clippy`. You do not have
to do anything.

The desktop shell additionally needs GTK 3 and WebKitGTK 4.1 development packages on the
host. [Linux setup](../getting-started/05-linux-setup.md) lists them per distribution. The
CLI and the server do not need any of that — they have no GUI.

```bash
git clone https://github.com/KNXBench-Labs/KNXBench.git
cd KNXBench
cargo build
```

The first build compiles a few hundred crates, SQLite among them (`rusqlite` is used with
its `bundled` feature, so there is no system SQLite to install). Make coffee.

## What is in the workspace

One Cargo workspace: thirteen crates under `crates/`, the `xtask` gate runner, and the
applications under `apps/`. The dependency arrows all
point downward — see [Architecture tour](03-architecture-tour.md) for why that matters and
which gate enforces it.

| Crate | What it is for |
| --- | --- |
| `crates/knx-core` | The domain model: addresses, entities, the DPT codec, validation. No IO, no XML, no SQL, no UI. |
| `crates/knx-app` | Application services: import orchestration, commands, undo/redo, search, reports. |
| `crates/knx-store` | The native `.knxdb` SQLite project format, its migration chain, and the opaque passthrough store. |
| `crates/knx-etsproj` | `.knxproj` reading: ZIP container, schema detection, tolerant XML parsing, mapping, import report. Reading only — the writer was removed on 2026-09-20 ([ADR-0028](../../adr/0028-no-knxproj-export.md)). |
| `crates/knx-productdb` | The product database: its own SQLite file, its own migration chain, `.knxprod` package installation, enrichment. |
| `crates/knx-projection` | Pure display projections, exported to TypeScript with `ts-rs`. Depends on `knx-core` only. |
| `crates/knx-csv` | Reader and writer for "KNXBench group-address CSV v1", a format this project defines and owns. |
| `crates/knx-diff` | Typed comparison of two projects. Formatting is left to the caller. |
| `crates/knx-report` | Renders a project into one self-contained HTML document. |
| `crates/knx-net` | KNXnet/IP: discovery, tunneling, routing, cEMI, telegrams. |
| `crates/knx-secure` | The isolated key-material subsystem. Deliberately reaches neither `knx-core` nor `serde`. |
| `crates/knx-testsupport` | Test-fixture paths and nothing else. A `[dev-dependencies]` entry only. |
| `crates/knx-build-stamp` | Decides which commit a binary names and refuses a release build from a modified tree (`KNX_REQUIRE_CLEAN_TREE=1`). Used by the CLI's and server's `build.rs`. |
| `xtask` | The repository's own verification tasks: the layering, header, anchor, ledger, corpus-gate and AppImage checks. |

And the applications:

| Application | What it is |
| --- | --- |
| `apps/knx-cli` | The headless entry point. Binary name: `knx`. |
| `apps/knx-server` | The axum HTTP API plus static frontend serving. Binary name: `knx-server`. |
| `apps/knx-desktop` | The Tauri v2 shell. Its Cargo crate is `apps/knx-desktop/src-tauri`. |
| `apps/knx-web` | The React + Vite frontend. An npm package, not a Cargo workspace member. |

`apps/knx-desktop` has no `package.json` of its own. The frontend used to live there and
now lives in `apps/knx-web`; anything telling you to run `npm ci` inside `apps/knx-desktop`
is out of date.

## Building the server

```bash
cargo build -p knx-server
cargo run -p knx-server
```

Unset, it listens on port `8080`, serves only the API (no frontend), and stores projects in
your system's temporary directory. That is fine for a quick look and wrong for anything you
want to keep. See [Environment variables](#environment-variables) below.

To serve the built frontend from the same port, build the frontend first and point
`KNX_STATIC_DIR` at its output:

```bash
npm ci --prefix apps/knx-web
npm run build --prefix apps/knx-web
KNX_DATA_DIR="$PWD/data" KNX_STATIC_DIR="$PWD/apps/knx-web/dist" cargo run -p knx-server
```

The binary also has two modes that are not a server:

```bash
cargo run -p knx-server -- --version
cargo run -p knx-server -- --hash-password <<<'your password'
```

`--hash-password` reads the password from standard input, never from a command-line
argument, because `ps` shows every process's arguments to every user on the machine.

## Building the CLI

```bash
cargo build -p knx-cli
cargo run -p knx-cli -- --help
cargo run -p knx-cli -- --version
```

The binary is called `knx`, so a release build lands at `target/release/knx`. What each
subcommand does is in [The command line](../user-guide/10-command-line.md).

## Building the frontend

```bash
npm ci --prefix apps/knx-web
npm run dev --prefix apps/knx-web
```

The dev server listens on port `1420` with `strictPort` set, and proxies `/api` to
`http://127.0.0.1:4777`. That port is the `DEV_PORT` constant in
`apps/knx-server/src/lib.rs`, and the Vite config and the desktop shell both refer to it —
it is source, not deployment configuration, so there is nothing to configure.

For a production bundle:

```bash
npm run build --prefix apps/knx-web
```

That runs `tsc` and then Vite, and writes to `apps/knx-web/dist`.

## Building the desktop shell

The Tauri CLI is a separate cargo binary; install it once.

```bash
cargo install tauri-cli --version "^2" --locked
npm ci --prefix apps/knx-web
cd apps/knx-desktop
cargo tauri dev
```

Run `cargo tauri dev` from `apps/knx-desktop`, not from the repository root and not from
`src-tauri` — the CLI looks for `src-tauri/tauri.conf.json` relative to the working
directory. Its `beforeDevCommand` starts the frontend's Vite dev server for you
(`npm --prefix ../knx-web run dev`), so you do not start it yourself; `npm ci` is still
needed once, to put the dependencies there for it to start.

In development the shell binds the fixed dev port `4777` for the API and lets Vite serve
the frontend with hot reload. In a release build there is no Vite: `knx-server` serves both
the API and the bundled frontend on an ephemeral loopback port, and the WebView points at
that.

> **Note**
>
> `cargo tauri dev` failing with a linker error about `webkit2gtk` or `gtk` means the host
> packages are missing, not that the code is broken. [Linux setup](../getting-started/05-linux-setup.md)
> has the list.

## Building the AppImage

```bash
npm ci --prefix apps/knx-web
cd apps/knx-desktop
NO_STRIP=1 APPIMAGE_EXTRACT_AND_RUN=1 cargo tauri build --bundles appimage --ci
```

The result lands in `target/release/bundle/appimage/`. This is the same command the
`Linux AppImage` GitHub Actions workflow runs; that workflow pins Tauri CLI `2.11.4`
exactly, which is worth matching if you are chasing a difference between your artifact and
CI's.

The workflow also validates the artifact afterwards, and you can run that check yourself:

```bash
cargo run -p xtask -- check-appimage
cargo run -p xtask -- check-appimage --tag v0.1.0-alpha.4
```

The tagged form additionally checks that the artifact's file name matches the tag.

> **Note**
>
> The release workflow exists and has never been run, so there is no published AppImage to
> download. Building one locally is currently the only way to have one. It has been built
> and launched on exactly one host: x86_64 Arch Linux under XWayland, with GTK 3 and
> WebKitGTK 4.1. That is the tested boundary, not a portability claim.

## Building the Docker image

```bash
docker build -t knxbench-server -f apps/knx-server/Dockerfile .
```

Three stages: a `node:22-alpine` stage builds the frontend, a `rust:1.98-slim` stage builds
`knx-server`, and a `debian:bookworm-slim` runtime stage carries the two results. The
runtime image sets `KNX_STATIC_DIR=/app/frontend`, `KNX_DATA_DIR=/data` and `KNX_PORT=8080`,
declares `VOLUME /data`, and exposes `8080`.

Pass `KNX_BUILD_SHA` as a build argument if you want `--version` to name the commit; a
Docker build has no git history to ask.

How to run it — including why a published port needs a password to work at all — is in
[Web and Docker deployment](../user-guide/11-web-and-docker.md).

## Running the tests

Rust, the whole workspace:

```bash
cargo test --workspace
```

A single crate, which is what you usually want while working:

```bash
cargo test -p knx-etsproj
cargo test -p knx-core addresses
```

Frontend, from `apps/knx-web` after `npm ci`:

```bash
npx vitest run
npx tsc --noEmit
```

End-to-end, with Playwright. This builds the frontend and drives it against a server, so it
is slower and needs Playwright's browsers installed:

```bash
npm run test:e2e --prefix apps/knx-web
```

The Docker path has its own smoke test — build, boot, health check, and a native
save-and-reopen cycle:

```bash
apps/knx-server/scripts/smoke-test.sh
```

It can also import a real ETS project through the running image, if you have one to point
it at:

```bash
KNXBENCH_REFERENCE_PROJECT="/path/to/reference.knxproj" \
  apps/knx-server/scripts/smoke-test.sh
```

The full list of checks a change has to pass, and what each one protects, is in
[Contributing](01-contributing.md).

## Environment variables

`knx-server` reads all of these. Nothing else in the workspace reads the authentication
ones — `apps/knx-server/src/main.rs` is the only place.

| Variable | Meaning | Default |
| --- | --- | --- |
| `KNX_PORT` | Listening port | `8080` |
| `KNX_DATA_DIR` | Where projects are stored; created at startup if missing | the system temporary directory |
| `KNX_STATIC_DIR` | Directory of the built frontend; unset means API only | unset |
| `KNX_AUTH_PASSWORD_HASH` | The credential, as printed by `knx-server --hash-password` | unset |
| `KNX_AUTH_PASSWORD` | A plaintext password, hashed at startup; weaker, because the value is visible in `docker inspect` and `/proc/<pid>/environ` | unset |
| `KNX_AUTH_COOKIE_SECURE` | Marks the session cookie `Secure`, for use behind TLS | unset |
| `KNX_TUNNEL_ROUTE_BACK` | Ask the gateway to answer the packet's source (KNXnet/IP Route Back); for tunnelling from Docker's bridge network | unset |
| `KNX_BUILD_SHA` | Build-time only: the commit for `--version`, when git is not available | unset |

With no credential configured, the server binds `127.0.0.1` instead of `0.0.0.0` and says
so on startup (ADR-0026). That is a deliberate refusal to serve an unauthenticated API to
the network, and it is why a container with no password cannot be reached through a
published port.

The frontend asks `GET /api/auth/status` before it renders anything: no password required
means the workbench appears directly, which is what the Tauri shell relies on; a password
required means a login card instead. `knx_server::app()` still builds an unguarded router
for exactly that desktop case — only the standalone binary insists on a credential before
it leaves loopback.

> **Note**
>
> There is still no TLS, no user accounts, no roles and no audit trail: one shared password
> is the whole identity model. See
> [Known issues](../known-issues.md) and
> [`KNOWN_LIMITATIONS.md` §22](../../KNOWN_LIMITATIONS.md#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback).

Two more variables are used by the tests rather than by the applications:
`TS_RS_EXPORT_DIR`, which tells `knx-projection`'s test where to write the generated
TypeScript bindings (CI sets it to `apps/knx-web/src/bindings` and then fails if `git diff`
shows a change), and the per-fixture overrides `knx-testsupport` offers so nobody has to
hard-code the maintainer's corpus paths.

The desktop build ignores all of the above. It picks its own data directory —
`~/.local/share/com.knxbench.knxbench-labs/projects` — and its own port.

[Manual index](../README.md) · Next: [Architecture tour](03-architecture-tour.md) →
