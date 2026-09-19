//! Builds the local debug-report bundle and redacts addresses, paths and hostnames out of it.
//!
//! T29. Two things live here, and nothing else:
//!
//! 1. A redaction pass that works **by pattern class**, never by a list of
//!    known values — any IPv4 dotted quad, any IPv6 literal, the user's home
//!    directory prefix, the machine's hostname. A sweep for one remembered
//!    literal is not a sweep: RFC 1918 ranges are exactly where a real
//!    gateway address hides, and the only honest way to catch them is to
//!    catch the shape.
//! 2. The bundle itself: which files it holds for a given set of opt-ins,
//!    what each one contains, and how it becomes a zip on disk.
//!
//! Redaction covers `report.md`, `environment.json` and `log.json` — and
//! covers the four pattern classes above, not KNX addresses: `log.json`
//! records import conflicts by group address and by element name, which is
//! the only thing that makes a conflict diagnosable, and the dialog says so
//! rather than implying the file is anonymous. It does
//! **not** cover `bus-telegrams.json` — the KNX individual and group
//! addresses in that file are the entire debugging content of it, which is
//! precisely why it is opt-in, off by default, and labelled in the dialog as
//! carrying the addresses of real devices. Half-redacting a file and letting
//! the user believe it is clean would be worse than not offering it.
//!
//! Nothing in this module sends anything anywhere. There is no upload, no
//! telemetry and no server of ours to receive one; the zip is written to a
//! path the user picked, and that is the end of it.

use std::io::Write;
use std::path::Path;

use serde_json::Value;

/// What an IPv4 dotted quad is replaced with.
pub(crate) const IPV4_PLACEHOLDER: &str = "[redacted-ipv4]";
/// What an IPv6 literal is replaced with.
pub(crate) const IPV6_PLACEHOLDER: &str = "[redacted-ipv6]";
/// What the machine's own hostname is replaced with.
pub(crate) const HOST_PLACEHOLDER: &str = "[redacted-host]";

pub(crate) const REPORT_MD: &str = "report.md";
pub(crate) const ENVIRONMENT_JSON: &str = "environment.json";
pub(crate) const LOG_JSON: &str = "log.json";
pub(crate) const PROJECT_SUMMARY_JSON: &str = "project-summary.json";
pub(crate) const BUS_TELEGRAMS_JSON: &str = "bus-telegrams.json";

// ---------------------------------------------------------------------
// Redaction
// ---------------------------------------------------------------------

/// The redaction pass, with the two machine-specific strings it cannot
/// derive from a pattern (home directory, hostname) supplied explicitly so
/// tests can pin the behaviour without depending on the machine they run on.
pub(crate) struct Redactor {
    home: Option<String>,
    hostname: Option<String>,
}

impl Redactor {
    pub(crate) fn new(home: Option<String>, hostname: Option<String>) -> Self {
        Self {
            // A one-character home (`/`) would turn every path separator in
            // the bundle into a tilde, which redacts nothing and destroys
            // everything.
            home: home.filter(|h| h.len() > 1),
            hostname: hostname.filter(|h| h.chars().count() > 1),
        }
    }

    /// Reads the home directory and hostname from the environment this
    /// process is actually running in.
    pub(crate) fn from_environment() -> Self {
        Self::new(home_dir(), hostname())
    }

    /// `text` with every pattern class above replaced. Order matters twice:
    /// the home prefix goes first so a hostname or address *inside* a path
    /// is still seen afterwards, and IPv4 goes before IPv6 so that an
    /// IPv4-mapped literal (`::ffff:192.0.2.1`) loses its dotted quad rather
    /// than being left whole by an IPv6 parser that rejects the mapped form.
    pub(crate) fn apply(&self, text: &str) -> String {
        let mut out = text.to_string();
        if let Some(home) = &self.home {
            out = replace_home(&out, home);
        }
        if let Some(hostname) = &self.hostname {
            out = replace_hostname(&out, hostname);
        }
        out = redact_ipv4(&out);
        redact_ipv6(&out)
    }
}

fn home_dir() -> Option<String> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .filter(|h| !h.is_empty())
}

/// The machine's hostname, best effort. Linux-first: `/proc/sys/kernel/
/// hostname` is authoritative and always present there, `/etc/hostname` is
/// the configured name, and the environment variables are the last resort
/// (a login shell usually exports `HOSTNAME`, a service manager often does
/// not). `None` is an ordinary outcome, not an error — it means one pattern
/// class fewer, and the caller has no better source either.
fn hostname() -> Option<String> {
    for path in ["/proc/sys/kernel/hostname", "/etc/hostname"] {
        if let Ok(text) = std::fs::read_to_string(path) {
            let name = text.trim().to_string();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .ok()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
}

/// One decided replacement: the byte range *inside* the candidate run that
/// really is a literal, and what takes its place. The range matters because
/// a run is greedy — a sentence-final `10.0.0.5.` arrives with the full stop
/// attached, and only the part in front of it is an address.
struct Hit {
    start: usize,
    end: usize,
    placeholder: &'static str,
}

/// True when `c` may sit *inside* a host or address token, and therefore
/// means a candidate run that touches it is part of a longer word rather
/// than a standalone literal. This is what keeps `knx_core::Project` and
/// `ApiError::bad_request` from being read as compressed IPv6 addresses:
/// both have an ASCII letter immediately before the `::`.
fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn boundary_ok(bytes: &[u8], start: usize, end: usize) -> bool {
    let before = start
        .checked_sub(1)
        .map(|i| is_token_char(bytes[i] as char))
        .unwrap_or(false);
    let after = bytes
        .get(end)
        .map(|b| is_token_char(*b as char))
        .unwrap_or(false);
    !before && !after
}

/// Replaces every dotted quad — four decimal groups of one to three digits,
/// each 0-255 — wherever one appears *inside* a run of digits and dots, not
/// only where the whole run happens to be exactly one.
///
/// A run is greedy over `.`, so a sentence-final address arrives as
/// `10.0.0.5.` (five groups), one after an ellipsis as `...10.0.0.5` (also
/// five, three of them empty), and a typo'd fifth octet or a glued VLAN tag
/// as `192.168.1.1.5` or `5.192.168.1.1` (also five, all non-empty). A test
/// that requires the run to split into *exactly* four groups says "not an
/// address" about every one of those and leaks the real one. Instead this
/// slides a four-group window across the run's dot-separated groups,
/// left to right, and takes the first valid quad it finds, then resumes
/// scanning right after it — so `1.2.3.4.5.6.7.8` yields two hits, not zero.
///
/// Two shapes deliberately survive this regardless: a KNX group address
/// (`1/2/3`, no dots between the numbers at all — `/` is not in the scan
/// class) and this project's own version string (`0.1.0-alpha.1`, three
/// groups then a non-digit, never four in a row). A bare, unlabelled
/// five-or-more-part number that happens to be a version rather than an
/// address (`1.2.3.4.5`) is not distinguishable from an address by shape
/// alone and is treated as one; see `docs/KNOWN_LIMITATIONS.md` §104.
fn redact_ipv4(text: &str) -> String {
    scan_and_replace(text, |c| c.is_ascii_digit() || c == '.', ipv4_hits)
}

/// Byte offsets of the `.`-separated groups in `run`, in the same order
/// `str::split('.')` would produce them, so that an empty group from a
/// leading, trailing or doubled dot still occupies its slot in the window
/// below rather than being silently skipped.
fn dotted_groups(run: &str) -> Vec<(usize, usize)> {
    let mut groups = Vec::new();
    let mut start = 0usize;
    for (i, b) in run.bytes().enumerate() {
        if b == b'.' {
            groups.push((start, i));
            start = i + 1;
        }
    }
    groups.push((start, run.len()));
    groups
}

fn is_valid_octet(group: &str) -> bool {
    (1..=3).contains(&group.len())
        && group.bytes().all(|b| b.is_ascii_digit())
        && group.parse::<u16>().is_ok_and(|n| n <= 255)
}

/// Every non-overlapping dotted quad in `run`, found by sliding a
/// four-group window left to right over its dot-separated groups and
/// jumping past a match rather than merely stepping into it — so the two
/// quads in `1.2.3.4.5.6.7.8` are found as `1.2.3.4` and `5.6.7.8`, not as
/// four overlapping, mutually destructive candidates.
fn ipv4_hits(run: &str) -> Vec<Hit> {
    let groups = dotted_groups(run);
    let mut hits = Vec::new();
    let mut i = 0usize;
    while i + 4 <= groups.len() {
        let window = &groups[i..i + 4];
        if window.iter().all(|&(s, e)| is_valid_octet(&run[s..e])) {
            hits.push(Hit {
                start: window[0].0,
                end: window[3].1,
                placeholder: IPV4_PLACEHOLDER,
            });
            i += 4;
        } else {
            i += 1;
        }
    }
    hits
}

/// Replaces every IPv6 literal. Validation is `std::net::Ipv6Addr`'s own
/// parser rather than a hand-written grammar — the standard library already
/// knows every compressed and full form there is, and a second opinion here
/// would only be a worse one.
fn redact_ipv6(text: &str) -> String {
    scan_and_replace(
        text,
        |c| c.is_ascii_hexdigit() || c == ':',
        |run| ipv6_hit(run).into_iter().collect(),
    )
}

/// Finds the address inside one candidate run, trying four sub-runs in a
/// fixed order: the whole run, the run without a trailing `:`, the run
/// without a single leading `:`, and the run without both.
///
/// The trailing-colon retry catches the residue of the IPv4 pass: once
/// `::ffff:192.0.2.1` has become `::ffff:[redacted-ipv4]`, the run left
/// behind is `::ffff:`, which no parser accepts and which is still an
/// address prefix worth removing. It also keeps the punctuation of
/// `fe80::1: connection refused`, where the second colon belongs to the
/// sentence and not to the address.
///
/// The leading-colon retry catches an address glued to the word in front of
/// it — `peer:2001:db8::1`, where the run starts at the separating colon and
/// the boundary rule would otherwise see the `r` of `peer` and refuse. One
/// colon only, deliberately: a run that starts with `::` after a word
/// character is a Rust path (`knx_core::Project`), and handing that to the
/// parser is exactly the mistake the boundary rule exists to prevent.
fn ipv6_hit(run: &str) -> Option<Hit> {
    let leading = usize::from(run.starts_with(':'));
    let trailing = usize::from(run.ends_with(':'));
    for start in [0, leading] {
        for end in [run.len(), run.len() - trailing] {
            if start >= end {
                continue;
            }
            let candidate = &run[start..end];
            if candidate.contains(':') && candidate.parse::<std::net::Ipv6Addr>().is_ok() {
                return Some(Hit {
                    start,
                    end,
                    placeholder: IPV6_PLACEHOLDER,
                });
            }
        }
    }
    None
}

/// Walks `text` once, hands every maximal run of `in_class` characters to
/// `verdict`, and substitutes the placeholder it returns over each range it
/// names. A run may contain more than one hit — `1.2.3.4.5.6.7.8` is one run
/// and two addresses — so `verdict` returns every candidate it finds, in
/// left-to-right, non-overlapping order; each is independently checked
/// against [`boundary_ok`] and applied if it passes.
///
/// ASCII-only by construction: every character class used here is ASCII, so
/// byte indices and character indices agree inside a run, and a multi-byte
/// character simply ends the run (its first byte is >= 0x80, which is in no
/// class).
fn scan_and_replace(
    text: &str,
    in_class: impl Fn(char) -> bool,
    verdict: impl Fn(&str) -> Vec<Hit>,
) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    let mut copied = 0usize;
    while i < bytes.len() {
        if !bytes[i].is_ascii() || !in_class(bytes[i] as char) {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii() && in_class(bytes[i] as char) {
            i += 1;
        }
        let run = &text[start..i];
        for hit in verdict(run) {
            let (from, to) = (start + hit.start, start + hit.end);
            if !boundary_ok(bytes, from, to) {
                continue;
            }
            out.push_str(&text[copied..from]);
            out.push_str(hit.placeholder);
            copied = to;
        }
    }
    out.push_str(&text[copied..]);
    out
}

/// Replaces the home directory prefix with `~`, but only where the path
/// really ends there. A plain substring replace turns
/// `/home/andrea/secret.knxproj` into `~a/secret.knxproj` when `$HOME` is
/// `/home/knxbench` — over-redaction rather than a leak, but it garbles a path
/// a maintainer has to read. A following word character means a different
/// directory whose name merely starts the same way.
fn replace_home(text: &str, home: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut copied = 0usize;
    let mut from = 0usize;
    while let Some(offset) = text[from..].find(home) {
        let start = from + offset;
        let end = start + home.len();
        let glued = bytes.get(end).is_some_and(|b| is_token_char(*b as char));
        if !glued {
            out.push_str(&text[copied..start]);
            out.push('~');
            copied = end;
        }
        from = end.max(start + 1);
    }
    out.push_str(&text[copied..]);
    out
}

/// Replaces `hostname` wherever it stands as its own token, case-insensitively
/// (hostnames are), leaving a longer word that merely contains it alone. A
/// trailing dot is a boundary on purpose, so an FQDN loses its host part and
/// keeps its domain — `workshop.example.org` becomes
/// `[redacted-host].example.org`, which still says "this was an FQDN"
/// without saying whose machine it was.
fn replace_hostname(text: &str, hostname: &str) -> String {
    let needle = hostname.to_ascii_lowercase();
    let haystack = text.to_ascii_lowercase();
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut copied = 0usize;
    let mut from = 0usize;
    while let Some(offset) = haystack[from..].find(&needle) {
        let start = from + offset;
        let end = start + needle.len();
        if boundary_ok(bytes, start, end) {
            out.push_str(&text[copied..start]);
            out.push_str(HOST_PLACEHOLDER);
            copied = end;
        }
        from = end.max(start + 1);
    }
    out.push_str(&text[copied..]);
    out
}

// ---------------------------------------------------------------------
// The bundle
// ---------------------------------------------------------------------

/// One file in the bundle, already rendered and already redacted where
/// redaction applies.
pub(crate) struct BundleFile {
    pub(crate) name: &'static str,
    pub(crate) bytes: Vec<u8>,
}

/// Everything the bundle is built from. The three opt-in artifacts are
/// `Option`s rather than `bool` plus data: a flag and its payload cannot
/// drift apart if there is only one of them, which is also what makes the
/// manifest exact — it lists what was built, not what was requested.
pub(crate) struct BundleInput {
    pub(crate) description: String,
    pub(crate) app_version: Option<String>,
    pub(crate) server_version: String,
    pub(crate) shell: Option<String>,
    pub(crate) ui_language: Option<String>,
    pub(crate) theme: Option<String>,
    pub(crate) project_open: bool,
    pub(crate) log: Option<Value>,
    pub(crate) project_summary: Option<Value>,
    pub(crate) bus_telegrams: Option<Value>,
}

/// The built bundle plus the `report.md` text on its own, because the
/// prefilled GitHub issue body is exactly that text and the caller would
/// otherwise have to dig it back out of a `Vec<u8>`.
pub(crate) struct Bundle {
    pub(crate) files: Vec<BundleFile>,
    pub(crate) report_markdown: String,
}

/// The commit a build knew about, out of `version_line()`'s
/// `<name> <version>[+g<sha>]` — SemVer 2.0.0 build metadata, so `+g` is
/// the only place a `+` appears.
pub(crate) fn build_commit_of(version_line: &str) -> Option<&str> {
    version_line.split_once("+g").map(|(_, sha)| sha)
}

fn unset(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("unknown")
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

/// Renders `report.md` — the file a maintainer reads first. English, like
/// the rest of the bundle: it is read by whoever is handed the issue, not
/// shown in the application, and the dialog that explains the bundle is the
/// surface that follows the user's language.
fn report_markdown(input: &BundleInput, included: &[&'static str]) -> String {
    let description = input.description.trim();
    let mut md = String::new();
    md.push_str("# KNXBench debug report\n\n");
    md.push_str("## What happened\n\n");
    if description.is_empty() {
        md.push_str("_No description was given._\n\n");
    } else {
        md.push_str(description);
        md.push_str("\n\n");
    }
    md.push_str("## Versions\n\n");
    md.push_str(&format!(
        "- Application: {}\n- Server: {}\n- Build commit: {}\n\n",
        unset(&input.app_version),
        input.server_version,
        build_commit_of(&input.server_version).unwrap_or("not recorded in this build"),
    ));
    md.push_str("## Environment\n\n");
    md.push_str(&format!(
        "- OS: {}\n- Architecture: {}\n- Runs in: {}\n- UI language: {}\n- Theme: {}\n\
         - A project is open: {}\n\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
        unset(&input.shell),
        unset(&input.ui_language),
        unset(&input.theme),
        yes_no(input.project_open),
    ));
    md.push_str("## Bundle contents\n\n");
    for name in included {
        md.push_str(&format!("- `{name}` — {}\n", describe_file(name)));
    }
    md.push_str(
        "\nIP addresses, the home directory prefix and the machine hostname have been \
         replaced by placeholders in `report.md`, `environment.json` and `log.json`. \
         `bus-telegrams.json`, when present, keeps its KNX individual and group addresses \
         — that is what makes it useful, and why it is off by default.\n",
    );
    md
}

fn describe_file(name: &str) -> &'static str {
    match name {
        REPORT_MD => "this file",
        ENVIRONMENT_JSON => "the same facts, machine-readable",
        LOG_JSON => "this session's log entries",
        PROJECT_SUMMARY_JSON => "counts and structural statistics only, no names or addresses",
        BUS_TELEGRAMS_JSON => "the bus monitor buffer, including real device addresses",
        _ => "an artifact this build does not describe",
    }
}

fn environment_json(input: &BundleInput, included: &[&'static str]) -> Value {
    serde_json::json!({
        "appVersion": input.app_version,
        "serverVersion": input.server_version,
        "buildCommit": build_commit_of(&input.server_version),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "shell": input.shell,
        "uiLanguage": input.ui_language,
        "theme": input.theme,
        "projectOpen": input.project_open,
        "files": included,
    })
}

/// Builds every file the bundle holds, in a fixed order: the two mandatory
/// artifacts first, then whichever opt-ins were asked for.
pub(crate) fn build_bundle(input: &BundleInput, redactor: &Redactor) -> Bundle {
    let mut included: Vec<&'static str> = vec![REPORT_MD, ENVIRONMENT_JSON];
    if input.log.is_some() {
        included.push(LOG_JSON);
    }
    if input.project_summary.is_some() {
        included.push(PROJECT_SUMMARY_JSON);
    }
    if input.bus_telegrams.is_some() {
        included.push(BUS_TELEGRAMS_JSON);
    }

    let report = redactor.apply(&report_markdown(input, &included));
    let environment = redactor.apply(&pretty(&environment_json(input, &included)));

    let mut files = vec![
        BundleFile {
            name: REPORT_MD,
            bytes: report.clone().into_bytes(),
        },
        BundleFile {
            name: ENVIRONMENT_JSON,
            bytes: environment.into_bytes(),
        },
    ];
    if let Some(log) = &input.log {
        files.push(BundleFile {
            name: LOG_JSON,
            bytes: redactor.apply(&pretty(log)).into_bytes(),
        });
    }
    // Not redacted: counts only, and nothing here is a name, an address or
    // free text from the project.
    if let Some(summary) = &input.project_summary {
        files.push(BundleFile {
            name: PROJECT_SUMMARY_JSON,
            bytes: pretty(summary).into_bytes(),
        });
    }
    // Not redacted, deliberately — see this module's own doc comment.
    if let Some(telegrams) = &input.bus_telegrams {
        files.push(BundleFile {
            name: BUS_TELEGRAMS_JSON,
            bytes: pretty(telegrams).into_bytes(),
        });
    }

    Bundle {
        files,
        report_markdown: report,
    }
}

fn pretty(value: &Value) -> String {
    // A `serde_json::Value` cannot fail to serialize; the fallback exists
    // only so this function never panics on a future caller's behalf.
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

/// Writes `files` to `path` as a zip. Deflated, like every other zip this
/// workspace writes (`knx-etsproj`'s exporter) — a debug bundle is mostly
/// text and compresses well enough to matter on an issue attachment.
pub(crate) fn write_zip(path: &Path, files: &[BundleFile]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("cannot create the bundle: {e}"))?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    for entry in files {
        writer
            .start_file(entry.name, options)
            .map_err(|e| format!("cannot start {} in the bundle: {e}", entry.name))?;
        writer
            .write_all(&entry.bytes)
            .map_err(|e| format!("cannot write {} into the bundle: {e}", entry.name))?;
    }
    writer
        .finish()
        .map_err(|e| format!("cannot finish the bundle: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain() -> Redactor {
        Redactor::new(None, None)
    }

    #[test]
    fn an_ipv4_address_is_redacted_in_prose_in_a_path_and_in_a_json_value() {
        let r = plain();
        assert_eq!(
            r.apply("tunnel to KNX_GATEWAY:3671 failed"),
            format!("tunnel to {IPV4_PLACEHOLDER}:3671 failed")
        );
        assert_eq!(
            r.apply("/var/log/10.0.0.7/session.log"),
            format!("/var/log/{IPV4_PLACEHOLDER}/session.log")
        );
        assert_eq!(
            r.apply("{\"gateway\":\"192.168.1.20\"}"),
            format!("{{\"gateway\":\"{IPV4_PLACEHOLDER}\"}}")
        );
    }

    #[test]
    fn rfc_1918_and_loopback_ranges_are_caught_by_the_pattern_not_by_a_list() {
        let r = plain();
        for address in [
            "10.0.0.1",
            "172.16.0.1",
            "172.31.255.254",
            "192.168.0.1",
            "127.0.0.1",
            "169.254.1.1",
            "8.8.8.8",
            "0.0.0.0",
            "255.255.255.255",
        ] {
            assert_eq!(
                r.apply(&format!("host {address} here")),
                format!("host {IPV4_PLACEHOLDER} here"),
                "{address} should have been redacted by shape alone"
            );
        }
    }

    #[test]
    fn a_knx_group_address_is_not_mangled_by_the_ipv4_pattern() {
        let r = plain();
        assert_eq!(r.apply("wrote to 1/2/3"), "wrote to 1/2/3");
        assert_eq!(
            r.apply("range 0/0/1 .. 31/7/255"),
            "range 0/0/1 .. 31/7/255"
        );
    }

    #[test]
    fn a_knx_individual_address_survives_because_it_has_only_three_groups() {
        let r = plain();
        assert_eq!(
            r.apply("device 1.1.220 answered"),
            "device 1.1.220 answered"
        );
    }

    #[test]
    fn a_version_string_is_not_mistaken_for_an_address() {
        let r = plain();
        assert_eq!(
            r.apply("knx-server 0.1.0-alpha.1+g4cde085"),
            "knx-server 0.1.0-alpha.1+g4cde085"
        );
        assert_eq!(
            r.apply("schema 11, core v01.06.02"),
            "schema 11, core v01.06.02"
        );
    }

    #[test]
    fn an_octet_above_255_is_not_an_address() {
        let r = plain();
        assert_eq!(r.apply("build 1.2.3.999"), "build 1.2.3.999");
        assert_eq!(r.apply("build 256.1.1.1"), "build 256.1.1.1");
    }

    #[test]
    fn a_dotted_quad_glued_to_a_word_is_left_alone() {
        // `v1.2.3.4` is a version tag, not a host; the boundary rule is what
        // tells them apart.
        let r = plain();
        assert_eq!(r.apply("v1.2.3.4"), "v1.2.3.4");
    }

    #[test]
    fn a_dotted_quad_next_to_a_dot_is_still_an_address() {
        // Every line here leaked before the run was trimmed: the scan class
        // includes `.`, so a sentence-final address is a five-group run and
        // a four-group test says "not an address" about the address.
        let r = plain();
        let probes = [
            ("gateway is 192.168.1.1.", "gateway is {P}."),
            ("at 10.0.0.5. It failed", "at {P}. It failed"),
            ("...192.168.1.1", "...{P}"),
            ("{\"note\":\"host 10.1.2.3.\"}", "{\"note\":\"host {P}.\"}"),
            // The control: parentheses were never in the scan class, so this
            // one worked all along and must keep working.
            ("(192.168.1.1)", "({P})"),
        ];
        // Every probe is checked before anything is asserted, so a
        // regression reports all the shapes it broke, not merely the first.
        let leaks: Vec<String> = probes
            .iter()
            .filter(|(input, want)| r.apply(input) != want.replace("{P}", IPV4_PLACEHOLDER))
            .map(|(input, _)| format!("{input:?} -> {:?}", r.apply(input)))
            .collect();
        assert!(leaks.is_empty(), "not redacted: {leaks:#?}");
    }

    #[test]
    fn a_version_tag_with_a_trailing_dot_is_still_not_an_address() {
        // The trimming above must not cost the boundary rule its teeth.
        let r = plain();
        assert_eq!(r.apply("v1.2.3.4."), "v1.2.3.4.");
        assert_eq!(r.apply("see 0.1.0-alpha.1."), "see 0.1.0-alpha.1.");
    }

    #[test]
    fn a_dotted_quad_glued_to_an_extra_digit_and_dot_group_is_still_an_address() {
        // Trimming the run's leading and trailing dots closed the
        // sentence-final case, but it trims dots, not digits: a typo'd
        // fifth octet or a glued extra group still arrives as a run of
        // more than four groups, and a test that requires exactly four
        // says "not an address" about the address hiding inside it.
        let r = plain();
        let probes = [
            // The scan is greedy left to right and takes the first valid
            // quad it finds; here that is `5.192.168.1`, a syntactically
            // valid address in its own right, leaving `.1` over. Which
            // four-group window is chosen is not the point — that the real
            // address never survives intact is checked separately below.
            ("5.192.168.1.1", "{P}.1"),
            ("addr=192.168.1.1.5", "addr={P}.5"),
            (
                "gateway 192.168.1.1.2 unreachable",
                "gateway {P}.2 unreachable",
            ),
        ];
        let leaks: Vec<String> = probes
            .iter()
            .filter(|(input, want)| r.apply(input) != want.replace("{P}", IPV4_PLACEHOLDER))
            .map(|(input, _)| format!("{input:?} -> {:?}", r.apply(input)))
            .collect();
        assert!(leaks.is_empty(), "not redacted: {leaks:#?}");

        // The literal address must not survive anywhere in the output, not
        // merely differ from a hand-picked expectation.
        for input in ["5.192.168.1.1", "addr=192.168.1.1.5"] {
            let redacted = r.apply(input);
            assert!(
                !redacted.contains("192.168.1.1"),
                "{input:?} -> {redacted:?} still leaks the address"
            );
        }
    }

    #[test]
    fn a_long_glued_run_yields_every_address_hiding_inside_it() {
        // A run can hide more than one quad. Both must go, not just the
        // first the scan happens to trip over.
        let r = plain();
        let redacted = r.apply("1.2.3.4.5.6.7.8");
        assert!(!redacted.contains("1.2.3.4"), "{redacted}");
        assert!(!redacted.contains("5.6.7.8"), "{redacted}");
        assert_eq!(redacted.matches(IPV4_PLACEHOLDER).count(), 2, "{redacted}");
    }

    #[test]
    fn a_bare_multi_part_version_number_is_treated_as_an_address_this_is_a_documented_residue() {
        // `1.2.3.4.5` glued to nothing could be a five-part version number
        // instead of an address with a typo'd fifth octet — shape alone
        // cannot tell them apart, and this pass picks the side that
        // protects the user's data. Documented in KNOWN_LIMITATIONS.md
        // §104 rather than left as a silent surprise.
        let r = plain();
        assert_eq!(r.apply("1.2.3.4.5"), format!("{IPV4_PLACEHOLDER}.5"));
    }

    #[test]
    fn an_ipv6_literal_glued_to_the_word_in_front_of_it_is_still_redacted() {
        let r = plain();
        assert_eq!(
            r.apply("peer:2001:db8::1 left"),
            format!("peer:{IPV6_PLACEHOLDER} left")
        );
        // The sentence colon belongs to the sentence, not to the address.
        assert_eq!(
            r.apply("fe80::1: connection refused"),
            format!("{IPV6_PLACEHOLDER}: connection refused")
        );
    }

    #[test]
    fn ipv6_literals_are_redacted_in_every_form_that_appears_in_practice() {
        let r = plain();
        for address in ["::1", "fe80::1", "2001:db8::1", "2001:db8:0:0:0:0:2:1"] {
            assert_eq!(
                r.apply(&format!("bound to {address} now")),
                format!("bound to {IPV6_PLACEHOLDER} now"),
                "{address} should have been redacted"
            );
        }
        assert_eq!(
            r.apply("listening on [2001:db8::1]:3671"),
            format!("listening on [{IPV6_PLACEHOLDER}]:3671")
        );
    }

    #[test]
    fn an_ipv4_mapped_ipv6_literal_loses_both_halves() {
        let redacted = plain().apply("peer ::ffff:192.0.2.128 closed");
        assert!(!redacted.contains("192.0.2.128"), "{redacted}");
        assert!(redacted.contains(IPV4_PLACEHOLDER), "{redacted}");
        assert!(redacted.contains(IPV6_PLACEHOLDER), "{redacted}");
    }

    #[test]
    fn a_rust_path_is_not_read_as_a_compressed_ipv6_address() {
        let r = plain();
        for text in [
            "knx_core::Project",
            "ApiError::bad_request",
            "domain::export_documentation_impl",
            "std::net::Ipv6Addr",
        ] {
            assert_eq!(r.apply(text), text, "{text} must survive the IPv6 pass");
        }
    }

    #[test]
    fn a_timestamp_is_not_read_as_an_ipv6_address() {
        let r = plain();
        for text in [
            "2026-09-19T14:03:05Z",
            "2026-09-19T14:03:05.123456+02:00",
            "took 00:01:30",
        ] {
            assert_eq!(r.apply(text), text, "{text} must survive the IPv6 pass");
        }
    }

    #[test]
    fn the_home_directory_prefix_becomes_a_tilde_and_a_single_slash_home_is_ignored() {
        let r = Redactor::new(Some("/home/knxbench".into()), None);
        assert_eq!(
            r.apply("opened /home/knxbench/projects/villa.knxproj"),
            "opened ~/projects/villa.knxproj"
        );
        let slash = Redactor::new(Some("/".into()), None);
        assert_eq!(slash.apply("/a/b/c"), "/a/b/c");
    }

    #[test]
    fn a_sibling_home_directory_is_not_half_rewritten() {
        let r = Redactor::new(Some("/home/knxbench".into()), None);
        assert_eq!(
            r.apply("/home/andrea/secret.knxproj"),
            "/home/andrea/secret.knxproj"
        );
        assert_eq!(r.apply("saved in /home/knxbench."), "saved in ~.");
    }

    #[test]
    fn the_hostname_is_replaced_as_a_token_and_not_inside_a_longer_word() {
        let r = Redactor::new(None, Some("workshop".into()));
        assert_eq!(
            r.apply("workshop said no"),
            format!("{HOST_PLACEHOLDER} said no")
        );
        assert_eq!(
            r.apply("WORKSHOP.example.org"),
            format!("{HOST_PLACEHOLDER}.example.org")
        );
        assert_eq!(r.apply("workshopping is fine"), "workshopping is fine");
        assert_eq!(r.apply("a_workshop_thing"), "a_workshop_thing");
    }

    #[test]
    fn a_one_character_hostname_is_ignored_rather_than_shredding_the_text() {
        let r = Redactor::new(None, Some("a".into()));
        assert_eq!(r.apply("a b a"), "a b a");
    }

    fn input() -> BundleInput {
        BundleInput {
            description: "It exploded.".into(),
            app_version: Some("0.1.0-alpha.1".into()),
            server_version: "knx-server 0.1.0-alpha.1+g4cde085".into(),
            shell: Some("tauri".into()),
            ui_language: Some("de".into()),
            theme: Some("graphite".into()),
            project_open: true,
            log: None,
            project_summary: None,
            bus_telegrams: None,
        }
    }

    fn names(bundle: &Bundle) -> Vec<&'static str> {
        bundle.files.iter().map(|f| f.name).collect()
    }

    #[test]
    fn with_every_opt_in_off_the_bundle_is_exactly_the_two_mandatory_files() {
        let bundle = build_bundle(&input(), &plain());
        assert_eq!(names(&bundle), vec![REPORT_MD, ENVIRONMENT_JSON]);
    }

    #[test]
    fn each_opt_in_adds_exactly_its_own_file_and_no_other() {
        let with_log = BundleInput {
            log: Some(serde_json::json!([])),
            ..input()
        };
        assert_eq!(
            names(&build_bundle(&with_log, &plain())),
            vec![REPORT_MD, ENVIRONMENT_JSON, LOG_JSON]
        );

        let with_summary = BundleInput {
            project_summary: Some(serde_json::json!({})),
            ..input()
        };
        assert_eq!(
            names(&build_bundle(&with_summary, &plain())),
            vec![REPORT_MD, ENVIRONMENT_JSON, PROJECT_SUMMARY_JSON]
        );

        let with_telegrams = BundleInput {
            bus_telegrams: Some(serde_json::json!([])),
            ..input()
        };
        assert_eq!(
            names(&build_bundle(&with_telegrams, &plain())),
            vec![REPORT_MD, ENVIRONMENT_JSON, BUS_TELEGRAMS_JSON]
        );

        let all = BundleInput {
            log: Some(serde_json::json!([])),
            project_summary: Some(serde_json::json!({})),
            bus_telegrams: Some(serde_json::json!([])),
            ..input()
        };
        assert_eq!(
            names(&build_bundle(&all, &plain())),
            vec![
                REPORT_MD,
                ENVIRONMENT_JSON,
                LOG_JSON,
                PROJECT_SUMMARY_JSON,
                BUS_TELEGRAMS_JSON
            ]
        );
    }

    /// The manifest is a promise, not a wish list: `environment.json`'s
    /// `files` array and `report.md`'s "Bundle contents" section must name
    /// exactly the files the zip will hold — for every combination of
    /// opt-ins, including all three off. A bundle that advertises a file it
    /// did not build is the one failure mode that would have the user
    /// believing they sent something they did not.
    #[test]
    fn the_manifest_names_exactly_the_files_the_bundle_holds_for_every_combination() {
        for (log, summary, telegrams) in [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (false, false, true),
            (true, true, false),
            (true, false, true),
            (false, true, true),
            (true, true, true),
        ] {
            let combination = BundleInput {
                log: log.then(|| serde_json::json!([])),
                project_summary: summary.then(|| serde_json::json!({})),
                bus_telegrams: telegrams.then(|| serde_json::json!([])),
                ..input()
            };
            let bundle = build_bundle(&combination, &plain());
            let built = names(&bundle);

            let environment = bundle
                .files
                .iter()
                .find(|f| f.name == ENVIRONMENT_JSON)
                .expect("every bundle carries environment.json");
            let parsed: Value = serde_json::from_slice(&environment.bytes).unwrap();
            let listed: Vec<&str> = parsed["files"]
                .as_array()
                .expect("environment.json must list its files")
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            assert_eq!(
                listed, built,
                "environment.json's manifest disagrees with the bundle for \
                 (log, summary, telegrams) = ({log}, {summary}, {telegrams})"
            );

            for candidate in [LOG_JSON, PROJECT_SUMMARY_JSON, BUS_TELEGRAMS_JSON] {
                let advertised = bundle.report_markdown.contains(&format!("- `{candidate}`"));
                assert_eq!(
                    advertised,
                    built.contains(&candidate),
                    "report.md {} {candidate} for (log, summary, telegrams) = \
                     ({log}, {summary}, {telegrams})",
                    if advertised { "advertises" } else { "omits" }
                );
            }
        }
    }

    #[test]
    fn the_report_names_the_versions_the_environment_and_every_included_file() {
        let all = BundleInput {
            log: Some(serde_json::json!([])),
            project_summary: Some(serde_json::json!({})),
            bus_telegrams: Some(serde_json::json!([])),
            ..input()
        };
        let bundle = build_bundle(&all, &plain());
        let md = &bundle.report_markdown;
        assert!(md.contains("It exploded."), "{md}");
        assert!(md.contains("knx-server 0.1.0-alpha.1+g4cde085"), "{md}");
        assert!(md.contains("4cde085"), "{md}");
        assert!(md.contains("tauri"), "{md}");
        assert!(md.contains("graphite"), "{md}");
        for name in [
            REPORT_MD,
            ENVIRONMENT_JSON,
            LOG_JSON,
            PROJECT_SUMMARY_JSON,
            BUS_TELEGRAMS_JSON,
        ] {
            assert!(md.contains(name), "{name} missing from the report: {md}");
        }
    }

    #[test]
    fn a_build_without_a_commit_says_so_instead_of_inventing_one() {
        let no_sha = BundleInput {
            server_version: "knx-server 0.1.0-alpha.1".into(),
            ..input()
        };
        assert_eq!(build_commit_of(&no_sha.server_version), None);
        let bundle = build_bundle(&no_sha, &plain());
        assert!(
            bundle
                .report_markdown
                .contains("not recorded in this build"),
            "{}",
            bundle.report_markdown
        );
    }

    #[test]
    fn the_report_and_the_log_are_redacted_but_the_telegrams_are_not() {
        let all = BundleInput {
            description: "gateway KNX_GATEWAY dropped us".into(),
            log: Some(serde_json::json!([{ "message": "tunnel KNX_GATEWAY closed" }])),
            bus_telegrams: Some(
                serde_json::json!([{ "source": "1.1.5", "gateway": "KNX_GATEWAY" }]),
            ),
            ..input()
        };
        let bundle = build_bundle(&all, &plain());
        let text = |name: &str| {
            String::from_utf8(
                bundle
                    .files
                    .iter()
                    .find(|f| f.name == name)
                    .unwrap()
                    .bytes
                    .clone(),
            )
            .unwrap()
        };
        assert!(!text(REPORT_MD).contains("KNX_GATEWAY"));
        assert!(!text(LOG_JSON).contains("KNX_GATEWAY"));
        assert!(text(LOG_JSON).contains(IPV4_PLACEHOLDER));
        // The whole point of the opt-in: this file keeps its addresses.
        assert!(text(BUS_TELEGRAMS_JSON).contains("KNX_GATEWAY"));
        assert!(text(BUS_TELEGRAMS_JSON).contains("1.1.5"));
    }

    #[test]
    fn redacted_json_is_still_json() {
        let all = BundleInput {
            log: Some(serde_json::json!([{ "message": "at 10.0.0.7" }])),
            ..input()
        };
        let bundle = build_bundle(&all, &plain());
        for file in &bundle.files {
            if file.name.ends_with(".json") {
                let text = std::str::from_utf8(&file.bytes).unwrap();
                serde_json::from_str::<Value>(text)
                    .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}\n{text}", file.name));
            }
        }
    }

    #[test]
    fn the_zip_holds_exactly_the_files_the_bundle_named() {
        let all = BundleInput {
            log: Some(serde_json::json!([])),
            ..input()
        };
        let bundle = build_bundle(&all, &plain());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bundle.zip");
        write_zip(&path, &bundle.files).unwrap();

        let file = std::fs::File::open(&path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut found: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        found.sort();
        let mut expected = vec![
            ENVIRONMENT_JSON.to_string(),
            LOG_JSON.to_string(),
            REPORT_MD.to_string(),
        ];
        expected.sort();
        assert_eq!(found, expected);
    }
}
