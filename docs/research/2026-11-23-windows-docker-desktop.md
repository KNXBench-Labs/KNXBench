# Windows Docker Desktop sources and verification boundary (2026-11-23)

This note records the architecture-critical facts used by
`docs/manual/user-guide/windows-docker-desktop.md` and the limits of what was
actually exercised. Date of source inspection: 2026-11-23.

## Sources

- Docker, "Install Docker Desktop on Windows"
  <https://docs.docker.com/desktop/setup/install/windows-install/> — system
  requirements: WSL 2.1.5 or later, 8 GB RAM, hardware virtualization,
  supported Windows 10/11 builds still inside Microsoft's servicing timeline;
  Windows Server unsupported; Arm is a separate Early Access offering;
  licensing terms are linked from Docker. Local capture:
  `sources/docker-windows.txt` in the task scratch.
- Docker, "WSL backend"
  <https://docs.docker.com/desktop/features/wsl/> — minimum WSL 2.1.5,
  preferably latest; Docker CLI works from Windows without a distribution, WSL
  integration is optional for that workflow. Local capture:
  `sources/docker-wsl.txt`.
- Microsoft, "Install WSL"
  <https://learn.microsoft.com/en-us/windows/wsl/install> — Windows 10 2004 /
  build 19041 or Windows 11; `wsl --install` from an administrator prompt and
  the required restart. Local capture: `sources/microsoft-wsl.txt`.
- Docker, "Host networking driver"
  <https://docs.docker.com/engine/network/drivers/host/> — host networking is a
  Linux-host driver and an opt-in Docker Desktop feature from 4.34; Desktop's
  host network works on layer 4 (TCP/UDP) and does not provide direct
  lower-layer access. Local capture: `sources/docker-host.txt`.
- PowerShell tag listing, Microsoft Container Registry
  <https://mcr.microsoft.com/v2/powershell/tags/list> — `7.5-debian-12` exists;
  used only for a syntax/parse preflight because no Windows host is available.

## What was actually exercised here

- The published image `knxbench/knxbench-server:0.1.0-alpha.23` was pulled
  successfully (digest
  `sha256:233f5fdd5b996332feefc04d2c999c45e205951409e90417a38aaf5fb76505b5`).
- `mcr.microsoft.com/powershell:7.5-debian-12` ran `pwsh -NoLogo -NoProfile`
  successfully (PowerShell 7.5.0) and parsed all 16 PowerShell blocks of the
  guide with no parser errors. This is a Linux container running PowerShell,
  **not** Windows PowerShell 5.1 on Windows.
- `knxbench/knxbench-server:0.1.0-alpha.23 --hash-password` produced a
  `$pbkdf2-sha256$i=600000$...` hash from piped stdin, matching the guide's
  regex.
- No Windows machine, WSL distribution, Docker Desktop instance, Windows
  container runtime, browser on Windows, KNXnet/IP gateway or physical bus was
  available or touched. The guide therefore explicitly does not claim Windows
  runtime verification or live KNX networking.

## Claims deliberately kept out

- No claim that Docker Desktop host networking equals Linux `--network host`
  multicast behaviour.
- No claim that the published `linux/arm64` image proves Windows-on-Arm support.
- No claim that container autostart after a Windows reboot happens before
  interactive sign-in; the guide requires starting Docker Desktop manually or
  via its sign-in preference.
