← Previous: [Settings, themes and languages](09-settings-and-appearance.md) · [Manual index](../README.md)

# The command line

KNXBench ships a headless binary called `knx`. It is not a cut-down version of the
application — it runs the same domain code — and it exists partly so that importing,
exporting and comparing projects can happen in a script or a CI job with no display
attached.

Run it with no arguments and it prints its own usage. Run `knx --version` (or `-V`) and
it prints its version, for example:

```text
knx 0.1.0-alpha.1+ge2e539a
```

If you built from source, the binary is at `target/debug/knx` or `target/release/knx`;
`cargo run -p knx-cli --` works too. See
[Building from source](../development/02-building-from-source.md).

## Before you go near the bus commands

Six subcommands live under `knx bus`, and **every one of them puts traffic on a real
KNX installation**. They are listed together near the end of this chapter, under a
heading that says so. `knx device download` writes to a device; it has its own section
at the very end. Everything before those headings touches files only.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Success. For `import` and `ga-import`, warnings still count as success |
| `1` | Failure: bad arguments, an I/O problem, a transport problem, or no usable data |
| `2` | `import` and `ga-import` only: a project was produced, but the report contains errors |

Exit code `2` is the interesting one. It means KNXBench got a project out of your file
but had to record things it could not use — a dangling reference, a duplicate id. In CI
that is the difference between "no project" and "a project with known holes in it", and
neither should be mistaken for a clean run. No other subcommand, including every `bus`
one, ever returns `2`.

## Working with projects

### `knx import` — read an ETS project

```bash
knx import house.knxproj --store house.knxdb --report-json import-report.json
```

Reads a `.knxproj` ETS project archive and writes KNXBench's own `.knxdb` SQLite
project file.

| Flag | Effect |
| --- | --- |
| `--store <path.knxdb>` | Where to write the project file |
| `--report-json <path.json>` | Write the full import report as JSON |
| `--product-db <path>` | Use this product database instead of the default one |
| `--no-product-db` | Do not touch a product database at all |

It prints a summary:

```text
imported house.knxproj
  38 devices
  212 group addresses
  641 communication objects
  3 unsupported feature(s), 12 opaque entries stored
  0 error(s), 1 unknown construct(s) — see the report for detail
  2 warning(s) — see the report for detail
```

"Opaque entries stored" is KNXBench keeping data it does not yet understand rather
than dropping it. The counts of unsupported features, unknown constructs, errors and
warnings are the ones worth reading; `--report-json` gives you the detail behind them.

### `knx export` — removed on 2026-09-20

There was a `knx export <store.knxdb> <out.knxproj>` subcommand. It is gone, along with
the `.knxproj` writer behind it — see
[ADR-0028](../../adr/0028-no-knxproj-export.md). A script that calls it now fails with
an unknown-subcommand error rather than quietly producing nothing, which is the better
of the two ways to find out. `knx ga-export` and `knx doc-export` below are unaffected;
neither writes an ETS format.

### `knx ga-export` — group addresses to CSV

```bash
knx ga-export house.knxdb group-addresses.csv
```

Prints the output path, a warning count, and each warning on its own line:

```text
exported group addresses to group-addresses.csv
  0 warning(s)
```

### `knx ga-import` — group addresses from CSV

```bash
knx ga-import house.knxdb group-addresses.csv --dry-run
```

Reads KNXBench's group-address CSV back into a project. `--dry-run` parses, plans and
reports without writing anything.

The output always ends with a line saying whether the project file was written, which
is the one line worth checking in a script:

```text
imported group-addresses.csv
  212 row(s) read, 4 created, 9 updated, 0 readdressed, 0 deleted, 199 unchanged
store written: yes
```

That last line is one of `store written: yes`, `no (rejected)`, `no (dry run)`,
`no (error)`, `no (nothing to do)`, `no (destructive confirmation required)` or
`no (stale confirmation)`. Ignored columns and per-row problems are printed above it.

An explicit CSV `readdress` or `delete` never writes on its first invocation. The
preview lists every operation and affected communication-object id/direction and prints a token:

```text
confirmation token: <opaque token>
store written: no (destructive confirmation required)
```

Review the plan, then repeat the exact command with `--confirm <opaque token>`. The token
binds the CSV and loaded project content; editing either one invalidates it, so you
must preview again. KNXBench performs the comparison and save under the same SQLite
write lock, so another writer cannot slip between them. This protects scripted use from
approving yesterday's plan against today's project — the spreadsheet equivalent of
checking the label before flipping the breaker.

### `knx doc-export` — the HTML document

```bash
knx doc-export house.knxdb house-documentation.html
```

Writes the self-contained HTML project documentation described in
[Documentation export and project comparison](08-reports-and-diff.md), then prints the
path, a warning count and each warning.

### `knx diff` — compare two projects

```bash
knx diff house-before.knxdb house-after.knxdb
```

Compares two `.knxdb` projects and prints a tree of differences using `+` for added,
`-` for removed, `~` for changed and `?` for ambiguous. When nothing differs it prints
`no differences found`. The result format is described in
[Documentation export and project comparison](08-reports-and-diff.md).

## Working with product databases

These four subcommands manage the shared product database — manufacturer data, products
and application programs. All of them accept `--product-db <path>` to point at a
specific database instead of the default one. See
[Products and product databases](../knx-basics/05-products-and-product-databases.md).

### `knx products list`

```bash
knx products list --manufacturer M-0083
```

Prints each manufacturer, then each of its application programs indented below:

```text
M-0083  Example Manufacturer
  M-0083_A-1234-1-0000  Switch actuator 8x  application 1234 v1.0  mask 07B0
```

Without `--manufacturer` it lists everything, which on a full database is a long
scroll.

### `knx products ingest`

```bash
knx products ingest catalogue.knxprod
```

Adds product data to the database. It accepts a `.knxproj` project (ingesting the
manufacturer data embedded in it) or a `.knxprod` product package.

For a `.knxprod` package it prints one line covering the scheme, member count, unknown
constructs, conflicts, captured translations and dropped duplicate datapoint types. For
a `.knxproj` it prints:

```text
4 manufacturer file(s) ingested, 1 already known
```

> **Note**
>
> The usage text also lists `file.vd2`. A `.vd2` file is recognized and then refused —
> legacy `.vd2` product data is not supported, and the installer says so rather than
> half-importing it. See [Known issues](../known-issues.md).

### `knx products show`

```bash
knx products show M-0083_A-1234-1-0000
```

Prints one program's identity and its size:

```text
M-0083_A-1234-1-0000  Switch actuator 8x
  manufacturer M-0083  application 1234 v1.0  mask 07B0
  24 communication object(s), 118 parameter(s)
```

### `knx products verify`

```bash
knx products verify
```

Re-hashes every ingested source file and reports any whose content no longer matches
what was recorded. It prints one line per mismatch, then a count:

```text
0 mismatch(es)
```

A mismatch exits with `1`, so this works as an integrity check in a scheduled job.

## Talking to a bus

> **Warning**
>
> Every command in this section sends KNX traffic to the installation your interface is
> connected to. `knx bus write` and `knx bus route-send` operate real equipment: lights,
> blinds, valves, heating. `knx bus scan` sends management frames to every address in
> the range you give it. Know what is connected, and who is in the building, before you
> run any of them.
>
> The addresses in the examples below are placeholders. `192.0.2.1` is a documentation
> address from a reserved range and belongs to no real network; `1/2/3` and `1.1.0` are
> generic. Substitute your own knowingly — never by copy and paste.

KNXBench never writes anything into a device: no application programs, no parameters, no
individual addresses, no commissioning of any kind. See the boundary section in
[Bus monitor and KNXnet/IP](07-bus-and-interfaces.md) for the full statement and the
guard rails behind it.

### `knx bus discover` — find interfaces

```bash
knx bus discover
```

Sends a multicast search and prints every KNXnet/IP interface that answers:

```text
15.15.0  KNX IP Interface  192.0.2.1:3671  [tunnelling]
```

An empty result prints `no gateways responded` and a hint to standard error, because
the usual cause is a network that does not carry your multicast — running inside a
container on Docker's default bridge, for instance. It still exits `0`: "nobody
answered" is an answer.

Discovery reads only. It changes nothing.

### `knx bus monitor` — watch telegrams over tunneling

```bash
knx bus monitor --gateway 192.0.2.1:3671 --project house.knxdb
```

Opens a tunneling connection and prints every telegram until you press Ctrl-C. It first
reports the connection and the individual address the interface assigned it, on
standard error:

```text
connected to 192.0.2.1:3671, assigned individual address 15.15.250. Ctrl-C to stop.
```

Then one line per telegram, source first:

```text
1.1.4 -> 1/2/3 (Kitchen ceiling light): GroupValueWrite 1
```

With `--project`, each destination is printed in the project's group-address style
(three-level, two-level or free), annotated with its name from the project, and the
value is decoded against that address's resolved datapoint type. If several
installations name the same address differently, all names are shown, separated by
` | `. Without `--project`, addresses are three-level and payloads raw. Dropped
telegrams are reported rather than hidden.

`--control` appends the priority and hop count each telegram travelled with, and marks
a telegram the medium repeated:

```text
1.1.4 -> 1/2/3 (Kitchen ceiling light): GroupValueWrite 1 [priority normal, hop count 6]
```

Without `--control` the line is exactly as above. The web monitor's JSON carries the
same fields as `control` (`priority`, `repeated`, `hopCount`); the table does not show
them yet.

Monitoring reads only.

### `knx bus write` — send a group value over tunneling

**This operates real equipment.** Try `--dry-run` first; it encodes the value and
prints exactly what would be sent, without opening any connection:

```bash
knx bus write --gateway 192.0.2.1:3671 --dpt DPST-1-1 --dry-run 1/2/3 1
```

```text
1/2/3 DPST-1-1 1 -> 0x01 (6-bit)
```

| Flag | Effect |
| --- | --- |
| `--gateway <host:port>` | The KNXnet/IP interface to connect to |
| `--project <path.knxdb>` | Resolve the DPT from the address's linked communication objects |
| `--dpt <DPST-m-s>` | Encode the value as this datapoint type explicitly |
| `--dry-run` | Encode and print; open no connection, send nothing |

With neither `--project` nor `--dpt`, the value falls back to raw `0`, `1` or hex.
With `--project`, type the group address in that project's style (for example `2049`
in a free-style project); without it, three-level. Everything — project, address,
datapoint type, value — is parsed and validated before a socket is opened, so a typo
fails without touching the network.

Without `--dry-run` the command connects, sends a group-value write, disconnects and
prints `wrote to 1/2/3`. Tunneling is confirmed, so a failure is reported rather than
assumed away.

### `knx bus route-monitor` — watch telegrams over routing

```bash
knx bus route-monitor --source-address 1.1.250 --project house.knxdb
```

Joins the KNX routing multicast group and prints telegrams in the same format as
`knx bus monitor`, including the project's address style and names. On Linux it
receives only the group it joined, not other groups joined elsewhere on the same
machine. `--source-address` is the individual address this machine uses on
the bus; pick one that no device owns. `--multicast-group <addr>` joins a different
IPv4 multicast address; omitted, it joins the standard `224.0.23.12`. The port is fixed
at `3671` either way.

Routing monitoring reads only.

### `knx bus route-send` — send a group value over routing

**This operates real equipment, and nothing confirms it.**

```bash
knx bus route-send --source-address 1.1.250 1/2/3 1
```

Sends one group-value write to the routing multicast group and prints `sent to 1/2/3`.
Note the wording: *sent*, not *wrote*. Routing is unconfirmed — the command can only
tell you the packet left this machine, not that any device received or acted on it. It
takes the same `--multicast-group` flag as `route-monitor`, and the value is raw `0`,
`1` or hex. There is no `--dry-run` here.

### `knx bus scan` — find which addresses are occupied

**This sends management frames to every address in the range.**

```bash
knx bus scan --gateway 192.0.2.1:3671 --line 1.1 --dry-run
```

Probes each candidate address on one line and reports what answered. Start with
`--dry-run`, which prints the candidate count, the first and last candidate and the
excluded list, then exits without opening a connection.

| Flag | Effect |
| --- | --- |
| `--gateway <host:port>` | The interface to connect through |
| `--line <area.line>` | Which line to scan |
| `--range <first>-<last>` | Two full addresses on that line; the first may not be device `0`, the line coupler's own address |
| `--exclude <addr>[,<addr>...]` | Never probe these. May be given repeatedly; every occurrence accumulates |
| `--timeout-ms <n>` | How long to wait for an answer per address |
| `--pause-ms <n>` | How long to wait between probes |
| `--project <path.knxdb>` | Print a comparison against the project |
| `--dry-run` | Print the plan and exit |

Each address probed prints as it resolves:

```text
1.1.4  occupied (mask 0x07b0)  84ms
1.1.5  vacant  6012ms
```

The outcome labels are `occupied`, `occupied-silent`, `busy`, `vacant`,
`indeterminate` and `self` — and they are deliberately not merged. "Occupied-silent"
means a device is there but did not return a descriptor; "indeterminate" means the scan
does not know, and says so rather than guessing `vacant`. A summary follows, naming the
policy actually used:

```text
24 probed: 11 occupied, 0 occupied-silent, 0 busy, 12 vacant, 0 indeterminate, 1 self, 0 excluded; elapsed 41230ms; policy: timeout=6000ms confirmations=1 pause=100ms
```

With `--project`, the command additionally prints a comparison: addresses the bus
answered that the project does not list, addresses the project lists that did not
answer, and — kept separate on purpose — project devices sitting on addresses that
`--exclude` removed from the scan, which never got the chance to answer at all. The
comparison is printed and never written back into the project.

A malformed `--exclude` address aborts the command before a single frame is sent.
KNXBench also compiles in a list of addresses that can never be probed regardless of
what you type; they are not generated into the candidate list in the first place.

## Writing to a device

### `knx device download` — download a project device's configuration *to the device*

**This rewrites a device's application, tables and parameters.** "Download" in KNXBench
always means *from KNXBench to the device, over the bus*; saving a project to a file is
never called a download (see the [glossary](../../GLOSSARY.md)).

```bash
knx device download 1.1.67 --project house.knxdb
```

Without `--confirm` the command only prints the plan and opens no connection: the
project device at that address, its application program, mask and manufacturer, each
memory segment with the octets that will be written and the octets the device keeps
(its own individual address), and every step of the product's load procedure:

```text
== download to device 1.1.67: plan (nothing sent yet) ==
device:  Push button 2-fold Plus (project device 1)
program: M-0083_A-0027-15-0BAC
mask 0701h, manufacturer 0083h
configuration: 3 parameter values, 1 group links
segments (octets the device keeps itself are not written):
  M-0083_A-0027-15-0BAC_AS-4000 at 4000h: 513 octets, 511 written, 2 kept
  ...
octets written to the device: 1416 (every one is read back)
steps: 25
   1: connect; check mask and manufacturer
  ...
written to the device: no (plan only; add --gateway and --confirm "I confirm download to 1.1.67" to write)
```

| Flag | Effect |
| --- | --- |
| `--project <path.knxdb>` | The saved project the configuration comes from (required) |
| `--product-db <path>` | The product database holding the device's application program |
| `--gateway <host:port>` | The KNXnet/IP interface to write through |
| `--confirm "<phrase>"` | Write. The phrase must read exactly `I confirm download to <address>` |

The checks run in this order, and each stops the command before the next: the address
is parsed and checked against the list of addresses KNXBench never contacts; the phrase
is compared with the one for *this* address; the project and product database are read
and the plan is built (a missing project file is an error, never a new empty one); only
then is a connection opened. The configuration is taken from the project and nothing
is filled in: a device without an address or program, two devices on the same address,
contradictory values or a link on an object the parameters do not activate are refused
by name.

While writing, every step and every data block sent to the device is printed as it
happens, with the running octet count; a block is printed only after it was read back
unchanged. The command ends with one of three lines, and exits `0` only for the first:

- `written to the device: yes, … octets, every one read back`
- `written to the device: no` — it stopped before the first write
- `written to the device: partially (stopped in step …)` — the device may not run until
  a complete download

A download that wrote everything but whose closing restart the device did not
acknowledge still ends `yes`, followed by `restart: NOT confirmed` and what to do about
it. Some devices never acknowledge that restart
([known limitations §136](../../KNOWN_LIMITATIONS.md#136-mask-0701h-bim-m112-devices-cannot-receive-an-application-download)).

Only memory downloads to products whose load procedure KNXBench can plan are supported;
anything else is refused while planning, before a connection opens.

### `knx device program-address` — give a device its individual address

**This changes a device's individual address and restarts it.** It is not a download:
it writes one thing, the address, to whichever device is in programming mode.

**Current safety gate (2026-10-01):** the plan-only form below remains
read-only. Even with the correct `--confirm`, this build refuses before
opening a tunnel because it has no verified durable pre-write recovery for
the button-selected device. Do not press a device's programming button
expecting this command to write it. The confirmed example and output below
describe retained simulator behavior, not an available live path.

```bash
knx device program-address 1.1.30
knx device program-address 1.1.30 --gateway 192.0.2.1:3671 \
    --confirm "I confirm individual-address programming to 1.1.30"
```

Without `--confirm` it prints the steps and opens no connection. Once
device-specific recovery is implemented, a confirmed command would ask the
bus about every two seconds which devices are in programming mode:

```text
== program individual address 1.1.30: waiting for a programming button ==
  [round 1] no device is in programming mode — press the programming button on the device to address
  [round 4] 2 devices are in programming mode (1.1.5, 1.1.9) — release all but one
  [round 6] 1 device is in programming mode (1.1.5)
  found 1.1.5 in programming mode; programming 1.1.30
== program individual address 1.1.30: finished ==
1.1.30 was free before
address written: yes, 1.1.5 -> 1.1.30; the device answered at 1.1.30
restart: acknowledged
```

Some devices never acknowledge the closing restart, and restart anyway (a real MDT push
button does). The last line then reads `restart: NOT confirmed (…)`. The address itself is
confirmed: the device answered at it before the restart was sent. If its programming LED is
still on, press the button once to end programming mode.

Then it runs the four steps of the KNX procedure (MP §2.3): it checks that no *other*
device already has the new address and stops if one does; counts again; writes the
address; connects to the device at its new address, reads it back and restarts it, which
ends programming mode. A device that already has the address is not written again.

| Flag | Effect |
| --- | --- |
| `--wait <seconds>` | How long to wait for exactly one pressed button (default 120, at most 600). Waiting only reads |
| `--gateway <host:port>` | The KNXnet/IP interface to use |
| `--confirm "<phrase>"` | Requests programming only after durable recovery exists; currently refused before a tunnel. The phrase must name the exact address and also covers the closing restart |

When available in a future recovery-ready version, it exits `0` only when the
device answered at its new address. The last line then says
whether the address changed: `address written: no` when it gave up, found the address
taken, or lost the button before writing; and `address written: yes, but NOT confirmed`
when the write went out but the device did not answer at the new address afterwards.
In that last case, read the device before doing anything else; recovery is another
programming-mode session.

[Manual index](../README.md) · Next: [Web and Docker deployment](11-web-and-docker.md) →
