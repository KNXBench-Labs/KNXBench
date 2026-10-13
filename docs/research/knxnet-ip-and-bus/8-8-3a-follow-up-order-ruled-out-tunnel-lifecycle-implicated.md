# 8.8.3a Follow-up — order ruled out; tunnel lifecycle implicated (2026-09-18)

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="883a-follow-up--order-ruled-out-tunnel-lifecycle-implicated-2026-09-18"></a>
<!-- BEGIN preserved record-1-029 -->
#### 8.8.3a Follow-up — order ruled out; tunnel lifecycle implicated (2026-09-18)

The approved read-only comparison was run against the same installation with
only `A_DeviceDescriptor_Read`, no property reads, no authorisation and no
writes **[V]**. The target list was the nine explicit addresses from R-SAFE-2;
`1.1.220` was asserted excluded before any socket opened.

In reverse order on one shared tunnel, `1.1.32` answered with mask `0701h` and
all eight subsequent targets timed out. This rules out a special property of
`1.1.24`: whichever approved target owns the first management session on a
fresh shared tunnel answers, while later sessions do not.

With one newly opened and cleanly disconnected tunnel per target, still in
reverse order, `1.1.32`, `1.1.30`, `1.1.28`, `1.1.26` and `1.1.24` answered
with `0701h`; `1.1.31`, `1.1.29`, `1.1.27` and `1.1.25` timed out. The strict
alternation means a fresh tunnel per target is not by itself a reliable fix;
immediate tunnel teardown/recreation or gateway channel lifecycle is also in
the causal path. It does not prove which endpoint retains state or which delay,
sequence or acknowledgement is missing. R20 therefore stays open, and callers
must continue using the independent scan probe for presence rather than
interpreting a `ManagementSession` timeout as absence.


<!-- END preserved record-1-029 -->
