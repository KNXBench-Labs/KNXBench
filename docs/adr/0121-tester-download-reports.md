# ADR 0121: Tester download reports, and a maintainer review before "verified"

Date: 2026-10-13
Status: Accepted implementation decision (owner request 2026-10-13)
Session: KL-170 follow-up / tester verification, part A

## Context

A download is `verified` only when `crates/knx-app/data/verified_downloads.json`
names its program and scope with a documented hardware run (ADR-0049). One
program is listed. Every other download is `untested`: allowed after the
acknowledgement phrase, with a backup first and every octet read back, but
nobody learns whether it worked. Testers with real devices are the only way
to grow the list, and there was no structured way for them to report. The
owner asked for one on 2026-10-13: explained step by step in the device
download tab, reported by issue or by mail to `issue@knxbench.com` (a working
mailbox, for tester reports only), with the parameter values included.

## Decision

1. **A report per run, built by the application layer.**
   `knx_app::download_report` builds a `DownloadReport` (kind
   `knxbench-download-report`, format 1) from the prepared download, its
   support level and the witnessed outcome: program id, name, application
   version and the SHA-256 of the program file it was read from; manufacturer
   and mask; scope; support level; the plan's inferences and warnings; steps,
   octets and group-link count; the outcome (`completed`, `written`,
   `restart`, `stoppedInStep`); and the parameter values written, by
   `ParameterRef` id.
2. **Nothing that identifies an installation.** No project name, device name,
   individual address, gateway, key, backup path or free-text error (error
   texts may contain the address). Parameter texts may still be personal; the
   form and the CLI say so, and the mail route is not public.
3. **Prepared before anything is sent.** The server builds the report when a
   run is admitted and stores it with the session; the CLI builds it before
   connecting when `--report <file>` is given. A report that cannot be
   prepared stops the write, so a run never ends without the report that was
   asked for. The outcome is added when the run, including its cleanup, has
   ended (`GET /api/device-download/report`, 409 while running).
4. **Two ways in, both by hand.** The report offers an issue link to the
   `download-report.yml` form, with program, version and outcome filled in
   (GitHub fills issue-form text fields from query parameters named by field
   id), and a `mailto:` link to `issue@knxbench.com` with a summary. The file
   itself is attached by the tester. Nothing is sent automatically.
5. **A report is evidence, not a verification.** A maintainer reviews it
   ([contract](../contracts/download-verification-reports.md)) and records
   the program and scope in `verified_downloads.json` with an evidence file;
   the next build ships it. A failure report can revoke an entry.

## Alternatives considered

- **Upload reports from KNXBench automatically.** Rejected: it would send
  project-derived data without a person reviewing it, and KNXBench has no
  network path of that kind.
- **Mark a program verified on the tester's machine after a good run.**
  Rejected: read-back proves the octets, not the device's behaviour, and one
  local claim must not change what other users are told.
- **Leave out parameter values.** Rejected by the owner: they are what
  settles byte-order questions (ADR-0119, ADR-0120).
- **Put the whole report into the issue URL.** Rejected: GitHub answers
  `414 URI Too Long` beyond its limit, and values do not belong in a URL.

## 2026-10-14 amendment — pre-plan support inquiries

The owner extends the existing `issue@knxbench.com` mailbox scope to
**device-programming support-gap inquiries**, including refusals before a plan,
connection or report exists. The [support-guidance contract](../contracts/device-programming-support-guidance.md)
uses `analysis.yml`, not a fabricated `download-report.yml` outcome, for that case.
An unsent mail draft has no attachment or project-derived URL/body metadata. This
is not confirmed private-original intake or a claim of tested delivery,
confidentiality, encryption, retention or deletion. Actual ended-run reports and
maintainer-owned verification evidence stay unchanged.

## Consequences

- CLI and server produce reports (part A). Part B, the device download
  tab: a hint on untested plans, the plan's warnings (plain words for known
  rules) and inferences, and after every ended download **Help verify this
  device** with the steps, a checklist derived from the report (scope, links,
  warnings, restart), the privacy note, **Save report**, **Open issue form**
  and **Send by mail**. The report is fetched only on **Create report**; a
  409 asks to try again.
- The issue form's field ids are a contract with `issue_url()`; a test reads
  the form and fails when they drift.
- `verified_downloads.json` stays maintainer-owned and versioned with the
  code.
