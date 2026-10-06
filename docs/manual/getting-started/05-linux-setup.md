← Previous: [Installation](04-installation.md) · [Manual index](../README.md)

# Linux setup

This chapter covers the host side: what the desktop shell needs to draw a
window, where each installation method keeps its data, and one networking
detail that trips people up with Docker.

## Desktop shell dependencies

The KNXBench desktop shell is a Tauri application. Tauri on Linux draws its
window and renders the frontend through the system's own GTK and WebKit
libraries rather than bundling its own browser engine. Building or running
the desktop shell (`cargo tauri dev`, `cargo tauri build`) needs:

- **GTK 3**
- **WebKitGTK 4.1**

on the host, along with the usual Rust build toolchain.

## The tested boundary

KNXBench's desktop build has been built and run on exactly **one machine**:
an x86_64 Arch Linux host, under XWayland, with a compatible glibc, GTK 3,
and WebKitGTK 4.1. That is the tested boundary — not a claim that it works on
other distributions, other architectures, or native Wayland. If you run it
somewhere else and it works, good; if it does not, that is expected until
someone tests and documents that combination.

There is currently no ARM64 build, no distribution package, and no
auto-updater for the desktop shell.

## Where KNXBench keeps your data

**Desktop (AppImage or `cargo tauri dev`).** Projects and application data
live under your XDG data directory:

```text
$XDG_DATA_HOME/com.knxbench.knxbench-labs/projects
```

which on a typical Linux system with default XDG settings is:

```text
~/.local/share/com.knxbench.knxbench-labs/projects
```

**Docker / web server.** The server stores projects under `KNX_DATA_DIR`,
which the shipped image sets to `/data` inside the container. That path is
meaningless on its own — you make it durable by mounting a host directory
onto it, as shown in [Installation](04-installation.md):

```bash
-v "$(pwd)/data:/data"
```

Whatever host directory you mount there is where your projects actually
live; back that directory up like you would any other data you cannot
recreate.

## What the server needs from the network

The web/Docker server is a plain HTTP service. It needs its port reachable
from wherever your browser runs — `8484` in the examples in
[Installation](04-installation.md), or whatever you choose to set `KNX_PORT`
to. It needs no other inbound network access. Which interface it binds depends
on one thing only: with a password configured it listens on `0.0.0.0`, without
one it stays on loopback and says so.
[Installation](04-installation.md) explains the trade.

## Multicast and `knx bus discover`

One command needs more than a plain HTTP port: `knx bus discover`, part of
the CLI, sends its search request to a standard KNXnet/IP multicast address
and waits for gateways to answer. If you run the `knx` CLI inside a
container of your own on Docker's default bridge network, that multicast
traffic never leaves the bridge — the request goes out, nothing comes back,
and the result looks exactly like "no gateway found" rather than a network
problem you can diagnose from the output.

This is documented and was locally verified, not guessed: see
[`docs/KNOWN_LIMITATIONS.md` §79](../../KNOWN_LIMITATIONS.md#79-discovery-needs-ip-multicast-which-dockers-default-bridge-network-does-not-carry).
On Linux, running that container with `--network host` instead puts the
request on the real network interface, exactly like running the CLI outside a
container. This now applies to the shipped `knxbench-server` image too:
The bus monitor's interface **Search** in the web UI calls `POST /api/bus/discover`, which uses
the same multicast implementation. Bridge mode supports project work and all
ordinary HTTP traffic, but not bus tunnelling: the gateway answers to the
container's private address
([§155](../../KNOWN_LIMITATIONS.md#155-tunnelling-from-a-container-on-dockers-bridge-network-gets-no-answer)).
`KNX_TUNNEL_ROUTE_BACK=1` lets a tunnel through where the gateway supports
KNXnet/IP Route Back.

> **Tip**
>
> The verified recommendation is Linux Docker Engine with `--network host`.
> Docker Desktop 4.34 and later has an opt-in host-networking feature, but
> KNXBench has not verified that layer for multicast discovery. Linux-first
> remains the supported boundary.

We do not give general firewall advice beyond what is documented above — your
network, your rules.

[Manual index](../README.md) · Next: [First start](06-first-start.md) →
