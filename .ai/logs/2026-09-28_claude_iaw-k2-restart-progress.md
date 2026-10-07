# 2026-09-28 — Claude — K2 + R1/R2: unconfirmed restart, live progress, what "download" means

## K2: loaded, restart unconfirmed
- Spec read directly from the PDF (MP v02.01.02): a Basic Restart has no AL confirmation (§3.7.1.1.3 p. 80). The server may *"not react at all"* (§3.7.1.1.2 p. 78). The `T_Disconnect` may not reach the bus (§3.7.3 p. 89). **No clause requires a `T_ACK`.**
- `RestartOutcome` in `knx_net::commissioning::memory_download`. `Unconfirmed` requires three things: an error that is a silence (`ConnectionReleased`/`ConnectionLost`/`NoAnswer`/`Lagged`), all machines `Loaded`, and only `Disconnect` after it. Otherwise it is still an `Err`.
- Simulator: `unanswered_restart_seqs()` shows that there is one request (a single sequence number, only TL repeats it).
- Mutants (6/6 caught): old Err behaviour, missing Loaded check, missing tail check, every error counting, Acknowledged not recorded, `is_confirmed` always true.

## R1: status and data while programming (user requirement)
- `run_memory_download_observed(session, plan, observe)`, with `Progress::{Started, StepStarted, DataWritten, StepDone}`. `DataWritten` only comes after a successful read-back.
- Tests: order, exactly the octets sent, running total, stops at the failure, observer changes nothing (identical `seen()`).

## R2: "download" = to the device (user requirement)
- `docs/GLOSSARY.md`, with evidence from Architecture p. 19, Glossary p. 9 and CP §3.7.5.5.2 p. 62.
- Open (goal.md): rename the File menu's "Download project".

## Still open
- K2 [W]: frame trace on `1.1.67`, needs a go.
