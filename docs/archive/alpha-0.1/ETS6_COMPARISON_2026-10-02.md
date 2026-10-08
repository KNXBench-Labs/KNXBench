# KNXBench vs. ETS6

*Snapshot: October 2, 2026. Current capabilities, not roadmap wishful thinking.*

KNXBench is an independent, Linux-first alpha—not a certified or fully compatible ETS replacement. Repository evidence: [overview](../../../README.md), [implementation status](../../IMPLEMENTATION_STATUS.md), [limitations](../../KNOWN_LIMITATIONS.md), and [compatibility](../../COMPATIBILITY.md).

| Topic | KNXBench | ETS6 | Practical verdict |
| --- | --- | --- | --- |
| Linux | Linux-first desktop plus browser access. | Windows application; Linux is unsupported.[1] | **KNXBench advantage:** Windows can stay outside the building. |
| Cost & transparency | AGPL-3.0-or-later source; no paid application license. | Commercial licenses; project/device limits depend on the edition.[2] | **KNXBench advantage:** inspect the code, spare the license budget. Build/support effort is not free. |
| Deployment & tooling | Desktop, CLI, HTTP API, browser and Docker workflows. | Windows desktop installation.[1] | **KNXBench advantage:** more comfortable in a Linux/server toolbox. Browser access is not collaborative editing. |
| Everyday project editing | Topology, group addresses, links, supported parameters, undo/redo and search. | Topology, parameters, group addresses and full engineering workflows.[9] | **Mixed:** KNXBench covers useful daily work, not every manufacturer's configuration semantics. |
| Returning to ETS | Reads supported `.knxproj`; saves `.knxdb`. **No `.knxproj` export.** | Exports `.knxproj` for compatible ETS versions.[7] | **KNXBench disadvantage:** a one-way door, not a revolving one. |
| Import confidence | Explicit unknown/unsupported-data reports; retained source bytes. Schema and encrypted-project coverage remain limited. | Native ETS project workflow.[7][9] | **Mixed:** transparency is useful; preserving bytes does not make unsupported behavior work. |
| Manufacturer ecosystem | Local product database and supported `.knxprod` imports; no vendor plug-in execution. | Manufacturer product/application data and vendor plug-ins.[1][9] | **ETS advantage:** broader device workflows; KNXBench cannot run a DLL by politely asking it. |
| Commissioning | Experimental, device-specific download evidence; address-write paths remain safety-gated. | Downloads addresses, applications, group tables and parameters.[3] | **ETS advantage:** KNXBench is not a general production commissioning replacement. |
| KNX Secure | No operational Data Secure or IP Secure support. | Supports secure commissioning and Data/IP Secure.[10] | **ETS advantage:** this is a missing capability, not a checkbox with stage fright. |
| Maturity & delivery | Active alpha; no published release. Native-platform acceptance gaps remain. | Official KNX Association engineering tool.[9] | **ETS advantage:** KNXBench still needs more field evidence and release hardening. |

**Bottom line:** KNXBench is attractive for Linux-based inspection, supported editing and automation. Keep ETS for broad commissioning, secure installations, plug-in-dependent devices and ETS project handoff. No performance superiority is claimed—benchmarks, unlike enthusiasm, need measurements.

## Sources

[1] https://support.knx.org/hc/en-us/articles/360018452180-ETS6-Software-requirements
[2] https://support.knx.org/hc/en-us/articles/21546945829010-License-types-and-prices
[3] https://support.knx.org/hc/en-us/articles/360007474340-Download-functions
[7] https://support.knx.org/hc/en-us/articles/360020990259-Project-export
[9] https://support.knx.org/hc/en-us/articles/21037139544722-What-is-ETS
[10] https://support.knx.org/hc/en-us/articles/115001825964-Security
