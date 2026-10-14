# Help improve KNXBench — one report at a time

**[Deutsch: vollständige Anleitung](DEUTSCH.md)** · English below

Found a project or product that KNXBench cannot handle as expected? Tell us.
You do **not** need Git, a terminal, XML knowledge, or a KNX specification.
A short description is already useful. A report ZIP helps us investigate.

> **Before sharing:** this repository and its attachments are public. Never
> attach your original `.knxproj` / `.knxprod`, customer data, passwords,
> keyrings, security keys, or confidential manufacturer files. Start with
> KNXBench's **reduced report**, not extra XML files. Reduced is not a guarantee
> of anonymity: inspect what you share.

## The short route

**Choose file → Analyze → Preview → Give permission → Download ZIP → Attach to an issue → Submit.**

The app explains the next action and has a **How to contribute — step by step**
guide in the analysis dialog. Nothing is automatically submitted.

[Open the contribution form](https://github.com/KNXBench-Labs/KNXBench/issues/new?template=analysis.yml)

**No ZIP or no analysis menu?** You can still use the form. Describe what you
tried and what happened; leave optional fields empty. Do not attach the original
file instead. No need to build KNXBench from source just to report a problem.

## Before you start

- For an issue, you need a GitHub account and must sign in. You do not need Git.
- For an analysis, use a KNXBench build containing **File → Analyze support gaps…**.
  In German: **Datei → Unterstützungslücken analysieren…**.
- Have an existing `.knxproj` project or `.knxprod` product package that you are
  allowed to analyze. The raw file must be at most **32 MiB**. Do not unpack it.
- Choose a file from your own permitted material, not a customer's confidential
  installation. Permission to use a product does not automatically allow public
  redistribution of its files.

## 1. Select the file

Open **File → Analyze support gaps…** and select your file.

**What happens:** selection alone does not upload it. You can still close the
dialog. Other extensions, empty files and oversized files cannot be analyzed here.
Do not rename a file to get around that check.

## 2. Start analysis

Click **Upload & analyze on this instance** and wait for the results.

**Where does my file go?** To the KNXBench instance you are using, which may run
on another computer. Not to GitHub. Analysis does not modify your open project
or product database and does not connect to KNX hardware.

**What do the results mean?**

- **Measured:** that check ran; this is not complete support.
- **Refused:** an importer could not accept the format. That is useful evidence,
  not a mistake you must repair before reporting.
- **Partial:** some work could not finish, for example because of a scan limit.
- **Unavailable / Not examined:** no result for that check, **not zero problems**.
- **Analysis complete:** the stated analysis finished, not full ETS compatibility.

You do not need to interpret the XML, finding IDs or technical counts.
Offline planning does not prove that a physical device works.

### Additional offline procedure observations

Local source builds can show **Offline procedure sequences** for the first
modern product family (`MV-07B0`, AP1). **Sequence expanded only** means its
declared order was reconstructed, not that a download plan or a working device
is available. Unknown steps and missing/conflicting references remain visible.
Other masks/variants and complete device semantics are outside this slice.

You can report these observations through the same preview/ZIP/issue route.
Local sequence details include identities, source fingerprints and original
values: do not copy them blindly into a public issue. The reduced ZIP omits
those details while keeping value-free issue-code counts. Additional XML needs
separate selection, review and permission. Nothing is automatically sent.

## 3. Keep the simple, reduced report

Leave **Public GitHub issue** selected. Leave **all optional context samples
unchecked**. This produces a reduced report with results and structural
observations, without the original file or source-value samples.

**Why not add all the samples?** Those are complete, unmodified XML files. They
may contain names, addresses, device identifiers and proprietary data. They are
**not anonymized**. Start without them; a maintainer can ask for more context.
Only add a sample if it is needed, you reviewed it and you may publish it.

## 4. Preview and give permission

Click **Preview selected evidence**. Look at the actual outgoing files:

| File | In plain language |
| --- | --- |
| `findings.json` | The analysis results, with private source values/details reduced. |
| `manifest.json` | The file list, sizes and fingerprints; no manual calculation needed. |
| `README.md` | How a maintainer should review the report. |
| `samples/member-N.xml`, if selected | Unchanged original XML: inspect especially carefully. |

Check that you may publish the contents, then tick the permission checkbox.
If something looks confidential, **do not share it**. Reduction is not a complete
secret detector. Never add an original to a public issue.

**Download disabled?** You need a completed preview and the permission checkbox.
Changing the file, samples or sharing channel clears them; preview and confirm again.

## 5. Download the ZIP

Click **Download evidence ZIP**. Check your browser's downloads and save
**`knxbench-evidence.zip`** somewhere you can find it. Leave it packed.

**What happens:** the app requests a download. It does not know that you saved
it and does not upload it to GitHub. Public ZIPs are limited to 24,000,000 bytes,
below GitHub's documented 25 MB limit for other attachments. [1]

## 6. Attach and submit on GitHub

1. Click **Open public contribution form**, or use the form link above.
2. Sign in if asked. A normal GitHub account is enough; no developer setup.
3. Enter a short title, for example **“Product analysis is refused”**.
4. In **What did you do, and what happened?**, write a few sentences. Example:
   “I selected a product package and clicked Analyze. The import check says
   Refused. I expected the product to be available. I do not know the reason.”
   This is an **example description**, not a measured KNXBench result.
5. In **Report ZIP (optional)**, drag the saved ZIP into the text field, or use
   the attachment chooser. **Wait until the attachment link appears.** [1], [3]
6. Leave other optional fields empty if you do not know the answer. The app's
   link supplies the analyzer-version field when an analysis exists; GitHub
   supports field-ID query prefills. Check it if present. [2], [3]
7. Confirm the two publication checkboxes and click **Create issue**. [2]

> **Important:** attachment upload starts when you attach the file, before you
> submit the issue. Public attachments can be opened without authentication.
> Check permission and privacy **before attaching**, not only before submitting. [1]

**How do I know it worked?** GitHub opens your issue page with an issue number.
Keep that page's address; it is where follow-up questions appear. An open form,
a download, or an attachment link alone does not mean the issue was submitted.
Creating an issue is not a promise of a fix, compatibility approval or response time.

## If something goes wrong

- **No analysis menu:** report in the form without a ZIP. Describe your installed
  build/version if you know it; otherwise leave the version blank.
- **Analysis refused or partial:** share the reduced report if export is available.
  Do not change schema numbers, XML or extensions to make the result look successful.
- **No export available:** describe the error in your own words. Do not post the
  source archive, passwords, screenshots containing customer data or confidential logs.
- **Browser blocked the download:** check the browser's download notice and retry
  only after reviewing the same preview. Do not assume a ZIP was saved.
- **ZIP too large:** remove optional XML samples and preview again. Do not split
  or publish the original archive instead.
- **Cannot attach on GitHub:** check that you signed in, wait for the upload to
  finish, and try the reduced ZIP again. A description without a ZIP is acceptable.
- **Private original blocked:** the app found known key-like fields or unexamined
  content. This guard cannot be overridden here. Do not publish that file manually.

## Device programming refused before a plan exists

In the device download tab, the notice shows the reason, a concrete next action,
and **Instructions**, **Report support gap**, and **Email draft**. This source-build
feature works before any plan, connection or download report exists. Screenshots
of Alpha 11 still show the released UI, not this main-only addition.

- **Untested**: a complete supported plan has no matching hardware verification.
  It still needs the additional exact target-bound acknowledgement and ordinary
  confirmation. All identity/key, freshness, backup/readback and bus guards remain.
- **Unsupported**: a technical planning gap, not missing approval. Do not force it.
  Missing application data calls for the exact package/version in the catalogue;
  configuration errors call for the correction named in the local details;
  missing byte-order/encoder/procedure/module support calls for evidence-based review.
- A server/transport failure is not a device compatibility verdict. Check server
  availability and retry deliberately. Required executable-extension behavior that
  is unresolved remains outside this workflow; absence of a `.dll` filename proves
  nothing. Plan-scoped backup is not full-device backup or universal rollback.

To report a pre-plan refusal: open **Report support gap**, sign in to GitHub,
describe what you tried and the shown reason/next action, review optional fields,
and submit the `analysis.yml` support-gap form yourself. No ZIP is required.
A refusal without a connection has no post-run download report: do not fabricate
one. If you separately analyze permitted source data, use the reduced ZIP only
after its existing preview and sharing checks.

Alternatively, **Email draft** opens an unsent inquiry to **issue@knxbench.com**.
This mailbox accepts device-programming support-gap inquiries as well as tester
download reports. No attachments or project/device metadata are added. Write and
review your own description, check the recipient and send deliberately. Email is
not a promise of delivery, encryption, confidentiality, retention or deletion.
Do not send private originals, customer data, passwords, keys or backups without
a separately confirmed handling agreement. `contribute@knxbench.com` is not a
confirmed alternative intake route.

**Technical details (local)** may contain names, addresses, paths and values.
The copy button only copies locally; it does not sanitize or authorize public
sharing. The three fixed links contain no project-derived metadata. Nothing
uploads, opens a bus connection, submits an issue or sends mail automatically.
Use the separate **Download report** flow only after an actual ended run.

## Download reports are separate

Tested a device download? That has its own route: the **Download report** issue form
or **issue@knxbench.com** (the existing mailbox, also accepting device-programming support-gap inquiries), with the
report file KNXBench writes
([how](../manual/user-guide/10-command-line.md#helping-to-verify-a-device)). The
proposed mailbox below is for analysis contributions and is still unconfirmed.

## Confidential material and email

**Private intake is not confirmed yet.** `contribute@knxbench.com` is a proposed
mailbox, not a verified working inbox. First ask for a working private contact
route **without sending confidential data**. Do not assume an email will arrive.

Only after a maintainer confirms that route and its handling terms: choose
**Private contact / email draft**, review the intended contents, and obtain any
required original permission. Private originals may still be blocked. A mail
draft neither attaches nor sends the ZIP: attach it manually, check the recipient,
and send yourself. No guarantee of delivery, encryption, retention or deletion.
Never send passwords, keyrings or keys. Keep originals off public issues/history.

## What happens after submission?

A maintainer reviews the report, may ask for permitted context, compares actual
project/product/specification facts, and writes a regression test before a fix.
Samples do not run as plugins and a new schema is not accepted just because its
number changed. Updates follow on the issue; there is no automatic fix or
promised response time. Any public regression fixture needs permission review.
Public contributions may be indexed or copied; deletion cannot guarantee that
all copies disappear. Security vulnerabilities are not ordinary sample reports:
do not post secrets or confidential exploit-bearing originals here.

## Sources for GitHub steps

[1] https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/attaching-files

[2] https://docs.github.com/en/issues/tracking-your-work-with-issues/creating-an-issue

[3] https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-githubs-form-schema
