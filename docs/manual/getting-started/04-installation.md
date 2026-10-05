← Previous: [Project status](03-project-status.md) · [Manual index](../README.md)

# Installation

There are three ways to run KNXBench: the Linux desktop AppImage, a Docker
container serving the web frontend, or building it yourself from source.
Pick whichever fits how you work; all three run the same domain code
underneath.

## a) Linux AppImage

The AppImage is the intended first Linux desktop package for KNXBench (see
[ADR-0021](../../adr/0021-appimage-is-the-first-linux-package.md)), but there
is no published release yet — the GitHub Actions workflow that would build
and publish one exists in the repository and has never been run. For now,
"installing the AppImage" means building it yourself. Chapter
[Linux setup](05-linux-setup.md) covers the host libraries it needs and
exactly what has been tested.

To build it from a checkout of the repository:

```bash
npm ci --prefix apps/knx-web
cd apps/knx-desktop
NO_STRIP=1 APPIMAGE_EXTRACT_AND_RUN=1 cargo tauri build --bundles appimage --ci
```

Once you have an AppImage file — built locally, or, once one exists,
downloaded from a GitHub release — make it executable and run it:

```bash
chmod +x "KNXBench_0.1.0-alpha.1_amd64.AppImage"
"./KNXBench_0.1.0-alpha.1_amd64.AppImage"
```

> **Note**
>
> If a future release publishes an AppImage under a different file name, use
> that name instead. No specific download location exists to link to today.

**Updating.** There is no auto-updater. Close KNXBench, replace the AppImage
file with a newly built one, and start it again.

**Removing.** Delete the AppImage file. Its application data — your projects
— lives separately (see [Linux setup](05-linux-setup.md)) and is not removed
with it; delete that directory yourself only once you no longer need the
projects stored there. The AppImage does not install or modify your system's
GTK or WebKitGTK packages, so removing it leaves your system libraries alone.

## b) Docker / web

This runs the KNXbench server and the built web frontend behind one HTTP
port, from a container.

```bash
docker build -t knxbench-server -f apps/knx-server/Dockerfile .
docker run -d --name knxbench -p 8484:8080 \
  -e KNX_AUTH_PASSWORD='pick something long and boring' \
  -v "$(pwd)/data:/data" knxbench-server
curl -sf http://127.0.0.1:8484/healthz
```

This published-port form supports project work only. It cannot reach the
bus: a tunnel to a gateway gets no answer, because the server tells the
gateway its private container address ([§155](../../KNOWN_LIMITATIONS.md#155-tunnelling-from-a-container-on-dockers-bridge-network-gets-no-answer)),
and gateway discovery needs multicast that Docker's default bridge does not
carry onto the LAN. On Linux, use host networking for anything that talks to
the bus:

```bash
docker run -d --name knxbench --network host \
  -e KNX_PORT=8484 \
  -e KNX_AUTH_PASSWORD='pick something long and boring' \
  -v "$(pwd)/data:/data" knxbench-server
```

Open `http://127.0.0.1:8484` once the health check passes, and sign in with
that password. The `data/` directory on the host is mounted into the container
at `/data`, so your projects survive container restarts and rebuilds.

The password is not optional here, and the reason is worth one paragraph.
`knx-server` refuses to serve an unauthenticated API to the network: with no
credential set it binds loopback only, and inside a container that is the
*container's* loopback, which a published port cannot reach. Give it a
credential and it binds `0.0.0.0`, the published port works, and the browser
asks you for the password before it shows anything.

If you only want a quick local look and no password at all on Linux Docker
Engine, run it on the host's own loopback instead:

```bash
docker run -d --name knxbench --network host \
  -e KNX_PORT=8484 -v "$(pwd)/data:/data" knxbench-server
```

That form is reachable from that machine and nowhere else. Docker Desktop
4.34 and later also offers opt-in host networking, but KNXBench has not
verified multicast discovery or loopback-only exposure through that layer;
keep authentication enabled there.

The image reads these environment variables:

| Variable | Meaning | Default |
| --- | --- | --- |
| `KNX_PORT` | Internal listening port | `8080` |
| `KNX_DATA_DIR` | Where projects are stored | `/data` |
| `KNX_STATIC_DIR` | Where the built frontend is served from | set by the image |
| `KNX_AUTH_PASSWORD_HASH` | The login credential, as printed by `knx-server --hash-password` | unset |
| `KNX_AUTH_PASSWORD` | A plaintext password, hashed at startup | unset |
| `KNX_AUTH_COOKIE_SECURE` | Marks the session cookie `Secure`, for use behind TLS | unset |

> **Warning**
>
> Read this before you make the server reachable from another machine.
>
> The password protects the API; it is not a security perimeter. There is one
> shared password, no user accounts, no roles, no audit trail and **no TLS**.
> Over plain HTTP the password and the session cookie both cross the network in
> the clear, so anything beyond your own machine wants a TLS-terminating
> reverse proxy in front of it.
>
> Prefer `KNX_AUTH_PASSWORD_HASH` over `KNX_AUTH_PASSWORD` for anything that
> lasts: a plaintext password in the environment is readable in
> `docker inspect`, in `/proc/<pid>/environ`, and in your shell history. The
> server says so at startup, every time.
>
> Never publish the credential-less form. Whoever reaches that port holds your
> project, the file browser and the KNX bus routes, and "it's just my home LAN"
> is not a threat model.
>
> [Web and Docker deployment](../user-guide/11-web-and-docker.md) has the full
> picture, including the reverse-proxy setup and what the login does and does
> not defend.

To verify the image works end to end (build, boot, health check, and a
native save/reopen cycle):

```bash
apps/knx-server/scripts/smoke-test.sh
```

**Updating.** Rebuild the image, stop and remove the old container, and start
a new one with the same port and volume:

```bash
docker build -t knxbench-server -f apps/knx-server/Dockerfile .
docker stop knxbench
docker rm knxbench
docker run -d --name knxbench -p 8484:8080 \
  -e KNX_AUTH_PASSWORD='pick something long and boring' \
  -v "$(pwd)/data:/data" knxbench-server
curl -sf http://127.0.0.1:8484/healthz
```

`docker rm knxbench` removes only the container, not the `data/` directory or
the projects inside it. If your running container has a different name,
find it with `docker ps` first.

**Removing.** Stop and remove the container (`docker stop knxbench && docker
rm knxbench`), then remove the image (`docker rmi knxbench-server`) if you
want it gone entirely. Delete the `data/` directory separately, and only once
its projects are no longer needed.

## c) From source

Building from source gives you the CLI, the server, and the desktop shell.

Prerequisites:

- The Rust toolchain pinned in the repository: **1.98.0** (`rust-toolchain.toml`
  selects it automatically if you use `rustup`).
- **Node.js 22.12 or later**, needed for the web frontend used by both the
  server and the desktop shell.

Build everything and run the server directly:

```bash
cargo build
cargo run -p knx-server
```

`cargo run -p knx-server` reads the same `KNX_PORT`, `KNX_DATA_DIR`, and
`KNX_STATIC_DIR` variables described above; unset, it listens on `8080` and
stores projects under your system's temporary directory, which is convenient
for a quick look and wrong for anything you intend to keep.

To run the native desktop shell instead:

```bash
cargo install tauri-cli --version "^2" --locked  # once
npm ci --prefix apps/knx-web
cd apps/knx-desktop
cargo tauri dev
```

`npm ci --prefix apps/knx-web` installs the frontend's dependencies — the
frontend now lives in `apps/knx-web`, not inside `apps/knx-desktop` — and
`cargo tauri dev` starts the desktop shell, which in turn starts the web
frontend's own development server for you. See
[Linux setup](05-linux-setup.md) for the host packages `cargo tauri dev`
needs to have anything to draw a window with.

[Manual index](../README.md) · Next: [Linux setup](05-linux-setup.md) →
