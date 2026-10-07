//! The certificate behind the server's own TLS listener (ADR-0088).
//!
//! Two sources and one rule between them. A deployer who names PEM files
//! gets exactly those files or a server that refuses to start; a deployer
//! who names nothing gets a self-signed certificate this module generates
//! once, keeps under the data directory and reuses until it is close to
//! expiring or no longer names what it was asked to name.
//!
//! Everything here takes `now` as a parameter rather than reading the
//! clock, so the renewal and expiry rules are pure and testable at any
//! date — including the ones two years from now that no test could
//! otherwise wait for.

use std::fmt::Write as _;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rcgen::{
    CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose, SanType,
};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};

/// The directory under `KNX_DATA_DIR` holding the generated certificate.
/// The `/api/fs/*` and `/api/project/*` path resolvers refuse it
/// (`paths.rs`), so no route can list, read or overwrite the key.
pub const TLS_DIR_NAME: &str = ".knxbench-tls";
const CERT_FILE: &str = "cert.pem";
const KEY_FILE: &str = "key.pem";
/// The subject names the stored certificate was generated for, one per
/// line. Kept beside it so a changed `KNX_TLS_SAN` is noticed without
/// parsing the certificate's extensions.
const NAMES_FILE: &str = "names.txt";

/// Apple's ceiling for a TLS server certificate's validity, which applies
/// to certificates from user-added roots too (support.apple.com/103769).
/// Longer would be convenient and would stop working on every Mac,
/// iPhone and iPad the moment someone chose to trust the certificate.
pub const SELF_SIGNED_VALIDITY_DAYS: i64 = 825;
/// A generated certificate closer than this to expiring is replaced at
/// startup; a provided one this close earns a warning.
pub const RENEW_WITHIN_DAYS: i64 = 30;
/// `notBefore` is set this far in the past so a client whose clock runs a
/// little behind the server's does not see a certificate from the future.
const BACKDATE: Duration = Duration::hours(1);
const SELF_SIGNED_COMMON_NAME: &str = "KNXBench knx-server (self-signed)";
/// RFC 1035 §2.3.4 limits, applied to `KNX_TLS_SAN` entries.
const MAX_DNS_NAME_LEN: usize = 253;
const MAX_DNS_LABEL_LEN: usize = 63;

/// One name a certificate is valid for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubjectName {
    Dns(String),
    Ip(IpAddr),
}

impl SubjectName {
    /// An IP address if it parses as one, otherwise a DNS name if it is a
    /// syntactically valid one, otherwise an error naming the entry.
    pub fn parse(raw: &str) -> Result<Self, String> {
        let raw = raw.trim();
        if let Ok(ip) = raw.parse::<IpAddr>() {
            return Ok(SubjectName::Ip(ip));
        }
        if is_dns_name(raw) {
            return Ok(SubjectName::Dns(raw.to_ascii_lowercase()));
        }
        Err(format!(
            "`{raw}` is neither an IP address nor a valid DNS name"
        ))
    }
}

impl std::fmt::Display for SubjectName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubjectName::Dns(name) => f.write_str(name),
            SubjectName::Ip(ip) => write!(f, "{ip}"),
        }
    }
}

/// Letters, digits and hyphens in dot-separated labels of 1–63
/// characters, no label starting or ending with a hyphen. No wildcards:
/// a self-signed certificate for `*.lan` is a certificate for every
/// device on the network.
fn is_dns_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_DNS_NAME_LEN
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= MAX_DNS_LABEL_LEN
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

/// The names a generated certificate covers: loopback in both address
/// families and by name, the machine's own host name when it has a usable
/// one, and whatever `KNX_TLS_SAN` adds. Duplicates are dropped, first
/// occurrence wins, so the order is stable and the stored list compares
/// equal across restarts.
pub fn self_signed_names(
    hostname: Option<&str>,
    extra: Option<&str>,
) -> Result<Vec<SubjectName>, String> {
    let mut names = vec![
        SubjectName::Dns("localhost".to_string()),
        SubjectName::Ip(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        SubjectName::Ip(IpAddr::V6(Ipv6Addr::LOCALHOST)),
    ];
    // The host name is a convenience, not configuration: one that is not a
    // valid DNS name (a container ID is fine; an empty file is not) is left
    // out rather than turned into a startup failure nobody asked for.
    if let Some(Ok(name @ SubjectName::Dns(_))) = hostname.map(SubjectName::parse) {
        names.push(name);
    }
    for entry in extra.unwrap_or("").split(',') {
        if entry.trim().is_empty() {
            continue;
        }
        names.push(SubjectName::parse(entry).map_err(|e| format!("KNX_TLS_SAN: {e}"))?);
    }
    let mut unique = Vec::with_capacity(names.len());
    for name in names {
        if !unique.contains(&name) {
            unique.push(name);
        }
    }
    Ok(unique)
}

/// A certificate chain and key, already accepted by rustls as a matching
/// pair, plus what the startup banner says about it.
pub struct ServerIdentity {
    pub config: Arc<ServerConfig>,
    /// SHA-256 over the end-entity certificate's DER, as colon-separated
    /// upper-case hex — the form browsers show in their certificate viewer.
    pub fingerprint: String,
    pub not_before: OffsetDateTime,
    pub not_after: OffsetDateTime,
}

/// Loads a deployer's PEM files. Every failure is an error: someone who
/// named these files wants these files, and quietly serving a generated
/// certificate instead would hide the mistake behind a browser warning.
/// An expired or not-yet-valid certificate is *not* a failure here; see
/// [`validity_notice`].
pub fn load_provided(cert_path: &Path, key_path: &Path) -> Result<ServerIdentity, String> {
    let chain = CertificateDer::pem_file_iter(cert_path)
        .map_err(|e| format!("cannot read {}: {e}", cert_path.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("{} is not a PEM certificate file: {e}", cert_path.display()))?;
    if chain.is_empty() {
        return Err(format!("{} contains no certificate", cert_path.display()));
    }
    let key = PrivateKeyDer::from_pem_file(key_path).map_err(|e| {
        format!(
            "{} does not hold a usable PEM private key: {e}",
            key_path.display()
        )
    })?;
    identity(chain, key).map_err(|e| {
        format!(
            "{} and {} cannot be used together: {e}",
            cert_path.display(),
            key_path.display()
        )
    })
}

/// What is wrong with a certificate's validity window at `now`, if
/// anything. A warning rather than a refusal: a server that will not start
/// after an unattended expiry is a worse failure than a browser that
/// complains, and the browser will complain regardless.
pub fn validity_notice(identity: &ServerIdentity, now: OffsetDateTime) -> Option<String> {
    if identity.not_after <= now {
        return Some(format!(
            "THE TLS CERTIFICATE EXPIRED on {}. Starting anyway; browsers will refuse \
             or warn about it until KNX_TLS_CERT/KNX_TLS_KEY name a current one.",
            utc(identity.not_after)
        ));
    }
    if identity.not_before > now {
        return Some(format!(
            "The TLS certificate is not valid until {}. Starting anyway; check this \
             machine's clock and the certificate's dates.",
            utc(identity.not_before)
        ));
    }
    let left = identity.not_after - now;
    if left < Duration::days(RENEW_WITHIN_DAYS) {
        return Some(format!(
            "The TLS certificate expires on {}, in {} day(s). Replace KNX_TLS_CERT/KNX_TLS_KEY \
             before then.",
            utc(identity.not_after),
            left.whole_days()
        ));
    }
    None
}

/// The generated certificate under `dir`, reused if it is still good and
/// regenerated if it is not. Returns the identity and, when it had to make
/// a new one, the reason — for the startup banner, because a fingerprint
/// that changed without explanation looks exactly like an attack.
pub fn ensure_self_signed(
    dir: &Path,
    names: &[SubjectName],
    now: OffsetDateTime,
) -> Result<(ServerIdentity, Option<String>), String> {
    create_private_dir(dir)?;
    let reason = match load_stored(dir, names, now) {
        Ok(identity) => return Ok((identity, None)),
        Err(reason) => reason,
    };
    let (cert_pem, key_pem) = generate_self_signed(names, now)?;
    // Key first, names last: a crash between two writes leaves a set that
    // fails to load or to match, which the next start regenerates.
    write_private(dir, KEY_FILE, key_pem.as_bytes())?;
    write_private(dir, CERT_FILE, cert_pem.as_bytes())?;
    write_private(dir, NAMES_FILE, names_text(names).as_bytes())?;
    let identity = load_stored(dir, names, now).map_err(|e| {
        format!(
            "the certificate just generated in {} does not load: {e}",
            dir.display()
        )
    })?;
    Ok((identity, Some(reason)))
}

/// Why the stored certificate cannot be reused, or the identity if it can.
fn load_stored(
    dir: &Path,
    names: &[SubjectName],
    now: OffsetDateTime,
) -> Result<ServerIdentity, String> {
    let cert_path = dir.join(CERT_FILE);
    if !cert_path.exists() {
        return Err("first start, no certificate yet".to_string());
    }
    let stored_names = std::fs::read_to_string(dir.join(NAMES_FILE))
        .map_err(|_| "its list of names is missing".to_string())?;
    if stored_names != names_text(names) {
        return Err("the names it should cover changed (KNX_TLS_SAN or host name)".to_string());
    }
    let identity = load_provided(&cert_path, &dir.join(KEY_FILE))
        .map_err(|e| format!("the stored one is unusable: {e}"))?;
    if identity.not_after - now < Duration::days(RENEW_WITHIN_DAYS) {
        return Err(format!(
            "the stored one expires on {}",
            utc(identity.not_after)
        ));
    }
    Ok(identity)
}

fn names_text(names: &[SubjectName]) -> String {
    names.iter().fold(String::new(), |mut text, name| {
        let _ = writeln!(text, "{name}");
        text
    })
}

/// A self-signed P-256 certificate that meets Apple's requirements for a
/// trusted TLS server certificate, since a deployer may choose to trust
/// this one: names in the SAN extension (not the common name), the
/// `serverAuth` extended key usage, SHA-2, and at most 825 days.
pub(crate) fn generate_self_signed(
    names: &[SubjectName],
    now: OffsetDateTime,
) -> Result<(String, String), String> {
    let mut params = CertificateParams::default();
    let mut subject = DistinguishedName::new();
    subject.push(DnType::CommonName, SELF_SIGNED_COMMON_NAME);
    params.distinguished_name = subject;
    params.subject_alt_names = names
        .iter()
        .map(|name| match name {
            SubjectName::Dns(dns) => dns
                .as_str()
                .try_into()
                .map(SanType::DnsName)
                .map_err(|e| format!("`{dns}` cannot be a certificate name: {e}")),
            SubjectName::Ip(ip) => Ok(SanType::IpAddress(*ip)),
        })
        .collect::<Result<_, _>>()?;
    params.not_before = now - BACKDATE;
    params.not_after = params.not_before + Duration::days(SELF_SIGNED_VALIDITY_DAYS);
    params.is_ca = IsCa::ExplicitNoCa;
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let key = KeyPair::generate().map_err(|e| format!("cannot generate a TLS key: {e}"))?;
    let cert = params
        .self_signed(&key)
        .map_err(|e| format!("cannot sign the TLS certificate: {e}"))?;
    Ok((cert.pem(), key.serialize_pem()))
}

fn identity(
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<ServerIdentity, String> {
    let leaf = chain.first().ok_or("no certificate")?;
    let (not_before, not_after) = validity(leaf)?;
    let fingerprint = fingerprint(leaf);
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    // `with_single_cert` rejects a key that does not belong to the
    // certificate (rustls' `CertifiedKey::from_der` → `keys_match`).
    let mut config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .map_err(|e| e.to_string())?;
    // axum is built without its `http2` feature; offering h2 would be a
    // promise the connection handler cannot keep.
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(ServerIdentity {
        config: Arc::new(config),
        fingerprint,
        not_before,
        not_after,
    })
}

/// `notBefore` and `notAfter` from an X.509 certificate (RFC 5280 §4.1):
/// `Certificate ::= SEQUENCE { tbsCertificate, signatureAlgorithm,
/// signatureValue }`, and inside `tbsCertificate` an optional `[0]`
/// version, then serial number, signature algorithm, issuer, and the
/// validity sequence. Everything after it is skipped unread.
pub(crate) fn validity(der: &[u8]) -> Result<(OffsetDateTime, OffsetDateTime), String> {
    yasna::parse_der(der, |reader| {
        reader.read_sequence(|certificate| {
            let window = certificate.next().read_sequence(|tbs| {
                tbs.read_optional(|r| r.read_tagged(yasna::Tag::context(0), |r| r.read_der()))?;
                for _ in 0..3 {
                    tbs.next().read_der()?; // serialNumber, signature, issuer
                }
                let window = tbs.next().read_sequence(|validity| {
                    let not_before = read_time(validity.next())?;
                    let not_after = read_time(validity.next())?;
                    Ok((not_before, not_after))
                })?;
                while tbs.read_optional(|r| r.read_der())?.is_some() {}
                Ok(window)
            })?;
            while certificate.read_optional(|r| r.read_der())?.is_some() {}
            Ok(window)
        })
    })
    .map_err(|e| format!("not a parseable X.509 certificate ({e})"))
}

/// RFC 5280 §4.1.2.5: `Time ::= CHOICE { utcTime, generalTime }`.
fn read_time(reader: yasna::BERReader<'_, '_>) -> yasna::ASN1Result<OffsetDateTime> {
    if reader.lookahead_tag()? == yasna::tags::TAG_UTCTIME {
        Ok(*reader.read_utctime()?.datetime())
    } else {
        Ok(*reader.read_generalized_time()?.datetime())
    }
}

pub(crate) fn fingerprint(der: &[u8]) -> String {
    let digest = Sha256::digest(der);
    let mut text = String::with_capacity(digest.len() * 3);
    for (i, byte) in digest.iter().enumerate() {
        if i > 0 {
            text.push(':');
        }
        let _ = write!(text, "{byte:02X}");
    }
    text
}

/// `YYYY-MM-DD HH:MM UTC`, for notices.
pub(crate) fn utc(when: OffsetDateTime) -> String {
    chrono::DateTime::from_timestamp(when.unix_timestamp(), 0)
        .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| format!("{} (unix time)", when.unix_timestamp()))
}

/// The directory, created if absent and restricted to its owner.
fn create_private_dir(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("cannot restrict {}: {e}", dir.display()))?;
    }
    Ok(())
}

/// Writes `name` atomically: a temporary file in the same directory
/// (created `0600` by `tempfile`), then a rename over the old one. A
/// reader never sees half a key.
fn write_private(dir: &Path, name: &str, contents: &[u8]) -> Result<PathBuf, String> {
    use std::io::Write as _;
    let target = dir.join(name);
    let mut file = tempfile::NamedTempFile::new_in(dir)
        .map_err(|e| format!("cannot write in {}: {e}", dir.display()))?;
    file.write_all(contents)
        .and_then(|()| file.as_file().sync_all())
        .map_err(|e| format!("cannot write {}: {e}", target.display()))?;
    file.persist(&target)
        .map_err(|e| format!("cannot replace {}: {e}", target.display()))?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> OffsetDateTime {
        // 2026-10-07 12:00 UTC; any fixed instant would do.
        OffsetDateTime::from_unix_timestamp(1_791_374_400).unwrap()
    }

    fn names() -> Vec<SubjectName> {
        self_signed_names(Some("knx-box"), None).unwrap()
    }

    #[test]
    fn default_names_cover_loopback_and_the_host_name() {
        let shown: Vec<String> = names().iter().map(ToString::to_string).collect();
        assert_eq!(shown, ["localhost", "127.0.0.1", "::1", "knx-box"]);
    }

    #[test]
    fn extra_names_are_parsed_deduplicated_and_validated() {
        let names = self_signed_names(
            Some("Knx-Box"),
            Some(" knx.lan, 192.168.1.10,,knx-box ,::1"),
        )
        .unwrap();
        let shown: Vec<String> = names.iter().map(ToString::to_string).collect();
        assert_eq!(
            shown,
            [
                "localhost",
                "127.0.0.1",
                "::1",
                "knx-box",
                "knx.lan",
                "192.168.1.10"
            ]
        );
        for bad in ["*.lan", "-x.lan", "a..b", "knx lan", "ünïcode.lan"] {
            let error = self_signed_names(None, Some(bad)).unwrap_err();
            assert!(error.contains("KNX_TLS_SAN"), "{bad}: {error}");
        }
    }

    #[test]
    fn an_unusable_host_name_is_left_out_not_fatal() {
        let shown: Vec<String> = self_signed_names(Some("not a name"), None)
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(shown, ["localhost", "127.0.0.1", "::1"]);
    }

    #[test]
    fn the_validity_window_is_read_back_exactly_and_respects_apples_ceiling() {
        let (cert_pem, _) = generate_self_signed(&names(), now()).unwrap();
        let der = CertificateDer::from_pem_slice(cert_pem.as_bytes()).unwrap();
        let (not_before, not_after) = validity(&der).unwrap();
        assert_eq!(not_before, now() - BACKDATE);
        assert_eq!(
            not_after - not_before,
            Duration::days(SELF_SIGNED_VALIDITY_DAYS)
        );
    }

    #[test]
    fn garbage_is_not_a_certificate() {
        assert!(validity(b"\x30\x03\x02\x01\x01").is_err());
        assert!(validity(b"").is_err());
    }

    #[test]
    fn the_fingerprint_is_colon_separated_upper_case_sha256() {
        let fp = fingerprint(b"abc");
        assert_eq!(fp.len(), 32 * 3 - 1);
        assert!(fp.starts_with("BA:78:16:BF"), "{fp}");
    }

    #[test]
    fn first_start_generates_and_the_second_reuses() {
        let dir = tempfile::tempdir().unwrap();
        let tls = dir.path().join(TLS_DIR_NAME);
        let (first, reason) = ensure_self_signed(&tls, &names(), now()).unwrap();
        assert!(reason.unwrap().contains("first start"));
        let (second, reason) = ensure_self_signed(&tls, &names(), now()).unwrap();
        assert_eq!(reason, None);
        assert_eq!(first.fingerprint, second.fingerprint);
    }

    #[cfg(unix)]
    #[test]
    fn the_key_and_its_directory_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let tls = dir.path().join(TLS_DIR_NAME);
        ensure_self_signed(&tls, &names(), now()).unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&tls), 0o700);
        assert_eq!(mode(&tls.join(KEY_FILE)), 0o600);
    }

    #[test]
    fn changed_names_regenerate() {
        let dir = tempfile::tempdir().unwrap();
        let tls = dir.path().join(TLS_DIR_NAME);
        let (first, _) = ensure_self_signed(&tls, &names(), now()).unwrap();
        let wider = self_signed_names(Some("knx-box"), Some("knx.lan")).unwrap();
        let (second, reason) = ensure_self_signed(&tls, &wider, now()).unwrap();
        assert!(reason.unwrap().contains("names"));
        assert_ne!(first.fingerprint, second.fingerprint);
    }

    #[test]
    fn a_certificate_near_expiry_is_renewed_at_startup() {
        let dir = tempfile::tempdir().unwrap();
        let tls = dir.path().join(TLS_DIR_NAME);
        let made = now() - Duration::days(SELF_SIGNED_VALIDITY_DAYS - RENEW_WITHIN_DAYS + 1);
        let (old, _) = ensure_self_signed(&tls, &names(), made).unwrap();
        let (new, reason) = ensure_self_signed(&tls, &names(), now()).unwrap();
        assert!(reason.unwrap().contains("expires on"));
        assert_ne!(old.fingerprint, new.fingerprint);
        assert!(new.not_after - now() > Duration::days(SELF_SIGNED_VALIDITY_DAYS - 1));
    }

    #[test]
    fn a_damaged_stored_key_is_regenerated_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let tls = dir.path().join(TLS_DIR_NAME);
        ensure_self_signed(&tls, &names(), now()).unwrap();
        std::fs::write(tls.join(KEY_FILE), b"not a key").unwrap();
        let (_, reason) = ensure_self_signed(&tls, &names(), now()).unwrap();
        assert!(reason.unwrap().contains("unusable"));
    }

    fn write_pair(dir: &Path, made: OffsetDateTime) -> (PathBuf, PathBuf) {
        let (cert, key) = generate_self_signed(&names(), made).unwrap();
        let cert_path = dir.join("server.crt");
        let key_path = dir.join("server.key");
        std::fs::write(&cert_path, cert).unwrap();
        std::fs::write(&key_path, key).unwrap();
        (cert_path, key_path)
    }

    #[test]
    fn provided_files_load_and_report_no_problem_while_current() {
        let dir = tempfile::tempdir().unwrap();
        let (cert, key) = write_pair(dir.path(), now());
        let identity = load_provided(&cert, &key).unwrap();
        assert_eq!(validity_notice(&identity, now()), None);
    }

    #[test]
    fn provided_files_that_do_not_belong_together_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let (cert, _) = write_pair(dir.path(), now());
        let other = tempfile::tempdir().unwrap();
        let (_, foreign_key) = write_pair(other.path(), now());
        let error = load_provided(&cert, &foreign_key).err().unwrap();
        assert!(error.contains("cannot be used together"), "{error}");
    }

    #[test]
    fn broken_or_missing_provided_files_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let (cert, key) = write_pair(dir.path(), now());
        let junk = dir.path().join("junk.pem");
        std::fs::write(&junk, b"hello").unwrap();
        let missing = dir.path().join("missing.pem");
        for (c, k, expect) in [
            (&junk, &key, "contains no certificate"),
            (&cert, &junk, "usable PEM private key"),
            (&missing, &key, "cannot read"),
            (&cert, &missing, "usable PEM private key"),
        ] {
            let error = load_provided(c, k).err().unwrap();
            assert!(error.contains(expect), "{expect}: {error}");
        }
    }

    #[test]
    fn an_expired_provided_certificate_loads_with_a_loud_notice() {
        let dir = tempfile::tempdir().unwrap();
        let (cert, key) = write_pair(dir.path(), now() - Duration::days(900));
        let identity = load_provided(&cert, &key).unwrap();
        let notice = validity_notice(&identity, now()).unwrap();
        assert!(notice.contains("EXPIRED"), "{notice}");
    }

    #[test]
    fn a_provided_certificate_close_to_expiry_is_announced() {
        let dir = tempfile::tempdir().unwrap();
        let (cert, key) = write_pair(
            dir.path(),
            now() - Duration::days(SELF_SIGNED_VALIDITY_DAYS - 5),
        );
        let identity = load_provided(&cert, &key).unwrap();
        assert!(validity_notice(&identity, now())
            .unwrap()
            .contains("in 4 day(s)"));
        let future = load_provided(&cert, &key).unwrap();
        let early = future.not_before - Duration::days(1);
        assert!(validity_notice(&future, early)
            .unwrap()
            .contains("not valid until"));
    }
}
