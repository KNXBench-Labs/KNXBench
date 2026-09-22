# T13 Task 2: active-session group-address context

`BusSession` now shares one atomically replaceable `GroupAddressContext`
between the telegram drain and write paths. A successful
`POST /api/project/group-address-style` mutation rebuilds the whole context
under the project mutex, releases that mutex, and then updates an active
session under the async bus-session mutex. No locks are nested and the update
performs no tunnel lifecycle or traffic operation.

The route-level fake-session regression observed the intended RED
(`"0/0/1"` instead of `"1"`) before implementation. Final task gates passed:
11 bus unit tests, 14 write integration tests, 17 edit-route tests, warning-
denied server Clippy, formatting, 389 anchor links, and whitespace validation.
The full `knx-server` suite also passed. No KNX, LAN, multicast, gateway, or
hardware traffic occurred.

## Fix round 1

Review found concurrent restyle requests could snapshot in accepted order but
publish in async bus-lock order, allowing delayed A to overwrite newer B. The
route sequence is now split into prepare/publish phases with a monotonic
application-state revision assigned before snapshotting; publication under the
bus-session lock proceeds only for the latest accepted revision. A deterministic
test prepares A/Free, fully publishes B/TwoLevel, then resumes A and proves both
project and session remain TwoLevel, with one fake connection and no sends or
disconnect. RED was session `Some(Free)` versus `Some(TwoLevel)`; GREEN 1/1.
Full server, 11 bus unit, 14 write-route, 17 edit-route, Clippy and fmt gates
passed using `/var/tmp/knxbench-t13-target`. No hardware or network traffic.
