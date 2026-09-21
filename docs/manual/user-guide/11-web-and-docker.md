← Previous: [The command line](10-command-line.md) · [Manual index](../README.md)

# Web and Docker deployment

KNXBench can run as a server: one process, one HTTP port, the whole application in a
browser. This chapter covers what that process actually serves, how to configure it,
how the browser build differs from the desktop one, and — at length, because it is the
part that matters — what its security model is and is not.

[Installation](../getting-started/04-installation.md) has the short version of building
and running the container. This chapter is the long version.

## What `knx-server` serves

`knx-server` is a single binary that does two jobs on one port:

1. **The HTTP API.** Everything under `/api/` — open and save projects, edit the
   topology, the buildings and the group addresses, import and export, browse the data
   directory, run the bus monitor, generate a debug report.
2. **The frontend.** The built web application is served as static files, as a fallback
   for anything that is not an API route. That is why one port is enough.

Two small routes sit outside both groups: `GET /healthz`, which answers `ok` and is
meant for container health checks, and `GET /api/version`, which reports the build.

When it starts, it prints the address it is listening on:

```text
knx-server listening on 0.0.0.0:8080
```

Read that line. The address at the front of it is not decoration — see
[Authentication](#authentication) below.

## Configuration

Five environment variables, and nothing else. There is no configuration file.

| Variable | Meaning | Default |
| --- | --- | --- |
| `KNX_PORT` | The port to listen on | `8080` |
| `KNX_DATA_DIR` | Where projects live | The system temporary directory |
| `KNX_STATIC_DIR` | Where the built frontend is served from | Unset: no frontend, API only |
| `KNX_AUTH_PASSWORD_HASH` | The login credential, as a hash | Unset |
| `KNX_AUTH_PASSWORD` | The login credential, in plaintext | Unset |

One more, `KNX_AUTH_COOKIE_SECURE`, is covered under
[Authentication](#authentication).

`KNX_DATA_DIR` is the one to get right. Unset, it defaults to your system's temporary
directory, which is convenient for a look around and wrong for anything you intend to
keep — temporary directories are called that for a reason. The server creates the
directory at startup if it is missing, and refuses to start if it cannot.

`KNX_STATIC_DIR` unset means the server answers API calls and serves no web page at
all. That is the right setting for a headless deployment and the wrong one if you were
expecting a user interface.

## The container

The image is built from `apps/knx-server/Dockerfile`, in three stages: Node builds the
frontend, Rust builds the server, and a slim Debian image carries just those two
artifacts. It sets `KNX_STATIC_DIR=/app/frontend`, `KNX_DATA_DIR=/data` and
`KNX_PORT=8080`, exposes `8080`, and declares `/data` as a volume.

### Where your projects live

Inside the container, **in `/data`**. Everything you save, every project you upload and
every file the web file picker shows you is under that one directory.

`/data` is declared as a volume, which means Docker gives it storage that outlives the
container — but an anonymous volume is easy to lose track of and easy to delete by
accident. Mount a directory you chose yourself:

```bash
docker run -d --name knxbench -p 8484:8080 \
  -e KNX_AUTH_PASSWORD_HASH="$KNX_AUTH_PASSWORD_HASH" \
  -v "$(pwd)/data:/data" knxbench-server
```

Now `./data` on your host is `/data` in the container. Removing the container with
`docker rm` leaves it alone; only `rm -rf` on your own directory removes your projects.

### Uploads and the data directory

In the browser build, opening a project that is not already inside `/data` means
uploading it. Uploads land in `<data dir>/uploads` and are capped at 100 MiB per file.
The uploaded name is reduced to a plain file name, so nothing a client sends can climb
out of that directory.

The same confinement applies to browsing: `/api/fs/*` only ever speaks in paths
relative to the data directory, and a path that resolves outside it is rejected with
`path escapes the data directory`.

There is one deliberate exception, and it is worth understanding. The `/api/project/*`
routes accept **absolute** paths unchanged, because in the desktop build those paths
come from a native file dialog and must keep working — an absolute path there means
"the user picked this file in their own operating system's dialog". On a server, an
absolute path means whatever the authenticated caller typed. That is not a bug, but it
is a reason to care who is authenticated.

### Build metadata

`docker build` cannot see the git history (it is excluded from the build context), so
the commit is passed in:

```bash
docker build -t knxbench-server -f apps/knx-server/Dockerfile . \
  --build-arg KNX_BUILD_SHA=$(git rev-parse --short HEAD)
```

Without it, `knx-server --version` and `GET /api/version` report the manifest version
and no commit.

### Networking

The container's default bridge network supports project work, ordinary HTTP,
and tunnelling to a manually entered gateway. Discovery is different: the CLI
invokes the shared `KnxNetIpClient` directly, while the web UI's
**Discover gateways** action reaches it through the server. Both send IP
multicast from their process. Docker's bridge does not carry that request onto
the LAN, so either call returns an empty result when its process is inside the
bridge. See [The command line](10-command-line.md) and
[KNOWN_LIMITATIONS.md §79](../../KNOWN_LIMITATIONS.md#79-discovery-needs-ip-multicast-which-dockers-default-bridge-network-does-not-carry).

On Linux, start the server with `--network host` and set `KNX_PORT` to the
host port you want; `-p` has no effect in host mode.

## Authentication

This section is not optional reading. Nothing in it is a joke.

### What exists today

`knx-server` has one password. Not accounts, not users, not roles — one shared
password that grants everything.

- `POST /api/auth/login` takes `{"password": "..."}` and, on success, sets a session
  cookie named `knx_session` with `HttpOnly`, `SameSite=Strict` and `Path=/`.
- `POST /api/auth/logout` invalidates the session.
- `GET /api/auth/status` reports `{"required": true, "authenticated": false}` and needs
  no password itself. Asking whether a door is locked is not trespassing.

The credential is stored as PBKDF2-HMAC-SHA256 over a 16-byte random salt at 600,000
iterations, in a self-describing string, and is compared in constant time. Sessions
expire after 12 hours idle, refreshed on each use, and are held in memory only — a
restart logs everyone out.

Everything under `/api/` is behind that check, including `/api/version`, the bus routes
and the file browser. Exactly four things are reachable without a session: `/healthz`,
and the three `/api/auth/*` routes, and the static frontend files.

### What happens if you configure no password

The server refuses to put an unguarded API on the network. With neither
`KNX_AUTH_PASSWORD_HASH` nor `KNX_AUTH_PASSWORD` set, it binds `127.0.0.1` instead of
`0.0.0.0` and says so, loudly, on startup:

```text
knx-server: NO AUTHENTICATION CONFIGURED. Binding 127.0.0.1 instead of 0.0.0.0: this
server would otherwise hand the open project, the KNX bus routes and the host
filesystem browser to anyone who can reach the port. Set KNX_AUTH_PASSWORD_HASH (see
`knx-server --hash-password`) to listen on the network.
```

Inside a container, that means `-p 8484:8080` publishes a port with nothing listening
behind it and the health check fails. This is the intended failure, not a bug to work
around.

### Setting a password

Generate a hash. The command reads the password from standard input, never from an
argument, because command arguments are visible to every user on the machine:

```bash
knx-server --hash-password <<<'your password'
```

It prints a single line beginning `$pbkdf2-sha256$i=600000$`. Put that in
`KNX_AUTH_PASSWORD_HASH`. With the container:

```bash
KNX_AUTH_PASSWORD_HASH="$(docker run --rm -i knxbench-server --hash-password <<<'your password')"
docker run -d --name knxbench -p 8484:8080 \
  -e KNX_AUTH_PASSWORD_HASH="$KNX_AUTH_PASSWORD_HASH" \
  -v "$(pwd)/data:/data" knxbench-server
```

`KNX_AUTH_PASSWORD` accepts a plaintext password instead and hashes it at startup. It
is convenient and weaker, and the server says so in its own words: the value is readable
in `/proc/<pid>/environ`, in `docker inspect` and in your shell history. Set both and the
hash wins, with a notice about the one being ignored. A plaintext password shorter than
12 characters earns a second complaint; the server starts anyway, because it is not in a
position to argue.

> **Warning**
>
> The hash string contains `$` characters. `docker compose` interpolates those in
> `.env` files and in `compose.yml`. Double every `$` to `$$` when pasting a hash
> there, or Compose hands the container a truncated credential and no password you type
> will ever work. `docker run -e` does not interpolate and needs no doubling.

### Signing in from the browser

Before it renders anything, the web frontend asks `GET /api/auth/status`. A server that
wants no password renders the workbench directly — which is why the desktop shell never
shows a login. A server that wants one shows this instead:

![The KNXBench login card on an empty background: the KNXBench word mark, the sentence
"This server is password-protected. Enter the password to carry on.", a Password field, a
Sign in button, and the note "One password for the whole server, set when it was started.
There are no user accounts yet."](../../assets/screenshots/porcelain-login.png)

Type the password, press Enter, and the workbench appears. Verifying it is deliberately
slow work — 600,000 PBKDF2 iterations — so the button says **Signing in…** for a moment
rather than pretending to be instant.

If a session expires while you are working, the login card comes back over the
application rather than instead of it: the project stays open underneath, and signing in
again returns you to exactly what was on screen. That matters, because the server has no
endpoint that hands an open project back — an unmount would lose your unsaved work.

### What the password does not protect you from

- **There is no TLS.** Over plain HTTP the password crosses the network in the clear in
  the login request, and the session cookie crosses it in the clear on every request
  after that. Anyone who can watch that traffic can replay the cookie. Put a
  TLS-terminating reverse proxy in front of anything that matters, and set
  `KNX_AUTH_COOKIE_SECURE=1` when you do. Leave that variable unset on plain HTTP,
  where a `Secure` cookie would simply never be sent back.
- **One password means one identity.** Everyone holding it can do everything, including
  writing to the KNX bus. Nothing records who did what, because there is no "who".
- **Brute-force resistance is a delay, not a lockout.** A failed login costs the caller
  250 ms times the number of failures since the last success, capped at two seconds.
  The counter lives in memory and resets when the process restarts. There is no lockout,
  on purpose: locking out the only operator of a single-operator server is a denial of
  service against its owner.
- **There are no CSRF tokens.** `SameSite=Strict` on the session cookie is the whole
  defense, and that is browser behavior rather than a server-side check.
- **It is still one shared project.** Two authenticated browsers share one open project
  and one undo stack, with no conflict detection.

Run the container on a network you have thought about, behind a transport you secured
yourself. It is not built to face the internet. For the full list of what is and is not
defended, see [known limitations](../../KNOWN_LIMITATIONS.md).

## Web build versus desktop build

Both builds run the same frontend code and the same server code. The differences are
where they touch the operating system.

| | Desktop (Tauri) | Web (browser) |
| --- | --- | --- |
| Opening and saving files | Native OS file dialogs | An in-page file browser over the data directory, with upload and download |
| Reachable paths | Anywhere you can pick | Inside the data directory, plus uploads |
| Server | Started by the shell on loopback, on a private port | The one you deployed |
| Authentication | None, and none needed: loopback only | As configured above |
| Quit in the File menu | Present | Absent — a browser tab cannot close itself |
| Diagnostics window | A second application window | A second browser window, which a pop-up blocker can refuse |

The file dialog difference is the one you notice first. In the desktop shell, "Open
project" is your desktop's own dialog and can reach any file you have. In the browser,
it is KNXBench's own picker, listing the server's data directory, with an **Upload…**
button for files that are not there yet and a download for saving one back out.

Everything else — the editor, the inspector, the group address table, the bus monitor,
the documentation export, the comparison — is identical, because it is the same code.

## Running it without Docker

```bash
KNX_DATA_DIR="$HOME/knxbench-data" \
KNX_AUTH_PASSWORD_HASH='...' \
KNX_STATIC_DIR=apps/knx-web/dist \
cargo run -p knx-server
```

For frontend development the two halves run separately: `cargo run -p knx-server`
listens on port `4777`, and Vite's dev server on port `1420` proxies `/api` to it. See
[Building from source](../development/02-building-from-source.md).

## Checking that it works

```bash
curl -sf http://127.0.0.1:8484/healthz
```

`ok` means the process is up and serving. It does not mean you can log in, and it does
not mean the frontend was bundled — `/healthz` answers without a session on purpose, so
that an orchestrator does not restart a healthy container forever.

The repository also carries an end-to-end check that builds the image, boots it, health
checks it and does a save-and-reopen cycle:

```bash
apps/knx-server/scripts/smoke-test.sh
```

[Manual index](../README.md) · Next: [Keyboard shortcuts](../reference/01-keyboard-shortcuts.md) →
