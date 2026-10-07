# ADR 0088: `knx-server` terminates TLS itself, with a self-signed certificate by default

Date: 2026-10-07

Status: Accepted (amends ADR-0026's "What this does not give: TLS")

## Context

ADR-0026 gave `knx-server` a password and left TLS to the deployer:
"a reverse proxy, or nothing". In practice that meant nothing. Over
plain HTTP the login body carries the password in the clear and every
later request carries the session cookie in the clear, so anything on
the LAN that can watch traffic can replay a session into a server that
writes to a real KNX bus ([KNOWN_LIMITATIONS §22](../KNOWN_LIMITATIONS.md#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback)).

Two facts shaped the answer.

**The documented deployment uses host networking.** Gateway discovery
needs KNXnet/IP multicast on the LAN, which Docker's bridge does not
carry (§79, §155), so the manual runs the image with `--network host`.
A server with a password binds `0.0.0.0` (ADR-0026), which with host
networking means the host's own interfaces. A reverse proxy in front
would add an HTTPS door and leave the plain-HTTP door on the server's
port open beside it, unless the server could also be told to bind
loopback *with* a password, an option ADR-0026 deliberately does not
have.

**The scope is LAN and VPN, not the internet** (user decision,
2026-10-07). No public domain is assumed, so ACME is out and a
certificate a browser trusts out of the box is not available. The
realistic choices are a certificate the deployer brings and one the
server makes.

Verified while deciding:

* rustls' default crypto provider `aws-lc-rs` builds `aws-lc-sys`, whose
  licence expression includes `OpenSSL`, which `deny.toml` does not
  allow. `ring` is `Apache-2.0 AND ISC`, both allowed.
* rustls 0.23's `ConfigBuilder::with_single_cert` builds a
  `CertifiedKey::from_der`, which runs `keys_match` and rejects a key that
  does not belong to the certificate. A mismatched pair is therefore a
  startup error without extra code; a test pins it.
* Apple requires, for TLS server certificates including those from
  user-added roots: names in the SAN extension (not the CN), the
  `serverAuth` extended key usage, SHA-2 signatures, and a validity of at
  most 825 days ([support.apple.com/103769](https://support.apple.com/en-us/103769)).
  The 398-day limit applies only to certificates from Apple's preinstalled
  roots ([support.apple.com/102028](https://support.apple.com/en-us/102028)).
* axum 0.8 exposes `axum::serve::Listener`, so a TLS listener plugs into
  `axum::serve` without replacing it. axum is built here without its
  `http2` feature, so only `http/1.1` is offered via ALPN.

## Decision

**`knx-server` speaks HTTPS on its own port whenever it faces the network.
With no configuration it generates a self-signed certificate. A deployer
can supply their own PEM files instead.**

1. **When.** `KNX_TLS` is three-valued. Unset (or `auto`): TLS is on when
   a password is configured (the server binds `0.0.0.0`) or certificate
   files are named; a loopback-only server stays on plain HTTP, since
   browsers already treat `localhost` as a secure context. `KNX_TLS=on`
   forces TLS, loopback included. `KNX_TLS=off` forces plain HTTP and, on
   a networked server, prints a capitalised warning at every start. An
   unrecognised value means *on*: a typo should not be what switches
   encryption off.
2. **Which certificate.** `KNX_TLS_CERT` + `KNX_TLS_KEY` name the
   deployer's PEM chain and key. Otherwise the server keeps a generated
   certificate in `<KNX_DATA_DIR>/.knxbench-tls/`.
3. **Provided files are exact.** Unreadable, not PEM, no certificate, a
   key that does not match: the server refuses to start and says which
   file and why. It never falls back to a generated certificate, which
   would hide the mistake behind a browser warning. Half a pair, or files
   together with `KNX_TLS=off`, is likewise a startup error.
4. **Expiry warns, it does not refuse.** An expired, not-yet-valid or
   nearly expired (under 30 days) provided certificate starts the server
   with a loud notice. A server that will not come back after an
   unattended expiry is a worse failure than a browser that complains,
   and the browser complains regardless.
5. **The generated certificate.** ECDSA P-256 (rcgen's default), signed by
   itself, `CN=KNXBench knx-server (self-signed)`, not a CA, key usage
   `digitalSignature`, EKU `serverAuth`. SANs are `localhost`,
   `127.0.0.1`, `::1`, the kernel host name when it is a valid DNS name,
   and the comma-separated `KNX_TLS_SAN` entries (IP addresses or DNS
   names, no wildcards). Validity is exactly 825 days from one hour before
   generation, so a client clock slightly behind does not see a
   certificate from the future and a deployer who chooses to trust it on
   an Apple device is not refused. It is reused across restarts and
   replaced at startup when it is unusable, within 30 days of expiry, or
   generated for a different name list (kept in `names.txt` beside it). A
   replacement is announced with its reason, because a fingerprint that
   changes silently looks exactly like an attack.
6. **The banner names the fingerprint.** Every HTTPS start prints the
   certificate's SHA-256 fingerprint (colon-separated, upper case, as
   browsers show it), its expiry and its names, so the first browser
   warning can be checked instead of clicked away.
7. **Storage is private and out of the API's reach.** The directory is
   `0700`, files are written `0600` through a temporary file and a rename.
   `paths.rs` refuses every path at or under the directory — relative or
   absolute, read or write, literal or through a symlink — and
   `/api/fs/list` leaves it out of the data directory's listing. Without
   that, `save-as` could overwrite the key with a project file.
8. **Plain HTTP on the TLS port is redirected.** The listener peeks at
   the first byte: 22 (TLS handshake record, RFC 8446 §5.1) is TLS;
   anything else is answered as HTTP with `307` to `https://` + the same
   `Host` and path, or `400` with instructions when `Host` is missing or
   unusable. `307`, not `301`/`308`, because a permanent redirect is
   cached and would outlive a later `KNX_TLS=off`. Host and path are
   echoed only after checking every byte (origin-form path of visible
   ASCII, no `//` prefix; host of letters, digits and `.-:[]`), so a
   client cannot inject a header. Old `http://` bookmarks keep working.
9. **Handshakes never block accepting.** A background task accepts TCP
   connections and gives each its own task with a 10-second limit for the
   handshake or the plain request head; only finished TLS streams reach
   axum through a bounded channel. A client that connects and says
   nothing occupies one task for ten seconds and nobody else's time.
10. **The cookie follows the transport.** Over HTTPS the session cookie is
    always `Secure`. `KNX_AUTH_COOKIE_SECURE` remains for a plain-HTTP
    server behind someone else's TLS, and the "cookie not Secure" notice
    moved out of `resolve_auth` into `cookie_secure_notice`, because it
    depends on a TLS decision made afterwards.

The desktop shell is unchanged: it builds its router with `app()` and
serves it on loopback inside its own process (ADR-0026 §1), where TLS
would protect nothing. Only `main.rs` of the standalone binary reads the
TLS environment.

## What this does not give

* **A certificate a browser trusts out of the box.** The generated one
  draws a warning until the deployer trusts it, per browser or system.
  The fingerprint in the banner is how to do that knowingly.
* **ACME / Let's Encrypt.** Out of scope; a deployer with a public domain
  can still terminate TLS in a proxy and run this server with
  `KNX_TLS=off` and `KNX_AUTH_COOKIE_SECURE=1`, with the bypass caveat
  above.
* **Reload without restart.** Certificates are read at startup. Replacing
  the files, or a generated certificate's renewal, takes a restart, and a
  server running for longer than its certificate's remaining life will
  serve an expired one until then.
* **Client certificates, HSTS, HTTP/2.** None configured.
* **Protection against a host that knows the password**, or against a
  user who accepts a warning for a fingerprint they did not check.
* **A cap on concurrent pending handshakes.** Each is time-limited; their
  number is not. Proportionate for LAN/VPN, not for the internet, which
  this server still does not claim to face.

## Alternatives considered

**A reverse proxy (Caddy) shipped as a compose file.** No crypto code
here and automatic local certificates. Rejected because with the
documented host networking it leaves the server's own plain-HTTP port
reachable beside the proxy, and closing that needs a new "loopback even
with a password" bind option. It would also make the secure path depend
on a second program and a second configuration this repository does not
test.

**Opt-in TLS (`KNX_TLS=on`).** No surprise for existing deployments.
Rejected for the same reason ADR-0026 rejected an opt-in password:
correctness that depends on reading a document is not correctness. The
surprise it avoids — old `http://` bookmarks — is answered by the
redirect instead.

**A second port for redirects.** Rejected: with host networking it opens
another port on the host for no gain over peeking at the first byte.

**`aws-lc-rs`.** rustls' default and faster. Rejected on licence (see
Context), not merit.

**`x509-parser` for reading the validity window.** Already in the lock
file as an optional rcgen dependency, but never built. Rejected in favour
of about twenty lines over `yasna`, which rcgen already builds, with
tests against generated certificates.

**Generating a CA plus a leaf** (so a deployer trusts one CA and the server
can rotate leaves). Better for fleets; for one server it doubles the
files and the explanation. A deployer who wants that runs their own CA
(e.g. `mkcert`) and supplies the leaf through `KNX_TLS_CERT`.

## Consequences

**Easier.** The default deployment with a password encrypts its login and
cookie with no extra step. §22's "No TLS" bullet is gone. Deployers with
their own PKI point two variables at it.

**Harder.** A deployment that upgrades with a password set changes from
`http://` to `https://` on the same port and meets a certificate warning
once; the redirect and the startup banner explain it. `knx-server` gains
`rustls`, `tokio-rustls`, `ring`, `rcgen` (and through them
`rustls-webpki`, `rustls-pki-types`, `untrusted`, `pem`, `yasna`,
`zeroize`), all under licences `deny.toml` already allows.

**Enforced by.** `tls.rs`, `tls_cert.rs` and `tls_listener.rs` unit tests
(switch parsing, every contradiction, name validation, validity read-back
and the 825-day window, reuse/renewal/regeneration, `0700`/`0600`, provided
file failures, expiry notices, redirect construction and header-injection
attempts); `paths.rs` tests for the reserved directory including absolute
paths and symlinks; and `tests/https_listener.rs`, which runs a real
rustls client against the listener: trusted handshake by name and by IP,
a name the certificate does not cover, a plain request redirected on the
same port, two silent clients not delaying a third, and a client that
rejects the certificate not breaking the listener.
