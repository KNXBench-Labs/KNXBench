← Previous: [Supported and unsupported KNX/ETS functionality](02-supported-and-unsupported.md) · [Manual index](../README.md)

# Troubleshooting

This chapter is a list of things that actually go wrong, taken from the error strings and
guard rails in the code rather than imagined in advance. If your problem isn't here, the
[known issues](../known-issues.md) list and
[docs/KNOWN_LIMITATIONS.md](../../KNOWN_LIMITATIONS.md) are the next places to check, and
the [issue tracker](https://github.com/KNXBench-Labs/KNXBench/issues) is where to report
anything neither of those explains.

## The browser warns that the connection is not private

**You see:** a full-page warning such as "Your connection is not private" or "Warning:
Potential Security Risk Ahead" when you open the server's address.

**Why:** a server with a password speaks HTTPS with a certificate it generated itself
([Web and Docker deployment](../user-guide/11-web-and-docker.md#https)). Nobody your
browser trusts has signed it, so the browser cannot tell it apart from an impostor.

**Do this:** open the certificate details from the warning and compare its SHA-256
fingerprint with the line `HTTPS with a self-signed certificate ... fingerprint is ...`
in the server's startup log. If they match, accept the warning. If they do not match,
do not continue. Something between you and the server is presenting a different
certificate. A new fingerprint after the log line `Generated a new self-signed TLS
certificate` is expected; one without it is not.

## The server starts, but the browser shows nothing

**You see:** `knx-server listening on ...` in the terminal, but the page in your browser
is blank, or every request answers `404`.

**Why:** `knx-server` only serves a web page at all if you set `KNX_STATIC_DIR` to a
directory that actually contains the built frontend. A fresh checkout's
`apps/knx-web/dist` starts out empty — it only fills up after you run a frontend build —
and pointing `KNX_STATIC_DIR` at an empty or nonexistent directory does not stop the
server from starting. It just means the fallback that serves the frontend has nothing to
serve, so every page request answers `404` instead of your application.

**Do this:** Build the frontend before pointing the server at it —
`npm ci --prefix apps/knx-web && npm run build --prefix apps/knx-web` — then point
`KNX_STATIC_DIR` at `apps/knx-web/dist`. If you only need the HTTP API, leave
`KNX_STATIC_DIR` unset on purpose; that is a supported, headless configuration, not a
half-broken one. See
[Web and Docker deployment](../user-guide/11-web-and-docker.md#configuration) for the full
variable table.

## The port is already in use

**You see:** `knx-server` exits immediately with a message that includes `failed to bind`
and an operating-system error about the address already being in use.

**Why:** `KNX_PORT` defaults to `8080`, and `knx-server` binds it exclusively — it does
not share a port with anything else, including a previous copy of itself that never shut
down cleanly.

**Do this:** Find and stop whatever already holds that port, or set `KNX_PORT` to a free
one and update whatever you use to reach the server (a published Docker port, a
`curl` command, a browser bookmark) to match.

## `KNX_DATA_DIR` is missing or not writable

**You see:** `knx-server` exits immediately, with a message that includes
`failed to create data dir` and the path it tried.

**Why:** `KNX_DATA_DIR` is where every project, upload, and file the web file picker
shows you lives. On startup, the server creates that directory if it does not exist yet
and refuses to start if it cannot — a data directory it cannot write to is not a
condition it silently works around. If `KNX_DATA_DIR` is left unset entirely, it falls
back to your system's temporary directory, which is fine for a quick look and the wrong
place to keep anything you intend to open again next week.

**Do this:** Point `KNX_DATA_DIR` at a directory the server's process can create and
write to — for the Docker image, that means the host directory you mount onto `/data`
actually exists and is owned by a user the container can write as. See
[Web and Docker deployment](../user-guide/11-web-and-docker.md#the-container) for the
mount example.

## The login card keeps coming back

**You see:** you sign in, work for a while, and the password card reappears over the
application. Or you sign in, the button says **Signing in…** for a noticeable moment,
and then the card returns with an error.

**Why:** two different things, and the difference is the timing.

If it comes back *later*, your session ended. Sessions expire after 12 hours idle and
are held in the server process's memory only, so restarting `knx-server` — or recreating
the container — logs everyone out at once. The card appears *over* the workbench rather
than instead of it: your open project stays mounted underneath, and signing in again
returns you to exactly the screen you left.

If it comes back *immediately* with an error, the password was wrong. Each failed attempt
costs an extra 250 ms of delay, up to two seconds, so repeated typos feel progressively
slower. Nothing locks out; the counter resets when the process restarts.

**Do this:**

- Check the password you actually gave the server. If you used `KNX_AUTH_PASSWORD_HASH`
  in a `docker compose` file, check for the `$` problem: Compose interpolates `$` in
  `.env` files and `compose.yml`, so every `$` in a hash must be doubled to `$$`, or the
  container receives a truncated credential and no password will ever work. `docker run
  -e` does not interpolate and needs no doubling.
- If both `KNX_AUTH_PASSWORD_HASH` and `KNX_AUTH_PASSWORD` are set, the hash wins — the
  server says so at startup, and the plaintext one is ignored.
- Save your work before restarting the server. The session dies with the process.

The **Signing in…** pause is not a fault. Verifying a password costs 600,000 PBKDF2
iterations on purpose; the button admits the delay rather than pretending to be instant.
See [Web and Docker deployment §Authentication](../user-guide/11-web-and-docker.md#authentication)
for the whole model.

## The server binds `127.0.0.1` when you expected `0.0.0.0`

**You see:** a startup line beginning `NO AUTHENTICATION CONFIGURED. Binding 127.0.0.1
instead of 0.0.0.0`, and the server is unreachable from any machine but its own — inside
Docker, that means a published port with nothing answering behind it.

**Why:** this is a deliberate guard, not a bug. `knx-server` will not put an
unauthenticated API — including the project it holds open, the KNX bus routes, and the
host filesystem browser under `KNX_DATA_DIR` — on a network interface anyone else can
reach. Configuring neither `KNX_AUTH_PASSWORD_HASH` nor `KNX_AUTH_PASSWORD` is treated as
"this is a local, single-user session," and the bind address follows that assumption
exactly.

**Do this:** if you actually want the server reachable from other machines, set
`KNX_AUTH_PASSWORD_HASH` (generate it with `knx-server --hash-password`, reading the
password from standard input rather than an argument, since command arguments are
visible to every user on the machine). If you only ever meant to reach it from the same
host, this is working as intended, and no password is needed.

## An import reports errors or warnings

**You see:** after importing a `.knxproj` file, an **Import: N errors · M warnings**
button appears in the strip under the header.

**Why:** import reports distinguish unknown elements, inferred values, conflicts,
unsupported features and errors. Source bytes are retained where possible;
preservation does not mean editable semantics or a universally lossless mapping — see
[the import report](../user-guide/02-projects.md#the-import-report) for the full
breakdown of what falls into which bucket.

**Do this:** click that button, or open the Overview, where the same counts appear
alongside the project's other numbers. The session **Log** carries one entry per
finding, in English regardless of your interface language, written to be pasted straight
into a bug report. An error does not necessarily mean the import failed outright — a
single reported error can coexist with a project that opens and is otherwise usable; read
the entry's text before assuming the whole file is unusable.

> **Note**
>
> One specific, observed case: an `Installation` element whose `DefaultLine` attribute is
> present in the file but empty is reported as an unresolved reference. The import still
> completes and the resulting project is usable — the attribute is simply treated as a
> reference to nothing rather than as "not set." If you see exactly this, it does not mean
> your file is broken.

## A project won't open

**You see:** opening a `.knxdb` file fails, with a message printed by the server rather
than a generic crash.

**Why:** the two messages you're most likely to meet both come from the native store
refusing to guess:

- `project file is schema version N, this build supports up to M — no downgrade path
  exists` means the file was written by a newer KNXBench than the one you're running.
  There is no downgrade path by design — silently discarding whatever the newer schema
  added would be exactly the kind of data loss this project tries hard not to cause.
- A plain SQLite error such as a "file is not a database" message usually means the path
  you gave isn't a `.knxdb` file at all — a `.knxproj` opened through the native "Open"
  path instead of "Import," for instance, or a file that got truncated mid-copy.

**Do this:** for the schema-version message, update KNXBench to a version at least as new
as the one that wrote the file. For the "not a database" case, check that you picked the
right file and the right open action — importing an ETS `.knxproj` and opening a native
`.knxdb` are two different actions in the File menu, described in
[Projects: create, open, import, save, export](../user-guide/02-projects.md).

## Missing product data after an import

**You see:** a device's **Product data** tab shows **No product database** or **Not in
the product database** instead of a manufacturer name and application program details.

**Why:** communication object and parameter data that comes from a product database is
enrichment, not something the importer invents. **No product database** means nothing is
installed at all; **Not in the product database** means something is installed, but it
does not contain this particular product. Either way, the project itself imported
correctly — what's missing is a separate database, not project data.

**Do this:** install the product database package (a `.knxprod` file) that covers the
devices in your project, from the catalog browser or `knx products ingest` on the command
line. See
[Products and product databases](../knx-basics/05-products-and-product-databases.md) and
[Devices and products](../user-guide/05-devices-and-products.md#product-data). If a
`.knxproj` file already carried manufacturer data bundled inside it, that data is ingested
automatically during import and needs no separate step.

## Discovery finds no gateway

**You see:** `knx bus discover` on the command line returns no results, even though you
know an interface is on the network.

**Why:** discovery works by sending an IP multicast search request and waiting for
interfaces to answer. An empty result almost always means the request or the replies
never made it across your network, not that no interface exists. Docker's default bridge
network is the most common cause: it does not carry multicast traffic between a container
and the rest of your network at all, so the request goes nowhere and the symptom looks
identical to "no gateway found."

**Do this:** if you're running `knx bus discover` inside a container of your own on
Linux, add `--network host` and try again — that puts the request on the real network
interface, the same as running the CLI outside a container. This applies both
to the `knx` CLI and to the bus monitor's interface **Search** in the
`knxbench-server` web UI, because both use the same multicast discovery
implementation. See
[Linux setup §Multicast and `knx bus discover`](../getting-started/05-linux-setup.md#multicast-and-knx-bus-discover)
for the full explanation.

**Not in a container?** Then check your computer's firewall. The interface
answers the search directly (unicast) from its own address and UDP port
3671, and a firewall such as `ufw` with a "deny incoming" default drops
that answer, because it only expects replies from the multicast address
the request went to. `journalctl -k | grep 'UFW BLOCK'` shows such drops
with `SPT=3671`. Allow incoming UDP from source port 3671 on your local
network, for example
`sudo ufw allow proto udp from 192.168.1.0/24 port 3671` (use your own
network), or keep entering the gateway address by hand, which is not
affected.

## A bus session is refused because one is already running

**You see:** starting a bus monitor session answers with a message naming an existing
session, its id and its gateway, instead of connecting.

**Why:** KNXBench's server holds exactly one bus session at a time, on purpose — there is
one shared state for the telegram list and the compose form, and letting two sessions run
at once would mean deciding which one's telegrams belong in that shared list. This limit
is KNXBench's own; a second, independent cause with the same symptom is the interface
itself: most KNX IP interfaces only grant a small, fixed number of concurrent tunneling
connections — often exactly one — and answer a second connection attempt with their own
refusal, which KNXBench reports as *"gateway refused the connection."*

**Do this:** for KNXBench's own single-session limit, disconnect the existing session (in
the main window or the diagnostics companion window — they share one session, they don't
each get their own) before starting a new one. For a refusal coming from the interface
itself, check whether ETS, another KNXBench window, or some other tool already holds its
one tunneling slot, and disconnect that first. See
[Bus monitor and KNXnet/IP](../user-guide/07-bus-and-interfaces.md#ending-a-session) for
how a session ends and what KNXBench shows when it does.

## The desktop build fails to start

**You see:** `cargo tauri dev` or a built AppImage fails to launch, often with a
complaint about a missing shared library.

**Why:** Tauri's Linux backend draws its window and renders the frontend through
GTK and WebKit rather than bundling a browser engine. Building or running it from
source needs **GTK 3** and **WebKitGTK 4.1** on the machine; the AppImage carries its
own copies of both (checked on the 2026-10-06 candidate). KNXBench's desktop build has been exercised on
one x86_64 Arch Linux host: X11/Xvfb and native Hyprland/Wayland, with an
additional headless-Weston startup check for the post-alpha launcher. This
is not verification of other distributions, GPUs, accessibility tools or
complete native-UI behavior.

**Do this:** install your distribution's GTK 3 and WebKitGTK 4.1 packages before building
or running the desktop shell. See
[Linux setup §Desktop shell dependencies](../getting-started/05-linux-setup.md#desktop-shell-dependencies)
for what's tested and what isn't.

## The AppImage stops with "Failed to initialize GTK"

**You see:** starting the AppImage prints `Failed to initialize gtk backend!` and it
exits straight away.

**Why, for the withdrawn `v0.1.0-alpha.4` image:** that AppImage always opens its
window through X11; its GTK start-up script overrides any `GDK_BACKEND` you set.
Without a working X server — for example a Wayland session without Xwayland —
there is nothing to draw on
([known limitation §158](../../KNOWN_LIMITATIONS.md#158-the-appimage-starts-only-with-an-x-server)).

**Do this:** enable Xwayland in your compositor, or start the program natively on
Wayland from the unpacked AppImage:

```sh
./KNXBench_0.1.0-alpha.4_amd64.AppImage --appimage-extract
cd squashfs-root
export APPDIR="$PWD"
source apprun-hooks/linuxdeploy-plugin-gtk.sh
GDK_BACKEND=wayland WEBKIT_DISABLE_DMABUF_RENDERER=1 ./AppRun.wrapped
```

Both settings matter in **that older image**: without
`WEBKIT_DISABLE_DMABUF_RENDERER=1` the window closes with
"Error 71 (Protocol error)". The workaround was tested on one Hyprland machine.

**AppImages built from source with the post-alpha KL-158 change** no longer
force X11. A Wayland session hint selects native Wayland with X11 fallback;
explicit `GDK_BACKEND` values are respected. The needed DMABUF workaround is
set automatically when Wayland is permitted, unless you explicitly override
it. Start that newer build directly — no extraction is needed:

```sh
GDK_BACKEND=wayland ./KNXBench.AppImage
# Or explicitly choose a working X server:
GDK_BACKEND=x11 ./KNXBench.AppImage
```

The [launcher contract](../../APPIMAGE_LAUNCHER.md) names exactly what was
verified. The `v0.1.0-alpha.5` tagged source includes this launcher hook;
that source check is not a new native test of the downloaded image. If
neither display is reachable, the new launcher still cannot create a window.
Broader compositor/GPU compatibility is not guaranteed.

## `cargo tauri dev` fails because the frontend was never installed

**You see:** `cargo tauri dev`, run from `apps/knx-desktop`, fails during its automatic
frontend step rather than during the Rust build.

**Why:** the desktop shell's `beforeDevCommand` runs `npm run dev` inside `apps/knx-web`
for you, but that command needs `apps/knx-web`'s own dependencies installed first. On a
fresh checkout, or after a `package.json` change, those dependencies simply aren't there
yet, and `npm run dev` fails before Tauri gets anywhere near building the Rust side.

**Do this:** run `npm ci --prefix apps/knx-web` once before `cargo tauri dev`. This is
also the fix if the top-level `README.md`'s older desktop instructions led you to
`cd apps/knx-desktop && npm ci` — that directory has no `package.json` of its own; the
frontend lives in `apps/knx-web` and is installed from there.

[Manual index](../README.md) · Next: [FAQ](04-faq.md) →
