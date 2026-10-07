//! The standalone `knx-server` binary: HTTP API and built frontend on one port.

use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--version" | "-V") => {
            println!("{}", knx_server::version_line());
            return;
        }
        Some("--hash-password") => {
            hash_password_from_stdin();
            return;
        }
        _ => {}
    }
    let port: u16 = std::env::var("KNX_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    let static_dir = std::env::var("KNX_STATIC_DIR").ok().map(PathBuf::from);
    let data_dir = std::env::var("KNX_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    // Created up front rather than lazily: every path under `data_dir` is
    // resolved by canonicalizing it, which fails outright if the root
    // itself does not exist — a `KNX_DATA_DIR` pointing at a volume that
    // has not been created yet would otherwise turn every `/api/fs/*`
    // call into an opaque "data dir unreadable" 500. Same call the Tauri
    // shell already makes for its own data dir.
    std::fs::create_dir_all(&data_dir)
        .unwrap_or_else(|e| panic!("failed to create data dir {}: {e}", data_dir.display()));

    // The only place in this workspace that reads the authentication
    // environment. `resolve_auth` decides; this prints what it decided and
    // binds where it says (ADR-0026).
    let password_hash = std::env::var("KNX_AUTH_PASSWORD_HASH").ok();
    let plaintext_password = std::env::var("KNX_AUTH_PASSWORD").ok();
    let cookie_secure_requested = env_flag("KNX_AUTH_COOKIE_SECURE");
    let setup = knx_server::resolve_auth(
        password_hash.as_deref().filter(|s| !s.is_empty()),
        plaintext_password.as_deref().filter(|s| !s.is_empty()),
        cookie_secure_requested,
    )
    .unwrap_or_else(|e| panic!("authentication configuration is unusable: {e}"));
    let auth_required = setup.config.is_required();

    // TLS is decided after authentication because it depends on it: a
    // server on the network encrypts unless told not to (ADR-0088). This is
    // the only place that reads the TLS environment.
    let tls_switch = std::env::var("KNX_TLS").ok();
    let tls_cert = non_empty_env("KNX_TLS_CERT");
    let tls_key = non_empty_env("KNX_TLS_KEY");
    let tls_names = non_empty_env("KNX_TLS_SAN");
    let hostname = system_hostname();
    let tls = knx_server::resolve_tls(knx_server::TlsInputs {
        switch: knx_server::TlsSwitch::from_env_value(tls_switch.as_deref()),
        cert: tls_cert.as_deref(),
        key: tls_key.as_deref(),
        extra_names: tls_names.as_deref(),
        hostname: hostname.as_deref(),
        auth_required,
        data_dir: &data_dir,
    })
    .unwrap_or_else(|e| panic!("TLS configuration is unusable: {e}"));
    let prepared = knx_server::prepare_tls(&tls.plan, time::OffsetDateTime::now_utc())
        .unwrap_or_else(|e| panic!("TLS configuration is unusable: {e}"));
    let https = prepared.is_some();
    // Over HTTPS the cookie is always Secure; the variable is only for a
    // plain-HTTP server behind somebody else's TLS.
    let cookie_secure = cookie_secure_requested || https;
    let auth_config = setup.config.with_cookie_secure(cookie_secure);

    let notices = setup
        .notices
        .iter()
        .cloned()
        .chain(knx_server::cookie_secure_notice(
            auth_required,
            cookie_secure,
        ))
        .chain(tls.notices)
        .chain(prepared.iter().flat_map(|p| p.notices.clone()));
    for notice in notices {
        eprintln!("knx-server: {notice}");
    }

    let mut app_state = knx_server::AppState::with_user_product_db(data_dir);
    let return_path = tunnel_return_path(std::env::var("KNX_TUNNEL_ROUTE_BACK").ok().as_deref());
    if return_path == knx_net::TunnelReturnPath::RouteBack {
        eprintln!(
            "knx-server: KNX_TUNNEL_ROUTE_BACK is set: tunnels ask the gateway to answer the \
             packet's source (Route Back, for Docker's bridge network). Gateway discovery \
             still needs --network host."
        );
        app_state.connector = Box::new(knx_server::RealConnector::new(return_path));
    }
    let state = Arc::new(app_state);
    let app = knx_server::app_with_auth(state, static_dir, auth_config);

    // Never `0.0.0.0` without a password, and never silently: the address
    // is a pure function of whether authentication is configured, and the
    // notices above have already said why it came out the way it did.
    let host = knx_server::bind_address(auth_required);
    let listener = tokio::net::TcpListener::bind((host, port))
        .await
        .unwrap_or_else(|e| panic!("failed to bind {host}:{port}: {e}"));
    match prepared {
        Some(prepared) => {
            let listener = knx_server::TlsListener::new(listener, prepared.config)
                .unwrap_or_else(|e| panic!("failed to start the TLS listener: {e}"));
            println!("knx-server listening on https://{host}:{port}");
            axum::serve(listener, app).await.expect("server error");
        }
        None => {
            println!("knx-server listening on http://{host}:{port}");
            axum::serve(listener, app).await.expect("server error");
        }
    }
}

/// An environment variable, with an empty value read as unset.
fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

/// The kernel's host name, for the generated certificate's names. With
/// `--network host` a container shares it with the machine. Absent rather
/// than an error when unreadable: it is a convenience, and `KNX_TLS_SAN`
/// covers every other name.
fn system_hostname() -> Option<String> {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
}

/// Reads a password from standard input and prints its stored form, so an
/// operator can produce a `KNX_AUTH_PASSWORD_HASH` without starting a
/// server and without the password ever reaching an argument vector —
/// `ps` shows everyone on the machine what is in one.
///
/// The password is everything up to the first newline, with a trailing
/// `\r\n` or `\n` removed and nothing else trimmed: a password may
/// legitimately start or end with a space, and silently eating one would
/// produce a hash that never matches what the operator types.
fn hash_password_from_stdin() {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .unwrap_or_else(|e| panic!("failed to read the password from stdin: {e}"));
    let first_line = input.split('\n').next().unwrap_or("");
    let password = first_line.strip_suffix('\r').unwrap_or(first_line);
    if password.is_empty() {
        eprintln!("knx-server: refusing to hash an empty password");
        std::process::exit(2);
    }
    // On stderr, so the hash on stdout stays a hash: this is routinely
    // captured into a variable or a file, and a warning mixed into it
    // would produce a credential nothing can verify.
    if let Some(complaint) = knx_server::short_password_notice(password) {
        eprintln!("knx-server: {complaint}");
    }
    match knx_server::hash_password(password) {
        Ok(hash) => println!("{hash}"),
        Err(e) => {
            eprintln!("knx-server: {e}");
            std::process::exit(1);
        }
    }
}

/// `KNX_TUNNEL_ROUTE_BACK` read as a flag (KNOWN_LIMITATIONS §155).
fn tunnel_return_path(value: Option<&str>) -> knx_net::TunnelReturnPath {
    if flag_from_value(value) {
        knx_net::TunnelReturnPath::RouteBack
    } else {
        knx_net::TunnelReturnPath::LocalAddress
    }
}

/// An environment variable read as a flag.
fn env_flag(name: &str) -> bool {
    flag_from_value(std::env::var(name).ok().as_deref())
}

/// Present and not one of the obvious spellings of "no" means yes.
/// `KNX_AUTH_COOKIE_SECURE=false` switching a security attribute *on*
/// would be a memorable way to lose an afternoon.
fn flag_from_value(value: Option<&str>) -> bool {
    match value {
        None => false,
        Some(value) => !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "" | "0" | "false" | "no" | "off"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{flag_from_value, tunnel_return_path};

    #[test]
    fn route_back_is_opt_in_and_reads_like_every_other_flag() {
        use knx_net::TunnelReturnPath::{LocalAddress, RouteBack};
        for (value, expected) in [
            (None, LocalAddress),
            (Some(""), LocalAddress),
            (Some("0"), LocalAddress),
            (Some("false"), LocalAddress),
            (Some("1"), RouteBack),
            (Some("yes"), RouteBack),
        ] {
            assert_eq!(tunnel_return_path(value), expected, "{value:?}");
        }
    }

    #[test]
    fn a_flag_is_off_unless_it_says_something_affirmative() {
        for off in [
            None,
            Some(""),
            Some(" "),
            Some("0"),
            Some("false"),
            Some("FALSE"),
            Some("no"),
            Some("off"),
        ] {
            assert!(!flag_from_value(off), "{off:?} must read as off");
        }
        for on in [
            Some("1"),
            Some("true"),
            Some("yes"),
            Some("on"),
            Some("secure"),
        ] {
            assert!(flag_from_value(on), "{on:?} must read as on");
        }
    }
}
