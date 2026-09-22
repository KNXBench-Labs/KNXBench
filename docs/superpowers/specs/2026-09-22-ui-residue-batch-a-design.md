# UI Residue Batch A Design

**Date:** 2026-09-22
**Status:** Approved by the standing user instruction that comparable designs, specifications, and implementation plans are pre-approved
**Scope:** Goal task T12, `KNOWN_LIMITATIONS.md` §§19, 23, 24, 30, 96, and 118

## Intent

Close six small but visible gaps in existing project-navigation and file workflows without widening them into new subsystems. A search result must be visible in the Project Explorer, browser users must be able to upload several files and download the current project, a large project download must not be copied into one server-side byte buffer, and a load that completed after its HTTP response was lost must recover without a page reload and announce success accessibly.

Sections 49 and 50 remain outside this design. T12 explicitly requires the T14 report before their UI half may be built, and `.superpowers/sdd/goal/task-14-report.md` does not exist on this base. No `knx-report` API is extended here.

## Constraints

- Existing domain commands and full `ProjectTree` responses remain the only mutation path.
- No KNX bus, gateway, multicast, or hardware traffic occurs.
- Every new user-facing string is present in the English and German message catalogues.
- Group-address presentation continues to use the project's slash notation; this work adds no notation selector or alternate formatter.
- Styling uses existing `--knx-*` tokens. Drag feedback is stateful but motion-free, and no animation is added outside the existing reduced-motion guard.
- Each limitation is independently testable and independently committed.
- The browser file picker continues to return one path to its caller because project open/import accepts one project. “Multi-selection” means selecting or dropping several local files for a batch upload, after which the user explicitly chooses the one project to open.

## Alternatives considered

- Lifting every tree branch's open state into `App` would make arbitrary programmatic navigation possible, but it turns a one-shot reveal into a second tree-state model. A reveal generation preserves local ownership and is the smaller fit.
- A new multipart batch-upload route would reduce HTTP requests, but it would duplicate validation and weaken the existing per-request size bound. Sequential use of the proven single-file route is explicit and sufficient.
- Fetching the download into a JavaScript `Blob` offers easy error handling, but recreates whole-file buffering in the browser. Native anchor navigation preserves streaming end to end.
- Retaining every completed operation's projected tree would make recovery operation-addressable, but adds unbounded server history for no user requirement. `GET /api/project` intentionally returns current server truth: if another client has since loaded a newer project, showing that current project is preferable to resurrecting stale state.

## 1. Reveal search selections

`Search` already returns a typed `Selection`; `App` remains the canonical selection owner. A search pick additionally increments a one-shot reveal generation passed to `ProjectExplorer`. Ordinary tree clicks do not increment it, so manual collapse state is not continuously overridden.

`ProjectExplorer` derives which existing topology, building, and group-range ancestors contain the selected entity. A `TreeNode` receiving the current reveal generation opens itself when it is one of those ancestors. Once the selected row mounts, it calls `scrollIntoView({ block: "nearest" })`. Device rows are revealed in the topology branch, the canonical first occurrence, rather than scrolling two duplicate device renderings in succession.

Tests collapse each relevant branch type, select a search hit through `App`, and assert that its row becomes present and receives the scroll request. A separate assertion preserves manual collapse for ordinary selections.

## 2. Drag-and-drop and multi-file upload in `FsPicker`

The picker keeps `/api/fs/upload`'s one-file-per-request contract. A shared local upload routine accepts a list of browser `File` objects, uploads them sequentially, and refreshes the listing after all successful requests. Sequential requests keep error attribution deterministic and avoid multiplying the endpoint's 100 MiB per-request memory ceiling.

The existing file input gains `multiple`. The upload label is also a native file drop target. During `dragover`, it accepts only `DataTransfer.types` containing `Files`, calls `preventDefault`, sets `dropEffect` to `copy`, and exposes `data-drop-ready="true"`; complete `FileList` access occurs only at `drop`, matching the protected-mode lesson from T11. Token-based CSS provides the feedback without motion.

A batch upload does not auto-open an arbitrary member. It navigates to the existing `uploads` directory, refreshes the listing, announces the uploaded count in a polite status region, and leaves the user to choose one file. A failed file leaves earlier successful uploads intact and reports the failing filename and server error; it never claims the whole batch succeeded.

## 3. Stream project downloads

`fs_routes::download` still serializes the current in-memory project to a temporary `.knxdb`, so downloads include unsaved in-memory edits exactly as today. The response body is changed from `std::fs::read` plus `Vec<u8>` to `tower_http::services::ServeFile`, which streams bounded chunks from the temporary file. The response retains ownership of the temporary path until the body is dropped, preventing premature deletion while avoiding a leaked file.

The streaming helper has a focused test with a file larger than its configured chunk size. The test consumes body frames and requires more than one non-empty data frame; a whole-file `Vec` response therefore fails it even though both implementations eventually produce identical bytes.

## 4. Browser download command

The File menu gains a localized “Download project” button in the plain web build only. It is disabled when no project is open. Activation creates a short-lived anchor targeting `/api/project/download` with `download="project.knxdb"`, letting browser navigation consume the streaming response without first materializing a JavaScript `Blob`. The Tauri build keeps its native Save/Save As workflow and does not show the browser-only command.

The component test asserts visibility boundaries, disabled state, endpoint, filename, and activation. Server tests retain content and disposition coverage.

## 5. Recover a successful load after a lost response

The server exposes `GET /api/project`, returning the current `ProjectTree` rebuilt through the application/domain layer with import counts and undo/redo state. With no project open it returns a clear client error; the route does not mutate state and remains behind the existing API session guard.

`runLoad` keeps the existing exact client-token ownership rule. If the POST rejects but the final progress snapshot belongs to that token and says `succeeded`, the frontend fetches `GET /api/project`, installs that tree, derives `hasStorePath` from the snapshot kind (`open` versus `import`), clears the progress banner, and treats the load as successful. It does not show an error toast for the transport failure. A missing, foreign, running, or failed snapshot follows the existing failure path unchanged.

Tests cover server reconstruction, no-project behavior, exact-token recovery, foreign-token refusal, and failure of the recovery GET.

## 6. Announce successful project loads

Every successful `runLoad` exit—direct POST response or recovered current-tree fetch—pushes one localized status toast naming the loaded file. `ToastStack` already renders non-error entries as `role="status"`, so this is the established live-announcement path. The progress banner can still unmount immediately; assistive technology receives an explicit terminal success message instead of inferring it from the newly populated explorer.

The `App` test inspects the live status element and its localized message. It also asserts that recovery emits exactly the same success announcement and no error alert.

## Documentation and verification

After each item, its stable-number limitation entry is rewritten as resolved while retaining the anchor. `IMPLEMENTATION_STATUS.md` records the batch and the absence of §49/§50 work. Final verification runs focused RED-to-GREEN tests first, then TypeScript, all frontend tests, Rust formatting, Clippy, all workspace tests, layering, headers, anchors, and dependency policy. No command may initiate KNX or private-network traffic.
