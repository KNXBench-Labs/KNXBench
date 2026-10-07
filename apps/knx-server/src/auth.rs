//! Optional password authentication for `knx-server`: configuration, sessions and the route guard.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64URL;
use base64::Engine as _;
use tokio::sync::{Semaphore, SemaphorePermit};

use crate::auth_password::{hash_password, StoredPassword, DEFAULT_ITERATIONS};
use crate::errors::ApiError;

/// The cookie the session token travels in. Named rather than inlined
/// because the frontend never has to know it — the cookie is `HttpOnly`,
/// so no script reads it — but the tests do.
pub const SESSION_COOKIE: &str = "knx_session";

/// How long a session may sit unused before it stops working. Refreshed on
/// every authenticated request, so twelve hours is twelve hours of
/// *silence*, not twelve hours of work.
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(12 * 60 * 60);

/// Added to every failed login, multiplied by the number of failures since
/// the last success and capped by [`MAX_FAILURE_MULTIPLIER`].
const DEFAULT_FAILURE_DELAY: Duration = Duration::from_millis(250);

/// The ceiling on that multiplier: two seconds of penalty per attempt,
/// which — because attempts are serialised, see [`MAX_CONCURRENT_ATTEMPTS`]
/// — is two seconds per *guess* rather than per request, and merely
/// irritating for the operator who mistyped. There is deliberately no lockout — see the module's ADR
/// (0026): locking out the only operator of a single-operator server is a
/// denial of service against its owner.
const MAX_FAILURE_MULTIPLIER: u32 = 8;

/// How many login attempts may be in flight at once.
///
/// One. The delay below only costs an attacker anything if attempts are
/// *serial*: fired in parallel, a hundred guesses would each sleep through
/// the same two seconds and land a hundred PBKDF2 derivations on the
/// blocking pool together — a penalty that is no penalty and a free way to
/// burn every core the host has. With a single permit the delay is a rate
/// limit rather than a per-request tax, and the CPU cost of guessing is
/// bounded by one derivation at a time.
const MAX_CONCURRENT_ATTEMPTS: usize = 1;

/// Below this many characters, a password gets a startup complaint.
///
/// NIST SP 800-63B (§5.1.1.2) sets 8 characters as the floor for a
/// memorised secret it is willing to accept at all, on the assumption that
/// the verifier also rate-limits and that a compromise costs one account.
/// Here it is the *only* credential, it opens a file browser and a KNX bus,
/// and it is reachable from the network the moment it exists — so the
/// complaint starts higher than the floor. Twelve is not a policy: nothing
/// is rejected, because a server that refuses to start is a worse failure
/// than a server that says it is uneasy.
const MIN_RECOMMENDED_PASSWORD_LEN: usize = 12;

/// A session token's entropy, in bytes, from the operating system's
/// CSPRNG. 32 bytes is four times what a birthday bound over any
/// plausible number of sessions needs; the cost of the extra is a longer
/// cookie.
const TOKEN_BYTES: usize = 32;

/// What the server was told about authentication — resolved once at
/// startup and then immutable.
///
/// `credential` is the whole switch: `None` means authentication is off,
/// which is the desktop shell's permanent state and the standalone
/// binary's refusal-to-bind-anything-but-loopback state.
#[derive(Clone)]
pub struct AuthConfig {
    credential: Option<Arc<StoredPassword>>,
    cookie_secure: bool,
    idle_timeout: Duration,
    failure_delay: Duration,
}

/// Hand-written so the stored digest never lands in a log line. The hash
/// is designed to be stored where people can see it, which is not the same
/// as printing it every time something formats the server's state.
impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthConfig")
            .field(
                "credential",
                &self.credential.as_ref().map(|_| "<redacted>"),
            )
            .field("cookie_secure", &self.cookie_secure)
            .field("idle_timeout", &self.idle_timeout)
            .field("failure_delay", &self.failure_delay)
            .finish()
    }
}

impl AuthConfig {
    /// No password, no sessions, no guard: every route answers to anyone
    /// who can reach the socket. Correct for the Tauri shell, which is a
    /// single-user process talking to its own in-process router over
    /// loopback, and the reason [`crate::app`] can keep its signature.
    pub fn disabled() -> Self {
        Self {
            credential: None,
            cookie_secure: false,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            failure_delay: DEFAULT_FAILURE_DELAY,
        }
    }

    /// Authentication from an already-hashed credential — the preferred
    /// configuration, because the plaintext never reaches this process.
    pub fn from_password_hash(stored: &str) -> Result<Self, String> {
        Ok(Self {
            credential: Some(Arc::new(StoredPassword::parse(stored)?)),
            ..Self::disabled()
        })
    }

    /// Authentication from a plaintext password, hashed here and now at
    /// [`DEFAULT_ITERATIONS`]. Convenient for `docker run -e`, and weaker
    /// for exactly that reason; the caller is expected to say so out loud.
    pub fn from_plaintext_password(password: &str) -> Result<Self, String> {
        Ok(Self {
            credential: Some(Arc::new(StoredPassword::parse(&hash_password(password)?)?)),
            ..Self::disabled()
        })
    }

    /// Adds `Secure` to the session cookie. Not the default, and it cannot
    /// be: the documented deployment is plain HTTP on a LAN, where a
    /// `Secure` cookie is simply never sent back and the login silently
    /// achieves nothing.
    pub fn with_cookie_secure(mut self, secure: bool) -> Self {
        self.cookie_secure = secure;
        self
    }

    /// Overrides [`DEFAULT_IDLE_TIMEOUT`]. Tests use it to make expiry
    /// observable without waiting half a day for it.
    pub fn with_idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    /// Overrides [`DEFAULT_FAILURE_DELAY`]. Tests set it to zero so that
    /// proving a wrong password is refused does not also prove that
    /// `tokio::time::sleep` works.
    pub fn with_failure_delay(mut self, delay: Duration) -> Self {
        self.failure_delay = delay;
        self
    }

    /// Whether anything is guarded at all.
    pub fn is_required(&self) -> bool {
        self.credential.is_some()
    }

    /// The work factor of the configured credential, if there is one —
    /// read at startup so a suspiciously cheap hash can be reported.
    pub fn iterations(&self) -> Option<u32> {
        self.credential.as_ref().map(|c| c.iterations())
    }
}

/// The configuration plus the live session table. One per running router;
/// nothing in it survives a restart, which is deliberate — see ADR-0026.
#[derive(Debug)]
pub(crate) struct AuthState {
    config: AuthConfig,
    /// Token to the instant it stops being valid. A `HashMap` under a
    /// `std::sync::Mutex` rather than anything cleverer: it holds one
    /// entry per logged-in browser on a server whose own documentation
    /// says it hosts one project for one operator, and every operation on
    /// it is a hash lookup that never crosses an `.await`.
    sessions: Mutex<HashMap<String, Instant>>,
    /// Failed logins since the last successful one. Process-wide, which is
    /// the honest scope: there is one password, so per-account counting
    /// would be per-account in name only.
    failures: Mutex<u32>,
    /// The gate that makes the failure delay a rate limit instead of a
    /// per-request tax. Held across verification *and* the stall, so a
    /// second guess cannot start until the first has finished paying.
    attempts: Semaphore,
}

impl AuthState {
    pub(crate) fn new(config: AuthConfig) -> Self {
        Self {
            config,
            sessions: Mutex::new(HashMap::new()),
            failures: Mutex::new(0),
            attempts: Semaphore::new(MAX_CONCURRENT_ATTEMPTS),
        }
    }

    /// Waits for the login gate and returns the permit that holds it.
    ///
    /// The caller keeps the permit for as long as the attempt lasts —
    /// verification and any penalty delay — and dropping it lets the next
    /// attempt in. It is a `tokio` semaphore, not a `std` mutex, because
    /// this wait crosses `.await` points by design; no other lock in this
    /// module is held while it does, so a queue of guesses cannot slow a
    /// session check or touch the project state at all.
    ///
    /// The semaphore is never closed, so the acquire cannot fail.
    pub(crate) async fn begin_attempt(&self) -> SemaphorePermit<'_> {
        self.attempts
            .acquire()
            .await
            .expect("the login gate is never closed")
    }

    pub(crate) fn is_required(&self) -> bool {
        self.config.is_required()
    }

    pub(crate) fn cookie_secure(&self) -> bool {
        self.config.cookie_secure
    }

    /// The stored credential, cloned as an `Arc` so the caller can carry it
    /// into `spawn_blocking` without holding any lock while PBKDF2 runs.
    pub(crate) fn credential(&self) -> Option<Arc<StoredPassword>> {
        self.config.credential.clone()
    }

    /// Mints a session, sweeping expired ones on the way past. The sweep
    /// lives here, and only here, because login is the one operation
    /// whose cost nobody notices and the only one that can grow the map.
    pub(crate) fn issue_token(&self) -> Result<String, String> {
        let mut bytes = [0u8; TOKEN_BYTES];
        getrandom::fill(&mut bytes).map_err(|e| format!("failed to read session entropy: {e}"))?;
        let token = B64URL.encode(bytes);
        let now = Instant::now();
        let mut sessions = self.sessions.lock().expect("session lock poisoned");
        sessions.retain(|_, expiry| *expiry > now);
        sessions.insert(token.clone(), now + self.config.idle_timeout);
        Ok(token)
    }

    /// Is this token live? `refresh` decides whether asking also counts as
    /// use: authenticated API calls push the idle deadline out,
    /// `GET /api/auth/status` does not — a login screen polling to find
    /// out whether it is still needed would otherwise keep a session alive
    /// with nobody at the keyboard.
    ///
    /// An expired token is removed as it is rejected, so the map cannot
    /// accumulate corpses between logins.
    pub(crate) fn validate(&self, token: &str, refresh: bool) -> bool {
        let now = Instant::now();
        let mut sessions = self.sessions.lock().expect("session lock poisoned");
        match sessions.get(token) {
            None => false,
            Some(expiry) if *expiry <= now => {
                sessions.remove(token);
                false
            }
            Some(_) => {
                if refresh {
                    sessions.insert(token.to_string(), now + self.config.idle_timeout);
                }
                true
            }
        }
    }

    pub(crate) fn revoke(&self, token: &str) {
        self.sessions
            .lock()
            .expect("session lock poisoned")
            .remove(token);
    }

    /// Counts one failed login and returns how long the caller should
    /// stall before answering. It returns the duration instead of sleeping
    /// because a handler that slept here would be holding this mutex while
    /// it did — a self-inflicted lock convoy on exactly the path an
    /// attacker controls the rate of.
    pub(crate) fn record_failure(&self) -> Duration {
        let mut failures = self.failures.lock().expect("failure lock poisoned");
        *failures = failures.saturating_add(1);
        self.config.failure_delay * (*failures).min(MAX_FAILURE_MULTIPLIER)
    }

    pub(crate) fn clear_failures(&self) {
        *self.failures.lock().expect("failure lock poisoned") = 0;
    }
}

/// Where the standalone binary is allowed to listen.
///
/// The whole of ADR-0026's enforcement is this function: with a password
/// configured the server is reachable from the network it was deployed
/// onto, and without one it is reachable from its own machine and nowhere
/// else. Pure, so it can be tested without a socket, and total, so there
/// is no third outcome where an unauthenticated server ends up on
/// `0.0.0.0` because some branch forgot to check.
pub fn bind_address(auth_required: bool) -> IpAddr {
    if auth_required {
        IpAddr::V4(Ipv4Addr::UNSPECIFIED)
    } else {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    }
}

/// A resolved configuration and everything the operator needs told about
/// how it was resolved.
#[derive(Debug)]
pub struct AuthSetup {
    pub config: AuthConfig,
    /// Lines for the startup banner, in the order they should be printed.
    /// Returned rather than logged so the decision stays a pure function
    /// of its inputs and can be tested as one.
    pub notices: Vec<String>,
}

/// Turns the three authentication environment variables into a config.
///
/// The environment itself is read in `main.rs` and nowhere else; this
/// takes what it found. The precedence is fixed and the conflict is never
/// silent: a hash beats a plaintext password, because the hash is the form
/// that does not leak through `/proc/<pid>/environ`.
pub fn resolve_auth(
    password_hash: Option<&str>,
    plaintext_password: Option<&str>,
    cookie_secure: bool,
) -> Result<AuthSetup, String> {
    let mut notices = Vec::new();
    let config = match (password_hash, plaintext_password) {
        (Some(hash), plaintext) => {
            if plaintext.is_some() {
                notices.push(
                    "KNX_AUTH_PASSWORD_HASH and KNX_AUTH_PASSWORD are both set; \
                     using the hash and ignoring the plaintext password."
                        .to_string(),
                );
            }
            let config = AuthConfig::from_password_hash(hash)?;
            if let Some(iterations) = config.iterations() {
                if iterations < DEFAULT_ITERATIONS {
                    notices.push(format!(
                        "KNX_AUTH_PASSWORD_HASH was made with {iterations} iterations, \
                         below the {DEFAULT_ITERATIONS} this build would choose; \
                         re-run `knx-server --hash-password` to replace it."
                    ));
                }
            }
            config
        }
        (None, Some(plaintext)) => {
            notices.push(
                "KNX_AUTH_PASSWORD holds a plaintext password. It is readable in \
                 /proc/<pid>/environ, in `docker inspect` and in your shell history; \
                 prefer KNX_AUTH_PASSWORD_HASH from `knx-server --hash-password`."
                    .to_string(),
            );
            if let Some(complaint) = short_password_notice(plaintext) {
                notices.push(complaint);
            }
            AuthConfig::from_plaintext_password(plaintext)?
        }
        (None, None) => {
            notices.push(
                "NO AUTHENTICATION CONFIGURED. Binding 127.0.0.1 instead of 0.0.0.0: \
                 this server would otherwise hand the open project, the KNX bus routes \
                 and the host filesystem browser to anyone who can reach the port. \
                 Set KNX_AUTH_PASSWORD_HASH (see `knx-server --hash-password`) to listen \
                 on the network."
                    .to_string(),
            );
            AuthConfig::disabled()
        }
    };
    let config = config.with_cookie_secure(cookie_secure);
    Ok(AuthSetup { config, notices })
}

/// The complaint a password-protected server earns when its session
/// cookie is not `Secure`, or `None`.
///
/// Separate from [`resolve_auth`] because the answer depends on TLS, which
/// is decided after authentication (ADR-0088): a server that terminates
/// TLS itself marks the cookie `Secure` on its own, and only a plain-HTTP
/// server — `KNX_TLS=off`, typically behind a proxy — still needs telling.
pub fn cookie_secure_notice(auth_required: bool, cookie_secure: bool) -> Option<String> {
    (auth_required && !cookie_secure).then(|| {
        "The session cookie is not marked Secure because this server speaks plain HTTP. \
         Set KNX_AUTH_COOKIE_SECURE=1 when a TLS-terminating proxy is in front of it; \
         leave it unset otherwise, where a Secure cookie would never be sent back at all."
            .to_string()
    })
}

/// The complaint a short password earns, or `None` if it is long enough.
///
/// Public because `--hash-password` says the same thing at the same
/// threshold: hashing a four-character password produces a perfectly valid
/// hash string, and a deployer who is told nothing will reasonably assume
/// the work factor made it safe. It did not — 600 000 iterations multiply
/// the cost of *each* guess, and there are not many guesses to make.
///
/// Counted in `char`s rather than bytes, so a passphrase in a non-Latin
/// script is not flattered by UTF-8 into looking longer than it is.
pub fn short_password_notice(password: &str) -> Option<String> {
    let length = password.chars().count();
    (length < MIN_RECOMMENDED_PASSWORD_LEN).then(|| {
        format!(
            "This password is {length} characters long. Fewer than \
             {MIN_RECOMMENDED_PASSWORD_LEN} is guessable in a useful amount of time \
             once this server is on the network, whatever the work factor: the \
             delay between failed attempts buys time, not safety. Nothing is \
             refusing to start — this is the server saying it is uneasy."
        )
    })
}

/// Pulls the session token out of a request's `Cookie` headers.
///
/// Hand-written on purpose. A cookie jar crate would bring a parser, a
/// signing layer and a date implementation to solve a problem that is
/// "split on `;`, split on `=`, trim" — and this server needs to read
/// exactly one cookie, never to write a general one. RFC 6265 §4.2.1's
/// grammar is `name=value` pairs separated by `; `; a client may also send
/// several `Cookie` headers, so all of them are searched.
pub(crate) fn session_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(token_in_cookie_header)
        .map(str::to_string)
}

fn token_in_cookie_header(header: &str) -> Option<&str> {
    header.split(';').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name.trim() == SESSION_COOKIE).then(|| value.trim())
    })
}

/// The `Set-Cookie` value that starts a session.
///
/// No `Max-Age` and no `Expires`, which makes it a session cookie: the
/// browser drops it when it closes, and the server's own idle timeout is
/// the authority on how long the token behind it lives. A `Max-Age` would
/// have been a second, dumber clock that expires from issue rather than
/// from last use, and the two would disagree the moment anyone worked past
/// it.
pub(crate) fn set_cookie(token: &str, secure: bool) -> String {
    let mut cookie = format!("{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict");
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

/// The `Set-Cookie` value that ends one: same attributes, empty value,
/// `Max-Age=0`. The attributes have to match or the browser keeps the
/// original cookie alongside this one.
pub(crate) fn clear_cookie(secure: bool) -> String {
    let mut cookie = format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0");
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

/// The guard over everything under `/api/` that is not an auth route.
///
/// One layer over the group, never a check inside a handler: a
/// per-handler check is a rule that holds until somebody adds the
/// forty-second route and forgets, and `/api/fs/list` — which browses the
/// host filesystem — is not a route to be protected by a habit.
pub(crate) async fn require_session(
    State(auth): State<Arc<AuthState>>,
    request: Request,
    next: Next,
) -> Response {
    if !auth.is_required() {
        return next.run(request).await;
    }
    let authenticated = session_token(request.headers())
        .is_some_and(|token| auth.validate(&token, /* refresh */ true));
    if authenticated {
        next.run(request).await
    } else {
        ApiError::with_status(StatusCode::UNAUTHORIZED, "authentication required").into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth_password::hash_password_with_iterations;

    const TEST_ITERATIONS: u32 = 1_000;

    fn test_config() -> AuthConfig {
        let stored = hash_password_with_iterations("open sesame", TEST_ITERATIONS).unwrap();
        AuthConfig::from_password_hash(&stored).unwrap()
    }

    #[test]
    fn a_short_password_is_complained_about_and_a_long_one_is_not() {
        assert!(short_password_notice("hunter2").is_some());
        // Eleven characters: one short of the threshold, so still a
        // complaint. The boundary is asserted from both sides because an
        // off-by-one here would be silent.
        assert!(short_password_notice("12345678901").is_some());
        assert!(short_password_notice("123456789012").is_none());
        assert!(short_password_notice("a perfectly ordinary password").is_none());
        // Counted in characters, not bytes: eight code points that occupy
        // far more than twelve bytes are still eight characters.
        assert!(short_password_notice("паролька").is_some());
        let complaint = short_password_notice("short").unwrap();
        assert!(complaint.contains('5'), "{complaint}");
        assert!(complaint.contains("12"), "{complaint}");
    }

    #[test]
    fn a_plaintext_password_that_is_too_short_is_reported_at_startup() {
        let setup = resolve_auth(None, Some("hunter2"), false).unwrap();
        assert!(setup.config.is_required());
        assert!(
            setup
                .notices
                .iter()
                .any(|n| n.contains("7 characters long")),
            "got {:?}",
            setup.notices
        );
    }

    #[tokio::test]
    async fn only_one_login_attempt_runs_at_a_time() {
        let state = AuthState::new(test_config());
        let held = state.begin_attempt().await;
        // The second attempt must not be able to start while the first
        // holds the gate: without this, the failure delay would be a tax
        // each guess pays privately instead of a limit on the guess rate.
        assert!(state.attempts.try_acquire().is_err());
        drop(held);
        assert!(state.attempts.try_acquire().is_ok());
    }

    #[test]
    fn the_bind_address_is_loopback_exactly_when_there_is_no_password() {
        assert_eq!(bind_address(true), IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        assert_eq!(bind_address(false), IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert!(!bind_address(false).is_unspecified());
        assert!(bind_address(false).is_loopback());
    }

    #[test]
    fn a_cookie_header_yields_the_session_token_wherever_it_sits() {
        assert_eq!(token_in_cookie_header("knx_session=abc"), Some("abc"));
        assert_eq!(
            token_in_cookie_header("theme=dark; knx_session=abc; other=1"),
            Some("abc")
        );
        assert_eq!(token_in_cookie_header("  knx_session = abc "), Some("abc"));
        assert_eq!(token_in_cookie_header("knx_session="), Some(""));
        assert_eq!(token_in_cookie_header("theme=dark"), None);
        assert_eq!(token_in_cookie_header(""), None);
        // A prefix match must not count: `knx_session_backup` is a
        // different cookie and an obvious way to smuggle a value in.
        assert_eq!(token_in_cookie_header("knx_session_backup=abc"), None);
    }

    #[test]
    fn several_cookie_headers_are_all_searched() {
        let mut headers = HeaderMap::new();
        headers.append(axum::http::header::COOKIE, "theme=dark".parse().unwrap());
        headers.append(
            axum::http::header::COOKIE,
            "knx_session=second".parse().unwrap(),
        );
        assert_eq!(session_token(&headers), Some("second".to_string()));
        assert_eq!(session_token(&HeaderMap::new()), None);
    }

    #[test]
    fn the_session_cookie_carries_the_attributes_a_browser_needs() {
        let cookie = set_cookie("tok", false);
        assert_eq!(cookie, "knx_session=tok; Path=/; HttpOnly; SameSite=Strict");
        assert!(set_cookie("tok", true).ends_with("; Secure"));
        assert!(clear_cookie(false).contains("Max-Age=0"));
        assert!(clear_cookie(false).starts_with("knx_session=;"));
        assert!(clear_cookie(true).ends_with("; Secure"));
    }

    #[test]
    fn a_token_is_opaque_and_long_enough_to_be_unguessable() {
        let state = AuthState::new(test_config());
        let token = state.issue_token().unwrap();
        // 32 bytes, base64url without padding: ceil(32 * 4 / 3) = 43.
        assert_eq!(token.len(), 43);
        assert!(token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert_ne!(token, state.issue_token().unwrap());
    }

    #[test]
    fn a_session_is_valid_until_it_is_revoked() {
        let state = AuthState::new(test_config());
        let token = state.issue_token().unwrap();
        assert!(state.validate(&token, true));
        assert!(!state.validate("not a token", true));
        state.revoke(&token);
        assert!(!state.validate(&token, true));
    }

    #[test]
    fn an_idle_session_expires_and_is_forgotten() {
        let state = AuthState::new(test_config().with_idle_timeout(Duration::ZERO));
        let token = state.issue_token().unwrap();
        assert!(!state.validate(&token, true));
        assert!(
            state.sessions.lock().unwrap().is_empty(),
            "rejecting an expired token must also drop it"
        );
    }

    #[test]
    fn use_pushes_the_deadline_out_and_a_status_check_does_not() {
        let state = AuthState::new(test_config().with_idle_timeout(Duration::from_secs(60)));
        let token = state.issue_token().unwrap();
        let issued = *state.sessions.lock().unwrap().get(&token).unwrap();

        assert!(state.validate(&token, false));
        assert_eq!(*state.sessions.lock().unwrap().get(&token).unwrap(), issued);

        assert!(state.validate(&token, true));
        assert!(*state.sessions.lock().unwrap().get(&token).unwrap() >= issued);
    }

    #[test]
    fn issuing_a_token_sweeps_the_expired_ones() {
        let state = AuthState::new(test_config().with_idle_timeout(Duration::ZERO));
        let stale = state.issue_token().unwrap();
        let _fresh = state.issue_token().unwrap();
        assert!(
            !state.sessions.lock().unwrap().contains_key(&stale),
            "the second login must have swept the first, already-expired session"
        );
    }

    #[test]
    fn the_failure_delay_widens_and_then_stops_widening() {
        let state = AuthState::new(test_config().with_failure_delay(Duration::from_millis(10)));
        assert_eq!(state.record_failure(), Duration::from_millis(10));
        assert_eq!(state.record_failure(), Duration::from_millis(20));
        for _ in 0..50 {
            state.record_failure();
        }
        assert_eq!(
            state.record_failure(),
            Duration::from_millis(10) * MAX_FAILURE_MULTIPLIER
        );
        state.clear_failures();
        assert_eq!(state.record_failure(), Duration::from_millis(10));
    }

    #[test]
    fn nothing_configured_disables_authentication_and_says_so_loudly() {
        let setup = resolve_auth(None, None, false).unwrap();
        assert!(!setup.config.is_required());
        assert_eq!(
            bind_address(setup.config.is_required()),
            Ipv4Addr::LOCALHOST
        );
        assert_eq!(setup.notices.len(), 1);
        assert!(setup.notices[0].contains("NO AUTHENTICATION CONFIGURED"));
        assert!(setup.notices[0].contains("127.0.0.1"));
    }

    #[test]
    fn a_hash_enables_authentication_without_complaint() {
        // The iteration count is rewritten rather than paid. This test is
        // about the resolution rules, which read the count and never
        // recompute the digest; spending 600 000 iterations to prove a
        // `match` would be the slowest possible way to test one.
        let stored = hash_password_with_iterations("open sesame", TEST_ITERATIONS)
            .unwrap()
            .replace(
                &format!("i={TEST_ITERATIONS}"),
                &format!("i={DEFAULT_ITERATIONS}"),
            );
        let setup = resolve_auth(Some(&stored), None, true).unwrap();
        assert!(setup.config.is_required());
        assert!(setup.notices.is_empty(), "got {:?}", setup.notices);
        assert_eq!(
            bind_address(setup.config.is_required()),
            Ipv4Addr::UNSPECIFIED
        );
    }

    #[test]
    fn a_cheap_hash_is_accepted_and_reported() {
        let stored = hash_password_with_iterations("open sesame", TEST_ITERATIONS).unwrap();
        let setup = resolve_auth(Some(&stored), None, true).unwrap();
        assert!(setup.config.is_required());
        assert_eq!(setup.notices.len(), 1);
        assert!(setup.notices[0].contains("below the 600000"));
    }

    #[test]
    fn the_hash_wins_over_the_plaintext_and_the_conflict_is_reported() {
        let stored = hash_password_with_iterations("open sesame", TEST_ITERATIONS).unwrap();
        let setup = resolve_auth(Some(&stored), Some("something else"), true).unwrap();
        assert!(setup.config.is_required());
        assert!(setup.notices[0].contains("both set"));
        // The plaintext must not be what got configured.
        assert!(setup.config.credential.unwrap().verify("open sesame"));
    }

    #[test]
    fn a_malformed_hash_is_a_startup_error_not_a_disabled_server() {
        let error = resolve_auth(Some("not a hash"), None, false).unwrap_err();
        assert!(error.contains("must start with `$`"), "got {error}");
    }

    #[test]
    fn a_plain_http_deployment_is_told_why_the_cookie_is_not_secure() {
        assert!(cookie_secure_notice(true, false)
            .unwrap()
            .contains("KNX_AUTH_COOKIE_SECURE"));
        assert_eq!(cookie_secure_notice(true, true), None);
        // Loopback without a password has no cookie to worry about.
        assert_eq!(cookie_secure_notice(false, false), None);
    }
}
