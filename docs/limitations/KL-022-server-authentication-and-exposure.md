# Server authentication and exposure

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-022)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback--resolved-2026-09-20"></a>
## 22. One shared server password, no per-user rights or internet-security claim

**Deployment-default update (2026-10-12, newer source).** The standalone
server and source-built image default to port 8484. Linux run/update recipes
now choose host networking, not loopback-only bridge publication. With a
password the authenticated listener is reachable on all host interfaces;
restrict access to a trusted LAN/VPN with a firewall. Without a password,
it remains host-loopback-only. Dockerfiles cannot select network mode;
`--network host` is required at container creation and `-p` has no effect
there. Published Alpha.9/older images retain their original defaults, so
downloaded-image recipes explicitly set `KNX_PORT=8484`. Existing containers
are not automatically recreated. [Current networking guide](../manual/user-guide/11-web-and-docker.md#networking).

**Resolved 2026-09-20** ([ADR-0026](../adr/0026-server-authentication-or-loopback.md)).
The heading and the anchor above are kept so existing links still resolve;
what follows is what is true now, including the parts of the old
limitation that survived.

**What exists.** `apps/knx-server` has a password login, an in-memory
session table and one middleware layer over every route under `/api/`.
`POST /api/auth/login` takes `{"password": "..."}` and sets an
`HttpOnly; SameSite=Strict; Path=/` session cookie; `POST /api/auth/logout`
invalidates it; `GET /api/auth/status` reports `{"required", "authenticated"}`
so a frontend knows whether a login screen belongs on the screen at all.
The credential is PBKDF2-HMAC-SHA256 at 600 000 iterations (OWASP's 2023
floor) over a 16-byte random salt, stored in a self-describing PHC-style
string, and verified in constant time. Sessions time out after 12 hours
idle, refreshed on use, and do not survive a restart.

**The rule that makes it mandatory.** Authentication is optional at the
type level — `knx_server::app()` still builds an unguarded router, which
is what the Tauri desktop shell uses and why the desktop deliberately has
no login — but the standalone binary will not put an unguarded router on
the network. `bind_address(auth_required)` returns `0.0.0.0` with a
password configured and `127.0.0.1` without one, and it is the only
expression of a listening address in `main.rs`. An operator who sets
neither `KNX_AUTH_PASSWORD_HASH` nor `KNX_AUTH_PASSWORD` gets a loopback-
only server and an unmissable startup line saying so.

**Unauthenticated by design, and only these:** `/healthz` (a liveness
probe that needs a password is not a liveness probe), the static frontend
assets (they are the login screen), and the three `/api/auth/*` routes
above. Everything else under `/api/` — including `/api/version`, the
KNX bus routes, and `/api/fs/*`, which browses the host filesystem under
`KNX_DATA_DIR` (confined there by `paths::resolve_in_data_dir`, not by the
login) and was the sharpest edge of this limitation — answers `401` with the usual
`{"error": ...}` body to a caller with no valid session.

**What is still a limitation, and belongs to the deployer.**

- **TLS: since 2026-10-07 the server's own, self-signed by default**
  ([ADR-0088](../adr/0088-server-terminates-tls-itself.md)). A server with a
  password speaks HTTPS on its port unless `KNX_TLS=off`; plain HTTP on
  that port is redirected. What remains:
  - The generated certificate is **self-signed**: every browser warns until
    the deployer trusts it. The startup banner prints its SHA-256
    fingerprint so the warning can be checked rather than clicked away; a
    user who accepts it without comparing is not protected against an
    active attacker on the LAN.
  - **No ACME/Let's Encrypt, no HSTS, no HTTP/2, no client certificates.**
  - **Certificates are read at startup only.** Replacing provided files or
    renewing the generated one (within 30 days of its 825-day expiry) needs
    a restart; a provided certificate that expires while running is served
    expired, and one that is expired at startup is served with a loud
    warning rather than refused.
  - **Pending handshakes are time-limited (10 s) but not counted.** Fine for
    LAN/VPN; not a defence against a flood from the internet, which this
    server still does not claim to face.
  - `KNX_TLS=off` (e.g. behind a TLS-terminating proxy) brings back the old
    situation: password and cookie in the clear unless the proxy encrypts,
    and with `--network host` the server's port stays reachable beside the
    proxy. Set `KNX_AUTH_COOKIE_SECURE=1` there; over the server's own TLS
    the cookie is always `Secure`.
- **One password, no accounts, no roles, no audit trail.** Anyone holding
  the password can do everything, including writing to the KNX bus.
  Nothing records who did what, because there is no "who". Authentication
  here is not multi-user support: see
  [§63](../KNOWN_LIMITATIONS.md#63-knx-server-has-no-multi-userconcurrent-edit-support--one-shared-project-one-shared-undo-stack-no-conflict-detection-at-all),
  which is unchanged — two browsers with valid sessions still share one
  project and one undo stack.
- **Brute-force resistance is a delay, not a lockout, and it resets on
  restart.** A failed login costs the caller 250 ms × the number of
  failures since the last success, capped at two seconds. The counter is
  in memory and process-wide. A lockout is deliberately absent: locking
  out the only operator of a single-operator server is a denial of service
  against its owner.
- **`KNX_AUTH_PASSWORD` is the weaker configuration.** A plaintext
  password in the environment is readable in `/proc/<pid>/environ`, in
  `docker inspect` and in shell history. `knx-server --hash-password`
  reads a password from stdin and prints a hash for
  `KNX_AUTH_PASSWORD_HASH`; that is the configuration to prefer. The
  server says as much at startup when it sees the plaintext form.
- **No CSRF tokens.** `SameSite=Strict` on the session cookie is the whole
  defence. It is a browser behaviour, not a server-side check.

**Not claimed.** This is not a statement that the server is safe to expose
to the internet. It is safe to expose to a network you have thought about,
over a transport you have secured yourself — which, since ADR-0088, the
server does by default with a certificate you still have to decide to
trust.

**AR13 (2026-10-04) — verified offline, boundary retained.** The guard test
no longer samples seven routes: `every_declared_route_refuses_a_caller_without_a_session_except_the_documented_four`
reads every `.route(...)` declaration from `apps/knx-server/src/` (97
method/path pairs on 89 paths at the time) and expects `401` from each,
except `/healthz` and the three `/api/auth/*` routes; a reverting mutant that
adds one unguarded `/api` route fails it, while the old seven-route test
stays green. The listening address has no override: `main.rs` binds only
`bind_address(auth_required)`. Roles, audit and CSRF remain the
deployer's, exactly as stated above; TLS moved into the server with
ADR-0088 (2026-10-07).
<!-- END retained catalogue record -->
