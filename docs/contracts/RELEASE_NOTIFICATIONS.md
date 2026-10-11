# GitHub release notifications

## User-visible contract

After authenticated settings hydration, the workbench checks published releases
of the fixed public `KNXBench-Labs/KNXBench` repository. No project is required.
A semantically newer eligible version produces one localized informational toast
per distinct version per mounted workbench session. It uses the existing nine-second
animated toast lifetime and notification history; dismissing it never installs,
downloads, replaces the executable or changes a project. Its separately focusable
link opens the official version-specific GitHub release page with no opener/referrer.

File → About KNXBench retains the observed update information after the toast
expires and offers an explicit manual check. A failed check says unavailable,
not up-to-date; a previously observed release link may remain visible with that
failure status. Automatic checking is enabled by default; Settings → Updates can
disable it using the existing versioned server settings record. Explicit manual
checks remain possible while automatic checking is disabled. Unsupported settings
records, pending/unacknowledged writes and malformed preference values do not
authorize an external request. Failure to hydrate settings means no automatic check.

While the page is visible, automatic checks run at startup and then at most hourly.
Focus, becoming visible and reconnecting can trigger a due check; they do not
bypass the one-hour cooldown. Only one request chain is active. Unmount, disabling
or loss of settings admission cancels its owner; late replies cannot notify.
Deduplication/cooldown are session-local, not a new persisted release database.

## Adapter and version admission

The client uses credential-free `GET
https://api.github.com/repos/KNXBench-Labs/KNXBench/releases?per_page=100`;
redirects are refused, referrers omitted and response JSON bounded to 4 MiB and
100 records. The version read plus release check has a ten-second cancellation
deadline. No GitHub token, webhook receiver, new dependency or server background
job is needed. External response content is never executed or rendered as HTML.
The exact running version comes from authenticated `/api/version`, not the
frontend manifest. An unknown/non-SemVer running version makes the check unavailable.

Strict SemVer 2.0.0 precedence ignores build metadata, compares numeric identifiers
without floating-point precision loss and rejects leading zeroes. One literal
optional `v` tag prefix is supported; whitespace/custom release tags are not repaired.
Draft/unpublished/malformed rows and non-newer versions cannot produce a notice.
Stable running versions exclude both GitHub-marked and tag-declared prereleases;
a prerelease running version can see newer prereleases or stable releases. The
highest eligible version among returned records wins, not API position/date.
Release links are constructed from the fixed official repository and validated
tag, not an untrusted `html_url` field.

## Research and limitations

Primary sources inspected on 11 October 2026:

- [GitHub Releases REST API](https://docs.github.com/en/rest/releases/releases?apiVersion=2022-11-28):
  public release listing needs no authentication, max `per_page` is 100, and
  `/releases/latest` excludes prereleases/drafts. Therefore the current alpha uses
  the release list, not `latest`.
- [SemVer 2.0.0 §§9–11](https://semver.org/spec/v2.0.0.html): prerelease precedence,
  numeric/lexical identifiers and ignored build metadata.
- Actual official endpoint returned HTTP200 with `Access-Control-Allow-Origin: *`;
  an actual Chromium fetch from a local HTTP page passed CORS and returned five
  published alpha release tags, including Alpha.9. This is transport evidence,
  not certification of every future response or a native WebKitGTK session.

This is bounded polling, not immediate push delivery. Only one page/up to 100
returned releases is examined; older publications beyond it are not searched.
Network filtering, offline operation, GitHub rate limits, redirects or oversized
responses leave the app usable and the check unavailable. Multiple clients share
an outbound-IP API budget; explicit manual checks can consume it sooner. Each
client contacts GitHub directly, so GitHub receives that client's public IP and
ordinary HTTP/browser metadata, but no KNX project, credential or product payload.
Turning off checks cannot retract a request already sent. No automatic installer,
artifact signature verification, platform-specific asset selection, background
check while closed, migration approval or upgrade compatibility promise is added.
Users must review release notes and create independent backups before upgrading.
Common production web bundle/Chromium evidence does not certify interactive
AppImage/WebKitGTK, Docker deployment or live KNX operation. This feature is not
in the already published Alpha.9 release.
