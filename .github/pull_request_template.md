<!--
Thanks for the pull request! Short guide: CONTRIBUTING.md.
Security problems do not belong here; see SECURITY.md.
Never attach a real customer project, keyring, password or KNX Secure key.
-->

## What and why

<!-- What changes, and which problem does it solve? Link the issue. -->

Closes #

## How it was tested

<!-- Paste the last line of the gate run, or list the commands you ran. -->

- [ ] `python3 tools/run_package_gate.py --base upstream/main` passed
- [ ] New or changed behaviour has a test that fails without this change

**KNX bus or device programming** (pick one):

- [ ] Not affected
- [ ] Tested on real hardware: <!-- gateway and devices -->
- [ ] Affected, but not tested on real hardware

## Checklist

- [ ] One focused change, no unrelated refactoring
- [ ] Documentation updated (manual, `IMPLEMENTATION_STATUS.md`, `KNOWN_LIMITATIONS.md`, an ADR for a lasting decision), or not needed
- [ ] Imported data is kept or reported, never silently dropped (if import or export changed)
- [ ] No compatibility claim without evidence
- [ ] No customer data, manufacturer catalogues or licensed specifications
- [ ] No local machine details (`python3 tools/check_local_info.py --commits upstream/main..HEAD`)

By opening this pull request, I license my contribution under the
[AGPL-3.0-or-later](https://github.com/KNXBench-Labs/KNXBench/blob/main/LICENSE), the license of this project. There is no CLA.
