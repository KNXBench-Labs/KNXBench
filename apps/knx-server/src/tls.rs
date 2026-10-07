//! Whether `knx-server` speaks HTTPS, and with which certificate (ADR-0088).
//!
//! `main.rs` reads the environment and the host name and hands them here;
//! [`resolve_tls`] decides without touching the filesystem, and
//! [`prepare_tls`] carries the decision out. The split keeps every rule a
//! deployer can trip over — which variables conflict, when TLS switches
//! itself on — a pure function with tests, the same way ADR-0026 kept
//! `bind_address` pure.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustls::ServerConfig;
use time::OffsetDateTime;

use crate::tls_cert::{
    ensure_self_signed, load_provided, self_signed_names, utc, validity_notice, SubjectName,
    TLS_DIR_NAME,
};

/// `KNX_TLS`, read as three states rather than a flag: the default is not
/// "off" but "decide from whether this server faces the network".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsSwitch {
    Auto,
    On,
    Off,
}

impl TlsSwitch {
    /// Unset, empty or `auto` mean [`TlsSwitch::Auto`]; the usual spellings
    /// of "no" mean [`TlsSwitch::Off`]; anything else means
    /// [`TlsSwitch::On`]. An unrecognised value therefore errs toward
    /// encryption — a typo should not be what turns TLS off.
    pub fn from_env_value(value: Option<&str>) -> Self {
        match value.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
            None | Some("" | "auto") => TlsSwitch::Auto,
            Some("0" | "false" | "no" | "off") => TlsSwitch::Off,
            Some(_) => TlsSwitch::On,
        }
    }
}

/// Everything [`resolve_tls`] needs, as `main.rs` found it.
#[derive(Debug, Clone, Copy)]
pub struct TlsInputs<'a> {
    pub switch: TlsSwitch,
    /// `KNX_TLS_CERT` and `KNX_TLS_KEY`, empty strings already dropped.
    pub cert: Option<&'a str>,
    pub key: Option<&'a str>,
    /// `KNX_TLS_SAN`: extra comma-separated names for the generated
    /// certificate.
    pub extra_names: Option<&'a str>,
    /// The machine's host name, if it could be read.
    pub hostname: Option<&'a str>,
    /// Whether a password is configured, i.e. whether ADR-0026 lets the
    /// server bind `0.0.0.0`.
    pub auth_required: bool,
    pub data_dir: &'a Path,
}

/// What the server will listen with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlsPlan {
    /// Plain HTTP.
    Plain,
    /// A certificate generated and kept under `dir`.
    SelfSigned {
        dir: PathBuf,
        names: Vec<SubjectName>,
    },
    /// The deployer's own PEM files.
    Provided { cert: PathBuf, key: PathBuf },
}

/// A plan and the banner lines explaining it.
#[derive(Debug)]
pub struct TlsDecision {
    pub plan: TlsPlan,
    pub notices: Vec<String>,
}

/// Decides how the server listens.
///
/// * With `KNX_TLS` unset, TLS is on exactly when the server faces the
///   network (a password is set, so it binds `0.0.0.0`) or certificate
///   files were named. A loopback-only server stays on plain HTTP:
///   browsers already treat `localhost` as a secure context, and a
///   certificate warning there would protect nothing.
/// * `KNX_TLS=on` forces TLS, loopback included.
/// * `KNX_TLS=off` forces plain HTTP, and says so loudly when the server is
///   on the network.
///
/// Contradictions are errors, not guesses: half a certificate pair, or
/// certificate files together with `KNX_TLS=off`.
pub fn resolve_tls(inputs: TlsInputs<'_>) -> Result<TlsDecision, String> {
    let files = match (inputs.cert, inputs.key) {
        (Some(cert), Some(key)) => Some((PathBuf::from(cert), PathBuf::from(key))),
        (None, None) => None,
        _ => {
            return Err("KNX_TLS_CERT and KNX_TLS_KEY must be set together, or neither".to_string())
        }
    };
    let mut notices = Vec::new();
    let https = match inputs.switch {
        TlsSwitch::On => true,
        TlsSwitch::Auto => inputs.auth_required || files.is_some(),
        TlsSwitch::Off => {
            if files.is_some() {
                return Err(
                    "KNX_TLS=off contradicts KNX_TLS_CERT/KNX_TLS_KEY; unset one or the other"
                        .to_string(),
                );
            }
            if inputs.auth_required {
                notices.push(
                    "TLS IS SWITCHED OFF (KNX_TLS=off) on a server that listens on the network. \
                     The password and the session cookie cross it in the clear; anything \
                     between the browser and this server can read and replay them. Only do \
                     this behind a TLS-terminating proxy."
                        .to_string(),
                );
            }
            false
        }
    };
    if !https {
        if inputs.extra_names.is_some() {
            notices.push("KNX_TLS_SAN is set but TLS is off; it is ignored.".to_string());
        }
        return Ok(TlsDecision {
            plan: TlsPlan::Plain,
            notices,
        });
    }
    let plan = match files {
        Some((cert, key)) => {
            if inputs.extra_names.is_some() {
                notices.push(
                    "KNX_TLS_SAN is ignored: it only shapes the generated certificate, and \
                     KNX_TLS_CERT names your own."
                        .to_string(),
                );
            }
            TlsPlan::Provided { cert, key }
        }
        None => TlsPlan::SelfSigned {
            dir: inputs.data_dir.join(TLS_DIR_NAME),
            names: self_signed_names(inputs.hostname, inputs.extra_names)?,
        },
    };
    Ok(TlsDecision { plan, notices })
}

/// A ready server configuration and the banner lines that go with it.
pub struct PreparedTls {
    pub config: Arc<ServerConfig>,
    pub notices: Vec<String>,
}

/// Loads or generates the certificate a plan names. `None` for
/// [`TlsPlan::Plain`]; an error when provided files are unusable or the
/// generated ones cannot be written.
pub fn prepare_tls(plan: &TlsPlan, now: OffsetDateTime) -> Result<Option<PreparedTls>, String> {
    match plan {
        TlsPlan::Plain => Ok(None),
        TlsPlan::Provided { cert, key } => {
            let identity = load_provided(cert, key)?;
            let mut notices = vec![format!(
                "HTTPS with the certificate from {} (SHA-256 {}, valid until {}).",
                cert.display(),
                identity.fingerprint,
                utc(identity.not_after)
            )];
            notices.extend(validity_notice(&identity, now));
            Ok(Some(PreparedTls {
                config: identity.config,
                notices,
            }))
        }
        TlsPlan::SelfSigned { dir, names } => {
            let (identity, regenerated) = ensure_self_signed(dir, names, now)?;
            let mut notices = Vec::new();
            if let Some(reason) = regenerated {
                notices.push(format!(
                    "Generated a new self-signed TLS certificate in {} ({reason}). Browsers \
                     will ask once more before trusting it.",
                    dir.display()
                ));
            }
            let covered: Vec<String> = names.iter().map(ToString::to_string).collect();
            notices.push(format!(
                "HTTPS with a self-signed certificate for {}, valid until {}. Its SHA-256 \
                 fingerprint is {}; compare it with the one your browser shows before \
                 accepting the warning. Set KNX_TLS_CERT/KNX_TLS_KEY to use your own.",
                covered.join(", "),
                utc(identity.not_after),
                identity.fingerprint
            ));
            Ok(Some(PreparedTls {
                config: identity.config,
                notices,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(switch: TlsSwitch, auth_required: bool) -> TlsInputs<'static> {
        TlsInputs {
            switch,
            cert: None,
            key: None,
            extra_names: None,
            hostname: None,
            auth_required,
            data_dir: Path::new("/data"),
        }
    }

    #[test]
    fn the_switch_reads_three_ways_and_a_typo_means_on() {
        use TlsSwitch::*;
        for (value, expected) in [
            (None, Auto),
            (Some(""), Auto),
            (Some(" AUTO "), Auto),
            (Some("off"), Off),
            (Some("0"), Off),
            (Some("False"), Off),
            (Some("on"), On),
            (Some("1"), On),
            (Some("of"), On),
        ] {
            assert_eq!(TlsSwitch::from_env_value(value), expected, "{value:?}");
        }
    }

    #[test]
    fn auto_encrypts_exactly_when_the_server_faces_the_network() {
        let network = resolve_tls(inputs(TlsSwitch::Auto, true)).unwrap();
        assert!(matches!(&network.plan, TlsPlan::SelfSigned { dir, .. }
            if dir == Path::new("/data/.knxbench-tls")));
        assert!(network.notices.is_empty());
        let loopback = resolve_tls(inputs(TlsSwitch::Auto, false)).unwrap();
        assert_eq!(loopback.plan, TlsPlan::Plain);
        assert!(loopback.notices.is_empty());
    }

    #[test]
    fn on_encrypts_loopback_too() {
        let decision = resolve_tls(inputs(TlsSwitch::On, false)).unwrap();
        assert!(matches!(decision.plan, TlsPlan::SelfSigned { .. }));
    }

    #[test]
    fn off_on_the_network_is_allowed_and_shouted_about() {
        let decision = resolve_tls(inputs(TlsSwitch::Off, true)).unwrap();
        assert_eq!(decision.plan, TlsPlan::Plain);
        assert!(decision.notices[0].contains("TLS IS SWITCHED OFF"));
        let quiet = resolve_tls(inputs(TlsSwitch::Off, false)).unwrap();
        assert!(quiet.notices.is_empty());
    }

    #[test]
    fn named_files_switch_auto_on_and_win_over_generation() {
        let mut with_files = inputs(TlsSwitch::Auto, false);
        with_files.cert = Some("/etc/knx/cert.pem");
        with_files.key = Some("/etc/knx/key.pem");
        with_files.extra_names = Some("knx.lan");
        let decision = resolve_tls(with_files).unwrap();
        assert_eq!(
            decision.plan,
            TlsPlan::Provided {
                cert: "/etc/knx/cert.pem".into(),
                key: "/etc/knx/key.pem".into()
            }
        );
        assert!(decision.notices[0].contains("KNX_TLS_SAN is ignored"));
    }

    #[test]
    fn contradictions_refuse_to_start() {
        let mut half = inputs(TlsSwitch::Auto, true);
        half.cert = Some("/etc/knx/cert.pem");
        assert!(resolve_tls(half).unwrap_err().contains("together"));

        let mut off_with_files = inputs(TlsSwitch::Off, true);
        off_with_files.cert = Some("c");
        off_with_files.key = Some("k");
        assert!(resolve_tls(off_with_files)
            .unwrap_err()
            .contains("contradicts"));

        let mut bad_name = inputs(TlsSwitch::Auto, true);
        bad_name.extra_names = Some("*.lan");
        assert!(resolve_tls(bad_name).unwrap_err().contains("KNX_TLS_SAN"));
    }

    #[test]
    fn plain_prepares_nothing_and_self_signed_prepares_a_fingerprinted_banner() {
        assert!(prepare_tls(&TlsPlan::Plain, OffsetDateTime::now_utc())
            .unwrap()
            .is_none());
        let data = tempfile::tempdir().unwrap();
        let mut network = inputs(TlsSwitch::Auto, true);
        network.data_dir = data.path();
        let plan = resolve_tls(network).unwrap().plan;
        let prepared = prepare_tls(&plan, OffsetDateTime::now_utc())
            .unwrap()
            .unwrap();
        assert!(prepared.notices[0].contains("Generated a new self-signed"));
        assert!(prepared.notices[1].contains("SHA-256 fingerprint"));
    }
}
