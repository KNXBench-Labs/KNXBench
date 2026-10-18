# The maintainer's own house: diagnostic exports from every device

18 October 2026. With the owner's go for read-only bus contact, the shipped
[diagnostic export](../contracts/device-diagnostic-export.md) ran once against
every contactable device of the maintainer's house project (the installation of
[§19.12](commissioning/19-12-the-maintainers-own-house-device-by-device.md)).
This is **identity evidence, not a programming result**: nothing was written, no
download was attempted and no admission changes.

## The run

- Build: `knx 0.1.0-alpha.14+g611c5b7a` (current `main`), the unmodified CLI
  `knx device diagnostics … --confirm-read`, one device after the other, about two
  seconds apart, through the installation's own KNXnet/IP tunnelling interface
  (found by KNXnet/IP search; its network address is not recorded here).
- Project: a read-only copy of the owner's server project (35 devices). No
  product database was passed, so no run carried a mask or manufacturer
  assertion.
- `1.1.220` is on the project exclusion list. It was never contacted: the run
  script skipped it, and the CLI refuses it by itself
  (`diagnostics-target-excluded`, checked offline before the run).
- Before the run no KNXnet/IP tunnel was open on the host, and the shared tunnel
  lock was held for the whole run.
- 34 archives, all `simulated: false`, `written: false`, complete, disconnect
  attempt completed. They stay in the private evidence collection. The gate
  `device_reports_corpus` (house set) reads all 34 through the production reader,
  re-bundles each byte for byte and pins the counts below.

## What the devices answered

| Addresses | Device (project) | Mask | PID 12 | PID 78 | Application's PID 78 check |
| --- | --- | --- | --- | --- | --- |
| 1.1.1–1.1.9 | EIBMARKT presence detector | `0701` | `006A` | `00 00 00 00 00 03` | 10 octets, match |
| 1.1.10–1.1.13 | MDT switch actuator AMS (two application versions) | `0701` | `0083` | `00 00 00 00 00 0A` | 10 octets, match |
| 1.1.14–1.1.15 | MDT push button 4-fold | `0701` | `0083` | `00 00 00 00 01 26` | 10 octets, match |
| 1.1.16–1.1.21, 1.1.32 | MDT push button 8-fold | `0701` | `0083` | `00 00 00 00 01 24` | 10 octets, match |
| 1.1.22–1.1.23 | Gira SmartSensor | `0012` | connection ended by the device | not requested | none (System 1) |
| 1.1.24 | Merten blind/switch actuator | `0701` | `000C` | refused, 0 elements | none declared |
| 1.1.25 | MDT binary input 16-fold | `0701` | `0083` | `00 00 00 00 00 1E` | 10 octets, match |
| 1.1.26 | MDT binary input 8-fold | `0701` | `0083` | `00 00 00 00 01 1E` | 10 octets, match |
| 1.1.27–1.1.31 | MDT dimming actuator | `0701` | `0083` | `00 00 00 00 00 32` | 10 octets, match |
| 1.1.250 | EIBMARKT IP interface | `0701` | `006A` | `00 00 C5 07 00 03` | none declared |
| 1.1.253 | EIBMARKT IP interface ("Gateway") | — | — | — | none declared |

"Match" uses the existing assumed rule `property_matches` (`[A]`, device octets
then a zero tail). The applications are the ones the project's hardware-to-program
references name; the comparison values are read from the house project's own
application XML.

- **29 of 29** devices whose application declares a PID 78 comparison match it:
  eight applications, two manufacturers, all ten-octet data with a four-octet
  zero tail. Together with the [eleven emailed devices](2026-10-18-device-diagnostics-eleven-devices.md)
  and the [UP 258D12](2026-10-18-up258-device-diagnostics.md) that is 37 of 37
  real devices with a declared comparison, five manufacturers, three masks. The
  rule stays `[A]`: no source defines a manufacturer tool's comparison.
- **Refusals line up with the product data again.** The Merten actuator refuses
  PID 78 with zero elements, and its application declares no PID 78 comparison,
  exactly like the two Hager refusals. The same device refused PID 78 in the
  [§8.8.2 run](knxnet-ip-and-bus/8-8-2-raw-observations-by-address.md), so the
  reply is stable across runs. Still `refused-ambiguous`: the reply cannot tell
  denied access from a missing property.
- **The SmartSensors end the connection.** Both `MV-0012` (System 1) sensors
  answer the descriptor (`00 12`, 200 ms), then send `T_Disconnect` about
  6.3 seconds after the PID 12 request (`connection-lost`). PID 78 is never
  requested after that. The tunnel stayed up; the next device answered at once.
  This is the first real observation of the path the contract only assumed
  ("devices without property services … a released connection"). That the
  device ends the connection when its own transport-layer timeout runs out is an
  inference from the timing, not a verified cause.
- **`1.1.253` rejects the connection.** The gateway's `L_Data.con` for
  `T_Connect` was negative (`connect-rejected`) and no read was sent, the same
  outcome as the
  [§8.8.3f run](knxnet-ip-and-bus/8-8-3f-specification-timed-connection-confirmation-fix.md).
  The project lists the address; whether a device answers there is still not
  established.
- **The IP interface `1.1.250`** answers all three reads; its application
  declares no PID 78 comparison.
- Timing: 0.4–0.5 s per device for complete runs (0.1–0.2 s for the two IP
  interface addresses), 6.6–6.7 s for the two SmartSensors.

## What changes

- The export has now read 46 real devices: 12 by email, 34 here. Observed on
  hardware: answered, refused with zero elements, connection ended by the
  device, connection rejected. Never observed: a device that stays completely
  silent until the session timeout.
- Post-run behaviour: nobody has reported a device behaving differently after a
  run. This run cannot show it either; a reply from the owner would.
- No production code changes. The corpus gate gains the house set.
