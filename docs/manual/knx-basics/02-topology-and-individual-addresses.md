← Previous: [KNX in a few minutes](01-knx-in-a-few-minutes.md) · [Manual index](../README.md)

# Topology and individual addresses

Topology is the physical/logical shape of an installation: which devices share a wire
(or a routed IP segment), and how those segments connect to each other. An individual
address names one device's place in that shape. Neither of these is the thing a light
switch sends when you press it — that's a group address, covered in the next chapter.

## Areas, lines, devices

A KNX installation is organized into up to three levels:

- An **area** groups lines together.
- A **line** is a physical (or routed) segment that devices sit on.
- A **device** is one physical piece of hardware on a line.

A project can hold more than one **installation**, each with its own topology — normal
for a project covering several buildings or a phased rollout, not something you need to
worry about for a single small installation.

A device does not strictly need a line at all. KNXBench treats a device with no line
assignment as valid project state, not an error — you'll see this on partially planned
projects, or ones an import brought in with gaps.

## The `A.L.D` notation

An individual address is written `area.line.device`, e.g. `1.3.12`: area 1, line 3,
device 12. KNXBench parses and formats it exactly this way — parsing `"1.3.12"` and
immediately formatting the result reproduces the identical string — and that round trip
is covered by the domain model's own test suite.

Worked example: `2.4.7` is the seventh device on the fourth line of the second area. Two
addresses that share an area and line, like `2.4.7` and `2.4.8`, sit on the same physical
segment; two that share only an area, like `2.4.7` and `2.1.3`, sit on different lines the
same area's couplers connect.

## Why the structure exists: couplers

Areas and lines are not just an organizational convenience — a KNX bus segment has a
practical size limit, so larger installations are built from several segments joined by
**couplers**, devices that sit between two segments and forward traffic that needs to
cross. A line coupler joins a line to the area's backbone; without couplers, "areas and
lines" would just be "one very long line."

This shows up directly in address handling: on any line, device address `0` is
conventionally the line coupler's own address, not a device you would probe or assign to
ordinary equipment — KNXBench's bus line-scan feature deliberately excludes it from the
range it walks for exactly this reason.

## What an individual address is for — and isn't

An individual address identifies one device for **engineering and commissioning**
purposes: which device to load a program onto, which device a diagnostic reading came
from, which device to reach when troubleshooting. It is not what a sensor puts on the
bus when it reports something — that message carries a group address, and in the
ordinary case nothing about the sending device's own individual address matters to
whoever is listening.

> **Note**
>
> KNXBench's bus layer keeps a short, hard-coded list of individual addresses it will
> never contact under any circumstances — read, write, probe, or otherwise — regardless
> of what a scan or a project happens to name. On the installation this project is
> developed against, that list holds exactly one address, an alarm panel. It exists
> because "never touch this one address" is a safety requirement, not a suggestion, and
> the guard lives at the lowest layer that understands what an individual address is, so
> nothing built on top of it can forget to check.

## Valid ranges

KNXBench enforces the ranges the individual address format has room for: area and line
are each 4-bit fields, so both must be `0`–`15`; device is an 8-bit field, so it's
`0`–`255` and every value in that range is structurally valid. An address like `16.0.0`
or `3.20.0` is rejected as out of range; something that isn't three dot-separated numbers
at all, like `1.2`, is rejected as malformed. Neither of those two error kinds is
optional — a bad individual address is refused, not silently clamped or guessed at.

Where you'll actually assign and view these: see
[Buildings and topology](../user-guide/03-buildings-and-topology.md) for the topology
tree and device editing in KNXBench itself.

[Manual index](../README.md) · Next: [Group addresses](03-group-addresses.md) →
