← Previous: [Web and Docker deployment](11-web-and-docker.md) · [Manual index](../README.md) · Next: [AI agents over MCP](12-ai-agents.md) →

# Windows: Docker Desktop from download to first project

**Goal:** run the KNXBench browser workbench locally on your Windows PC, with
saved projects and settings that survive container replacement. No Git checkout,
Linux terminal or KNX hardware is needed. This guide uses the published
**Alpha 23** Linux image and Windows PowerShell **5.1** (also works with newer
PowerShell). Windows installation/runtime and Windows KNX bus networking have
**not been verified by KNXBench**; these instructions combine vendor guidance
with the server's documented behaviour. [Sources and evidence boundary](../../research/2026-11-23-windows-docker-desktop.md).

The steps below publish HTTPS only to **127.0.0.1**, so only this PC can open the
workbench. You need no incoming firewall rule, router port forwarding or internet
exposure. There is one shared KNXBench password, not separate user accounts.

## 1. Check your PC and download Docker Desktop

1. Open **Settings → System → About**. Check Windows edition, version and
   **System type**. This guide's simplest route is a supported **Windows 11,
   64-bit x86 PC**. Docker lists Windows 11 23H2/build 22631 or later for its
   WSL 2 backend, but also requires a Windows version still in Microsoft's
   servicing timeline: the build number alone is not sufficient. Check the
   current [Docker Windows requirements](https://docs.docker.com/desktop/setup/install/windows-install/#system-requirements)
   for your edition before continuing. Do not treat Microsoft's older minimum
   for installing WSL as Docker Desktop's support promise. Windows Server is
   not supported by Docker Desktop.
2. Check that the PC has at least **8 GB RAM**, a 64-bit processor with SLAT,
   and hardware virtualization enabled. **Task Manager → Performance → CPU**
   shows whether virtualization is enabled. If disabled, follow your PC vendor's
   BIOS/UEFI instructions or ask your administrator. Do not guess firmware settings.
3. Open the official [Docker Desktop for Windows download page](https://docs.docker.com/desktop/setup/install/windows-install/).
   Choose **x86_64** for an Intel/AMD 64-bit PC. Windows-on-Arm uses a different
   download, currently labelled **Early Access** by Docker. The available
   `linux/arm64` KNXBench image does not establish Windows-on-Arm verification.
4. Review Docker's linked subscription terms before installation. Docker Desktop
   has its own licence conditions; KNXBench's licence does not replace them.

## 2. Install or update WSL 2, then restart Windows

WSL 2 is the Linux engine underneath this setup. You will still enter Docker
commands in **Windows PowerShell**, not in Ubuntu.

Open Start, type **Windows PowerShell**, right-click it and select **Run as
administrator**. In that administrator window, check:

```powershell
wsl --version
```

If WSL is absent, enter:

```powershell
wsl --install
```

Wait for installation to finish, then **restart Windows** as Microsoft directs.
The default command also installs Ubuntu. If Ubuntu asks for a Linux username
and password on its first launch, complete that setup; these are separate from
Windows and the KNXBench password. Docker commands from Windows do not require
a particular Linux distribution or its WSL integration to be enabled.

If WSL is already installed, or after the restart, open administrator PowerShell
again and enter:

```powershell
wsl --update
wsl --version
wsl --status
```

Expect WSL **2.1.5 or later**, preferably current. Restart again if requested.
An old inbox WSL that prints no version details needs updating. If any command
fails, resolve it using [Microsoft's WSL installation guide](https://learn.microsoft.com/en-us/windows/wsl/install)
and [Docker's WSL backend guide](https://docs.docker.com/desktop/features/wsl/)
before continuing. Close the administrator window afterwards.

## 3. Install and start Docker Desktop

1. Run the downloaded **Docker Desktop Installer.exe**. Follow the installation
   wizard; per-user/all-users choices and administrator requirements depend on
   the current installer and your organisation's policy.
2. Select **Use WSL 2 instead of Hyper-V** when offered, or choose the **WSL 2**
   backend in newer installers. Restart Windows if requested.
3. Open **Docker Desktop** from Start. Installation alone does not start it.
   Review/accept its terms if appropriate, complete onboarding and wait until
   the engine is running.
4. In Docker Desktop **Settings → General**, enable **Use WSL 2 based engine**
   if the option is shown. On WSL-only systems it may already be selected and
   hidden. Apply/restart if requested.
5. Use **Linux containers**. If the tray menu offers **Switch to Linux
   containers…**, select it. If it offers **Switch to Windows containers…**,
   you are already in the right mode. KNXBench is a Linux image.

Open a **normal, non-administrator Windows PowerShell window**. All remaining
commands in this guide go there, from any directory, unless stated otherwise.
Keep that window open: later steps reuse its variables and function.

```powershell
docker version
if ($LASTEXITCODE -ne 0) { throw 'Start Docker Desktop and resolve the engine error first.' }
$EngineOS = docker info --format '{{.OSType}}'
if ($LASTEXITCODE -ne 0 -or $EngineOS -ne 'linux') { throw 'A running Linux Docker engine is required.' }
```

Expect both **Client** and **Server** sections from `docker version`, and no
error from the Linux-engine check. If `docker` is unknown, reopen PowerShell
following installation. Do not continue with a missing Server section.

## 4. Download the image and choose your login password

In the same normal PowerShell window:

```powershell
$Image = 'knxbench/knxbench-server:0.1.0-alpha.23'
docker pull $Image
if ($LASTEXITCODE -ne 0) { throw 'Image download failed; stop here.' }
```

Expect a successful download or an up-to-date image message. This pins Alpha 23;
`latest` instead follows published releases including alphas, not repository
`main`. No container has been replaced by a pull.

The following block asks for your password with masked input, sends it through
standard input to a temporary offline helper, and keeps only the resulting hash
for container configuration. Choose a unique password of at least 12 characters
and keep it in your password manager. Do not enter a literal password in a command.

```powershell
$SecurePassword = Read-Host 'Choose your KNXBench password' -AsSecureString
$PreviousEncoding = $OutputEncoding
try {
    $OutputEncoding = New-Object System.Text.UTF8Encoding($false)
    $PlainPassword = [System.Net.NetworkCredential]::new('', $SecurePassword).Password
    if ($PlainPassword.Length -lt 12) { throw 'Choose at least 12 characters.' }
    $PasswordHash = $PlainPassword | docker run --rm -i --network none $Image --hash-password
    if ($LASTEXITCODE -ne 0) { throw 'Password hashing failed; stop here.' }
    if ($PasswordHash -notmatch '^\$pbkdf2-sha256\$i=600000\$[A-Za-z0-9+/]+\$[A-Za-z0-9+/]+$') {
        throw 'Unexpected password hash; stop here.'
    }
}
finally {
    $PlainPassword = $null
    $SecurePassword = $null
    $OutputEncoding = $PreviousEncoding
}
```

Expect the prompt and then no error. The plaintext briefly exists in process
memory, but is not placed in command history, Docker arguments or the server's
environment. UTF-8 output preserves non-ASCII passwords in PowerShell 5.1.
The helper exits and is removed automatically. Treat the hash as sensitive too:
Docker administrators can inspect it. PowerShell does not re-expand `$` inside
`$PasswordHash`; **do not double its dollar signs** for these `docker run`
commands. Compose has different escaping rules.

## 5. Create persistent storage and the container

The next commands are for a **new installation**. If `knxbench` or either named
volume already exists, stop and inspect your existing installation first
(`docker ps -a` and `docker volume ls`). Do not attach unfamiliar data or remove
an existing container to make this example work.

In the same PowerShell window, create both named volumes:

```powershell
docker volume create knxbench-data
if ($LASTEXITCODE -ne 0) { throw 'Data volume creation failed.' }
docker volume create knxbench-config
if ($LASTEXITCODE -ne 0) { throw 'Config volume creation failed.' }
```

Expect the two volume names. Define this function by copying the complete block
into the same window. It keeps the creation options in one place for updates:

```powershell
function New-KnxbenchContainer {
    param([string]$SelectedImage, [string]$LoginHash)
    if ([string]::IsNullOrWhiteSpace($LoginHash)) { throw 'A login hash is required.' }
    $RunArguments = @(
        'run', '-d', '--name', 'knxbench', '--hostname', 'knxbench',
        '--restart', 'unless-stopped',
        '--publish', '127.0.0.1:8484:8484',
        '--env', 'KNX_PORT=8484',
        '--env', "KNX_AUTH_PASSWORD_HASH=$LoginHash",
        '--mount', 'type=volume,source=knxbench-data,target=/data',
        '--mount', 'type=volume,source=knxbench-config,target=/config',
        $SelectedImage
    )
    docker @RunArguments
    if ($LASTEXITCODE -ne 0) { throw 'Container creation failed; inspect the error before retrying.' }
}
New-KnxbenchContainer -SelectedImage $Image -LoginHash $PasswordHash
$PasswordHash = $null
```

Expect a long container ID. `--hostname knxbench` keeps the generated TLS names
stable across replacement. Authentication is required even with loopback-only
publishing: without it, KNXBench binds **inside the container** to its own
loopback address, which the published port cannot reach.

`127.0.0.1:8484:8484` means Windows loopback port **8484** forwards to container
port **8484**. If the Windows port is occupied, change only the first number to
`127.0.0.1:8485:8484` in the function and open port 8485. Keep internal
`KNX_PORT=8484` and the last mapping number unchanged. Use that modified function
for every later recreation too.

## 6. Open HTTPS, check the certificate and save a project

In PowerShell:

```powershell
docker ps --filter name=knxbench
docker logs knxbench
```

Expect the container to be running and a startup line with
`https://0.0.0.0:8484`, plus a **SHA-256 certificate fingerprint**. That is the
container listener; the Windows mapping still restricts access to this PC.
Open **https://127.0.0.1:8484** in your Windows browser. A self-signed certificate
warning is expected. Open the browser's certificate details and compare its
SHA-256 fingerprint with the startup log **before** accepting the exception for
this local address. Stop if they differ; do not disable browser certificate checks
globally. Then sign in with the password you chose.

Create a fictional project with **New project…**, save it as `.knxdb`, close and
reopen it from KNXBench's file picker. [First start](../getting-started/06-first-start.md)
walks through that workflow. Importing a Windows `.knxproj` in the browser uploads
a copy to the server; a server save goes into `/data`, not automatically into
Windows Downloads. Download/export a separate copy when needed.

## 7. Understand storage and day-to-day starting

| Location | What is kept there |
| --- | --- |
| `knxbench-data` mounted at `/data` | Saved/uploaded projects, product database, app data and `.knxbench-tls` certificate/private key |
| `knxbench-config` mounted at `/config` | Configuration-root secrets, including the separate Reko encryption key (published since Alpha 23) |

These are **Docker-managed Linux volumes inside Docker Desktop's storage**, not
folders next to your PowerShell prompt or in Windows Documents. Inspect them in
Docker Desktop or with `docker volume inspect knxbench-data knxbench-config`.
Do not edit Docker's virtual disk or application database files directly. Removing
a container keeps named volumes; resetting/uninstalling Docker Desktop or deleting
volumes can lose them. Keep backups outside Docker Desktop.

Always reuse **both** volumes and the hostname on replacement, including when
using Reko later. Its encrypted account is in `/data`, its matching key in
`/config`; retaining only one is insufficient. See [key handling and recovery](11-web-and-docker.md#reko-configuration-and-key-persistence).

Save and finish active work first. Enter one of these in normal PowerShell:

```powershell
docker stop -t 130 knxbench
```

Expect `knxbench`. To start that same stopped container again:

```powershell
docker start knxbench
```

Expect `knxbench` again. To restart it without replacing the image:

```powershell
docker restart -t 130 knxbench
```

The restart policy acts only while Docker's engine runs. After restarting Windows,
**sign in and start Docker Desktop**, or enable its **Start Docker Desktop when
you sign in** preference. Wait for the engine; a deliberately stopped container
still needs `docker start knxbench`. This is not an unattended Windows service
or a promise of startup before login.

## 8. Back up and restore both storage locations

Save your projects and stop normally with `docker stop -t 130 knxbench`. Check
that it succeeds before copying anything; keep it stopped throughout both backups.
This produces a consistent pair without live database writes. In the same
PowerShell window, choose a new dated backup folder under your Windows profile:

```powershell
$BackupRoot = Join-Path $env:USERPROFILE ('KNXBench-backup-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
$BackupData = Join-Path $BackupRoot 'data'
$BackupConfig = Join-Path $BackupRoot 'config'
New-Item -ItemType Directory -Path $BackupData, $BackupConfig -ErrorAction Stop | Out-Null
docker run --rm --network none --mount type=volume,source=knxbench-data,target=/source,readonly --mount "type=bind,source=$BackupData,target=/backup" --entrypoint tar $Image -czf /backup/data.tar.gz -C /source .
if ($LASTEXITCODE -ne 0) { throw 'Data backup failed; keep the server stopped.' }
docker run --rm --network none --mount type=volume,source=knxbench-config,target=/source,readonly --mount "type=bind,source=$BackupConfig,target=/backup" --entrypoint tar $Image -czf /backup/config.tar.gz -C /source .
if ($LASTEXITCODE -ne 0) { throw 'Config backup failed; keep the server stopped.' }
Write-Host "Backups saved under $BackupRoot"
```

Expect two archives in the displayed Windows folder. Keep an independent copy
outside Docker Desktop. Restrict access to the backup folders; store the config
archive separately from the data archive, recording that they are a matching pair.
Anyone obtaining both can decrypt saved Reko credentials. The data archive also
contains the HTTPS private key; never post either archive in an issue. These
archives do not contain the Docker login hash/runtime options: keep your password
in a password manager and privately record image tag, hostname and any modified
creation options. Restart with `docker start knxbench` after successful backup.

**Restore:** keep KNXBench stopped. Use empty new volumes so a restore never mixes
old and new databases. In normal PowerShell, set `$BackupData` and `$BackupConfig`
to the respective Windows folders containing your retained archives, and `$Image`
to the image tag that created that backup (pull it successfully first). Enter:

```powershell
docker volume create knxbench-restored-data
if ($LASTEXITCODE -ne 0) { throw 'Restore data volume creation failed.' }
docker volume create knxbench-restored-config
if ($LASTEXITCODE -ne 0) { throw 'Restore config volume creation failed.' }
docker run --rm --network none --mount type=volume,source=knxbench-restored-data,target=/restore --mount "type=bind,source=$BackupData,target=/backup,readonly" --entrypoint tar $Image -xzpf /backup/data.tar.gz -C /restore
if ($LASTEXITCODE -ne 0) { throw 'Data restore failed; do not start KNXBench.' }
docker run --rm --network none --mount type=volume,source=knxbench-restored-config,target=/restore --mount "type=bind,source=$BackupConfig,target=/backup,readonly" --entrypoint tar $Image -xzpf /backup/config.tar.gz -C /restore
if ($LASTEXITCODE -ne 0) { throw 'Config restore failed; do not start KNXBench.' }
```

Choose unused restore-volume names if those names already exist. The tar archives
retain Linux modes, including key permissions. Preserve the originals. Remove only
the stopped `knxbench` container with `docker rm knxbench` (never `-v`), then
repeat the password-hash step and the creation function, changing its two volume
sources to `knxbench-restored-data` and `knxbench-restored-config`. Keep the same
hostname, port and other settings. Verify HTTPS fingerprint, login, saved projects,
products and Reko credential status before retiring any original storage. This
file restore procedure has no Windows runtime verification yet.

## 9. Update by recreating safely

A pull or restart does not replace a running container's image. Read the target
release notes first. **Alpha 23 migrates project schema 21 → 22 and product database
33 → 34; Alpha 22 and older refuse the upgraded files.** Returning requires the
matching pre-upgrade backups, not just an older image.

This recipe applies to the installation created above. If you changed the function
or added TLS/environment/mount options, preserve those options; inspect with
`docker inspect knxbench` privately (its output includes the password hash).
For installations with different options, do not apply the defaults over them.

In normal PowerShell, keep/re-enter the `New-KnxbenchContainer` function from
step 5 with your actual settings. Choose a published target tag in `$NewImage`:

```powershell
$NewImage = 'knxbench/knxbench-server:0.1.0-alpha.23'
docker pull $NewImage
if ($LASTEXITCODE -ne 0) { throw 'Pull failed; leave the existing container in place.' }
$Existing = docker inspect knxbench | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Cannot read existing configuration; stop here.' }
$HashEntry = @($Existing[0].Config.Env | Where-Object { $_ -like 'KNX_AUTH_PASSWORD_HASH=*' })
if ($HashEntry.Count -ne 1) { throw 'Expected one configured login hash; stop here.' }
$PasswordHash = $HashEntry[0].Substring('KNX_AUTH_PASSWORD_HASH='.Length)
if ([string]::IsNullOrWhiteSpace($PasswordHash)) { throw 'Missing login hash; stop here.' }
```

The successful pull and inspection leave the existing server in place. Now save
and finish work, then perform step 8's **stopped, independent backup of both
volumes** before proceeding. Do not continue if either archive fails. In the same
window, with the server still stopped and the same function/settings:

```powershell
docker rm knxbench
if ($LASTEXITCODE -ne 0) { throw 'Removal failed; stop here.' }
New-KnxbenchContainer -SelectedImage $NewImage -LoginHash $PasswordHash
$PasswordHash = $null
$Image = $NewImage
```

Check logs, fingerprint, login and reopening the saved project as in step 6.
If creation/startup fails after removal, the service remains unavailable until you
correct the error or restore a backup. There is **no automatic rollback**, and
reinstalling an older image cannot undo file migrations. Keep the backup pair.

**Remove KNXBench:** after a successful independent backup, stop with
`docker stop -t 130 knxbench`, check success, then `docker rm knxbench`.
This retains both named volumes. Do not use `docker rm -v`, `docker compose down
-v`, volume deletion or prune as an update/removal shortcut. Delete stored data
only as a separate deliberate decision after verifying your backups.

## 10. Common Windows problems and the KNX networking boundary

| Symptom | What to check |
| --- | --- |
| Docker engine/pipe connection error | Start Docker Desktop and wait for the engine; reopen PowerShell and run step 3's checks |
| WSL version error or virtualization unavailable | Update WSL, reboot when requested, check Task Manager and vendor firmware guidance; ask the administrator on managed PCs |
| Windows image/platform mismatch | Switch Docker Desktop to Linux containers; check the download architecture |
| Name already in use | Inspect `docker ps -a`; use `docker start` for your existing installation rather than creating another |
| Port already allocated | Change only the Windows-side mapping port in the creation function, as in step 5 |
| Browser cannot connect | Check `docker ps`, `docker logs`, Docker Desktop and the exact **https** URL/port; do not open firewall ports to fix local access |
| Login fails after copy/paste | Generate a new hash using step 4; do not paste a hash into an interpolating string or double `$` for `docker run` |
| After reboot the page is unavailable | Sign in, start Docker Desktop, wait for the engine, then start a deliberately stopped container |
| A project/account disappears after recreation | Inspect both mounts; an anonymous or newly created volume is not your previous storage; restore the matching backup pair |
| Gateway search finds nothing | This Windows guide is for file work, not verified live KNX networking |

Docker Desktop runs the Linux container behind a virtualized network. The published
HTTPS port does not publish KNXnet/IP discovery or configure gateway replies.
Docker Desktop **4.34 and later offers opt-in host networking**, but Docker documents
layer-4 TCP/UDP support and no direct host-interface access. That is not evidence
of native Linux multicast behaviour or working KNX discovery/tunnelling. Do not
add `--network host` and assume commissioning will work. No Windows/WSL gateway
or physical bus operation was tested for this guide. Use the documented
[Linux networking route](11-web-and-docker.md#networking) for the established
deployment model; real bus access still requires its own explicit authorization
and hardware-specific validation.

[Manual index](../README.md) · Next: [AI agents over MCP](12-ai-agents.md) →
