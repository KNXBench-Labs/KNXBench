# Synthetic flow-study keyboard identity correction

- During LCARS publication, complete Chromium attempt 1 had 161 expected / 1 unexpected. The unrelated standalone flow-study keyboard assertion used a live sorted `first()` locator for focus and again for assertion. New synthetic nodes can change that identity.
- Original trace confirms the selected-device inspector is **9.1.10** with **Current values**, while the failed assertion resolves the now-first **9.1.1** and expects it selected. The fixture/engine/model are identical to published main and set no LCARS presentation. This is assertion-target drift, not failed keyboard selection.
- Capture the initially chosen button's name, resolve that stable accessible identity, explicitly assert focus, send the actual Enter key, and retain the selected/current-value assertions. No sleeps, retries, skips, mouse clicks or product-code workaround.
- Targeted un-retried repeat: **20 expected, 0 unexpected/flaky/skipped**. Complete combined gates rerun after the fix; their final result is recorded in the ambient publication receipt.
- Commit this two-path test/evidence correction separately from the LCARS feature. Original failed JSON/trace retained privately until package closure; publish only this synthetic identity evidence.
