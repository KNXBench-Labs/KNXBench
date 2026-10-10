# Docker upload and companion return contract

## Reported failures (2026-10-09)

GitHub issues #1 and #2 concern alpha.6 in Docker: a 115 MB `.knxproj`
rejected before import, and Diagnostics' return control doing nothing.
Before the fixes, `/api/fs/upload` capped the entire multipart request at
100 MiB. The browser return control only asked `window.opener.focus()` and
was disabled without an opener. Neither was a KNX format or domain failure.

## Verified platform facts

- [Axum Multipart](https://docs.rs/axum/0.8.9/axum/extract/struct.Multipart.html)
  supports `Field::chunk()` and configurable `DefaultBodyLimit`; its default
  extractor ceiling is 2 MB. The project's route overrides that ceiling.
- [MDN Window.focus](https://developer.mozilla.org/en-US/docs/Web/API/Window/focus)
  describes a request, not guaranteed foregrounding, and returns no success
  acknowledgment. An enabled control cannot infer successful navigation from it.

## Fix acceptance

- Accept files through 256 MiB, with a separate 16 KiB multipart-envelope
  allowance. Stream file chunks into the existing temporary-file publication
  path rather than retaining the entire field in memory.
- Refuse oversized files with HTTP 413 and an explicit 256 MiB message;
  delete incomplete staging files and never overwrite earlier uploads.
- Upload acceptance is not import compatibility. Keep the importer's existing
  64 MiB per-member limit unchanged. The total expanded-archive limit is
  1024 MiB after the separately requested 2026-10-10 increase.
  No claim is made that the reporter's undisclosed archive imports successfully.
- The browser's explicit return button navigates the current document to the
  same-origin editor URL. It removes `source` and changes `view` to the explicit
  `editor` return marker, retaining other query parameters, deployment path
  and hash. The entrypoint consumes that marker and, after authentication,
  reads `/api/project` to resume the currently open server project, including
  its dirty state and saved-file authority. Ordinary editor mounts still start
  at the welcome screen. Missing projects stay there; other read failures are
  reported. Canceled resumes and responses after another project has loaded
  are ignored. No project is reopened, reimported, replaced or saved by return.
  It works without an opener or when an opener is closed, does not close
  another window, submit edits, or stop the bus session.
- Native desktop return still focuses the existing main webview and remains
  unavailable if that window does not exist. Source-bound Flow row navigation
  keeps its existing focus behavior and is not converted into a reload.
- No core, persistence schema, protocol, manufacturer-data or dependency change.

## Limits

The upload ceiling is deliberately finite. Reverse proxies may enforce their
own lower limit; server-mounted files avoid the HTTP upload ceiling, not the
importer's safety checks. A browser return reloads the editor in the current
tab; the separately opened editor is not closed. Existing server project state
is retained, but transient editor selection is not transferred. Native focus
is covered by mocked adapter tests, not new real-desktop acceptance.
