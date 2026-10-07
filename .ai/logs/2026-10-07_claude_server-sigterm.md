# 2026-10-07 — Claude — knx-server ignored SIGTERM (docker stop → exit 137)

## Report

User: the old container (`knxbench-pre-tls`) ended with exit code 137, so
`docker stop` had to kill it after 10 s; `knx-server` as PID 1 does not react
to SIGTERM.

## Evidence before the fix

- `docker inspect knxbench-pre-tls`: `ExitCode 137`, no `--init`, default
  stop signal and timeout.
- Throwaway container from image `knxbench-server` (4cc0ec05):
  `/proc/1/status` `SigCgt: 0000000100000440` (SIGBUS, SIGSEGV, glibc RT
  only), `SigIgn: …1000` (SIGPIPE). SIGTERM (bit 14, `0x4000`) not caught.
  `docker stop` → exit 137.
- Source: `main.rs` called `axum::serve(..).await` without
  `with_graceful_shutdown`; no `tokio::signal` anywhere in `knx-server`.
- Linux delivers no default action for a signal to PID 1 of a PID
  namespace unless the process installed a handler, so the ignore is total.

## Fix

- `apps/knx-server/src/graceful_stop.rs` (`GracefulStop`, `release_bus`):
  handlers installed first thing in `main`, before slow start-up work.
  State machine Running → Requested → Forced(GraceExpired | SecondSignal)
  over a `watch` channel; real signals feed an mpsc channel so tests drive
  the same code with a paused clock.
- `main.rs`: both listener arms use `with_graceful_shutdown(stop.requested())`
  wrapped in `stop.serve(..)`; afterwards `release_bus` (bus monitor
  `BusSession::stop` → tunnel `disconnect` sends `DISCONNECT_REQUEST`; line
  scan `cancel`), bounded 2 s. Forced end → `process::exit(0)`, because
  dropping the runtime waits for `spawn_blocking` work without a limit.
- Grace 5 s + bus 2 s < Docker's default 10 s.

## Side finding: Docker build context

Rebuilding failed: `open data/.knxbench-tls: permission denied`. The repo
root `data/` is the user's bind mount; since ADR-0088 it contains the
root-owned `0700` TLS directory. `.dockerignore` did not exclude `data/`,
so the build context also carried user projects (and `OriginalData/`,
132 MB private corpus) into the builder stage. Excluded now together with
other private/local data listed in `.gitignore`.

## Verification

- Unit (7): no signal, drained, grace expired at exactly 5 s, second signal,
  server error passthrough, bus monitor tunnel disconnected (`FakeTunnel`),
  idle bus.
- Real binary (4, `tests/signal_stop.rs`, isolated `XDG_DATA_HOME`/data dir):
  SIGTERM with idle keep-alive connection exits 0 in < 3 s without "cut
  off"; SIGINT; half-sent request forces the grace path (≥ 4 s, exit 0);
  second signal ends it at once.
- `cargo test -p knx-server`: 674 passed, 0 failed, 45 ignored. Clippy
  `-D warnings`, fmt, check-layering/headers/anchors/ledger/corpus-gates.
- Docker image `knxbench-server:sigterm-probe`: PID 1 `SigCgt
  0000000100004442` (SIGINT + SIGTERM); `docker stop` 181 ms (HTTP), 306 ms
  (HTTPS with throwaway password), both exit 0. Probe containers removed.

## Not done

- User's live `knxbench` container not replaced (needs the user's go).
- No real-gateway check that the tunnel is freed on stop.
- Device download/address programming not waited for (KL §163).
