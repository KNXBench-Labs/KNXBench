# Contributing to KNXBench

Thank you for wanting to help. KNXBench is a free, Linux-first KNX engineering
tool with a single maintainer, so a clear bug report is often worth more than a
patch. A patch that arrives with tests and a documentation update is worth more
than three that do not.

This page is the short version. The full guide, with every gate explained, is
the manual's [Contributing chapter](docs/manual/development/01-contributing.md).

## Before you start

- The [Code of Conduct](CODE_OF_CONDUCT.md) applies everywhere in the project.
- **Security problems are never reported in public.** Follow
  [SECURITY.md](SECURITY.md).
- **Never post somebody else's installation.** Project files, bus captures,
  logs and screenshots can contain floor plans, customer names, addresses,
  gateway IP addresses and KNX Secure keys. Use a synthetic file that shows the
  same problem, or one you are free to publish.
- **Never post material you may not share,** such as manufacturer product
  catalogues or licensed specification documents.

## Ways to contribute

| You have | Do this |
| --- | --- |
| A bug | Open an [issue](https://github.com/KNXBench-Labs/KNXBench/issues): what you did, what you expected, what happened (error text verbatim), your version and distribution. **File → Debug report…** collects most of it. |
| A project or product file KNXBench cannot read | **File → Analyze support gaps…**, then the [contribution form](https://github.com/KNXBench-Labs/KNXBench/issues/new?template=analysis.yml). Read the [step-by-step guide](docs/contribution-intake/README.md) ([Deutsch](docs/contribution-intake/DEUTSCH.md)) first. |
| A device KNXBench refuses to program | **File → Device support report**, see the [reporting guide](docs/manual/user-guide/08-reports-and-diff.md#universal-device-support-reporting). |
| An idea or a question | Open an issue and describe the problem it solves. Questions are welcome; "the manual should have said that" is a good outcome. |
| A documentation fix | Open a pull request. Typos, wrong commands and outdated screenshots count. |
| Code | Read on. |

## Contributing code

1. **Talk first about anything bigger than a small fix.** Open an issue or
   comment on one. That is a cheaper conversation than a rejected branch, and
   [Ideas and roadmap](docs/manual/ideas-and-roadmap.md) may already have an
   opinion.
2. **Fork, branch from `main`, and keep the change focused.** One change per
   pull request; no unrelated refactors on the way.
3. **Add tests and update the documentation in the same pull request.**
   Behaviour that changed without its documentation is not finished. The
   manual chapter lists
   [where each kind of fact belongs](docs/manual/development/01-contributing.md#documentation-is-part-of-the-change).
4. **Run the gates locally.** CI is switched off at the moment, so a missing
   GitHub check means nothing; the local gates are what counts. One command
   plans and runs the checks your change needs:

   ```bash
   python3 tools/run_package_gate.py --base upstream/main --dry-run   # show the plan
   python3 tools/run_package_gate.py --base upstream/main             # run it
   ```

   `--base` is the commit your branch started from (`upstream/main` if that is
   your name for this repository). The individual commands and what they
   protect are listed under
   [the gates a change has to pass](docs/manual/development/01-contributing.md#the-gates-a-change-has-to-pass)
   and in [VERIFICATION.md](docs/VERIFICATION.md).
5. **Keep your machine out of the history.** The repository is public, and a
   guard rejects absolute home paths, host names and real network addresses in
   files and commit messages:

   ```bash
   python3 tools/check_local_info.py
   python3 tools/check_local_info.py --commits upstream/main..HEAD
   ```

6. **Write commit messages that are right first and funny second.** The
   history uses a conventional shape (`fix(server): …`). Humour is welcome,
   inaccuracy is not. See [Commits](docs/manual/development/01-contributing.md#commits).

## Rules specific to KNXBench

- **Data integrity comes first.** An importer never silently drops
  information: it keeps what it does not understand, or marks and reports it
  as unsupported.
- **Never claim compatibility you have not tested.** The wording is
  "KNX-compatible". Never "KNX-certified", never "full ETS compatibility".
- **No guessing at proprietary formats or protocol behaviour.** Write down the
  evidence (a real file, a captured frame, a published source) in
  [`docs/research/`](docs/RESEARCH.md) or the matching document, and say
  plainly what is still unverified.
- **Keep the layers apart.** The KNX core does not depend on the UI, and
  external formats do not dictate the domain model. `cargo run -p xtask --
  check-layering` enforces it; the
  [Architecture tour](docs/manual/development/03-architecture-tour.md) explains why.
- **No manufacturer products in code.** Manufacturer data arrives through the
  product database layer, never as hard-coded devices.
- **Say how bus code was tested.** A change that sends telegrams or programs
  devices states whether it ran against real hardware, and which.
- **AI-assisted contributions are welcome.** Much of KNXBench is written with
  AI coding agents. You are responsible for every line you submit: read it,
  run it and test it as if you had typed it yourself.

## Licensing

KNXBench is licensed under the
[GNU Affero General Public License v3.0 or later](LICENSE). Your contribution
is licensed under the same terms. There is no contributor licence agreement
and no copyright assignment
([ADR-0054](docs/adr/0054-no-contributor-license-agreement-agpl-only.md)). New
dependencies must pass `cargo deny check`; GPL-licensed crates are not
accepted into the runtime dependency graph.

## What to expect

One maintainer, evenings and weekends, no service level. Issues and pull
requests may wait a while. Small, focused pull requests with a test and a
documentation update are merged fastest.
