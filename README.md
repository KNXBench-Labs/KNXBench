<p align="center">
  <img src="docs/assets/KNXBench-logo-small.png" alt="KNXBench logo" width="160">
</p>

<h1 align="center">KNXBench</h1>

<p align="center">
  <b>Open your KNX project. Finally see what's in it.</b><br>
  Import ETS projects, edit them with undo, document them and watch the bus talk —
  in your browser, or natively on Linux.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/status-alpha-orange" alt="Status: alpha">
  <a href="https://github.com/KNXBench-Labs/KNXBench/releases"><img src="https://img.shields.io/github/v/release/KNXBench-Labs/KNXBench?include_prereleases&label=pre-release" alt="Latest pre-release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-AGPL--3.0--or--later-green" alt="License: AGPL-3.0-or-later"></a>
  <img src="https://img.shields.io/badge/runs%20on-Linux%20%7C%20Docker%20%2B%20browser-informational" alt="Runs on Linux, or in Docker with a browser">
</p>

<p align="center">
  <a href="https://www.knxbench.com">Website</a> ·
  <a href="#getting-started">Get started</a> ·
  <a href="https://hub.docker.com/r/knxbench/knxbench-server">Docker Hub</a> ·
  <a href="docs/manual/README.md">Manual</a> ·
  <a href="docs/ProjectStats.md">Project statistics</a> ·
  <a href="docs/community/HALL_OF_FAME.md">Bug Hunters’ Hall of Fame</a> ·
  <a href="https://www.knxbench.com/story/">How this happened</a>
</p>

> [!WARNING]
> **KNXBench is alpha software.** It works, it is tested, and it will still change under
> your feet. Keep backups of every project you would miss — a KNX project is the map of a
> building somebody paid for.

<p align="center">
  <img src="docs/assets/workflows/add-device.gif" alt="The real KNXBench add-device wizard in the light Porcelain theme: a fictional switch actuator is chosen and placed on line 1.1, the server previews the free address 1.1.3, and the device is created" width="100%">
</p>

<p align="center"><sub>Pick a product, check the preview, create the device. Real app, fictional house, no hardware was bothered.</sub></p>

## What is this?

KNXBench is an independent, open-source engineering application for **KNX** building
automation — the wired standard behind a great many light switches, blinds and heating
valves. It is made for integrators, electricians and the occasional homeowner with more
switch actuators than sense.

Bring an ETS project (`.knxproj`) or start from scratch. KNXBench shows every device,
address and link in one place, lets you change them with undo, writes the paperwork for
you, and listens to the bus over KNXnet/IP. No Windows VM required just to look up a
group address.

It is not a copy of ETS and doesn't pretend to be one: it reads ETS project files,
preserves unsupported source data where technically possible, and reports the limits.
Retained bytes are not a promise of complete ETS semantics.

## Highlights

- 📥 **Imports ETS projects and admits what it doesn't understand.**
  Every `.knxproj` import comes with a report of warnings, unsupported constructs and
  conflicts. Supported opaque constructs and retained source bytes preserve otherwise
  unmodelled information; remaining preservation and mapping limits are reported.
  Password-protected ETS4/ETS5 archives are supported experimentally (so far tested on
  synthetic files only).
- ✏️ **Edit what matters. Undo what you regret.**
  Topology, buildings and rooms, devices, individual and group addresses, links,
  datapoint types, flags and parameters. Projects live in a versioned `.knxdb` file
  (SQLite) that migrates forward safely — and a file from the future is refused, not guessed.
- 🧙 **Wizards that show their homework.**
  The new-project wizard sets up areas, lines, rooms and group ranges in one step. The
  add-device wizard lets the server preview names and addresses *before* anything is created.
- 📚 **A product database that eats `.knxprod` for breakfast.**
  Install manufacturer packages, or use the product data inside your projects. A
  manufacturer ZIP with several packages inside installs in one go, previewed first. In a
  test run over 853 public manufacturer downloads, 852 installed (the biggest ones need a
  command-line flag).
- 🧠 **Watch your bus think.**
  Gateway discovery, KNXnet/IP tunnelling, a live group monitor and a
  [flow view](docs/manual/user-guide/07-bus-and-interfaces.md#the-flow-view) where
  devices light up as they talk. You can send group values, too — when you mean it.
- 📄 **Paperwork, automated.**
  Group-address CSV export and import, a self-contained HTML project document with print
  preview, and a diff between two projects with an exit code your CI will love.
- ⌨️ **Keyboard first, mouse welcome.**
  Command palette, <kbd>Ctrl</kbd>+<kbd>K</kbd> search, project explorer, properties
  inspector, drag & drop.
- 🎨 **Looks the way you like it.**
  Porcelain, Graphite, Cupertino, LCARS and a phosphor-green CRT theme. English and
  German, plus Boarisch (Bavarian) and Klingon packs for the brave. And 42 optional
  achievements, because engineering work deserves a little applause.
- 🤖 **Your AI agent gets a workbench, not the bus keys.**
  `knx-mcp` lets an MCP client such as Claude Code or Hermes answer questions about your
  saved projects — read-only by default, no bus. An explicit opt-in permits a
  previewed, confirmed group-address naming recipe; address suggestions remain a CSV you apply yourself ([AI agents over MCP](docs/manual/user-guide/12-ai-agents.md), experimental).
- 🐧 **Linux-native, browser-ready.**
  A Linux desktop app, or a Docker container you open in a browser — plus `knx`, a
  command line for the scriptable bits.

> [!NOTE]
> **Alpha 14:** a parameter workspace with declared grids, function choices right
> below each object, notes that get a say before a delete, manufacturer ZIPs that
> install in one step and a read-only diagnostic export for bug reports.
> [Release notes](docs/releases/alpha14.md) separate shipped editing from unfinished
> device support. Native schema **16** stays; the product database moves **23 → 24**.
> Backups before bravery.

## See it in action

<table>
  <tr>
    <td width="50%"><img src="docs/assets/workflows/new-project.gif" alt="New-project wizard in the Porcelain theme: details, optional topology, building and group structure, review, then the created project, marked as not saved yet"></td>
    <td width="50%"><img src="docs/assets/workflows/search.gif" alt="Ctrl+K search in the Porcelain theme: typing light filters the devices and group addresses of the fictional sample house"></td>
  </tr>
  <tr>
    <td align="center"><sub>New project: structure first, devices later</sub></td>
    <td align="center"><sub><kbd>Ctrl</kbd>+<kbd>K</kbd>, type, find. That's the whole trick.</sub></td>
  </tr>
</table>

<p align="center">
  <img src="docs/assets/readme/telegram-flow.gif" alt="The real telegram flow view of KNXBench under the green CRT theme: devices appear as telegrams are observed, values show up and fade" width="100%">
</p>

<p align="center"><sub>The flow view, fed with <b>synthetic traffic</b> from a fictional house —
no actual building was switched on and off for this clip.</sub></p>

<table>
  <tr>
    <td><img src="docs/assets/screenshots/porcelain-group-addresses.png" alt="Group address tree in the Porcelain theme after Expand all: the Lighting range with its Switching addresses 0/0/1 to 0/0/4, datapoint types and sending/receiving link counts"></td>
    <td><img src="docs/assets/screenshots/porcelain-device-tab-parameters.png" alt="The parameter workspace of a fictional switch actuator: an Areas register with General and Channel A to D, General open with Delay after bus voltage recovery set to 3"></td>
    <td><img src="docs/assets/screenshots/porcelain-topology.png" alt="Topology view with area 1 House: line 1.1 Ground floor and line 1.2 First floor with their device tiles"></td>
  </tr>
  <tr>
    <td align="center"><sub>Group addresses</sub></td>
    <td align="center"><sub>Parameter workspace</sub></td>
    <td align="center"><sub>Topology</sub></td>
  </tr>
  <tr>
    <td><img src="docs/assets/screenshots/graphite-group-addresses.png" alt="KNXBench group addresses in the dark Graphite theme"></td>
    <td><img src="docs/assets/screenshots/lcars-group-addresses.png" alt="KNXBench group addresses in the orange and lavender LCARS theme"></td>
    <td><img src="docs/assets/screenshots/user-modern-retro-green-crt-group-addresses.png" alt="KNXBench group addresses in the phosphor-green Modern Retro CRT theme"></td>
  </tr>
  <tr>
    <td align="center"><sub>Graphite</sub></td>
    <td align="center"><sub>LCARS</sub></td>
    <td align="center"><sub>Modern Retro Green CRT</sub></td>
  </tr>
</table>

<p align="center"><sub>Stills and workflow clips: the published Alpha 14 server and frontend, fictional Sample house, no live bus.</sub></p>

## Getting started

### Browser, via Docker

You need Docker and a browser — no Git checkout or build required. Download the
ready-made image from [Docker Hub](https://hub.docker.com/r/knxbench/knxbench-server),
choose your own password, and run it from the directory where you want to keep your data:

```bash
docker pull knxbench/knxbench-server:latest
docker run -d --name knxbench --network host \
  -e KNX_PORT=8484 \
  -e KNX_AUTH_PASSWORD='pick something long and boring' \
  -v "$(pwd)/data:/data" knxbench/knxbench-server:latest
```

Open <https://127.0.0.1:8484>, sign in with that password, and choose **New project…** or
import a `.knxproj`. Your projects live in `data/` and survive restarts.

Images are available for **x86-64 (`amd64`) and 64-bit Arm (`arm64`)**. `latest`
follows published releases, including alphas, not the newest source. To stay on a
specific release, use `knxbench/knxbench-server:0.1.0-alpha.14` in both commands
([tags and details](docs/manual/user-guide/11-web-and-docker.md#ready-made-image-from-docker-hub)).

Want changes newer than the release? [Build the container from source](docs/manual/getting-started/04-installation.md#build-the-container-from-source).
The compilation coffee break is now optional. The source server and new source-built
images default to port **8484**. The explicit `KNX_PORT=8484` in downloaded-image
recipes also supports Alpha.9 and older images, whose internal default was different.

With a password set, the server speaks HTTPS with a self-signed certificate, so the
browser warns once: compare the fingerprint in `docker logs knxbench` before you accept
it. It is one shared password, made for a LAN or VPN, not for the open internet.
The Linux default uses host networking: **with a password, port 8484 listens on all
host interfaces**, not only localhost. Restrict access to a trusted LAN/VPN with
your firewall. `-p` is not used in host mode — see
[Web and Docker deployment](docs/manual/user-guide/11-web-and-docker.md), which also has
the [Docker Hub update](docs/manual/user-guide/11-web-and-docker.md#updating-a-docker-hub-installation)
recipe. For a lasting setup, prefer a [password hash](docs/manual/user-guide/11-web-and-docker.md#setting-a-password)
over the quick-start plaintext password, which is visible in the container environment
and may enter shell history.

### Linux desktop, via AppImage

```bash
REL=https://github.com/KNXBench-Labs/KNXBench/releases/download/v0.1.0-alpha.14
curl -LO "$REL/KNXBench_0.1.0-alpha.14_amd64.AppImage"
curl -LO "$REL/SHA256SUMS"
sha256sum -c --ignore-missing SHA256SUMS
chmod +x KNXBench_0.1.0-alpha.14_amd64.AppImage
./KNXBench_0.1.0-alpha.14_amd64.AppImage
```

This is the `v0.1.0-alpha.14` pre-release, x86-64 only, built by the release workflow.
`--ignore-missing` skips the `knx-mcp` binary listed in the same checksum file.
Source changes after this tag still need a
[source build](docs/manual/getting-started/04-installation.md#c-from-source). The
[Linux setup](docs/manual/getting-started/05-linux-setup.md) chapter lists the host
packages it needs and what has been tested.

> [!IMPORTANT]
> Before upgrading, close KNXBench and back up native project files and Docker
> data, including the product database. Alpha 14 keeps native schema 16 and moves the
> product database to schema 24, which Alpha 13's code refuses (read in its source, not
> run); Alpha 11 and earlier refuse native schema 16 files.
> To go back, restore the pre-upgrade backup. Internal versions are not external backups.

### Where it runs

| Platform | Status |
| --- | --- |
| Linux, Docker + browser | Tested; images for amd64 and arm64 on Docker Hub; the bus needs host networking |
| Linux x86-64, AppImage | Built by CI with launch checks (Xvfb, headless Weston); host-dependent |
| Linux, from source | Rust 1.98 and Node.js 22.12+ ([building from source](docs/manual/development/02-building-from-source.md)) |
| Windows, macOS | No native build. Docker Desktop is plausible, but untested |

All the details: [Installation](docs/manual/getting-started/04-installation.md). Then try
[your first project](docs/manual/getting-started/06-first-start.md#tutorial-create-save-and-reopen-your-first-project)
— no KNX hardware or ETS file needed.

### Release notifications (since Alpha 10)

KNXBench checks GitHub at startup and hourly while the workbench is
visible. A newer eligible release gets a toast with its release-notes link;
**File → About KNXBench** keeps the information and offers **Check for updates**.
**Settings → Updates** turns automatic checks off. Nothing is downloaded or
installed for you. [How it works and what GitHub receives](docs/manual/user-guide/09-settings-and-appearance.md#release-notifications-since-alpha-10).
Alpha.10 is the first release with it; Alpha.9 and older packages do not check.

## Project status

**Alpha — and we mean it.** The current pre-release is
[`v0.1.0-alpha.14`](https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.14).
The test suite is large, the documentation is honest, and things still move.

- **Solid:** importing real ETS projects with a full report, editing with undo, the
  product database, documentation export, project diffs, and watching the bus
  (verified live against one gateway).
- **Early:** downloading a configuration to a real device works, but has been verified on
  narrowly recorded device/application cases. Programming individual addresses is refused for now, on purpose.
- **Missing:** KNX Secure, AES-protected ETS6 projects, USB interfaces and multi-user
  editing. Import is one-way: KNXBench does not write `.knxproj` files.

The row-by-row picture lives in
[Implementation status](docs/manual/implementation-status.md) and
[Supported and unsupported](docs/manual/reference/02-supported-and-unsupported.md).

## Project statistics

📊 **[Project Statistics](docs/ProjectStats.md)** is the numerical engine room: commits and
code churn, AI tokens, models and tools, prompt-cache savings, and a fun-facts section
that converts it all into *Lord of the Rings* re-reads and ant colonies. It is a
generated snapshot (last run 7 October 2026), so read its numbers as "as of".

The [test catalogue](docs/TEST_CATALOGUE.md) explains what gets exercised; each
[release note](docs/releases/alpha14.md) names its own scope. Test results belong to
an exact source snapshot, not a perpetually growing badge. The
[architecture decisions](docs/adr/README.md) explain the engineering behind it.

## Documentation

- 📖 **[The KNXBench manual](docs/manual/README.md)** — installation, a KNX primer, the user
  guide, reference and troubleshooting. No ETS experience required.
- 🧭 **[Documentation hub](docs/README.md)** — which shelf to read for which question.
- 🧪 **[Test catalogue](docs/TEST_CATALOGUE.md)** — categorized suites, short descriptions,
  source links and privacy safeguards. An inventory, not a fresh passing-test report.
- **Tutorials:** [your first project](docs/manual/getting-started/06-first-start.md#tutorial-create-save-and-reopen-your-first-project)
  and [a complete editing workflow](docs/manual/user-guide/06-configuration-workflow.md)
  (add a device, link a group address, save — no bus writes).
- **Guides:** [group addresses](docs/manual/user-guide/04-group-addresses.md),
  [devices and products](docs/manual/user-guide/05-devices-and-products.md),
  [bus monitor](docs/manual/user-guide/07-bus-and-interfaces.md),
  [reports and diff](docs/manual/user-guide/08-reports-and-diff.md),
  [command line](docs/manual/user-guide/10-command-line.md),
  [keyboard shortcuts](docs/manual/reference/01-keyboard-shortcuts.md),
  [FAQ](docs/manual/reference/04-faq.md) and
  [troubleshooting](docs/manual/reference/03-troubleshooting.md).
- **Under the hood:** the [architecture tour](docs/manual/development/03-architecture-tour.md),
  the [architecture](docs/ARCHITECTURE.md) document and the [decision records](docs/adr/README.md).

## Known issues & roadmap

The short version of what bites first:

- **Keep your original `.knxproj`.** Your working copy is `.knxdb`; there is no ETS export.
- **Autosave is not a backup.** Project versions live inside the same file, so keep your
  own copies somewhere else.
- **One server, one project, one undo stack.** Sharing a password is not collaboration.
- **Hardware support is narrow.** A project edit is not commissioning, and undo does not
  take back a telegram.

Details: [Known issues](docs/manual/known-issues.md). What is planned — and what was
decided against: [Ideas and roadmap](docs/manual/ideas-and-roadmap.md) and
[Open work](docs/OPEN_WORK.md). Bugs live in the
[issue tracker](https://github.com/KNXBench-Labs/KNXBench/issues).

## Why this exists

I moved to Linux and couldn't find a KNX tool that felt at home there. So I started an AI
assistant and began building one; several hundred euros and a few million tokens later,
KNXBench had its first working alpha. In hindsight: I must have been drunk.

The whole story, reluctantly narrated, is at
[knxbench.com/story](https://www.knxbench.com/story/).

## Bug Hunters’ Hall of Fame

**Our field-test crew earns the medals. We just fix what they find.** Every external
issue author, issue-thread replier and PR author gets a place here, from their first contribution.
Five XP per issue, one per reply and a one-time ten-XP bonus for a completed report.
PRs earn five XP while open, **25 total when merged** or **15 total when closed
without merge**. Separate PR counters and XP sources keep the points transparent.
Sixteen KNX-flavoured titles, report/completion medals and separate PR/merge medals;
ten reports earn the **Golden Bus Probe**. No actual
bus probe is shipped — this one is certified for Markdown only.

<!-- HALL-OF-FAME:START -->
| Place | Bug hunter | Rank & title | Medal | XP | Issues | Replies | Resolved | PRs | Open PRs | Merged PRs | Closed unmerged | XP sources |
| ---: | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 1 | [@thengsty](https://github.com/thengsty) | Tunnel Trouble Exorcist | 🏅 Golden Bus Probe<br>🏅 Double-Decade Fault Finder<br>🏅 Loose End Lasso<br>🏅 Regression Repair Ribbon<br>🏅 Acknowledgement Ace<br>🏅 Patch Pilot<br>🏅 Green Merge Ribbon | **335** | 21 | 15 | 19 | 1 | 0 | 1 | 0 | 105 issue + 15 reply + 190 completion + 25 PR |
| 2 | [@hagbardZ](https://github.com/hagbardZ) | Bus Bug Scout | — | **18** | 1 | 3 | 1 | 0 | 0 | 0 | 0 | 5 issue + 3 reply + 10 completion + 0 PR |

*Snapshot: 2026-10-18T20:00:11Z. Maintainers and the scoreboard bot sit this one out.*
<!-- HALL-OF-FAME:END -->

[All ranks, scoring rules and the full honour roll](docs/community/HALL_OF_FAME.md).

## Contributing & support

- **Found a bug?** Open an [issue](https://github.com/KNXBench-Labs/KNXBench/issues).
  **File → Debug report…** writes a log-based report with IP addresses removed; read it
  before you attach it, because it still names your group addresses.
- **A project or product file KNXBench chokes on?** **File → Analyze support gaps…** builds
  a reduced report for the
  [contribution form](https://github.com/KNXBench-Labs/KNXBench/issues/new?template=analysis.yml).
  Read the [step-by-step guide](docs/contribution-intake/README.md)
  ([Deutsch](docs/contribution-intake/DEUTSCH.md)) first, and never attach customer files.
- **Unknown device or programming refused?** **File → Device support report** works
  even without an open project. Preview the evidence, save locally and attach it
  deliberately; this reports a support gap, not an unlock.
  [Reporting guide](docs/manual/user-guide/08-reports-and-diff.md#universal-device-support-reporting).
- **A maintainer asked for data from your device?** **File → Export diagnostic data**
  (since Alpha 14) reads three identity values from one device, read-only, and saves a
  ZIP you can check before you send it. Nothing is programmed.
  [Five-step guide](docs/manual/user-guide/08-reports-and-diff.md#send-diagnostic-data).
- **An idea?** Open an issue and describe the problem it solves.
- **Code?** Quality gates and conventions are in
  [Contributing](docs/manual/development/01-contributing.md).

There is no sponsorship page. A good bug report is the currency we accept.

## License & trademarks

KNXBench is free software under the
[GNU Affero General Public License v3.0 or later](LICENSE). Use it privately or
commercially, modify it, share it — and if you offer a modified version over a network,
offer its source too.

KNXBench is an independent project. It is not affiliated with, endorsed or certified by
KNX Association. KNX and ETS are registered trademarks of KNX Association cvba. KNXBench
does not bundle manufacturer product catalogs; you install the product files you are
allowed to use.
