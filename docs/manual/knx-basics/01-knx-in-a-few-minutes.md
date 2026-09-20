← Previous: [First start](../getting-started/06-first-start.md) · [Manual index](../README.md)

# KNX in a few minutes

This is not a KNX history lecture. It is the minimum you need to make sense of the rest
of this manual: what the bus is, what the two kinds of address are for, and why you spend
most of your time editing a file instead of a building.

## A bus, not a control box

A KNX installation is a set of devices wired to (or, over IP, routed onto) one shared
bus. There is no central controller running the show. A light switch does not send a
command to a "gateway" that then decides what to do — it puts a message on the bus, and
every device that cares about that message reacts to it. A dimming actuator does not ask
permission to dim; it simply listens for the messages it has been configured to react to.

This has one practical consequence that matters immediately: a KNX device is either a
**sensor** (it sends — a switch, a motion detector, a temperature sensor) or an
**actuator** (it receives and acts — a relay, a dimmer, a valve drive), and quite a few
devices are both at once. Nothing in this arrangement needs to know about anything else
by name. It only needs to agree on which messages matter to it, which is exactly the job
of the group address (next chapter).

## Two questions, two kinds of address

KNX asks two different questions about every device and every message, and it uses a
different kind of address for each:

- **"Which device is this, for engineering purposes?"** Answered by the device's
  **individual address**, written `1.1.5`-style. You use it to reach one specific device
  when configuring it — loading its program, checking its status. See
  [Topology and individual addresses](02-topology-and-individual-addresses.md).
- **"What is this message about?"** Answered by a **group address**, written
  `1/2/3`-style (or two-level, or plain decimal — see the next chapter after that). A
  switch does not send to a device; it sends to a group address, and whichever actuators
  are configured to listen react. See [Group addresses](03-group-addresses.md).

Mixing these two up is the single most common source of confusion for anyone new to KNX.
A light switch pressing "on" does not know or care which actuator answers it — and
that's the point.

## Engineering happens offline, in a project

You do not configure a KNX installation live, one device at a time, by walking around
with a laptop and hoping you remember what you did. You build a **project**: a file that
describes every area, line, device, group address and parameter value you intend the
installation to have. You edit that file at your desk, review it, and only then push the
result to the devices (a step called commissioning, which is a later chapter's problem,
not this one). KNXBench's own working format for that file is `.knxdb`; it can also read
the `.knxproj` format that ETS itself uses, though it never writes one. Chapter
[Projects: create, open, import, save, export](../user-guide/02-projects.md) covers the
practical side.

## Where ETS fits in

ETS is the KNX Association's own engineering software, and for a long time it has been
the only serious way to build a KNX project. It works. It is also Windows-only, and
that's the entire reason KNXBench exists — not a grudge against ETS, just a Linux user
who got tired of a virtual machine. KNXBench reads the same `.knxproj` project format ETS
uses — reads it only, and never writes one — and it is its own independent
implementation: its own parser, its own data model, its own product database. It is
**KNX-compatible**; it does not claim to be KNX-certified, it does not claim full ETS
compatibility, and being able to read an ETS file is not the same as replacing ETS.

> **Note**
>
> If a term above was unfamiliar, that's fine — it was supposed to introduce it, not
> explain it fully. The next four chapters go one level deeper on topology, group
> addresses, datapoint types and products, in that order, and each one links straight
> to the screen in KNXBench where the concept turns into something you click on.

[Manual index](../README.md) · Next: [Topology and individual addresses](02-topology-and-individual-addresses.md) →
