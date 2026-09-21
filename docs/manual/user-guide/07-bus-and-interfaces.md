← Previous: [A complete configuration workflow](06-configuration-workflow.md) · [Manual index](../README.md)

# Bus monitor and KNXnet/IP

Everything so far happened inside a file. This chapter is the one where KNXBench
talks to an actual installation — and, just as importantly, the chapter that says
plainly what it will not do there.

## What KNXnet/IP is

A KNX installation is a twisted-pair bus that no computer can plug into directly.
KNXnet/IP is the standard that puts KNX telegrams inside IP packets, so a KNX IP
interface or router on your network can act as the door between the two.

KNXBench never speaks to the bus itself. It speaks to that interface over IP, and
the interface does the electrical part.

## Discovery: finding an interface

A KNXnet/IP interface announces itself when asked. KNXBench sends a multicast
`SEARCH_REQUEST` to the standard group `224.0.23.12` on port `3671`, and every
interface that hears it answers with its name and address.

Discovery is available in the UI and on the command line:

```bash
knx bus discover
```

The graphical bus monitor offers **Discover gateways** through the same
server-side discovery call. The CLI remains useful for terminal diagnostics.
Line scan still requires a known control endpoint to be entered manually.

> **Note**
>
> An empty discovery result usually means the request never left your machine rather
> than that you own no interfaces. Multicast and Docker's default bridge network get
> along about as well as two cats in one carrier — see
> [Web and Docker deployment](11-web-and-docker.md).

## Tunneling and routing

There are two ways to carry KNX over IP, and KNXBench treats them as two different
things because they are.

**Tunneling** is a point-to-point conversation with one interface. You connect to a
host and port, the interface hands your session its own individual address, and it
forwards telegrams in both directions. It works across normal switched networks, and
it is confirmed: the interface tells you whether your frame made it.

**Routing** is multicast. Frames go to a group address on the network and every KNX
IP router listening there picks them up. Nothing confirms anything. It needs a network
that actually carries multicast between the machines involved.

The graphical bus monitor supports **tunneling only**. Routing is command line only,
through `knx bus route-monitor` and `knx bus route-send`, both described in
[The command line](10-command-line.md).

## Connecting the bus monitor

Open **Bus monitor** from the navigation sidebar. Type your interface's address into
the **Gateway address** field as `host:port` and press **Connect**.

![The KNXBench bus monitor before a session starts, with the gateway address field and the Connect button](../../assets/screenshots/porcelain-bus-monitor.png)

Notice the eyebrow above the title: *KNXnet/IP · Tunnelling*. That is the panel
telling you which transport it uses, and it is the only one it offers. The telegram
table below is empty because no session is running yet.

While a session runs, the gateway field is locked — KNXBench tells you to *"Disconnect
the running session before changing the gateway address."* Once connected, the panel
shows `Session <id>`, followed by the individual address the interface assigned to
your session.

## What the monitor shows

Every telegram the interface forwards becomes one row, with these columns:

| Column | What it holds |
| --- | --- |
| Seq | A running number, so gaps are visible |
| Time | When KNXBench received the telegram |
| Source | The individual address that sent it |
| Destination | The group address (or individual address) it went to |
| Service | The KNX service, for example a group-value write |
| Payload | The raw bytes |
| Decoded | The value, interpreted as a datapoint type |

The **Decoded** column is the interesting one. If a project is open when you connect,
KNXBench takes a snapshot of that project's group addresses and their datapoint types
and decodes each telegram against it. A raw `0x01` becomes something you can read. See
[Datapoint types](../knx-basics/04-datapoint-types.md) for what those types are.

That snapshot is frozen at connect time and never re-resolved, and KNXBench says so
rather than quietly lying to you. Edit the project while a session runs and a notice
appears: *"The project changed after this session started. Decoded values below come
from the snapshot taken at connect time, and sending is locked. Reconnect to decode
against the current project."*

A filter box above the table narrows rows by destination or name. If telegrams arrive
faster than the buffer can keep them, KNXBench reports how many were lost instead of
hiding the gap.

## Sending a value

The bus monitor includes a **Send a value** form, with three fields — Destination,
DPT, Value — and a **Send** button.

This form transmits a group-value write to the connected installation. Lights change,
blinds move, heating setpoints shift. The application says it in the form itself:
*"Sends to the connected bus. Project Undo cannot reverse this action."* KNXBench's
undo history covers the project file, not the physical world.

Before you use it, be certain that the group address you typed is the one you mean, and
that operating it now is safe for whoever is in the building. Sending is disabled when
the session is closed, and also when the project snapshot has gone stale, because a
value encoded against an out-of-date datapoint type is a value you did not intend to
send.

Clicking a row in the telegram table prefills the form's destination with that row's
address. It does not send anything.

## The session log and the diagnostics window

The **Log** entry in the sidebar opens the session log: everything KNXBench did this
session, filtered by Error, Warning and Info.

![The KNXBench session log with Error, Warning and Info filters and one info entry recording an opened project](../../assets/screenshots/porcelain-log.png)

Log entries are the server's own text and stay in English even when the interface is
translated — a deliberate choice, so an error message you paste into a bug report
still means the same thing to whoever reads it.

The **Diagnostics window** button opens the bus monitor and the log in a second window,
which is useful on two screens: watch the bus on one, edit the project on the other.
That window is read-only with respect to the project. It has no undo, it shares the
main window's bus session rather than opening a second one, and it cannot edit
anything. It can still connect and still send a value — the restriction is about the
project, not about the bus.

## Ending a session

Press **Disconnect**. KNXBench prints a summary of what the session saw, in the form
*"Stopped session 3: 412 telegrams seen, 0 dropped."*

A session can also end without you: the interface can close it — the panel adds
*"— closed by gateway"* — or the other window can end it, in which case you see *"The
bus session was ended elsewhere."* There is one session at a time; starting a new one
replaces the old one and clears its rows.

## What KNXBench does and does not do on a bus

This section is deliberately plain.

**What KNXBench reads from a bus.** Telegrams that the interface forwards to it. The
individual address an interface assigns to a tunneling session. During a line scan, the
device descriptor that a device returns when asked. That is the complete list.

**What KNXBench writes to a bus.** Group-value writes, and only group-value writes,
from exactly three places: the `Send a value` form in the bus monitor, the `knx bus
write` command over tunneling, and the `knx bus route-send` command over routing. A
line scan additionally sends connection-oriented management frames — a connect, a
device-descriptor read, a disconnect — to each candidate address. Those are traffic on
the bus, and they occupy the addressed device briefly, but they do not change anything
in it.

**What KNXBench does not do at all.** It does not program devices. It does not download
an application program, parameters, group links, or an individual address into any
device. It does not put a device into programming mode, read or write device memory, or
perform any part of commissioning. None of that exists in KNXBench today, in the user
interface or on the command line, and it is not hiding behind a flag. If you need to
commission an installation, you need a tool that does commissioning; this is not one.

**The guard rails that actually exist in the code.**

- There is a management-session layer in the networking crate that could in principle
  write to a device. It refuses twice over: a session that was not handed a write
  authorisation returns an error before touching the transport, and a session whose
  transport is not a simulator refuses to write at all. No command, no HTTP route and
  no button reaches that layer.
- KNXBench compiles in a list of individual addresses that must never be contacted. The
  scan planner never generates them into a candidate list in the first place, and the
  address type used for probing cannot be constructed without passing that check — so
  "we forgot to check" is not a reachable state rather than a discipline.
- The line scan's `--exclude` flag accumulates across every occurrence, and a malformed
  address aborts the command before a single frame is sent.
- Both `knx bus write` and `knx bus scan` accept `--dry-run`, which encodes and prints
  exactly what would go out without opening a connection.
- The HTTP server exposes four bus routes and no more: start a monitor, stop it, poll
  telegrams, write a group value.

For the longer catalogue of what is missing and why, see
[Supported and unsupported KNX/ETS functionality](../reference/02-supported-and-unsupported.md)
and the project's own [known limitations](../../KNOWN_LIMITATIONS.md).

[Manual index](../README.md) · Next: [Documentation export and project comparison](08-reports-and-diff.md) →
