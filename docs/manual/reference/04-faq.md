← Previous: [Troubleshooting](03-troubleshooting.md) · [Manual index](../README.md)

# FAQ

Questions people new to KNXBench actually ask, answered plainly. If your question isn't
here, [Known issues](../known-issues.md), [Implementation status](../implementation-status.md)
and [Ideas and roadmap](../ideas-and-roadmap.md) cover the longer version, and the
[issue tracker](https://github.com/KNXBench-Labs/KNXBench/issues) is where to ask something
new.

**Is KNXBench a replacement for ETS?**

No. KNXBench is an independent, **KNX-compatible** project editor — it reads and writes
project data, but it is not KNX-certified and does not claim full ETS compatibility. See
[What is KNXBench?](../getting-started/01-what-is-knxbench.md) for what it does and
deliberately does not do, and
[Supported and unsupported KNX/ETS functionality](02-supported-and-unsupported.md) for the
row-by-row evidence.

**Can KNXBench program or commission a device?**

Only narrowly. `knx device download` and the **Download to device** tab write a project
device's application tables and parameters after a plan and a per-device confirmation;
this has been verified on one device so far, and procedures KNXBench cannot plan are
refused. Programming an individual address is currently refused until durable recovery
exists, and unloading and secure devices are not supported. For anything beyond that,
use a commissioning tool that covers your devices. See
[Bus monitor and KNXnet/IP](../user-guide/07-bus-and-interfaces.md#what-knxbench-does-and-does-not-do-on-a-bus).

**Is my project safe with KNXBench?**

KNXBench never silently discards information it doesn't understand — unknown data is
preserved or reported, not dropped. That said, it is alpha software with no release ever
cut, so keep independent backups of any project you care about, the same way you would
for any pre-1.0 tool. [Project status](../getting-started/03-project-status.md) is worth
reading before you commit anything important to it.

**What happens to data KNXBench doesn't understand?**

It is kept, not thrown away. Unknown XML elements and attributes from a `.knxproj` import
are stored byte-for-byte in an opaque table, counted, and reported, so the import report
can tell you exactly what the file said that KNXBench could not interpret. Where
something can't be preserved opaquely — an unsupported
feature, a conflict — KNXBench reports it instead of guessing. See
[The import report](../user-guide/02-projects.md#the-import-report).

**Can I write a `.knxproj` out of KNXBench and open it in ETS?**

No. There was an exporter; it was withdrawn on 2026-09-20
([ADR-0028](../../adr/0028-no-knxproj-export.md)). It produced unsigned containers that
no real ETS installation had ever been asked to open, and maintaining a writer nobody
could verify was not worth what it returned. Import is one-way: a project that has been
read into KNXBench stays in `.knxdb`.

**Which file format should I actually keep?**

The native `.knxdb` file. It's KNXBench's own versioned SQLite format, it's the lossless
working format the whole application is built around, and since there is no `.knxproj`
export it is also the only format KNXBench writes a project to. Keep the `.knxproj` you
imported as well — KNXBench never modifies it.

**Does KNXBench run on Windows or macOS?**

Not tested, and not currently a project goal. KNXBench is Linux-first: the desktop shell
has been built and run on exactly one machine, an x86_64 Arch Linux host, and the web
server has no platform-specific code but is likewise only verified on Linux. See
[Linux setup](../getting-started/05-linux-setup.md) for the tested boundary.

**Can I use KNXBench without any KNX hardware?**

Yes, for most of it. Importing, editing, and exporting projects, browsing the product
catalog, and generating documentation all work on a file alone. Only the bus monitor and
the command-line bus commands need a real KNXnet/IP interface to talk to.

**Is KNXBench KNX-certified?**

No, and it never claims to be. **KNX-compatible** is the term this manual uses
throughout, and it means "tested against real files and real gateways where evidence
exists," not certification by the KNX Association.

**What does "alpha" mean for this project specifically?**

`0.1.0-alpha.1` on every component, no git tag, and no release ever published. Concretely:
`docs/KNOWN_LIMITATIONS.md` lists a large number of known limitations, several classified
critical, and nothing in KNXBench writes to a real device. See
[Project status](../getting-started/03-project-status.md) for the current count and what
it implies for trusting it with real work.

**Is KNXBench free, and under what license?**

Yes. It's licensed under the GNU Affero General Public License version 3 or later
(AGPL-3.0-or-later). You can use, modify, and redistribute it under that license's terms;
if you run a modified version as a network service, the AGPL requires offering its users
the corresponding source. See [`LICENSE`](../../../LICENSE) in the repository root.

**Does KNXBench send anything over the network on its own?**

Only what you tell it to: talking to a KNX gateway you configured, and — if you build the
web/Docker server — serving the HTTP API you deployed it as. There is no telemetry,
analytics, or update check anywhere in the code. The debug-report feature builds a
redacted diagnostic bundle locally, on request, and does not send it anywhere; what you do
with the resulting file is up to you.

**Who builds KNXBench?**

One author, working with an AI assistant, because he switched to Linux and couldn't find
a KNX tool he liked there. See
[Why KNXBench exists](../getting-started/02-why-knxbench-exists.md) for the full, mildly
self-deprecating story.

**How do I report a bug?**

Through the project's [issue tracker](https://github.com/KNXBench-Labs/KNXBench/issues).
If the bug involves an import, the server, or the bus monitor, the session **Log** and the
debug-report feature both produce text meant to be pasted straight into a report — see
[The session log and the diagnostics window](../user-guide/07-bus-and-interfaces.md#the-session-log-and-the-diagnostics-window).

**Can several people work on the same project at the same time?**

Not safely, today. `knx-server` holds exactly one project in memory with one shared undo
stack; a second browser editing the same server can undo the first one's changes, with no
conflict detection at all. If you deploy the web server for more than one person to reach,
treat it as a single-operator tool until this changes. See
[Supported and unsupported KNX/ETS functionality §Multi-user and collaboration](02-supported-and-unsupported.md#multi-user-and-collaboration).

[Manual index](../README.md) · Next: [Known issues](../known-issues.md) →
