# Browser upload and navigation limits

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-169)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 169. Browser uploads remain bounded and return navigation reloads the editor

**Local supplement, 2026-10-11.** The [login/feedback/size contract](../contracts/LOGIN_FEEDBACK_AND_FILE_LIMITS.md) raises streamed upload and contribution raw admission to 1024 MiB, with a separate 16 KiB envelope. Contribution expanded inventory also admits 1024 MiB, but XML scans stay at 8 MiB/member, normal project members at 64 MiB, manufacturer raw input at 256 MiB and selective source integration at 64 MiB. Large input admission does not prove complete scanning, compatible import or bounded peak RSS. This source package does not update existing release/container artifacts.

**Limitation.** The original source fix for GitHub #1/#2 accepts files up to 256 MiB
with a separate 16 KiB multipart-envelope allowance and streams into a staging
file. Larger HTTP uploads receive 413; a reverse proxy may impose a smaller
limit. Server-mounted files bypass HTTP admission, not the unchanged importer
limits (64 MiB per expanded member; 1024 MiB expanded archive). The reporter
has not supplied the 115 MB archive, so its import compatibility is unverified.

The explicit Diagnostics/Flow browser return navigates the current tab to the
editor, including without an opener, and resumes the current server project
with a read-only request. Server project state and the bus session remain open,
but transient editor selection is not transferred, and a separate
editor window is not closed. Native return remains a focus request to an existing
main webview; no new real-desktop focus acceptance is claimed.

**Why.** The old 100 MiB request ceiling rejected legitimate larger uploads.
Browser `Window.focus()` offers no guaranteed foregrounding or acknowledgment.
The local notification-history/detail supplement is memory-only and capped at
1000 UI records, independently of the server log. An evicted toast target is
reported, not matched by text. Clipboard permission is platform-dependent; raw
JSON can be selected/copied manually. JSON log export stays server-only v1 and
does not claim to include the UI history. Login version identifies the built
frontend, not a queried remote server or newest release.
The [contract](../contracts/DOCKER_UPLOAD_AND_RETURN.md) records primary references
and preserves bounded admission, no-clobber publication and source-bound Flow
selection independently of explicit return.

**Impact.** These source fixes are not in the published alpha.6 image/AppImage.
They are included in Alpha.7, published and externally verified on 2026-10-10
(AppImage/MCP checksums and anonymous multiarch Docker/HTTPS smoke; see
[release evidence](../evidence/release-alpha7-2026-10-10.json)). The admission,
proxy, transient-selection and native-focus limits above remain. No archive
compatibility or hardware support is implied by upload/navigation acceptance.
<!-- END retained catalogue record -->
