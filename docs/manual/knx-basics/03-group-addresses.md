← Previous: [Topology and individual addresses](02-topology-and-individual-addresses.md) · [Manual index](../README.md)

# Group addresses

Group addresses are where KNX starts looking slightly less like normal networking. Don't
worry — after one worked example it gets considerably less mysterious, and this is the
concept the rest of the manual leans on the most.

## Not a device address

A group address does not name a device, and it does not name a sender or a recipient. It
names **what a message is about** — "the living room ceiling light," "the corridor
temperature." Any device configured to send on that address puts a message there; any
device configured to listen reacts, regardless of how many are listening or which one
happened to send. This is exactly the opposite of an individual address (previous
chapter), which does name one specific device.

## One switch, two actuators, one group address

Say a hallway has a switch and two ceiling lights on separate circuits, both meant to
turn on and off together. You would not wire the switch to each actuator individually.
Instead:

1. Create one group address, say `1/1/1`, named something like "Hallway lights."
2. On the switch, its "Switching" communication object **sends** to `1/1/1`.
3. On each of the two actuator channels, its "Switching" communication object
   **receives** from `1/1/1`.

Press the switch, and it sends one message to `1/1/1`. Both actuators are listening on
that address, so both react — no actuator needed to know the other exists, and the switch
needed to know about neither. Add a third actuator later by linking it to the same
address, and nothing about the switch or the existing actuators changes.

## What links a communication object to a group address

The connection point on a device is its **communication object** — a Send or Receive slot
for one specific kind of value (e.g. "Switching," "Dimming value," "Current temperature").
A **group link** is the fact that one particular communication object sends or receives
on one particular group address; a communication object can hold more than one link, and
the same group address can be linked from as many communication objects, in either
direction, as the project needs. KNXBench's group-address view shows exactly this: for
each address, how many communication objects link to it, split into senders and
receivers.

## Three styles, one 16-bit number underneath

A group address is a 16-bit number. What changes between projects is only how that number
is *displayed and typed* — KNXBench calls this the project's group address style, and it
applies to the whole project, not per-address:

| Style | Written as | Bit layout |
| --- | --- | --- |
| Free | plain decimal, `0`–`65535` | the raw 16 bits, unsplit |
| Two-level | `main/sub` | 5 bits main (`0`–`31`) + 11 bits sub (`0`–`2047`) |
| Three-level | `main/middle/sub` | 5 bits main (`0`–`31`) + 3 bits middle (`0`–`7`) + 8 bits sub (`0`–`255`) |

Three-level is the style KNXBench starts a new project with. Switching a project's style
never changes which addresses exist — every one of the 65,536 possible 16-bit values
parses and formats losslessly under all three styles, which is exactly what you'd want
from a display choice that isn't supposed to be a capacity limit. Group ranges (the named
groupings you see in the group address tree, like "Lighting" or "Heating") nest up to two
levels deep, matching the main/middle structure.

## How a group address ends up carrying a datapoint type

A group address has no datapoint type field of its own — only the communication objects
linked to it do, and they state it (or don't). KNXBench works out a group address's
effective type by looking at every communication object linked to it, in either
direction, and:

- if none of them state a usable type, the address has none — this is the ordinary case
  for a fair number of addresses in a typical project, not a defect;
- if they all agree, that's the address's type;
- if they disagree, KNXBench reports a **conflict** naming every type stated, rather than
  guessing which one is "right."

That last point matters in practice: two devices from different manufacturers, or two
different application program versions, occasionally do disagree about what a shared
group address carries. KNXBench surfaces that as a conflict instead of silently picking
one side. The next chapter,
[Datapoint types](04-datapoint-types.md), explains what those types actually are.

See [Working with group addresses](../user-guide/04-group-addresses.md) for the group
address table, its filters, and the CSV import/export built on top of all this.

[Manual index](../README.md) · Next: [Datapoint types](04-datapoint-types.md) →
