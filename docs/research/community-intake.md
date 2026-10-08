# Community evidence intake: GitHub first, email for private samples

- Researched: 2026-10-08 06:45 CEST
- Status: historical platform research; temporary pre-launch intake published. Owner clarification below makes the future public main repository the canonical destination.

## 2026-10-08 — Consolidated into the public main repository

The main repository went public on 2026-10-08; the issue form and guides were
moved there the same day (see ADR-0091). The section below is the plan that
led to it.

## 2026-10-08 — Owner clarification: one main repository after public launch

The main repository is private temporarily while remaining work is closed; the
owner intends it to become public. Guides, issue forms and support-gap reports
should then be consolidated in `KNXBench-Labs/KNXBench`. The separate public
`KNXBench-Contributions` is transitional, not a required permanent architecture.
Earlier publication/ref verification below remains historical execution evidence.

Do not change visibility or remove the temporary repo now. Before launch, keep
working public links rather than silently redirecting non-members to the private
main repo. The coordinated launch must move/install the native issue form, retain
EN/DE guides, update app/guide/contact targets, verify public access and preserve
existing reports before agreeing retirement of the temporary endpoint. No launch
date or data deletion is implied. Disclosure, consent, no-public-original and
unverified private-mailbox boundaries remain unchanged. ADR-0091 records the
superseding hosting decision.

## 2026-10-08 — Approved and implemented follow-up

User accepted Q8 and explicitly authorized the analysis/export implementation and
separate public intake. `KNXBench-Labs/KNXBench-Contributions` is PUBLIC with issues
enabled; README and both form/config files are byte-equal to the reviewed
`docs/contribution-intake/` allowlist at remote commit
`d6e1269085283a5e85e79a6322f66484074414b8`. The application repository remains
PRIVATE. These were freshly read back after publication. Local service/UI/CLI
verification and retained exclusions are in [COMMUNITY_EVIDENCE.md](../COMMUNITY_EVIDENCE.md).
No mailbox, private handling policy, source sample upload or automatic delivery
was established. Proposal/open-decision paragraphs below are historical, not
remaining blockers for this authorized first package.
- Scope: receiving maintainer-reviewable evidence of KNXBench project/product support gaps.

## 2026-10-08 — Beginner guide and field-prefill follow-up

Re-fetched GitHub's primary Creating an issue, Attaching files and form-schema
pages over HTTPS. Issue forms support ID-based URL-query prefills; textareas
support manual attachments. The app passes only `analyzer-version` plus its
fixed template selector, never filenames/findings. Keep version/ZIP/technical
metadata optional so a plain-language report works without export. The form
provides both English/German publication notices; attachments are uploaded and
publicly accessible before posting, so review/permission comes before attaching.
These instructions are separately published and exact-read-back verified at
`8db689cdf9e80ec21fd0aa4b85c8480aafd48c7d`; four reviewed guide/form/config files,
no application code or original samples. Mailbox/private handling remain unverified.

Sources checked for this follow-up:
- https://docs.github.com/en/issues/tracking-your-work-with-issues/creating-an-issue
- https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/attaching-files
- https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-githubs-form-schema

## User direction

Prefer hosting the contribution workflow on GitHub. Email is an acceptable alternative; the user proposed `contribute@knxbench.com` or another suitable prefix. No mailbox existence, configuration or successful delivery has been verified. Existing Q1–Q7 decisions remain: own-instance analysis first, explicit sharing tiers, maintainer validation, no automatic compatibility promotion or hardware contact.

## Verified platform constraints

Primary sources [1]–[3] were fetched directly over HTTPS (HTTP 200), converted to text and inspected on 2026-10-08. Source [3] redirected to the canonical URL below. These are fresh GitHub facts, distinct from the earlier interview's corrected unverified KNX/OWASP reference leads.

1. GitHub Pages publishes static files; it does not provide server-side PHP/Ruby/Python execution [1]. A Pages contribution guide or form can link to another service, but Pages alone is not a private upload-processing backend.
2. Browser issue/PR/comment attachments include ZIP archives. GitHub documents a 25 MB limit for files other than the separately listed image/video types [2]. An evidence bundle needs its own narrower admission policy and a truthful oversized-file path; email attachment limits have not been verified.
3. Files attached in public repositories can be accessed without authentication. In private/internal repositories, viewing attachments requires repository access [2]. A hard-to-guess attachment URL is not confidential access control. Upload begins when the attachment is added, before issue submission [2]; consent must precede that action.
4. GitHub private vulnerability reporting is documented for security vulnerabilities in enabled public repositories [3]. Do not repurpose security advisories as the generic product/project sample inbox.
5. Live `gh repo view KNXBench-Labs/KNXBench --json nameWithOwner,visibility,hasIssuesEnabled,url` returned `visibility: PRIVATE`, issues enabled, exit 0. Do not change the main repository's visibility merely to enable community submissions. Its private status is an observed snapshot, not a permanent product policy.

## Proposed simplest workflow

- A separate public contribution repository, name not yet chosen or created, holds a guide, issue forms, public-safe reports, explicitly approved sample attachments and triage status. Pages is optional for a static guide.
- KNXBench analyzes on the contributor's own instance and exports a versioned evidence ZIP. Public contribution opens a prefilled GitHub issue/form in the browser; the contributor attaches the chosen bundle there. This is browser-mediated submission, not a completed background upload by KNXBench.
- Confidential originals and contextual samples go privately to `contribute@knxbench.com`, if the mailbox is configured and accepted by the user. Export plus prepared email text is sufficient initially; do not claim a mailto action attaches a file or confirms delivery. Plain email is not a claim of end-to-end encryption; source secret handling needs a documented boundary.
- No maintainer PAT, mail password or reusable shared submission credential is embedded in the application or a Pages site. No direct GitHub-authenticated automation is currently proposed.
- Keep original source archives out of the public repository/history. If the maintainer derives a minimized regression fixture, review its semantics, identifying information and redistribution permission before publishing it.

## Open user decision

Q8: accept the split between explicitly public GitHub contributions and private email samples, or require all contributions to stay private? GitHub-public originals would contradict the previously accepted private-original tier. Neither repository creation nor mailbox setup is authorized by the hosting preference alone.

Still deferred: mailbox setup/provider/limits, confidentiality guidance, retention/deletion/backup expectations and direct authenticated submission automation. The local analysis/export package remains independently implementable once final alignment and go are given.

## Sources

[1] https://docs.github.com/en/pages/getting-started-with-github-pages/creating-a-github-pages-site — GitHub Docs, Creating a GitHub Pages site

[2] https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/attaching-files — GitHub Docs, Attaching files

[3] https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/report-privately — GitHub Docs, Privately reporting a security vulnerability
