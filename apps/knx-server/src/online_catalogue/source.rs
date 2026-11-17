//! The registry of known online catalogue sources and the validated HTTPS base address.

use axum::http::uri::PathAndQuery;
use axum::http::Uri;

/// One catalogue a user can pick in the dropdown.
#[derive(Debug, Clone, Copy)]
pub struct SourceSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub default_base_url: &'static str,
}

/// The Reko catalogue operated by a community host.
pub const REKO: SourceSpec = SourceSpec {
    id: "reko",
    label: "Reko",
    default_base_url: "https://reko.x3muha.de",
};

pub const SOURCES: [SourceSpec; 1] = [REKO];

/// Longest accepted configured address.
const MAX_BASE_URL_BYTES: usize = 256;
/// Longest accepted index-supplied relative download address.
pub const MAX_RELATIVE_URL_BYTES: usize = 2048;

/// An `https` origin plus an optional path prefix: the only thing credentials are ever sent to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseUrl {
    host: String,
    port: u16,
    prefix: String,
}

impl BaseUrl {
    pub fn parse(raw: &str) -> Result<Self, &'static str> {
        let raw = raw.trim();
        if raw.is_empty() || raw.len() > MAX_BASE_URL_BYTES {
            return Err("empty or too long");
        }
        if raw.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err("contains whitespace or control characters");
        }
        let uri: Uri = raw.parse().map_err(|_| "not a valid address")?;
        if uri.scheme_str() != Some("https") {
            return Err("only https addresses are allowed");
        }
        let authority = uri.authority().ok_or("no host")?;
        if authority.as_str().contains('@') {
            return Err("user information is not allowed");
        }
        if uri.query().is_some() || raw.contains('#') || raw.contains('?') {
            return Err("query and fragment are not allowed");
        }
        let host = authority.host().to_ascii_lowercase();
        if host.is_empty() {
            return Err("no host");
        }
        // The typed accessor returns None for an out-of-range explicit port too. Inspect the
        // literal suffix first so an invalid authority never silently becomes the HTTPS default.
        let suffix = if authority.as_str().starts_with('[') {
            authority.as_str().split_once(']').ok_or("invalid host")?.1
        } else {
            authority
                .as_str()
                .strip_prefix(authority.host())
                .ok_or("invalid host")?
        };
        let port = if suffix.is_empty() {
            443
        } else {
            suffix
                .strip_prefix(':')
                .ok_or("invalid port")?
                .parse::<u16>()
                .map_err(|_| "invalid port")?
        };
        if port == 0 {
            return Err("invalid port");
        }
        let path = uri.path().trim_end_matches('/');
        if path.contains("//") || path.contains("/../") || path.ends_with("/..") {
            return Err("invalid path");
        }
        Ok(Self {
            host,
            port,
            prefix: path.to_string(),
        })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// `https://host[:port]`, the scope an account is bound to.
    pub fn origin(&self) -> String {
        if self.port == 443 {
            format!("https://{}", self.host)
        } else {
            format!("https://{}:{}", self.host, self.port)
        }
    }

    /// The canonical text form, origin plus prefix without a trailing slash.
    pub fn canonical(&self) -> String {
        format!("{}{}", self.origin(), self.prefix)
    }

    /// The `Host` header value.
    pub fn host_header(&self) -> String {
        if self.port == 443 {
            self.host.clone()
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    /// Joins a root-relative path (with optional query) onto this base's prefix.
    pub fn join(&self, relative: &str) -> Result<PathAndQuery, &'static str> {
        validate_relative(relative)?;
        format!("{}{}", self.prefix, relative)
            .parse()
            .map_err(|_| "not a valid request path")
    }
}

/// An index-supplied download address must stay on the origin: root-relative, no authority.
pub fn validate_relative(relative: &str) -> Result<(), &'static str> {
    if relative.len() > MAX_RELATIVE_URL_BYTES {
        return Err("download address too long");
    }
    if !relative.starts_with('/') || relative.starts_with("//") || relative.starts_with("/\\") {
        return Err("download address is not root-relative");
    }
    if relative
        .chars()
        .any(|c| c.is_control() || c.is_whitespace() || c == '#' || c == '\\')
    {
        return Err("download address has forbidden characters");
    }
    relative
        .parse::<PathAndQuery>()
        .map(|_| ())
        .map_err(|_| "download address is not a valid path")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_base_parses_to_a_plain_origin() {
        let base = BaseUrl::parse(REKO.default_base_url).unwrap();
        assert_eq!(base.origin(), "https://reko.x3muha.de");
        assert_eq!(base.canonical(), "https://reko.x3muha.de");
        assert_eq!(base.host_header(), "reko.x3muha.de");
    }

    #[test]
    fn non_https_userinfo_query_and_fragment_are_refused() {
        for bad in [
            "http://reko.example",
            "ftp://reko.example",
            "https://user:pw@reko.example",
            "https://reko.example/?a=1",
            "https://reko.example/#frag",
            "https://reko.example/a b",
            "reko.example",
            "",
            "https://reko.example:0",
            "https://reko.example:65536",
            "https://reko.example:abc",
        ] {
            assert!(BaseUrl::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn port_and_prefix_are_kept_and_trailing_slash_is_dropped() {
        let base = BaseUrl::parse("https://Reko.Example:8443/catalog/").unwrap();
        assert_eq!(base.canonical(), "https://reko.example:8443/catalog");
        assert_eq!(base.host_header(), "reko.example:8443");
        let joined = base.join("/api/download?path=a%2Fb.vd5").unwrap();
        assert_eq!(joined.as_str(), "/catalog/api/download?path=a%2Fb.vd5");
    }

    #[test]
    fn relative_addresses_cannot_name_another_authority() {
        let base = BaseUrl::parse("https://reko.example").unwrap();
        for bad in [
            "https://evil.example/x",
            "//evil.example/x",
            "/\\evil.example",
            "api/download",
            "/a b",
            "/a#b",
            "/a\nb",
        ] {
            assert!(base.join(bad).is_err(), "{bad:?}");
        }
        assert!(base
            .join("/api/download?path=Hersteller%2F0001_Siemens%2F01.vd2")
            .is_ok());
    }
}
