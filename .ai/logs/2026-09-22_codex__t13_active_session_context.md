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
