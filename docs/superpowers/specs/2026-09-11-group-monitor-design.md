# Design — T15: Group Monitor GUI (gap D5, finishing E4's display side)

## Status and scope

**Status:** design, 2026-09-11. Builds directly on **T29**
(`2026-09-11-dpt-codec-design.md`, `E4-D1`…`E4-D9`, merged), which supplies
`knx_core::dpt::{decode, encode, resolve_group_address_dpt,
resolve_project_group_address_dpts}` and moved `GroupValue` into
`knx-core`. Decision numbering continues at **D8** here so a comment citing
"design D3" still means one thing per document, matching this repo's own
convention (T29 restarted at `E4-D1` because it belongs to a different gap;
T18's second slice continued at `D12` because it is the same thread as its
first slice — T15 is a new document but the *same* gap thread as D5/E4, so
it restates D1–D7 from the brief before adding its own).

**Gap closed:** **D5** (`docs/GAP_ANALYSIS_ETS.md`) — "the bus-communication
features that exist have no desktop/web front end at all." Also finishes
what **E4** left open on the display side (T29 closed the codec/CLI half;
`docs/GAP_ANALYSIS_ETS.md` names T15 explicitly as the remainder).

**In scope.** A bus-monitor session owned by `apps/knx-server`, a small HTTP
API for starting/stopping it, polling it for telegrams, and writing a group
value through it; a React table in `apps/knx-web` consuming that API,
DPT-decoding rows against the open project, with send-from-the-table.
Tunnelling only (D7 below).

**Deliberately out of scope**, each argued in §7.

**Recommended shape: two slices**, not one. §9 argues why and states
exactly where the cut goes and what slice 1 closes of D5.

## 1. The problem, stated from the code

`apps/knx-cli/src/main.rs` already does the whole job at the CLI:
`run_bus_monitor`/`run_bus_monitor_async` (~line 1417) subscribe to a
`TunnelClient`'s `broadcast::Receiver<TunnelEvent>` and print each
`Telegram`, decoded via `format_decoded_value` (~line 2076) against a DPT
map built once from the open project by `load_group_address_dpts`
(~line 1987); `resolve_write_value` (~line 1609) turns a `--dpt`/`--project`
/raw triple into an encoded `GroupValue` for `bus write`. None of this is
reachable from `apps/knx-server`, `apps/knx-desktop`, or `apps/knx-web`,
because `knx-server` does not depend on `knx-net` today (brief fact 2,
verified: `apps/knx-server/Cargo.toml` has no `knx-net` line; `apps/knx-cli`
is the sole existing consumer, confirmed in its own `Cargo.toml`). A user of
the desktop or web app who wants to watch or write bus traffic has no way
to do it short of a terminal.

Everything the GUI needs to decode and encode already exists in
`knx-core::dpt`. This task wires a session and a table to it — it does not
build another codec.

## 2. Evidence

Facts below are **[V]**, verified by reading the code in this worktree,
unless marked **[R]** (a judgment call/ruling, not a fact) or **[S]**
(sourced from the KNX Standard corpus).

- **[V]** `apps/knx-server/src/domain.rs:25` `AppState` — every field
  `pub`, all mutable state behind `Mutex`/`Option`: `project:
  Mutex<Option<knx_core::Project>>`, `store_path`, `opaque`,
  `manufacturer_refs`, `command_stack`, `import_counts`, `product_db:
  Option<Mutex<...>>`, `session_log: Mutex<SessionLog>`, `data_dir:
  PathBuf`. `AppState::new(data_dir)`/`impl Default` are the only
  constructors; tests elsewhere build a state with `AppState { field: ...,
  ..Default::default() }` (`apps/knx-server/tests/http_product_install.rs:
  40-67`), which is exactly how a test would inject a fake bus connector.
- **[V]** `apps/knx-desktop/src-tauri/src/lib.rs` embeds `knx-server` as a
  library behind `Arc<AppState>`, running `axum::serve` via
  `tauri::async_runtime::spawn` in both dev (`DEV_PORT`, `apps/knx-server/
  src/lib.rs`) and release (ephemeral port + bundled static frontend)
  builds. `apps/knx-server/src/lib.rs` (42 lines): `pub type SharedState =
  Arc<AppState>`; `pub fn app(state, static_dir) -> Router` merges
  `routes::project_routes()` + `fs_routes::fs_routes()` + `/healthz`.
  Anything added to `knx-server` reaches the desktop app automatically, as
  the brief states.
- **[V]** `crates/knx-net/src/lib.rs` re-exports: `ApplicationService,
  Destination, LDataFrame, LDataMessageKind` (from `cemi`); `BusConnection,
  BusError, DiscoveredGateway, KnxNetIpClient, RoutingClient, TunnelClient,
  TunnelEvent` (from `client`); `GroupValue` (from `knx_core`, moved there
  by T29).
- **[V]** `crates/knx-net/src/client.rs:76-101`: `BusConnection` is a
  **native `async fn` in a trait**, explicitly not `dyn`-safe by its own
  doc comment ("no `async-trait` dependency needed as long as nothing needs
  `dyn BusConnection`, which this cycle never does... single implementer,
  called directly"). `KnxNetIpClient` is its only implementer.
  `connect_tunnel(&self, gateway: SocketAddrV4) -> Result<TunnelClient,
  BusError>` is reachable **only** through this trait — `TunnelClient` has
  no public constructor of its own.
- **[V]** `TunnelClient` (`client.rs:227-382`): `pub fn
  assigned_address(&self) -> IndividualAddress`; `pub fn subscribe(&self)
  -> broadcast::Receiver<TunnelEvent>`; `pub async fn send(&self,
  destination: Destination, service: ApplicationService) -> Result<(),
  BusError>` (takes `&self` — concurrent-safe to call while another task
  holds the receiver); `pub async fn disconnect(self) -> Result<(),
  BusError>` — **consumes `self`**; `impl Drop for TunnelClient` is a
  best-effort safety net for the case where `disconnect` was never called.
- **[V]** `TunnelEvent` (`cemi.rs`/`client.rs`): `Telegram(LDataFrame) |
  Closed`. `broadcast::Receiver::recv()` additionally yields
  `Err(RecvError::Lagged(n))` when the channel's ring buffer overwrote `n`
  unread messages, and `Err(RecvError::Closed)` when the sender side is
  gone.
- **[V]** `BusError` (`client.rs:29-39`): `Io(io::Error) | Timeout |
  ConnectionRefused(u8) | NotImplemented | Protocol(String)`.
- **[V]** `apps/knx-server/src/session_log.rs` (608 lines) — the project's
  existing idiom for "a capped buffer that never loses data silently":
  `SessionLog { entries: Vec<LogEntry>, dropped: usize }`, `MAX_ENTRIES =
  1000`. `push()` on overflow evicts real entries to seat a **pinned
  synthetic entry at index 0** naming how many were dropped, refreshing
  that entry's count on every subsequent overflow rather than letting the
  drop count itself silently scroll off. Its test suite pins the invariant
  "dropped + retained == total pushes".
- **[V]** `apps/knx-server/src/errors.rs`: `ApiError { status: StatusCode,
  message: String }`, constructors `bad_request` (→ 400, caller-fixable)
  and `internal` (→ 500, environment problem), `IntoResponse` producing
  `(status, Json(json!({"error": message})))`. Every existing handler in
  `routes.rs` follows `.map(Json).map_err(ApiError::bad_request)` or the
  `internal` equivalent.
- **[V]** `apps/knx-server/src/routes.rs:15-...`: `pub fn project_routes()
  -> Router<SharedState>` — one flat `Router::new().route(path, method(
  handler))...` chain, merged into `app()` in `lib.rs`. Request DTOs are
  `#[derive(Deserialize)] #[serde(rename_all = "camelCase")]` structs
  local to `routes.rs`, e.g. `SetComObjectDptBody`.
- **[V]** `apps/knx-web/src/api.ts:430-454` — the `LogEntry` interface and
  its comment: `// LogEntry (apps/knx-server/src/session_log.rs) —
  server-local, no ts-rs binding, hand-written to match its
  #[serde(rename_all = "camelCase")] JSON shape`. `apps/knx-web/src/
  bindings/` holds only `ts-rs`-generated types for `knx-projection`
  crate types; every other server-local DTO in this codebase is a
  hand-written TS interface in `api.ts`.
- **[V]** `apps/knx-web/package.json`: `"test": "vitest run"`. Dependencies
  include `react`, `react-dom`, `@tauri-apps/api`, `vitest`, `happy-dom`.
- **[V]** `xtask/src/main.rs`/`xtask/src/layering.rs`: `check_layering()`
  checks exactly six roots — `knx-core`, `knx-etsproj`, `knx-productdb`,
  `knx-projection`, `knx-csv`, `knx-report`, `knx-diff` — against various
  forbidden-target lists. **`knx-server` is never a checked root, and
  `knx-net` never appears in any forbidden-target list.** A `knx-server →
  knx-net` edge trips no rule the tool enforces.
- **[V]** `apps/knx-server/Cargo.toml`: already depends on `knx-store`,
  `knx-etsproj`, `knx-productdb` (other Infrastructure-layer crates) and
  already carries `tokio = { workspace = true, features = ["full"] }`. No
  new async runtime is needed for a `knx-net` dependency, only the
  dependency line itself.
- **[V]** root `Cargo.toml` `[workspace.dependencies]`: no `async-trait`
  crate present anywhere in the workspace. `ts-rs = "12"` is present, used
  only by `knx-projection`.
- **[V]** `docs/KNOWN_LIMITATIONS.md` §61 — the DPT codec's own coverage
  statement: fourteen main types (1, 2, 3, 5, 6 except `6.020`, 7, 8, 9,
  12, 13, 14, 16, 17, 18); resolution is inference over linked
  communication objects, not a stored fact (measured: 194/514 = 38% of
  group addresses in the reference corpus resolve to no DPT at all,
  110/514 = 21% have no linked object at all); `GroupAddress/@DatapointType`
  is preserved but not modelled; scene numbers carry no +1 display offset;
  DPT-16 cannot represent an interior NUL; an out-of-range `Short` is
  rejected, not masked.
- **[R]** No corpus lookup was needed for this document: every question T15
  raises is answered by this repository's own code and prior design docs,
  not by the KNX Standard text itself (the wire protocol and DPT encoding
  were already settled by knx-net and T29). Where §4/§5 below state a
  session-lifecycle or polling-interval choice, they are labelled `[R]`
  rulings, not Standard citations.

## 3. Decisions carried from the brief (D1–D7), restated and checked

### D1. The bus-monitor service lives in `apps/knx-server`, no new crate

Confirmed right, not just asserted: §2's `xtask` evidence shows this edge
violates no layering rule, and `knx-server` already reaches three other
Infrastructure crates directly, so a fourth (`knx-net`) is not a new kind
of edge for it, only a new instance of an existing pattern. CLAUDE.md's
"avoid speculative abstractions" argues against a `knx-bus-service` crate
for a single consumer. **No evidence found against this decision; adopted
as stated.**

### D2. Cursor-based polling, not SSE/WebSocket

`GET /api/bus/monitor/telegrams?since=<seq>`. Confirmed consistent with
the existing `/api/*` surface, which is uniformly request/response
(`routes.rs`), and testable with ordinary `axum::body` HTTP tests the way
every other route already is (see `apps/knx-server/tests/http_*.rs`).
Neither Tauri's webview nor a browser needs a long-lived-connection
lifecycle added to the state machine this way.

**Trade-off, stated plainly:** display latency is bounded below by the
poll interval. §5 sets the interval at **500 ms**, chosen as a ruling
`[R]`: fast enough that a human watching the table perceives it as "live"
(sub-second), slow enough that polling an idle bus costs one small JSON
request every half second rather than a request storm. It is a client-side
constant (`apps/knx-web`), not a server-enforced minimum — nothing stops a
future revision from tuning it, and the design does not need to reopen to
change it.

### D3. Capped buffer, monotonic sequence, explicit gap signal

Mirrors `SessionLog`'s honesty (never a silent hole), not its exact
mechanism, per the brief. §4.3 gives the concrete design: a `VecDeque` of
`TelegramRow` capped at `MAX_TELEGRAMS`, a `next_seq: u64` counter, and —
critically — a `dropped_before: u64` watermark advanced both by ring-buffer
eviction **and** by `RecvError::Lagged(n)` (§4.2), so a lagged receiver's
missed telegrams are accounted exactly like an evicted-from-the-buffer
telegram: both make `dropped_before` jump, both surface to a polling client
as a gap, never as a silently missing sequence number.

### D4. Decoding via the already-open project

Uses `knx_core::resolve_project_group_address_dpts(&project)` +
`knx_core::decode`, computed once per monitor-session start (mirroring the
CLI's `load_group_address_dpts`, called once, not per telegram) and cached
in the session. No project open, or an unresolved/unsupported DPT: the row
carries the raw payload plus a stated reason, in the same four-way
vocabulary as `format_decoded_value` (`apps/knx-cli/src/main.rs:2076`) —
§6 gives the exact strings.

### D5. Send-from-the-table reuses the active session's tunnel

`POST /api/bus/write` looks up the one active session and calls
`TunnelClient::send(&self, ...)` on it — `send` takes `&self`, so this does
not conflict with the session's own receive task holding the subscription.
No active session: `409 Conflict` (not 400 — the request is well-formed,
the *server state* is wrong), telling the caller to start a session first.
It never opens a second connection to the gateway. §7 details the request
shape.

### D6. The testability seam — the single most important decision here

**Problem, evidenced:** `TunnelClient` cannot be constructed directly, and
`BusConnection` (§2) is explicitly not `dyn`-safe — "single implementer,
called directly" is a design choice made *for* `knx-net`'s own purposes,
correctly, but it means `knx-server` cannot inject a fake by pointing a
`Box<dyn BusConnection>` at test code. If `knx-server` calls
`KnxNetIpClient::connect_tunnel` directly, every test of the monitor
session either needs a real gateway (forbidden by D6) or needs to mock at
the HTTP-handler level only, leaving the actual event-loop/buffer/lagged-
receiver logic untested.

**The seam.** `apps/knx-server` defines its own trait pair, narrower than
`BusConnection` — it needs exactly the two operations a monitor session
uses, not discovery or routing (D7 excludes routing):

```rust
// apps/knx-server/src/bus.rs

/// What a monitor session needs from something that can open a tunnel.
/// `knx_net::KnxNetIpClient` is the production implementer via
/// `RealConnector`; tests use `FakeConnector`. Manually boxed futures,
/// not `async-trait` — this crate already pulls in `axum`/`tokio`
/// directly, so there's no reason to add a proc-macro dependency for a
/// three-method trait, and `dyn` safety is exactly the point here, unlike
/// in knx-net's own `BusConnection`.
pub trait GatewayConnector: Send + Sync {
    fn connect_tunnel(
        &self,
        gateway: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>;
}

/// What a monitor session needs from an open tunnel. Mirrors
/// `knx_net::TunnelClient`'s three operations it actually uses.
pub trait BusTunnel: Send + Sync {
    fn assigned_address(&self) -> IndividualAddress;
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent>;
    fn send(
        &self,
        destination: Destination,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>>;
    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>>;
}
```

`disconnect` takes `self: Box<Self>` rather than a bare `self` — `Box<dyn
BusTunnel>` cannot offer an unboxed consuming method, and this is the
standard way to keep a consuming API through a trait object. It preserves
the real `TunnelClient::disconnect(self)`'s "the tunnel is gone after
this" contract without weakening it to `&mut self`.

**Production implementation.** `RealConnector` wraps `knx_net::
KnxNetIpClient` (constructed once at server startup, held in `AppState`
alongside the connector trait object — it is stateless and `Sync`) and
implements `GatewayConnector::connect_tunnel` by calling
`KnxNetIpClient::connect_tunnel` and wrapping the returned `TunnelClient`
in `RealTunnel(knx_net::TunnelClient)`, whose three trait methods delegate
directly (`subscribe`/`assigned_address` forward as-is; `send` maps
`knx_net::BusError` to `BusSessionError`; `disconnect` unboxes and calls
the real consuming `disconnect(self)`).

**Test implementation.** `FakeConnector` holds a
`Mutex<Vec<FakeTunnelScript>>` (or a single shared handle, for the common
one-session-per-test case) so a test can decide in advance what
`connect_tunnel` returns — success with a controllable `FakeTunnel`, or a
`BusSessionError` to exercise the "gateway refused" path. `FakeTunnel`
exposes a `broadcast::Sender<TunnelEvent>` the test holds directly (send a
`Telegram`, send enough of them to force a real `Lagged` from the
*session's* receiver by giving it a small channel capacity in the test, or
send `Closed` to simulate the gateway dropping the connection), plus a
`Mutex<Vec<(Destination, ApplicationService)>>` call log so a write test
can assert exactly what was sent without a socket existing anywhere.

**Wiring.** `AppState` gains `pub connector: Box<dyn GatewayConnector>` and
`pub bus_session: Mutex<Option<BusSession>>` (§4.1). `AppState::new`/
`Default` construct `RealConnector::default()`; a test constructs
`AppState { connector: Box::new(FakeConnector::new(...)), ..Default::
default() }` — exactly the struct-update pattern already used by
`apps/knx-server/tests/http_product_install.rs:40-67` for `product_db`, so
no new test-only constructor is needed.

**Tests this makes possible**, all gateway-free:
1. `POST /api/bus/monitor/start` opens a session via a `FakeConnector` that
   always succeeds; asserts the response and that `bus_session` is `Some`.
2. A second `start` while one is active returns `409` and the original
   session's tunnel is untouched (call log has no second `connect_tunnel`).
3. Feeding `Telegram` events through the fake's sender and polling
   `/telegrams?since=0` returns them, decoded against a project fixture.
4. Feeding enough events to overflow a small test buffer, or forcing
   `RecvError::Lagged` with a small test channel, and asserting the gap
   signal's `droppedBefore`/`gap` fields account for exactly the right
   count (D3).
5. Sending `Closed` through the fake and asserting the session transitions
   to stopped and a subsequent poll reports that, per §4.1's lifecycle.
6. `POST /api/bus/write` with no session active returns `409`; with one
   active, asserts the fake tunnel's call log recorded the exact
   `Destination`/`ApplicationService` the request should have produced.

None of these need `#[ignore]`, a `KNX_GATEWAY` env var, or a real socket —
unlike `apps/knx-net/tests/live_gateway.rs`, which stays exactly what it is
today.

### D7. Tunnelling only; routing is a stated CLI-only limitation

`GatewayConnector`/`BusTunnel` above expose only tunnel operations, not
`RoutingClient`. This is not an oversight: it is symmetrical with the
CLI's own split (`bus monitor` vs `route-monitor` are separate commands
today), keeps the seam in D6 to exactly the surface a browser/desktop user
needs, and is recorded as a limitation in §7/§8 rather than left implicit.

## 4. Settling the open questions

### 4.1 Session lifecycle

**A monitor session is:** one open `TunnelClient` (via the connector seam),
one background tokio task draining its `broadcast::Receiver<TunnelEvent>`
into the session's telegram buffer, and the buffer/cursor state itself.
`AppState.bus_session: Mutex<Option<BusSession>>` holds at most one.

- **Start** — `POST /api/bus/monitor/start { "gateway": "<ip>:<port>" }`.
  If `bus_session` is already `Some` and its task is still running: `409
  Conflict`, body names the existing session (its `id` and `gateway`) —
  never silently replaced, matching D5's "no silent second connection"
  rule for writes, applied here to starts too. Otherwise: calls
  `connector.connect_tunnel(gateway)`; on success, computes the DPT map
  once (D4), spawns the drain task (§4.2), stores the new `BusSession`,
  returns its `id`. On `BusSessionError` from the connector: `502 Bad
  Gateway` with the error's text (an environment/remote-endpoint problem,
  not a caller mistake — matching `errors.rs`'s 400/500 split, extended
  with 502 for "the *far* end refused", still surfaced through the same
  `ApiError` shape since axum accepts any `StatusCode` there).
- **Stop** — `POST /api/bus/monitor/stop`. If no session: `409`. If one
  exists: sends a `oneshot::Sender<()>` stop signal the drain task selects
  against (see §4.2), awaits the task's join handle so the response only
  returns after the tunnel is actually gone, calls `disconnect(self: Box
  <Self>)` on the tunnel from inside that task (this is exactly why
  `disconnect` consuming `self` is not a problem: the task owns the boxed
  tunnel outright, nothing else holds a reference to it, so consuming it
  on the way out is natural, not awkward), clears `bus_session`, returns
  the final buffer stats (telegram count, dropped count).
- **Gateway drops mid-session (`TunnelEvent::Closed`, or the broadcast
  channel's sender being dropped surfacing as `RecvError::Closed`):** the
  drain task treats either as terminal, records a synthetic "session
  closed by gateway" marker in the buffer (so a polling client sees *why*
  telegrams stopped, not just that they did), and sets the session's
  status to `Stopped` in place — it does **not** clear `bus_session`
  immediately, so a client that was mid-poll still finds the buffer and
  its final telegrams; a subsequent `/telegrams` poll response carries
  `"status": "closed"` (§4.3) so the UI can show that state before the
  user explicitly stops it. `POST /api/bus/monitor/start` after this still
  hits the `409` branch until the client calls `/stop` (which is now cheap
  — the task has already exited, `disconnect` was already attempted, `stop`
  just tears down the stored session) — **`[R]` ruling:** requiring an
  explicit `/stop` even after the gateway closed keeps exactly one code
  path that clears `bus_session`, rather than a race between the drain
  task and a client-initiated stop both trying to do it.
- **No auto-reconnect.** `[R]` ruling, argued: KNXnet/IP tunnel connections
  are per-gateway stateful resources (a `CONNECTIONSTATE` handshake, a
  channel id) — silently reopening one behind the user's back after a drop
  is exactly the kind of protocol behaviour CLAUDE.md's "only implement
  protocol behaviour that is technically verified" warns against doing
  speculatively. The user sees the closed state and re-starts explicitly.
- **Session identity.** A session carries a `Uuid`-or-incrementing `id`
  (a `u64` is enough — one process, one server lifetime, no persistence
  requirement) so a future multi-tab client could in principle tell which
  session a poll response belongs to, though this slice only ever has one.

### 4.2 Where the tokio task lives, how it terminates, `Lagged(n)` accounting

The server is already async (`axum::serve`, `tokio` with `"full"`
features). The drain task is spawned with `tokio::spawn` from the `start`
handler, holding: the boxed `BusTunnel`'s `broadcast::Receiver<TunnelEvent>`
(from `subscribe()`), a `Arc<Mutex<TelegramBuffer>>` shared with the HTTP
handlers, the DPT map (owned, immutable for the session's lifetime — the
project could change mid-session via other routes, and §7 states that as a
deliberate non-goal, not a bug), and the `oneshot::Receiver<()>` stop
signal.

```rust
loop {
    tokio::select! {
        _ = &mut stop_rx => {
            // disconnect() happens here, after the loop, using the
            // Box<dyn BusTunnel> this task owns outright.
            break;
        }
        event = receiver.recv() => match event {
            Ok(TunnelEvent::Telegram(frame)) => buffer.lock().unwrap().push_telegram(frame, &dpt_map),
            Ok(TunnelEvent::Closed) => { buffer.lock().unwrap().push_closed_marker(); break; }
            Err(RecvError::Lagged(n)) => buffer.lock().unwrap().record_lagged(n),
            Err(RecvError::Closed) => { buffer.lock().unwrap().push_closed_marker(); break; }
        }
    }
}
```

This mirrors `run_bus_monitor_async`'s own `tokio::select!` between
`ctrl_c()` and `telegrams.recv()` (`apps/knx-cli/src/main.rs:~1417`),
substituting the stop `oneshot` for the CLI's Ctrl-C.

**`Lagged(n)` accounting (closing D3's requirement exactly):**
`record_lagged(n)` adds `n` to the buffer's `dropped_before` counter — the
*same* counter the ring-buffer eviction path increments, and the *same*
counter surfaced to the client as a gap (§4.3). A lagged receiver's missed
telegrams are indistinguishable, from the client's point of view, from
telegrams the buffer itself evicted for being too old: both are "we know
`n` telegrams existed between your cursor and what we can show you, and we
cannot show them to you." This is the concrete mechanism promised in D3 —
mirroring `SessionLog`'s honesty, not its exact pinned-synthetic-entry
implementation (a telegram buffer's `dropped_before` counter reported
alongside a sequence range is a better fit for a stream with a numeric
cursor than `SessionLog`'s single "N entries dropped" notice entry, since
the client here needs to know *which range* it lost, not just that
something did).

**Termination:** the task ends on stop-signal, `Closed` event, or receiver
`Closed` error — always via `break` out of the loop, after which
`disconnect()` runs and the task function returns, dropping its
`JoinHandle`'s result into whatever awaited it (the `/stop` handler, or —
for the gateway-drop case — nothing awaits it synchronously, but the task
still exits cleanly; `bus_session`'s status flips via the shared buffer's
state, not via the handler observing the join).

### 4.3 The wire shape

Four endpoints, all under `/api/bus`, registered in a new
`pub fn bus_routes() -> Router<SharedState>` in `apps/knx-server/src/
routes.rs` (or a sibling module, merged into `app()` in `lib.rs` next to
`project_routes()`/`fs_routes()`), following the existing handler idiom
exactly: `#[serde(rename_all = "camelCase")]` DTOs, `Result<Json<T>,
ApiError>` returns.

**`POST /api/bus/monitor/start`**
- Request: `{ "gateway": "192.168.1.10:3671" }` — `gateway: String`,
  parsed as `SocketAddrV4`; unparsable → `400`.
- Response `200`: `{ "sessionId": 1, "assignedAddress": "1.1.5" }`
  (`assignedAddress` from `TunnelClient::assigned_address()`, formatted
  with `IndividualAddress`'s existing `Display`).
- Errors: `409` (session already active, body names it), `400` (bad
  gateway address), `502` (connector reported `BusSessionError`, e.g.
  timeout or connection refused — message from `BusError`'s own
  `Display`).

**`POST /api/bus/monitor/stop`**
- Request: none.
- Response `200`: `{ "sessionId": 1, "telegramCount": 42, "droppedCount": 0
  }`.
- Errors: `409` (no active session).

**`GET /api/bus/monitor/telegrams?since=<seq>`**
- `since: u64` query param, default `0` (a client's first poll can omit it
  to get everything currently buffered, up to the cap).
- Response `200`:
  ```json
  {
    "sessionId": 1,
    "status": "active",
    "nextSince": 43,
    "droppedBefore": 0,
    "telegrams": [
      {
        "seq": 42,
        "timestamp": "2026-09-11T14:03:21.512Z",
        "source": "1.1.5",
        "destination": "1/2/3",
        "destinationName": "Living room / light / switch",
        "service": "GroupValueWrite",
        "rawPayload": "0x01 (6-bit)",
        "decoded": { "kind": "value", "dpt": "1.001", "text": "On" }
      }
    ]
  }
  ```
  `status` is one of `"active" | "closed"` (§4.1's gateway-drop case —
  `"closed"` means the session ended but the buffer is still readable
  until `/stop` is called). `droppedBefore` is the buffer's current
  `dropped_before` counter at response time — **not** scoped to the
  request's `since` value; the client compares it against what it already
  knew to detect a fresh gap, the same way a client of a `Content-Range`-
  style API compares against its last known state, rather than the server
  trying to guess what the client already rendered. `decoded.kind` is one
  of `"value" | "unresolved" | "conflict" | "error"`, with `decoded.text`
  carrying the human-readable string in each case (see §6's four-way
  vocabulary — `decoded` is a small tagged object rather than a single
  pre-formatted string precisely so the frontend does not have to regex
  `format_decoded_value`'s output back apart to, say, colour conflicts
  differently from clean decodes). `error` case additionally carries
  `decoded.error: string` (the `DptCodecError` display text).
- No active session and `since` given: `404` (there is nothing to page
  through — distinct from `409`, which means "you tried to mutate session
  state that doesn't exist"; a `GET` against a session that was never
  started is a not-found, not a conflict).
- `since` pointing at a sequence already evicted below the buffer's
  current floor is **not** an error — it is exactly what `droppedBefore`
  communicates; the response includes every telegram still in the buffer
  from `since` forward, whatever that is (possibly empty), never a hole
  with no explanation.

**`POST /api/bus/write`**
- Request:
  ```json
  {
    "destination": "1/2/3",
    "dpt": "1.001",
    "value": "on"
  }
  ```
  `dpt` is optional. If present, encodes `value` via `knx_core::encode`
  exactly as the CLI's explicit `--dpt` path does. If absent, resolves the
  destination's DPT from the session's cached DPT map (§4.4, §6) exactly
  as `resolve_write_value`'s `--project` path does; `None`/`Conflict`
  resolution is a `400` (caller must supply `dpt` explicitly), never a
  silent guess.
- Response `200`: `{ "encodedPayload": "0x01 (6-bit)", "service":
  "GroupValueWrite" }`.
- Errors: `409` (no active session — D5), `400` (bad group-address syntax,
  encode failure, unresolved/conflicting DPT with none given), `502`
  (`BusSessionError` from the tunnel's `send`).

**TypeScript bindings: hand-written, not generated.** `apps/knx-web/src/
bindings/` holds only `ts-rs`-generated types for `knx-projection` crate
types (verified §2); every other server-local DTO in this codebase — the
precedent is `LogEntry`, hand-written in `api.ts` with a comment naming its
Rust source and the fact that it is *not* `ts-rs`-bound — follows the same
pattern. These bus DTOs (`TelegramRowDto`, `MonitorStartResponse`,
`MonitorTelegramsResponse`, `BusWriteRequest`, `BusWriteResponse`) are
server-local to `knx-server`, not `knx-projection` types, so they get
hand-written TS interfaces in `api.ts` matching this document's field names
exactly, with the same kind of provenance comment `LogEntry`'s has. No
`ts-rs` derive is added to any bus-related Rust struct.

### 4.4 The row model

| Column | Source |
| --- | --- |
| `seq` | The buffer's own monotonic counter, assigned when the telegram is pushed (§4.2/§4.3) — not from the wire, KNXnet/IP telegrams carry no sequence number. |
| `timestamp` | Server wall-clock time (`chrono`/`std::time`, matching whatever `SessionLog`'s `LogEntry.timestamp` already uses) at the moment the drain task received the event — **not** a bus-side timestamp; KNXnet/IP tunnelling carries none, so this is explicitly "when the server saw it," stated as such, never implied to be device-side. |
| `source` | `LDataFrame.source: IndividualAddress`, `Display`-formatted (e.g. `"1.1.5"`) exactly as `format_telegram` renders it. |
| `destination` | `LDataFrame.destination: Destination::Group(GroupAddress)` — only the `Group` case is monitorable traffic a table row is for; `Destination::Individual` frames (if any reach this path) are out of scope for this table and are not rendered as rows at all (§7). Formatted via `GroupAddress::format(style)` using the open project's `info.group_address_style` (`crates/knx-core/src/project.rs:152`), or raw `u16` if no project is open. |
| `destinationName` | Looked up in the session's DPT-resolution pass's sibling name map — built the same way `load_group_address_names` builds it (`entry.address.raw() -> entry.name` across every installation's `group_addresses`), from the project **open in `AppState` at session-start time**. `null` if no project is open or the address has no matching entry. |
| `service` | `LDataFrame.service`'s variant name: `"GroupValueRead" \| "GroupValueResponse" \| "GroupValueWrite" \| "Other"` (mirrors `format_service`). |
| `rawPayload` | For `GroupValueWrite`/`GroupValueResponse`: `format_group_value_payload`'s exact rendering (`"0x01 (6-bit)"` / `"[2a, 99]"`). `GroupValueRead` carries no payload; `Other{apci, data}` renders as the APCI/data hex form `format_service` already uses. |
| `decoded` | D4's four-way outcome — see §6 for the exact vocabulary, computed against the DPT map cached at session start. |

**DPT map staleness is explicit, not silent:** the map is computed once
when the session starts (§4.1), like the CLI's `load_group_address_dpts`
called once per `bus monitor` invocation. If the user edits the project
(renames a group address, changes a DPT override) while a session is
running, rows already decoded do not change and newly arriving rows keep
using the session-start snapshot until the session is restarted. This is
the same trade-off the CLI already has today — `bus monitor` does not
re-read the project mid-run either — so the GUI is not introducing a new
kind of staleness, only inheriting the CLI's existing one. §7 states it
as a limitation regardless, since a GUI user is more likely to have the
project open and editable in the same window than a CLI user is.

## 5. Filtering

**In scope, client-side only:** a text filter over `destination` (raw or
formatted address) and `destinationName`, and a service-type checkbox
filter (show/hide `GroupValueRead`/`Response`/`Write`/`Other`), applied in
`apps/knx-web` against the already-fetched rows. `[R]` ruling: filtering
client-side means the server's buffer and its `since`/`droppedBefore`
accounting stay filter-independent — every client sees the same gap
signal regardless of what it currently has hidden, and the buffer does not
need per-client filter state, which the single-session, single-buffer
design (D3/D6) does not otherwise carry. ETS's Group Monitor filter is
richer (multiple simultaneous criteria, sender/receiver-specific, saved
filter sets); this slice's filter is a visibility toggle over an
already-fetched table, not a query language.

**Deliberately not provided:** server-side filtering (the `/telegrams`
endpoint always returns everything from `since` forward — a filtered-out
row still advances the client's cursor correctly, because the client
filters what it already has, not what it asks for); filtering by DPT main
type or by decode success/failure; persisted/named filter sets; filtering
that reduces what the buffer retains (a filtered-out telegram is still
capped/evicted on the same schedule as every other one, so switching a
filter off later still shows what was buffered, not a permanently
narrowed view).

## 6. Send-from-the-table

**Interaction:** each row in the table (or, more usefully, an explicit
"Send" action in a form above the table, since a *received* telegram's row
is a record of an event, not necessarily the thing to overwrite) opens a
compose form prefilled from the clicked row when triggered from a row:
`destination` prefilled with that row's group address, `dpt` prefilled
with that row's resolved DPT if the row decoded successfully via `Single`
resolution (D4), left blank for `None`/`Conflict`/error rows. The
resolution rule mirrors `resolve_write_value` exactly:

- If the user has typed an explicit DPT into the compose form, that wins —
  encode with `knx_core::encode(dpt, value)`.
- Else, the session's cached resolution (§4.4) is consulted: `Single(dpt)`
  → encode with it; `None` → the send is rejected client-side before the
  request is even sent, with the message `"No DPT resolved for this group
  address — enter one explicitly."`; `Conflict(dpts)` → rejected the same
  way, message `"Conflicting DPTs for this group address: {names} — enter
  one explicitly."` (both messages are the GUI's own text, deliberately
  echoing `format_decoded_value`'s vocabulary rather than inventing a
  different tone for the same fact).
- No project open at all: DPT must be entered explicitly; the form states
  this rather than silently disabling itself.

**Feedback:** on `200`, a transient success indicator showing the encoded
payload that was actually sent (from `BusWriteResponse.encodedPayload` —
so the user can visually confirm "0x01" went out for "on", closing the
loop the same way the CLI's `--dry-run` preview does, except here it is
the real send's own echo, not a preview). On `400`/`409`/`502`, the error
message from `ApiError`'s body is shown inline on the form, not just a
toast — a `502` in particular (gateway refused the write) is exactly the
kind of thing a KNX engineer needs to read fully, not glance past.

## 7. What this slice deliberately does not do

Itemised because this section feeds `docs/KNOWN_LIMITATIONS.md` directly
(§8 acceptance criterion 10 requires a new entry, not a rewrite of §61,
which stays exactly as it is — the codec's own coverage limitations are
unchanged and inherited, not newly discovered here).

1. **No routing (`route-monitor`) support.** D7. Tunnelling only; routing
   stays CLI-only. Stated here as a limitation of this GUI slice, not
   silently absent.
2. **No auto-reconnect after a gateway-side disconnect.** §4.1. The user
   must explicitly restart the session.
3. **No live re-resolution of the DPT map while a session runs.** §4.4.
   Editing the project mid-session does not change already-cached
   resolutions until the session is restarted — inherited from the CLI's
   existing `bus monitor` behaviour, but more likely to surprise a GUI
   user who can edit and monitor in the same window.
4. **No multi-session / multi-gateway support.** One active `TunnelClient`
   at a time per server process (D5's `409`). A user who wants to watch
   two gateways needs two server instances (or, for the desktop app, is
   simply not offered this — out of scope for this slice's data model).
5. **No persistence of the telegram buffer across a server restart or
   session stop.** It is purely in-memory, like `SessionLog`; stopping a
   session and starting a new one begins a fresh buffer and a fresh
   sequence counter at 0.
6. **No server-side filtering, saved filter sets, or filtering that
   narrows what the buffer retains.** §5.
7. **`Destination::Individual` frames are not rendered as table rows.**
   This slice's row model is a group-monitor table; addressing to an
   individual address is a different diagnostic than the group monitor is
   for, and is left for a future slice rather than shoehorned into this
   table's columns (`destinationName`/DPT resolution both assume a group
   address).
8. **No enumeration/unit vocabulary beyond what `knx_core::dpt` already
   decodes.** Inherited unchanged from `docs/KNOWN_LIMITATIONS.md` §61 —
   this slice does not touch the codec, so its coverage (fourteen main
   types, `UnsupportedDpt` for the rest) and its inference-not-fact
   resolution model are exactly as documented there. **This existing
   limitation entry is not modified, downgraded, or superseded by this
   design** — it still fully applies to every decoded value the GUI shows.
9. **No verification against real hardware.** Every claim in this document
   about what the GUI *does* is a claim about code and tests reachable
   without a gateway (D6). Nothing here has been run against a physical
   KNX installation, and this document does not claim otherwise.
10. **No KNX certification or ETS-parity claim.** This is a monitor/write
    table, not a certified diagnostic tool and not a claim of matching
    ETS's Group Monitor feature-for-feature (§5 already states the
    filtering gap explicitly).

## 8. Acceptance criteria

1. `apps/knx-server` gains a `knx-net` dependency; `cargo run -p xtask --
   check-layering` still passes (confirms D1's evidence in §2/§3 holds
   after the edge is real, not just before).
2. `apps/knx-server/src/bus.rs` (or equivalent module) defines
   `GatewayConnector`/`BusTunnel` (D6) with `RealConnector`/`RealTunnel`
   wrapping `knx_net::KnxNetIpClient`/`TunnelClient`, and
   `FakeConnector`/`FakeTunnel` usable from `apps/knx-server/tests/`.
   `AppState` gains `connector: Box<dyn GatewayConnector>` and
   `bus_session: Mutex<Option<BusSession>>`; existing tests that build
   `AppState` via `..Default::default()` are unaffected (i.e., compile and
   pass unchanged).
3. All four endpoints in §4.3 exist with exactly the request/response
   JSON shapes and status codes stated there, covered by
   `apps/knx-server/tests/http_bus_monitor.rs` and
   `http_bus_write.rs` (naming convention matching the existing
   `http_*.rs` files), none requiring a real gateway (D6) — including the
   six test cases enumerated at the end of §3's D6 discussion.
4. A `RecvError::Lagged(n)` on the session's receiver increments the same
   `droppedBefore` counter a buffer eviction would, verified by a test
   that forces a lag with a deliberately small test channel and asserts
   the exact count reaches the client on the next poll (§4.2/§4.3).
5. Stopping a session with `POST /api/bus/monitor/stop` only returns after
   the drain task has actually exited and `disconnect()` has been called
   on the fake tunnel (assert via the fake's call log) — not a
   fire-and-forget stop.
6. A second `POST /api/bus/monitor/start` while one is active returns
   `409` and does not touch the existing session's tunnel (fake's
   `connect_tunnel` call count stays at 1).
7. `POST /api/bus/write` with no active session returns `409`; with one
   active and no `dpt` given against a `Single`-resolved address, encodes
   and records the send on the fake tunnel's call log with the exact
   `Destination`/`ApplicationService` expected; against a `None`- or
   `Conflict`-resolved address with no `dpt` given, returns `400` without
   touching the fake tunnel at all.
8. `apps/knx-web` gains a bus/telegram table view, its own bindings
   hand-written in `api.ts` (§4.3) matching the JSON field names in this
   document exactly, a filter control per §5, and a compose/send form per
   §6, each covered by at least one `vitest` test using a mocked `fetch`
   (matching whatever existing `apps/knx-web` component tests already do
   for other tables — no new testing pattern introduced).
9. `docs/KNOWN_LIMITATIONS.md` gains a new, dated entry for this slice
   listing items 1–7 and 9–10 of §7 (item 8 is explicitly a pointer to the
   existing §61 entry, left unmodified — this criterion is not satisfied
   by editing §61). `docs/GAP_ANALYSIS_ETS.md`'s D5 row and E4 row are
   updated to say precisely what closed (tunnelling monitor + write GUI)
   and what did not (routing, filtering depth, multi-session). No existing
   limitation entry anywhere in the docs set is deleted, weakened, or
   silently reworded.
10. Six gates green: `cargo fmt --all --check`, `cargo clippy --workspace
    --all-targets -- -D warnings`, `cargo test --workspace`, `cargo run -p
    xtask -- check-layering`, `cargo deny check`, and (from `apps/knx-web`)
    `npm run test` (`vitest run`, confirmed from `package.json` — not
    guessed).

## 9. Scope check — this is two slices, not one

Read against what is actually being asked to land at once, T15 as
described in the brief is a new cross-crate dependency, a new
concurrency-bearing subsystem in the server (background task, shared
mutable buffer, a testability seam built from scratch), a four-endpoint
HTTP API, *and* a new React table view with its own filter and compose-form
UI and its own test suite — in one coherent unit of review. That is larger
than either sibling precedent this document was asked to read: T29 was
codec-plus-CLI-wiring only (no GUI, stated explicitly as its own
boundary); T18's second slice was evaluator-logic-only with "no UI" as an
explicit non-goal. This repository's own history treats "backend
capability" and "GUI consuming it" as separable units of work even when,
as here, they are the same gap.

**Recommendation: split into two slices**, cutting exactly at the HTTP
boundary in §4.3 — the same cut T29/T15 already made one level up (codec
vs. GUI), applied one level down (server capability vs. web table).

- **Slice 1 — server-side bus session.** Everything in §3 D1/D3/D4/D5/D6/D7
  and §4.1–§4.4 and the four endpoints in §4.3, fully tested per §8
  criteria 1–7, with `docs/KNOWN_LIMITATIONS.md`/`GAP_ANALYSIS_ETS.md`
  updated to say precisely that the *backend* capability exists and is
  reachable via `curl`/HTTP but has no UI yet. **This honestly closes half
  of D5**: "the bus-communication features that exist have no desktop/web
  front end at all" becomes false for the *server*, still true for the
  *user* — a user still cannot watch or write bus traffic without a
  terminal (`curl` is not a front end). D5 stays open, explicitly, until
  slice 2 lands; the gap-analysis update for slice 1 says this in as many
  words rather than claiming completion.
- **Slice 2 — the web table.** §5/§6, the frontend bindings, the table,
  filter and compose form, §8 criterion 8. This is what actually closes D5
  for a user, and is the smaller, more UI-focused slice once slice 1's API
  is a fixed, tested contract to build against — exactly the shape the
  frontend team (or a later session) needs: a stable JSON shape, not a
  moving target.

This split is not padding to justify a predetermined shape: it is the
shape recommended *because* D6's seam and D3's gap-accounting are each
substantial enough to deserve their own focused review and test pass
before a table's worth of frontend code is built on top of them, and
because a reviewer evaluating "does this correctly account for every
dropped telegram" should not simultaneously be evaluating "does this
filter control read well" in the same diff. If forced to ship one slice
only, slice 1 is the one to build first regardless — it is the one that
makes slice 2 straightforward rather than speculative, and it is where
D6's testing seam — this document's most load-bearing decision — actually
lives.

## 10. What the accompanying ADR should decide

`docs/adr/` currently ends at **ADR-0016** (`0016-dpt-codec-in-knx-core.md`,
T29). The next ADR should be **ADR-0017**, deciding the one structural
change this design makes to the dependency graph: **`knx-server` gains a
direct dependency on `knx-net`**, with no new crate interposed (D1). It
should record, in ADR-0016's own style (Date/Status/Session header,
Context citing this design document, Decision, and the layering evidence
from §2/§3 D1 of this document — that `xtask check-layering` treats
neither crate specially and that `knx-server` already reaches other
Infrastructure crates directly), the alternative considered and rejected
(a separate `knx-bus-service` crate) and why: a single consumer today, and
CLAUDE.md's standing guidance against speculative abstraction. This design
document does not write that ADR; §9's split does not change what the ADR
decides, since both slices share the same `knx-server → knx-net` edge —
the ADR belongs with slice 1.
