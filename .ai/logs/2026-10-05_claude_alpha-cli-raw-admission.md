# AR06Y CLI raw-size admission (2026-10-05 10:14 CEST)

Traced real `install_package` callers after AR06X. CLI `products ingest`
(apps/knx-cli/src/main.rs) opened/created the product DB and read the whole
file before the 256 MiB guard. HTTP catalog install already caps the body at
256 MiB via DefaultBodyLimit.

TDD: new cli_package_raw_admission.rs (4 real-binary tests, sparse files).
RED run (64 GiB case skipped for host safety): 2 passed, 1 failed exactly on
"an oversized input must be refused before the product DB is opened or
created". Fix: length check before read and DB open, bounded read (bound+1),
public MAX_PACKAGE_INPUT_BYTES. GREEN: 4/0/0; related CLI ingest tests pass.
Mutants (in place, leases held, restored): guard-unwired and
boundary-inclusive both compiled and failed exactly their intended test.
