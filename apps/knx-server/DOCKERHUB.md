# KNXBench — your KNX project, in a browser

**Open your KNX project. Finally see what's in it.**

Import ETS projects, edit them with undo, document them and watch the bus talk.
This image brings the KNXBench web application and server in one container.
No Windows VM required just to look up a group address. Your laptop may quietly
thank you.

[Website](https://www.knxbench.com) ·
[Source code](https://github.com/KNXBench-Labs/KNXBench) ·
[Manual](https://github.com/KNXBench-Labs/KNXBench/blob/main/docs/manual/README.md) ·
[Releases](https://github.com/KNXBench-Labs/KNXBench/releases)

> **Alpha software. Keep backups.** KNXBench is an independent, open-source KNX
> engineering application, not ETS and not a promise of full ETS compatibility.
> Test on copies before trusting it with a building somebody paid for.

![The real KNXBench add-device wizard, with a fictional house and no hardware involved](https://raw.githubusercontent.com/KNXBench-Labs/KNXBench/main/docs/assets/workflows/add-device.gif)

## What's inside?

- **Import with a report, not a shrug.** Open `.knxproj` files and review warnings,
  unsupported constructs and conflicts. Supported opaque data and retained source
  bytes preserve otherwise unmodelled information where technically possible;
  preservation limits are reported. Retained bytes do not imply complete ETS semantics.
- **Edit what matters. Undo what you regret.** Topology, buildings and rooms,
  devices, individual and group addresses, links, DPTs, flags and parameters.
  Save projects as versioned `.knxdb` files (SQLite).
- **Wizards that show their homework.** Set up a project or add a device with a
  preview before anything is created.
- **A product database with an appetite.** Install manufacturer `.knxprod`
  packages or use the product data inside your projects.
- **Watch your bus think.** KNXnet/IP gateway discovery, tunnelling, a live group
  monitor and a telegram flow view. Real bus access needs suitable networking;
  see below before expecting a Docker bridge to perform magic.
- **Paperwork, automated.** Group-address CSV import/export and self-contained
  HTML project documentation with print preview.
- **A workbench, not a spreadsheet in disguise.** Project explorer, properties
  inspector, search, command palette and keyboard shortcuts. Light/dark themes,
  plus LCARS and phosphor-green CRT for engineers with excellent taste in nostalgia.

## Quick start: pull, run, open

The image includes the web frontend. No Git checkout, Rust installation or
compilation coffee break required. The commands below use Bash on Linux:

```bash
docker pull knxbench/knxbench-server:latest

read -rsp 'Choose a long KNXBench password: ' KNX_AUTH_PASSWORD
printf '\n'
export KNX_AUTH_PASSWORD

docker run -d --name knxbench \
  --restart unless-stopped \
  -p 127.0.0.1:8484:8080 \
  -e KNX_AUTH_PASSWORD \
  -v knxbench-data:/data \
  knxbench/knxbench-server:latest

unset KNX_AUTH_PASSWORD
```

Open **https://127.0.0.1:8484**, sign in with your chosen password, and create a
project or import a `.knxproj`. The named volume `knxbench-data` stores your saved
projects across container replacements. Back it up; a volume is storage, not a
backup department.

A password enables **HTTPS with a generated self-signed certificate**. Before
accepting the browser warning, compare the certificate fingerprint with the
startup output:

```bash
docker logs knxbench
```

The example publishes the port on your host's loopback interface only. There is
**one shared password**, not per-user access control. Use a trusted LAN or VPN;
**do not expose this server directly to the public internet**. No password is not
a shortcut: the server then binds to loopback inside the container.

## Tags and platforms

| Item | Meaning |
| --- | --- |
| `latest` | The most recently published release, including alpha pre-releases; not the newest `main` source |
| `0.1.0-alpha.6` | An example pinned release tag; choose a published version when updates should be your decision |
| `linux/amd64` | 64-bit x86 |
| `linux/arm64` | 64-bit Arm; requires a 64-bit host OS |

Ready-made images start at **0.1.0-alpha.6**. Docker selects the architecture
for your host. Both platforms are built and smoke-tested in CI; that is not a
claim that every Raspberry Pi, host OS or gateway has been hand-tested.
Linux is the tested deployment path. Docker Desktop on Windows/macOS is untested,
particularly for live-bus networking.

## Configuration and persistence

| Variable / path | Purpose |
| --- | --- |
| `KNX_AUTH_PASSWORD` | Shared login password; enables HTTPS |
| `KNX_AUTH_PASSWORD_HASH` | Alternative hashed credential; see the manual for creation and precedence |
| `KNX_PORT` | Server port inside the container; default `8080` |
| `KNX_DATA_DIR` | Persistent data directory; image default `/data` |
| `/data` | Mount a named volume or chosen host directory here |

For password hashes, TLS configuration, uploads and deployment details, use the
[Web and Docker manual](https://github.com/KNXBench-Labs/KNXBench/blob/main/docs/manual/user-guide/11-web-and-docker.md).
Never remove your data volume as part of a routine update.

## Talking to a real KNX bus

Project editing needs no hardware. Discovery and normal tunnelling to a real
KNXnet/IP gateway need **host networking on Linux**. Replace the quick-start
container with one using `--network host` and `-e KNX_PORT=8484`; omit `-p`, which
has no effect in host mode. **Host mode exposes the authenticated listener on the
host's network interfaces**: restrict access to your trusted LAN/VPN with a firewall.

Docker's default bridge does not carry discovery multicast onto the LAN. For a
manually entered gateway, `KNX_TUNNEL_ROUTE_BACK=1` can allow tunnelling through
a bridge **if the gateway supports Route Back**; it does not fix discovery.
The manual explains both paths. A green container health check does not prove
that a physical gateway is reachable.

## Updating without losing the plot

1. Save your project and back up the data volume.
2. Finish any device download or address programming before stopping the server.
3. Pull the chosen image tag, then `docker stop knxbench` and `docker rm knxbench`.
4. Run the new image with the **same volume, password and network settings**.

An image pull alone does not update a running container. Normal `docker stop`
allows an orderly server exit and tunnel disconnect; it does **not** wait for
an active device download or address programming to finish.

## Limits, help and licence

Import support is scoped and reported. Unsupported data are not a licence to
invent device behaviour, and a successful import is not a commissioning certificate.
Read the [known issues](https://github.com/KNXBench-Labs/KNXBench/blob/main/docs/manual/known-issues.md)
and [compatibility guide](https://github.com/KNXBench-Labs/KNXBench/blob/main/docs/COMPATIBILITY.md)
before work on real installations.

Found a bug? Bring a reproducible example to the
[issue tracker](https://github.com/KNXBench-Labs/KNXBench/issues), not your private
building project or credentials. Fictional houses make excellent witnesses.

**Licence:** [AGPL-3.0-or-later](https://github.com/KNXBench-Labs/KNXBench/blob/main/LICENSE).
The source and build recipe are public. KNXBench is independent of ETS and the
KNX Association.
