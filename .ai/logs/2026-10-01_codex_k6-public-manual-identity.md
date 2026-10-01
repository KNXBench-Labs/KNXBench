# K6 public MDT manual: family matches, generation unverified (2026-10-01)

The user provided MDT's public `BE-TA55xx-02_MDT_TM_V13_DE.pdf`. Downloaded
that exact URL and inspected its 51-page text offline. Cover: technical manual,
07/2025 version 1.3 for `BE-TA550x.x2`, `BE-TA55Px.x2` and `BE-TA55Tx.x2`.
The product list includes `BE-TA55P2.02` and `BE-TA55P2.G2`; commissioning
§2.5 describes programming-button, physical-address and application
programming through ETS. The manual is relevant to the Taster Plus 55 family,
unlike the previously supplied AMI switching-actuator document.

The historical physical `1.1.67` was operator-labelled `BE-TA55P2.G1`;
RESEARCH §8.8.5 documents that a matching `PID_HARDWARE_TYPE` in local product
data is not a proof of model/generation and `PID_ORDER_INFO` was not read.
Do not silently identify the `.G1` device with this `.x2`/`.G2` manual or
infer a complete persistent-memory/recovery scope from the user-facing ETS
instructions. RESEARCH §24 and the K6 goal status now record this distinction.
The user explicitly states that unavailable non-public manufacturer
information is not a blocker for continuing other work; this is not a new
hardware go and does not relax the durable pre-write recovery condition.
No bus access, key, private corpus, or Web source was used. The PDF was held
only in task-owned scratch and must be removed after verification.
