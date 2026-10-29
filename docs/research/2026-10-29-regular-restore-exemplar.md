# Regular restore exemplar binding: source review

29 October 2026, PRG-01. Original KNX Standard v3.0.0 resources were inspected
from the authorized local corpus. Licensed text and the rendered profile table
remain private; this note paraphrases only the contract-relevant facts.

| Source | Clause / printed page | Established fact |
| --- | --- | --- |
| `03_05_01 Resources v01.10.01 AS.pdf` | §4.2.11, p33 | PID 11 is the serial-number property, PDT_GENERIC_06, realizing the device serial resource. |
| Same | §4.22.1.1–1.2, p291 | Device-unique identifier within the manufacturer's code space; six octets, two manufacturer and four device-number octets. |
| Same | §4.22.1.3, p292; §4.22.3, p293 | Property and serial-address service support are related and profile-dependent; realization type 2 is the PID 11 property. |
| `06 Profiles v02.01.01.pdf` | Annex A.2.3, p138 | PID 11 cells for exact masks 0701h and 0705h are respectively 15/x and 3/x; the generic System 7 cell is optional. The actual table and footer were rendered and checked. |
| Same | Annex A.1.2.1, p133 | Parentheses distinguish optional support; x denotes read-only with privileged-access qualifications. |

Do not extend the exact-mask mandatory-support claim to all 070nh layouts.
Neither a matching serial response nor read-only access proves physical identity
against a misreporting or privilegedly modified device.

## Original gap and policy

Before this increment, the regular backup recorded address, product, prior
application, load states and written regions, but no captured serial. Its restore
could therefore replay one device's memory onto a replacement with the same
address/product/application. The [software-accepted implementation](../status/2026-10-29-regular-restore-exemplar.md)
adds the bounded replay precondition; the source facts and policy distinction
below remain unchanged.
The existing property-recovery contract already binds replay to the captured
serial and refuses unbindable all-zero/all-FF evidence
([configured recovery policy](../contracts/alpha16-system2-configured-download.md)).
Applying that necessary replay safety condition to regular restore is a project
policy, not a claim that KNX mandates KNXBench's backup format or sentinel rules.
Serial remains an observation for ordinary download admission.

A caller-only optional restore check is insufficient: the lowest public restore
derivation must return a plan that enforces its captured-exemplar precondition.
A dedicated typed precondition also avoids the generic padded InlineData
comparison and keeps manufacturer procedure identity distinct from runtime
replay binding. Backups taken during restore must retain canonical source
procedure identity rather than accidentally making the next restore impossible.

The [implementation contract](../contracts/regular-restore-exemplar-binding.md)
defines strict response correlation, persistence, compatibility narrowing,
consumers and executable acceptance. No whole-mutation, hardware/function or
broader layout support is admitted.
