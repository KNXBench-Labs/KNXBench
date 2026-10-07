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

## Gates

Attempts 1-2: chromium 130/1 (group-address-drag race at e2e line 53, root cause from error-context: device detail not yet rendered when `.all()` ran). Attempt 3: vitest worker crash (1999 passed, worker exit). User authorized a one-line test wait despite the Web lock (0533230b). Integrated public16 on 0533230b independently accepted: 16 exit0, Rust 3244/0/177 in 180 blocks, Web 2001, Chromium 131 plus probe 1, 886 frozen inputs, CLI knx 0.1.0-alpha.4+g0533230b. Three earlier attempts refused on Web tests (two group-address-drag race failures, one vitest worker crash), retained.
