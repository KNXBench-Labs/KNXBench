# 2026-09-28 — Claude — live application download to 1.1.67 (MDT 0701h, option C)

The user authorised it with "go". Target: 1.1.67 through gateway 172.18.250.1:3671. No access key was used or needed.

## Runs
| Run | Time | Result |
|---|---|---|
| 1 | afternoon | stopped at step 13 (GrOAT read-back): T_ACK received, answer missing |
| 2 | afternoon | stopped at step 8, same pattern |
| 3 | 16:55–16:58 | steps 0–22 OK; step 23 A_Restart without T_ACK |

Logs (gitignored): `OriginalData/DeviceBackups/1.1.67_MDT-0701_2026-09-28_{download-run,download-run-2,download-run-3,after-failed-download,diag*-*}.txt`.

## Diagnosis of runs 1 and 2
Read-only frame trace: the device answers with seq 1, and our T_ACK seq 1 has no L_Data.con. The device acknowledges the next request (seq 2) but holds its answer back, then repeats the old answer at 6 s and every 3 s after that. That matches TL v01.02.03 §5.4.1 (OPEN_WAIT, A9 repetition, A11 order). The client gave up after 3 s and had no SeqNoRcv.

## Fix (`aacd60a`)
- `receive_numbered`: E04 → T_ACK + deliver; E05 → T_ACK, discard; E06 → T_NAK. `recv_seq` is reset on connect (A12).
- `AnswerWait::RepetitionLadder` (4 × response_timeout) for normal requests. `OneTimeout` only in the load-state wait loop (RES §4.23.2.4.1, "half the TL-timeout").
- Simulator: `lost_ack_for_answer`, `answer_repeat_after`.
- Tests: `an_answer_held_behind_a_lost_t_ack_is_still_waited_for`, `a_repeated_answer_is_never_taken_for_a_new_one`, `the_wait_loop_polls_a_silent_device_once_per_timeout`, `the_short_wait_does_not_outlive_the_wait_loop`. Mutants 7/7.

## Result of run 3
Read-back on a fresh connection: all 1418 segment octets equal to the image (the 2 IA octets masked, `11 43` unchanged); load states all Loaded (01).
Open: A_Restart got no T_ACK. Whether the device restarted is unknown. MP §3.7.3 (5) ("ignore all telegrams … except negative TL-confirmations") does not settle it. Next: check on the device (button 1 → 2/0/53), and only then decide.
