# Download verification reports

ADR-0121. How a tester's download report is built, what it may contain, and
how a maintainer turns it into a `verified` entry. Change this file in the
same commit as the behaviour it describes.

## The report (format 1)

Built by `knx_app::download_report::prepare_report` before a run and
completed with the run's outcome. JSON, `camelCase`:

| Field | Content |
| --- | --- |
| `kind`, `format` | `knxbench-download-report`, `1` |
| `knxbenchVersion` | Build version, e.g. `0.1.0-alpha.10+gabc1234` |
| `program` | `id`, `name`, `applicationVersion`, `sourceSha256` (the ingested program file), `manufacturer` and `mask` (four hex digits) |
| `scope` | `complete`, `partial-parameters`, `partial-group-addresses`, `partial-both` |
| `support` | `verified` or `untested`, as admitted |
| `inferences`, `warnings` | The plan's, each `rule: detail (reference)` |
| `plan` | `steps`, `dataOctets`, `groupLinks` |
| `outcome` | `completed`, `written` (`yes`/`no`/`partially`), `restart` (`acknowledged`/`notInPlan`/`unconfirmed`, completed runs only), `stoppedInStep` |
| `parameterValues` | The project's values by `ParameterRef` id; absent ones took their product default |

**Never in a report:** project name, device name, individual address,
gateway, access key, backup path, free-text errors. Adding a field that
could identify an installation needs an ADR.

## Where reports come from

- **Server:** `GET /api/device-download/report[?downloadId=n]` after the run
  and its cleanup ended: `{report, fileName, summary, issueUrl, mailto}`.
  404 when there is no download (or it was replaced), 409 while it runs.
- **Web:** the device download tab's **Help verify this device**, after
  every ended download: **Create report** reads the endpoint above, **Save
  report** writes the file locally, **Open issue form** / **Send by mail**
  open the links. The checklist is local only and never sent.
- **CLI:** `knx device download … --gateway … --confirm … --report <file>`
  writes the file after a run that reached the device, completed or stopped,
  and prints the issue link and the mailbox. No report is written when no
  connection opened. `--report` without a write is refused.
- **Transport:** the tester attaches the file to the
  [`download-report.yml`](../../.github/ISSUE_TEMPLATE/download-report.yml)
  issue form (public) or mails it to `issue@knxbench.com` (not public).
  Nothing is sent automatically.

## Universal pre-plan intake

The separate [universal support envelope](universal-device-support.md) can now
project only an ended report's typed outcome **and exact partial/full scope**,
without copying its parameter values, names, warnings or free text. It can also
be created before any plan, for unknown devices. It is not a replacement for the
format1 download report or maintainer verification. Never infer that privacy
rules for the universal envelope retrospectively remove parameter values from
the older ended-download report.

## Pre-plan support inquiries remain separate

The existing mailbox `issue@knxbench.com` also accepts device-programming
support-gap inquiries. A refusal before a connection has no ended-run report:
use `analysis.yml` or an unsent inquiry. Source builds after Alpha11 can optionally
attach a reviewed [universal support ZIP](universal-device-support.md), created
before any plan and with separate disclosure consent; confidential originals
remain withheld. Opening a
form/draft does not submit/send; email is not a guarantee of delivery, privacy,
encryption, retention or deletion. No private-original intake is confirmed by
this scope amendment. See [support guidance](device-programming-support-guidance.md).

## Maintainer review

1. **Read the report.** Check `kind`/`format`, that `outcome.completed` is
   `true` with `written: yes`, and the tester's answer to *Does the device
   work as configured?* with what they checked. A completed run with an
   unconfirmed restart counts only if the tester confirms the device works.
2. **Check the program file.** Install the program's package and compare
   `program.sourceSha256` with the installed file's (`knx_productdb::query::
   program_identity`). Another hash means another product file: ask, do not
   record. Rebuilding the plan from `parameterValues` alone has no tool yet;
   where a project with those values is at hand, compare `plan.steps` and
   `plan.dataOctets` with its plan.
3. **Weigh the warnings.** A `base-image-low-octet-first` warning
   (ADR-0120) needs the tester to have checked multi-octet values (times,
   thresholds); say which ones in the evidence.
4. **Record it.** Add an evidence file under `docs/evidence/` (aggregates and
   the report's public fields; parameter values only if the tester agreed to
   publication) and an entry in `crates/knx-app/data/verified_downloads.json`
   for exactly this program and scope: `program_id`, `scopes`, a `device`
   description without the tester's address, `date`, `reference` to the
   evidence. Partial scopes need their own run each.
5. **Close the loop.** Answer the issue or mail; the next build ships the
   entry.

## Revocation

A report of a wrong configuration after a `verified` download removes the
program's scope from `verified_downloads.json` until it is understood, with
the reason in the evidence file and `docs/KNOWN_LIMITATIONS.md` where it
applies.
