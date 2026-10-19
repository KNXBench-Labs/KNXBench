# Security Policy

KNXBench talks to real building installations: it imports project files from
other people, holds KNX Secure keys and project passwords, and can program
devices on a live bus. A security problem here can open a door, sometimes
literally. Thank you for reporting it privately.

## Reporting a vulnerability

**Please do not open a public issue, pull request or discussion for a security
problem.**

Use one of these private channels:

1. **Preferred:** GitHub's
   [private vulnerability reporting](https://github.com/KNXBench-Labs/KNXBench/security/advisories/new)
   (repository **Security** tab → **Report a vulnerability**). The report, the
   discussion and any fix stay private until an advisory is published.
2. **If you cannot use GitHub:** email
   [contact@knxbench.com](mailto:contact@knxbench.com) with the subject line
   "Security". Ordinary email is not encrypted, so describe the problem first
   and send exploit details only when asked.

A useful report contains:

- the KNXBench version (`knx --version` or `knx-server --version`, or the
  footer of the web application) and how it runs: AppImage, Docker image or a
  source build;
- the steps that reproduce the problem, and what an attacker gains;
- a **synthetic** file that triggers it, if a file is involved. Never send a
  real customer project, a keyring, a password or a KNX Secure key. A small
  hand-made file is safer for you and easier to analyse.

## What happens next

KNXBench has one maintainer and no service level. Security reports are still
handled before any other work:

1. The report is acknowledged and reproduced, and you are told whether it is
   accepted as a vulnerability.
2. The fix is developed in private, with a regression test, and lands on
   `main`.
3. The fix ships in the next pre-release. A GitHub security advisory then
   describes the problem and the fixed version, and names you if you want to
   be credited.

Please keep the details private until the fixed release is published; the
date can be agreed in the report.

## Supported versions

KNXBench is in its `0.1.0-alpha` series. There are no maintenance branches and
no backports.

| Version | Security fixes |
| --- | --- |
| `main` | Yes |
| The newest pre-release on the [Releases page](https://github.com/KNXBench-Labs/KNXBench/releases) | Yes, through the next pre-release |
| Older pre-releases | No. Please update first and check whether the problem remains |

## In scope

- **`knx-server` and the web application:** signing in without the password,
  session theft or fixation, reaching projects, uploads or files outside the
  data directory, weaknesses in the TLS set-up, cross-site requests that work
  despite the `SameSite=Strict` session cookie.
- **Untrusted files:** `.knxproj`, `.knxprod`, vendor ZIPs, CSV and legacy
  product files. For example, archive members that escape their target
  directory, inputs that exhaust memory or disk beyond the
  [documented archive budgets](docs/KNOWN_LIMITATIONS.md#catalogue-kl-159),
  XML entity expansion, crashes from small inputs, or code execution.
- **Secrets:** KNX Secure keys, keyrings, project and device passwords that
  leak into logs, exports, debug reports or the activity history.
- **The bus:** any way to make KNXBench send telegrams or program a device
  without the confirmation the application asks for, or to a device other than
  the one confirmed.
- **Release artefacts:** the Docker image, the AppImage and the
  [knxbench.com](https://knxbench.com) website, including how they are built
  and published.

## Out of scope

These are known and documented. A report that shows a way around what the
documentation promises is still welcome.

- **The single shared password.** One password means one identity, failed
  sign-ins are delayed but never locked out, and there are no CSRF tokens. See
  [what the password does not protect you from](docs/manual/user-guide/11-web-and-docker.md#what-the-password-does-not-protect-you-from)
  and [limitation 22](docs/KNOWN_LIMITATIONS.md#catalogue-kl-022).
  KNXBench makes no claim that `knx-server` is safe to expose to the internet.
- **KNX itself.** Plain KNX telegrams, on twisted pair or KNXnet/IP, are not
  authenticated by design, and KNXBench does not implement KNX Secure or KNX IP
  Secure at runtime yet
  ([limitation 8](docs/KNOWN_LIMITATIONS.md#catalogue-kl-008),
  [limitation 26](docs/KNOWN_LIMITATIONS.md#catalogue-kl-026)). Anyone on an
  unsecured bus or network can send telegrams without KNXBench.
- **Other vendors' software and devices.** Vulnerabilities in ETS, in
  gateways, in devices or in manufacturer product files belong to their
  vendors.
- **The debug report's redaction.** It removes five pattern classes and
  nothing else ([limitation 106](docs/KNOWN_LIMITATIONS.md#catalogue-kl-106)).
  Read a report before you share it.
- **Dependency advisories without an effect on KNXBench.** Dependabot already
  tracks them; the GTK3 desktop stack is described in
  [limitation 16](docs/KNOWN_LIMITATIONS.md#catalogue-kl-016). Show how the
  advisory reaches KNXBench, and it is in scope.

## Safe harbour

Research that follows this policy is welcome. Test only installations and
accounts that you own or are allowed to test, never a stranger's building or
bus, and stop as soon as you reach data that is not yours.
