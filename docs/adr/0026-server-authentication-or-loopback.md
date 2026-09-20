# ADR 0026: `knx-server` authenticates, or it binds loopback and nothing else

Date: 2026-09-20

Status: Accepted

## Context

`apps/knx-server` has served its HTTP API and the built frontend to
`0.0.0.0` since the web/Docker deployment target shipped, with no login,
no session and no authorisation of any kind. Whoever reached the port
owned the open project.
[KNOWN_LIMITATIONS.md §22](../KNOWN_LIMITATIONS.md#22-knx-server-authenticates-with-one-password-or-refuses-to-leave-loopback)
recorded that as a deliberate scope decision — the
[2026-09-05 web/Docker design spec](../superpowers/specs/2026-09-05-web-docker-deployment-design.md)
says "LAN-only, no auth", and the stated use case was a self-hosted
container on a trusted network, not internet exposure.
[LIMITATION_TRIAGE.md](../LIMITATION_TRIAGE.md) ranks it **K1**: the
highest-risk open item in the repository that is not about commissioning.

**What was actually exposed.** Roughly forty routes, and they are not all
equal. `/api/fs/list` walks the host filesystem under `KNX_DATA_DIR` and
`/api/fs/upload` writes into it — in the shipped image that is the mounted
volume holding every project the deployment owns. `/api/project/open`,
`/save`, `/save-as` and `/import` read and overwrite those files.
`/api/bus/monitor/start` and `/api/bus/write` open a KNXnet/IP tunnel to a
real gateway and put frames on a real building's bus. `/api/debug-report`
assembles a diagnostic bundle about the host. The sharpest edge was
`/api/fs/*`: a file browser with no password on it is a different category
of problem from an editor with no password on it, because it is useful to
an attacker who has no interest in KNX whatsoever.

**Why "it is only on the LAN" stopped being an answer.** The LAN is not a
trust boundary that this process controls or can observe. It contains
whatever else is plugged in — a guest phone, a television, a compromised
printer — and the deployment that is "LAN-only" today is one `docker run
-p` and one router rule away from not being. The application had no way to
know which of those it was in, and said nothing either way.

**The constraint that shapes the answer.** `pub fn app(state: SharedState,
static_dir: Option<PathBuf>) -> Router` is called by 25 integration test
files in this crate and by the Tauri desktop shell
(`apps/knx-desktop/src-tauri/src/lib.rs`). The desktop shell is not a
deployment with a missing login. It is one operator, talking to a router
running inside their own process, over a loopback socket that no second
client can reach — which is the same property a login would be trying to
establish, already established by the operating system. Putting a password
prompt in front of a single-user desktop application would be security
theatre with a real usability cost.

**What the credentials can be built from.** `pbkdf2`, `sha2` and `base64`
are already workspace dependencies — `knx-secure` derives `.knxkeys` keys
with the same three (ADR-0008) — and `getrandom` is already in the
resolved graph via `tempfile` and `uuid`. So the whole of this ADR costs
the dependency graph one new *edge* and no new *crate*, which is why
`cargo deny check` has nothing new to say about it. ADR-0008's key-material
isolation is untouched: `knx-secure` is about KNX key material, this is an
HTTP login password, and neither type ever meets the other.

## Decision

**`knx-server` gains real password authentication. It is optional at the
type level and mandatory in practice: when authentication is not
configured, the binary refuses to bind anything except loopback.**

The enforcement is one pure function, `auth::bind_address(auth_required:
bool) -> IpAddr`, and `main.rs` has no other way to choose an address.
With a password it returns `0.0.0.0`; without one it returns `127.0.0.1`
and the startup banner says, in capitals, why. There is no configuration
flag that produces an unauthenticated server on `0.0.0.0`, because a flag
like that is only ever set by someone who has not read what it does.

Eight decisions make it up.

**1. Two constructors, and the desktop keeps the old one.**
`app(state, static_dir)` keeps its signature exactly and now delegates to
`app_with_auth(state, static_dir, AuthConfig::disabled())`. The Tauri
shell and all 25 test files compile and behave unchanged. **The desktop
application deliberately has no login and this is not an oversight** — see
the context above. `GET /api/auth/status` answers `{"required": false,
"authenticated": true}` there, which is how the frontend knows not to draw
a login screen at all.

**2. One password, no accounts.** There is one shared secret and no user
model. Authentication here answers "may this caller touch the project at
all", not "who is this caller". That is deliberately *not* a step toward
multi-user support: KNOWN_LIMITATIONS.md §63 — one shared project, one
shared undo stack, no conflict detection — is exactly as true after this
ADR as before it, and a session identifies a *browser*, not a person.

**3. PBKDF2-HMAC-SHA256, 600 000 iterations, 16-byte salt, 32-byte
output.** 600 000 is OWASP's *Password Storage Cheat Sheet* (2023) floor
for this construction. Argon2id would be the better primitive and is not
worth a new dependency here, where the attacker model is "someone on the
LAN guessing one password against a live HTTP endpoint that stalls on
every failure", not "someone offline with the hash and a GPU". The stored
form is PHC-shaped and self-describing:

```text
$pbkdf2-sha256$i=600000$<salt>$<hash>
```

`<salt>` and `<hash>` are standard base64 (RFC 4648 §4) with the padding
stripped, as PHC's own B64 alphabet prescribes; padded spellings are
rejected so one credential cannot have two written forms. The iteration
count travels *inside* the string, so raising the default later needs no
migration: an old hash keeps verifying at the count it was made with, and
a count below the current default is reported at startup rather than
silently accepted.

**4. Verification is constant-time, and hand-written.** A named
`constant_time_eq` with its own test, and no `subtle` dependency for six
lines. It compares lengths first — both operands are fixed-width digests,
so their length is public — and then accumulates the difference across the
whole buffer with no early exit and no branch on the data, inspecting the
accumulator through `std::hint::black_box`.

That is best-effort and the code says so. Rust guarantees nothing about
keeping a loop branch-free, and `black_box` is a hint rather than a
barrier; only inline assembly or a crate built for this would be more than
discouragement. It is proportionate here because of what is being
compared: both operands are PBKDF2 outputs over a salt the caller cannot
choose, so an attacker cannot steer the bytes and has nothing to learn
from where they first differ. A codebase where that stops being true
should take the dependency.

**5. `--hash-password` reads from stdin, never argv.** `knx-server
--hash-password` reads one line from standard input and prints the stored
string. It is not a flag that takes a value, because `ps` shows every
process's argument vector to every user on the machine.
`KNX_AUTH_PASSWORD_HASH` is the preferred configuration.
`KNX_AUTH_PASSWORD` accepts a plaintext password and hashes it at startup
as a `docker run -e` convenience, and says out loud that it is the weaker
option — the value is readable in `/proc/<pid>/environ`, in `docker
inspect` and in shell history. If both are set the hash wins and the
conflict is logged.

**6. Sessions are in memory and do not survive a restart.** A map from an
opaque token to an expiry instant, inside the router, gone when the
process ends. Logging everyone out on restart is correct behaviour for a
single-project engineering server, not a defect to fix with a session
store: there is no fleet to roll, no horizontal scaling, and a restart
already discarded the open project's unsaved state, which is the more
disruptive of the two. Tokens are 32 bytes from the operating system's
CSPRNG via `getrandom::fill`, base64url-encoded. `rand` was not chosen:
this needs one buffer of entropy from the OS, not a generator, a seed and
a distribution library.

**7. The cookie is `HttpOnly; SameSite=Strict; Path=/`, and `Secure` only
on request.** `HttpOnly` keeps the token away from any script, including
one injected into the frontend. `SameSite=Strict` is what stands in for
the CSRF tokens this design does not have — every mutating route is a
`POST` under `/api/`, and a strict-same-site cookie is not sent on any
cross-site request at all. `Secure` is set from `KNX_AUTH_COOKIE_SECURE`
and cannot be unconditional: the documented deployment is plain HTTP on a
LAN, where a `Secure` cookie is never sent back and the login would
silently achieve nothing. There is no `Max-Age`, which makes it a session
cookie the browser drops when it closes; the server's 12-hour idle timeout
— refreshed on every authenticated request — is the single authority on
how long the token lives, instead of two clocks that disagree the first
time somebody works past one of them. Expired tokens are removed when they
are next touched, and the whole map is swept on login. No background task.

**8. One middleware over the group, and a short list of exceptions.**
Everything under `/api/` is behind one `route_layer` —
`routes::project_routes()`, `fs_routes`, `bus_routes`,
`debug_report_routes` and `/api/version` alike — so a route added next
month is guarded by construction rather than by whoever reviews it
remembering. An unauthenticated call gets `401` with the same
`{"error": ...}` body as every other failure in `errors.rs`. The
exceptions are three and each earns its place: `/healthz`, because a
liveness probe that needs a password is not a liveness probe (an
orchestrator holds no session and would restart a perfectly healthy
container forever); the static frontend assets, because they *are* the
login screen; and `/api/auth/login`, `/logout`, `/status`, because they
are how a caller stops being unauthenticated. `route_layer` rather than
`layer` is deliberate: an unmatched path keeps falling through to a 404
instead of collecting a 401 from a guard it never reached a route behind.

**Brute force is answered with delay, not lockout.** Every failed login
increments a process-wide counter and the handler sleeps
`250 ms × min(failures, 8)` before answering — up to two seconds per
attempt, tedious for a script and merely irritating for the operator who
mistyped. The counter resets on success. There is deliberately no lockout:
locking out the only operator of a single-operator server is a denial of
service against its owner, which is a worse outcome than the guessing it
prevents.

A delay only costs an attacker anything if attempts cannot overlap, so
logins are serialised: one `tokio` semaphore permit, held across
verification *and* the stall. Without it the penalty is a private tax each
request pays concurrently — a hundred parallel guesses would sleep through
the same two seconds together and still land a hundred PBKDF2 derivations
on the blocking pool at once. With it, guessing proceeds at one attempt
per penalty, which is the rate the delay was meant to set, and the CPU
cost is bounded at one derivation at a time. The failure counter is
returned from the state rather than slept on inside it, and the semaphore
is an async one, so no `std` lock is held across any wait — otherwise the
penalty path would be a lock convoy whose rate an attacker controls.
PBKDF2 verification runs on `spawn_blocking` because half a second of
solid CPU on an async worker would stall every other request on the
runtime; the login gate does not help there, since the runtime being
starved is not the thing it guards.

None of this turns a weak password into a strong one. Two seconds per
attempt is a meaningful cost against a dictionary and no cost at all
against a four-character password, which is why one shorter than twelve
characters earns a complaint at startup — see *Consequences*.

## What this does not give

Stated plainly, because a login screen invites the assumption that the
rest came with it. This ADR gives **none** of:

* **TLS.** Still the deployer's problem — a reverse proxy, or nothing.
  Over plain HTTP the password crosses the network in the clear in the
  login request body, and the session cookie crosses it in the clear on
  every subsequent one. That is a real weakness of the LAN deployment and
  it is not fixed here.
* **User accounts, names, or roles.** One password. Everyone who has it
  can do everything, including writing to the KNX bus.
* **An audit trail.** Failed logins are printed to stderr. Nothing records
  who changed what; nothing could, because there is no "who".
* **Any protection against a compromised host on the same LAN**, which can
  read a plain-HTTP session cookie off the wire and replay it.
* **A rate limit that survives a restart.** The failure counter is in
  memory and process-wide. Restarting the container resets it, and
  anything that can restart the container has already won.
* **Multi-user isolation.** §63 stands. Two browsers holding valid
  sessions still share one project and one undo stack.
* **CSRF tokens.** `SameSite=Strict` is the whole defence, and it is a
  browser behaviour rather than a server-side check.

## Alternatives considered

**Loopback only, and no authentication at all.** The pure form of the
enforcement kept above: always bind `127.0.0.1`, and let anyone who wants
remote access put a reverse proxy in front. It satisfies the requirement
completely, costs no code and adds no dependency edge. Rejected because it
guts the Docker target's premise: a container bound to loopback is
reachable by nothing — not by another container, not by the host's own
browser through a published port, not by anything `docker run -p`
does — so the shipped image would have to be run with `--network host` to
be usable at all, and the deployment documentation would consist of an
apology. Worse, it moves the whole security decision into an unrelated
program that this repository does not ship, does not test and cannot
check, while the application continues to know nothing. The stance adopted
above keeps the loopback fallback exactly, as the *unconfigured* case, and
lets a deployer who has set a password out onto the network they chose.

**Reverse proxy with basic auth, documented rather than implemented.**
The zero-code option, and the one the old §22 effectively recommended.
Rejected for the same reason: correctness that depends entirely on a
document being read is not correctness, and this project's own priority
order starts at Correctness. It also cannot express the one rule that
matters most here — "never `0.0.0.0` without a password" — because by the
time the proxy exists, the server has already bound.

**A pre-shared API token in a header instead of a login.** Simpler:
no session table, no cookie attributes, no expiry, nothing to sweep.
Rejected because the frontend is a browser application, and a token a
browser must hold has to live in `localStorage` where any script can read
it — which is strictly worse than an `HttpOnly` cookie — or in a cookie,
at which point it is this design without the expiry. It would also make
`--hash-password` pointless: a bearer token is stored verbatim, so the
server would hold a secret that is the credential rather than a verifier
for one.

**Argon2id.** The better password hash, and the one a greenfield design
should choose. Rejected on dependency cost against a threat model that
does not reward it: the stored hash is not exposed by any route, the
attack this actually faces is online guessing against an endpoint that
stalls and logs, and `pbkdf2` at OWASP's floor is a documented, sufficient
answer that was already in the tree. If the hash ever becomes exposable —
a multi-user store, a credential file the frontend can reach — this should
be revisited, and the PHC-shaped stored form was chosen so a second
algorithm identifier can be added without breaking the first.

**`tower-sessions`, `axum-extra`'s `CookieJar`, or the `cookie` crate.**
Rejected: this server reads exactly one cookie and writes exactly two
values for it. RFC 6265 §4.2.1's `name=value; name=value` grammar is
"split on `;`, split on `=`, trim", which is nine lines with its own
tests, against a crate that would bring a parser, a signing layer, a date
implementation and a store abstraction for a `HashMap` this design wants
to keep visible.

**Persisting sessions across restarts.** Rejected as described in decision
6, and not merely as a cost: a persisted session table is a file full of
live credentials on the same volume as the projects, which would need its
own permissions, its own migration and its own answer to "what happens
when it is restored from a backup".

**Making the desktop shell log in too, for uniformity.** Rejected. The
shell already has the property a login would establish; adding one would
trade a real usability cost for no security gain, and would make
`AuthConfig::disabled()` a thing nothing uses, which is how a code path
rots.

## Consequences

**Easier.** The Docker image can be published on a network the deployer
chose, with the application enforcing the one rule that used to live in a
paragraph of README prose. §22 stops being the repository's top-ranked
risk. The `/api/fs/*` routes — a filesystem browser — stop being world-
readable by default. And the answer to "can I expose this?" becomes a
sentence about what it does and does not defend against, instead of "no".

**Harder.** Anyone running the image now has a step before it works:
produce a hash, or accept loopback. A deployment that upgrades into this
build without setting either variable will bind `127.0.0.1` and appear to
have stopped working — which is the correct failure, and is why the
startup line that explains it is impossible to miss. `cargo test -p
knx-server` also grew about seven seconds, spent by the single test that
pays the real 600 000-iteration work factor rather than the thousand
iterations the rest of the suite uses; a default nothing exercises is a
default nobody can trust.

**Uneasy, out loud.** A password shorter than twelve characters earns a
startup complaint — from `--hash-password` and from `KNX_AUTH_PASSWORD`
alike — and nothing is rejected. NIST SP 800-63B §5.1.1.2 puts the floor
for a memorised secret at eight characters where a verifier rate-limits
and a compromise costs one account; this credential is the only one there
is, it opens a file browser and a KNX bus, and it is on the network from
the moment it exists, so the complaint starts above the floor. It is a
complaint rather than a refusal because a server that will not start is a
worse failure than a server that says it is worried, and because the
deployer, not this file, knows what the port is reachable from.

**Enforced by.** `bind_address` is pure and tested in both directions, and
is the only expression of a listening address in `main.rs`. The route
guard is tested with one representative route per group, so adding a sixth
route group without guarding it fails `cargo test -p knx-server`.
Everything else is review — in particular, a route mounted outside
`app_with_auth`'s guarded group would be invisible to all of it.

**Recorded as a limitation.** KNOWN_LIMITATIONS.md §22 is rewritten to
what is true afterwards, keeping its number and its old anchor so existing
links still resolve. §63 keeps its meaning and loses one sentence that
this ADR made false: middleware *does* now read a cookie, and it still
cannot tell two operators apart, because both hold the same password.

**Not claimed anywhere.** No KNX certification, no ETS compatibility, and
no claim that this server is safe to expose to the internet. It is safe to
expose to a network you have thought about, over a transport you have
secured yourself.

## Amendment, 2026-09-20 — the frontend half (T01b)

The decision above is unchanged. This records how `apps/knx-web` meets it,
because the server's 401 is only half a login.

**The gate is a wrapper, not a route.** `AuthGate.tsx` asks
`GET /api/auth/status` once on mount and branches three ways: not required —
render the application and never mention sessions; required and already
authenticated — same, plus a logout control; required and not authenticated —
render `LoginScreen.tsx`. If the status request itself fails, the gate **fails
open** and renders the application: a server that did not answer is not a
server that refused, and conjuring a password prompt onto the desktop shell
because a fetch failed would be a worse lie than letting the next real 401
close the gate.

**Mid-session 401 does not unmount the workbench.** `api.ts` publishes one
event when a request outside `/api/auth/` comes back 401, through
`session.ts` — a `Set` of callbacks, no React, so `api.ts` stays
framework-free. `request()` reports for the calls that go through it;
`installProductPackage` and `FsPicker.tsx`'s two helpers hold their own
`fetch` for body reasons and call the exported `noteRefusal` by hand. That
hand-wiring is the weak seam: a future raw `fetch` that forgets it is a 401
nobody hears, which is precisely how `/api/fs/*` was missed in the first
round. The gate answers by covering the screen with the login panel while
leaving the application mounted behind it, `inert` so the hidden workbench is
unreachable by tab, pointer or screen reader. `display: contents` does not
defeat `inert`, which applies through the flat tree. This is the only way to
preserve unsaved work, because no endpoint hands a loaded project back to a
fresh mount. What it cannot preserve is a *restarted* server's copy of the
project, and the expiry notice says so in both languages rather than implying
a completeness it does not have.

**What the cover is and is not.** It is opaque in every theme, so a
deliberate logout hides the project from anyone looking at the screen. It is
not an unmount: the workbench's DOM stays in the document, where devtools or
a browser extension can still read it. That is deliberate, and the two cases
want it for different reasons — an expired session must keep the unsaved work
it was holding, and `login.signedOutNotice` promises the workbench comes back
exactly as it was left, which unmounting on logout would turn into a lie.
Privacy here means privacy from the room, not from the document.

**The password is never echoed.** It travels in a POST body, is cleared from
state before the application renders, and appears in no URL, log or error
message; a rejected attempt shows a fixed translated string, not the server's.
Submit is disabled while a request is in flight — the server serialises login
attempts behind a single permit and delays failures on purpose, and a client
that lets a user queue guesses turns that delay into a queue of pending
requests.

**The file picker closes itself.** `FsPicker.tsx` mounts on its own
`createRoot` attached to `document.body`, a sibling of `#root` rather than a
descendant of the gate's `inert` wrapper, so the cover does not reach it: a
401 arriving while a picker is open would raise the login screen over a dialog
that still held the focus trap. It therefore subscribes to the same expiry
event and resolves itself as a cancel. Closing it was chosen over marking it
`inert` because the user's next act is typing a password, a directory listing
fetched before the session ended is stale, and every call site already handles
the `null` that Cancel returns. It is also the narrower change: no second
registry of live roots for the gate to keep.

**The notice is the dialog's description.** A live region announces
mutations, and the expiry notice is present in the dialog's first paint, so
`role="status"` alone would let the honest half of "preserve unsaved work or
warn" pass a screen-reader user by. `aria-describedby` on the
`role="dialog"` element names it. The error keeps `role="alert"`, which is
correct: it *is* inserted in response to something the user just did.

**Not claimed.** This is a session gate on a single shared password, not a
user model. The logout control is absent entirely when authentication is off,
because a control that logs nobody out of nothing is worse than no control.
