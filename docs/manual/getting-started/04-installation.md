← Previous: [Project status](03-project-status.md) · [Manual index](../README.md)

# Installation

**Goal:** run KNXBench and reach its welcome screen. No KNX hardware is needed.
Choose one route below; you do not need all three. Collecting installations is
not an achievement.

| Route | Best for | You need |
| --- | --- | --- |
| [Docker / browser](#b-docker--web) | Trying the workbench, or a Linux server on your LAN | Docker and a browser; no checkout required |
| [Linux AppImage](#a-linux-appimage) | A native Linux window without a source build | An x86-64 Linux host; see [Linux setup](05-linux-setup.md) |
| [From source](#c-from-source) | Development or source changes newer than the pre-release | Rust, Node.js; native host libraries for the desktop shell |

The paths share domain code, not identical platform evidence. In particular,
Docker Desktop networking is not a verified substitute for Linux host networking.

## a) Linux AppImage

The current public pre-release is
[`v0.1.0-alpha.7`](https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.7) .
Download **both** `KNXBench_0.1.0-alpha.7_amd64.AppImage` and `SHA256SUMS` from
that release page into the same directory.

From that directory:

```bash
sha256sum -c --ignore-missing SHA256SUMS
chmod +x KNXBench_0.1.0-alpha.7_amd64.AppImage
./KNXBench_0.1.0-alpha.7_amd64.AppImage
```

`SHA256SUMS` also lists the `knx-mcp` binary; `--ignore-missing` checks only the
files you downloaded and still fails on any mismatch.

**Expected result:** the checksum reports `OK`, then the application opens a
welcome screen. Stop if the checksum differs; do not solve an integrity warning
by becoming less interested in it.

The package is built by GitHub Actions with Xvfb/headless-Weston launch checks. It prefers a
native Wayland window and can fall back to X11. [Linux setup](05-linux-setup.md)
and [troubleshooting](../reference/03-troubleshooting.md) explain host requirements.
The [launcher contract](../../APPIMAGE_LAUNCHER.md) records the tested boundary.

> [!IMPORTANT]
> The AppImage is a release snapshot. This manual also documents newer source
> changes. Compare the running build's version and commit before expecting a
> new wizard or editor change to appear in a downloaded build.

**Existing alpha.4 users:** alpha.5 stores data under
`~/.local/share/com.knxbench.knxbench-labs` and does not automatically read the
older data directory. Keep the old files and open/copy them deliberately; see
[known limitation §161](../../KNOWN_LIMITATIONS.md#161-alpha5-does-not-pick-up-the-alpha4-data-folder).

**Alpha.7 native-store upgrade:** close the app and make an independent backup
before opening an existing native project. Schema 10 migrates to schema 11;
Alpha.6 refuses upgraded files. Downgrading requires the pre-upgrade backup.
History versions inside the project are not disaster backups. Update `knx-mcp` too.

**Update:** close KNXBench, download and verify the replacement AppImage, then
start it. There is no auto-updater. **Remove:** delete the AppImage. Project data
lives separately and is not removed with it. Keep independent project backups.

## b) Docker / web

The ready-made image is downloadable from
[Docker Hub](https://hub.docker.com/r/knxbench/knxbench-server) for **x86-64
(`linux/amd64`) and 64-bit Arm (`linux/arm64`)**. Docker selects your host's
architecture. You do not need Git, Rust or a source checkout.

Run these commands in a terminal, from the directory where you want your `data/`
folder. Choose your own password instead of the example:

```bash
docker pull knxbench/knxbench-server:latest
docker run -d --name knxbench -p 127.0.0.1:8484:8080 \
  -e KNX_AUTH_PASSWORD='pick something long and boring' \
  -v "$(pwd)/data:/data" knxbench/knxbench-server:latest
```

Open **https://127.0.0.1:8484**.
A password enables HTTPS by default. The generated certificate is self-signed:
compare the browser's certificate fingerprint with the one in
`docker logs knxbench` before accepting it. Then sign in using your chosen password.

**Choose your update policy:** `latest` follows published releases, including
alpha pre-releases; it does not track `main`. Use
`knxbench/knxbench-server:0.1.0-alpha.7` in both commands to pin that release.
Images start at `0.1.0-alpha.6` (9 October 2026); see
[Ready-made image from Docker Hub](../user-guide/11-web-and-docker.md#ready-made-image-from-docker-hub)
for the tag policy and platform limits.

**Expected result:** the introduction or welcome screen appears. Saved projects
live in the host's `data/` directory and survive container replacement.
**New project…** works without an ETS file. Continue to [First start](06-first-start.md).

The example publishes the port on host loopback, for this machine only. For
remote access, authentication hashes, trusted certificates, host networking,
configuration variables and troubleshooting, use the canonical
[Web and Docker deployment](../user-guide/11-web-and-docker.md) guide.

> [!WARNING]
> Do not expose this server directly to the internet. It uses one shared
> password, not separate users or roles. The quick-start plaintext password is
> visible in the container environment and can enter shell history. Prefer
> `KNX_AUTH_PASSWORD_HASH` for a lasting setup, as described in the deployment guide.

### Common first-run problems

| Symptom | Check |
| --- | --- |
| Name `knxbench` already exists | Use `docker ps -a`; do not delete an unfamiliar container or its data |
| Port 8484 is occupied | Choose another **host** port in `-p`; open that port in the browser |
| An HTTP health check fails | This authenticated setup speaks **HTTPS**; follow [HTTPS](../user-guide/11-web-and-docker.md#https) for certificate-aware checks |
| No gateway is discovered | The Docker bridge does not carry KNX multicast; [networking](../user-guide/11-web-and-docker.md#networking) explains the separate bus setup |
| Refreshing loses the current edits | Save to `.knxdb`; a running server's memory is not a backup |

Do not run a second container with `--name knxbench` to change networking.
Stop and replace the existing one deliberately, preserving its data mount.
The [Docker Hub update procedure](../user-guide/11-web-and-docker.md#updating-a-docker-hub-installation)
pulls the replacement image before stopping the running container.

**Remove:** `docker stop knxbench && docker rm knxbench` removes the container,
not the bind-mounted project directory. `docker rmi knxbench/knxbench-server:latest`
removes the downloaded image if you no longer need it. Delete project files only
after backing them up.

### Build the container from source

Use this alternative when you need source changes newer than the published image.
It needs Git and Docker, and the first build can take several minutes — coffee is
still supported:

```bash
git clone https://github.com/KNXBench-Labs/KNXBench.git
cd KNXBench
docker build -t knxbench-server -f apps/knx-server/Dockerfile . \
  --build-arg KNX_BUILD_SHA="$(git rev-parse --short HEAD)"
```

Then use the same `docker run` command above, but change the final image name to
`knxbench-server`. Run it from the same data directory if you are replacing an
existing installation; do not start a second container with the same name.
The [source-update recipe](../user-guide/11-web-and-docker.md#updating-in-one-go)
keeps this separate from Docker Hub downloads.

## c) From source

Prerequisites: the repository-pinned **Rust 1.98.0** toolchain, **Node.js 22.12 or
later**, and Git. Desktop development also needs the host libraries listed in
[Linux setup](05-linux-setup.md).

From the repository root, build a browser workbench with persistent local storage:

```bash
npm ci --prefix apps/knx-web
npm run build --prefix apps/knx-web
cargo build --locked -p knx-server
mkdir -p data
KNX_DATA_DIR="$PWD/data" KNX_STATIC_DIR="$PWD/apps/knx-web/dist" \
  cargo run --locked -p knx-server
```

**Expected result:** the startup line names `http://127.0.0.1:8080`. Open that
address. This no-password variant binds loopback only; it is not a remote-access
recipe. Setting an authentication password changes the default to HTTPS.

For the native development shell:

```bash
cargo install tauri-cli --version 2.11.4 --locked  # once
npm ci --prefix apps/knx-web
cd apps/knx-desktop
cargo tauri dev
```

The shell starts Vite for you. The [source-build guide](../development/02-building-from-source.md)
also covers the CLI, AppImage packaging, tests and build metadata.

[Manual index](../README.md) · Next: [Linux setup](05-linux-setup.md) →
