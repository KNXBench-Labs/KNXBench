# Diagnostic sidebar selection (issue #23)

## Behaviour

The Bus monitor and Log entries in the navigation pane select a workspace;
they are not panel visibility toggles. Selecting an already active entry leaves
it active, with its existing subtree and local state. Selecting the other
entry switches panels; selecting a project view explicitly leaves diagnostics.
The rule applies with and without an open project and in every UI language.

Settings continues to open an overlay. Diagnostics window continues to use the
existing separate companion open/focus action. Neither action is converted
into a workspace or a toggle by this fix. Toolbar pane visibility controls and
command-palette navigation retain their existing meanings.

## Implementation boundary

Only the two sidebar event handlers in `App.tsx` change: set the requested
panel open and the other closed. Existing command-palette handlers already use
these semantics. There is no new state model, API, domain/storage/schema change,
protocol operation, reconnect or device write.

The former toggle exposed the remembered project view after a second click.
The reporter's video shows Bus monitor returning to Topology; the same source
mechanism affected Log. Settings and the companion have different handlers and
are not attributed this root cause.

## Verification

- Eight App regressions: English/German × project open/closed × monitor/log;
  repeated selection, active marker, subtree identity, no additional immediate
  panel reads, switching between diagnostic panels and explicit project-view exit.
- Existing import-log-scope tests leave via Overview before reopening Log;
  selecting the active entry no longer means leaving it.
- Four browser cases in `devices-navigation.e2e.ts`: English/German × 1440/400 px,
  selected monitor tab/input and Log filter survive repeated sidebar selection;
  Settings opens/closes without changing the underlying selected workspace;
  explicit Topology navigation leaves diagnostics; no hardware mutation request.
- Existing production Devices browser configuration exercises the same cases.

Native WebKitGTK and Orca are not certified by Chromium acceptance. This source
fix is not a new release or an update to already installed binaries. Runtime
results belong in the implementation history after actual execution.
