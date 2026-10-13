# Debug-report privacy limits

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-106)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 106. The debug report redacts five pattern classes, and nothing else

**Audit 2026-10-10 (retained source evidence; the update of the same day is
closed).** Imported projects keep source subtrees verbatim, and those can
carry a tunnelling target and connection parameters (`BusAccess/@Parameter`),
IP and MAC configuration (`IPConfig`), additional device addresses
(`AdditionalAddresses`) and ETS user names (`ProjectTraces/ProjectTrace/@UserName`).
Every output that can carry retained source evidence was checked against a
synthetic witness with planted values
(`knx_testsupport::retained_privacy_knxproj_bytes`):

- **Debug report.** Retained subtrees never enter it: `log.json` names them by
  location, size and hash only. Unknown-attribute samples do reach `log.json`
  verbatim, so MAC addresses are now a fifth redaction class (below). User
  names have no recognisable shape; where one appears in a sample or in the
  description it stays, and `report.md` and the dialog now say "user names"
  are kept and that retained ETS source data is never included
  (`debug_report::redaction_tests`).
- **MCP read path.** Loads only the domain model, never the retained-source
  store; no tool response or search hit carries a planted value
  (`apps/knx-mcp/tests/retained_source.rs`).
- **Contribution bundles.** Reduced findings drop every source value
  (`reduced_bundle_withholds_retained_network_and_user_values`). Selected
  context samples and a private original are unmodified on purpose; their
  README now names network endpoints, MAC addresses, additional device
  addresses and project-trace user names.
- **Selective import.** The original archive and opaque evidence, unselected
  parts included, are copied into the destination project and stay local;
  the preview note now names the same classes
  (`retention_note_names_network_endpoints_and_user_names`).
- **Project export (`.knxproj`).** Re-emits retained subtrees verbatim by
  design: it is the user's own project, and dropping them would break the
  roundtrip guarantee. It is not a sharing channel of the application.
- **Telemetry and achievements.** There is no telemetry; the achievements
  record holds ids, timestamps and counters only.

**Limitation.** The debug-report bundle (T29,
`apps/knx-server/src/debug_report.rs`) replaces exactly five things in
`report.md`, `environment.json` and `log.json`: any IPv4 dotted quad, any
IPv6 literal, any MAC (or longer EUI) address written as two-hex-digit groups
with one consistent `:` or `-` separator (since 2026-10-10), the user's
home-directory prefix, and the machine's hostname. Anything else identifying
that reaches those files travels with them — a user name, a device serial
number, a project file name sitting outside the home
directory, a hostname the machine does not report, or whatever the user types
into the description field beyond those four shapes.

**KNX addresses are not one of the five classes.** `log.json` is on by
default and can name group addresses and imported element names, because
`session_log.rs` puts `conflict.group_address`, `unknown.name` and
`unknown.sample` into its messages verbatim. That is deliberate: a debug log
stripped of the address the conflict is about cannot diagnose the conflict.
The dialog says so in the user's language — "with IP addresses removed", not
"with addresses removed" — and the privacy paragraph states plainly that KNX
addresses and project names are never replaced anywhere.

**Three knowable failure modes inside the address classes.** An IPv6 literal that
follows a word character with no separator at all (`peer2001:db8::1`) is not
redacted: the boundary rule that keeps `knx_core::Project` from being read as
a compressed address cannot tell that case from a Rust path. One separating
colon *is* handled (`peer:2001:db8::1`), two are not (`peer::2001:db8::1`
reads as a path). And the home-directory prefix is matched textually with a
word-boundary check on its right-hand side only, so `/home/knxbench-old` is
still rewritten to `~-old` when `$HOME` is `/home/knxbench` — over-redaction
that garbles a path rather than a leak, and the far more common
`/home/knxbencha` case is left alone.

A third is the mirror image of the second fix round's IPv4 change. The scan
now slides a four-group window across a whole run of digits and dots rather
than requiring the run to split into exactly four groups, which is what
closes a typo'd fifth octet or a glued extra group (`192.0.2.1.5`,
`5.192.0.2.1`) that used to survive intact. But a bare, unlabelled number
with five or more dot-separated parts is not distinguishable from an address
by shape alone, and a genuine version string in that shape (`1.2.3.4.5` with
nothing in front of it) is over-redacted the same way a real address would
be caught — this pass picks the side that protects the user's data. A
version string glued to a leading letter (`v1.2.3.4`) is unaffected: the
boundary rule still refuses it. This project's own version string never
takes the bare five-part shape (`0.1.0-alpha.1[+g<sha>]` has three digit
groups before the first non-digit), so the residue does not touch anything
this application prints; it would only bite a third party's version string
quoted verbatim into the report with no letter in front of it.

**Cause.** Deliberate, and scoped that way by the brief: redaction is by
pattern class rather than by a list of known values, and each class has to be
a shape that can be recognised without guessing. A dotted quad and an IPv6
literal have grammars; "an identifier that matters to this user" does not. A
broader filter would either miss things anyway or start mangling ordinary
text — the IPv6 pass already has to refuse `knx_core::Project`, which a
parser will happily read as a compressed address. Two classes also have a
knowable failure mode: `hostname()` reads `/proc/sys/kernel/hostname`,
`/etc/hostname` and then the environment, and a process where none of those
answer simply redacts one class fewer; `HOME` unset does the same for paths.

**Consequence.** The bundle is safer than an unfiltered log but is not
anonymous, and nothing in the product claims it is. The dialog names the
three redacted files, says that KNX addresses and project names survive in
all of them, and names what `bus-telegrams.json` carries: the individual and
group addresses of the installation plus the group address names the open
project knows for them ("Kitchen ceiling light"). Without those a telegram
dump says nothing, which is why they stay and why the file is opt-in. The
zip is written locally and shown to the user before anything is shared. The
GitHub path opens a prefilled issue page in the browser and stops there: no
token, no credential, no `POST` from the application, and no upload anywhere.

**MAC class, knowable edges (2026-10-10).** Six or more two-digit groups with
one consistent separator are redacted whether or not they are a hardware
address, so a six-pair number such as `12-34-56-78-90-12` is over-redacted.
Not matched: a bare twelve-digit hex string (hashes and identifiers in the
log share that shape), the dotted `0200.5e10.0001` form, mixed separators,
and a run glued to a word character. Eight colon-separated pairs are also a
valid IPv6 literal and become `[redacted-ipv6]` instead.

**Lifted when.** Nothing here is waiting on a fix. A further class goes in as
another shape-recognising pass next to the existing five, with the same
requirement that it name what it removes rather than silently blanking text.

**AR13 (2026-10-04) — audited with one fixture per class.**
`every_privacy_class_is_either_redacted_or_named_in_the_report` feeds IPv4
(private and public), IPv6 (including IPv4-mapped), the home prefix and the
hostname (bare and FQDN) through every input channel — description, the four
client facts, log message/location/detail — and finds none of them in
`report.md`, `environment.json` or `log.json`. Serial numbers, e-mail
addresses, paths outside the home directory, KNX addresses and names survive
as written (MAC addresses until the 2026-10-10 audit above), and `report.md` (also the GitHub issue body) now names
each of them. It also says `bus-telegrams.json` is not redacted and carries
values — text values included — and timestamps, which together can show when
the installation was in use; before, only its addresses were mentioned.
`project-summary.json` stays counts-only (existing test). The in-app dialog
text is Web-owned; the matching wording is handed to the Web-lock holder.
<!-- END retained catalogue record -->
