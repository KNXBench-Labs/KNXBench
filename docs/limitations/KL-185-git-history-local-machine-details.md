# Git history still carries local machine details

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-185)

<!-- BEGIN retained catalogue record -->

## Current tree clean, history not rewritten (17 October 2026)

Since [ADR-0135](../adr/0135-repository-carries-no-local-machine-details.md)
the tracked files carry no local machine details, and
`tools/check_local_info.py` keeps it that way. Earlier commits are unchanged:
their file contents and seven commit messages (one of them the commit tagged
`v0.1.0-alpha.5`) still name local paths, the host name or the kernel release.
Anyone who clones the repository can read them with `git log -p`.
Removing them needs a history rewrite and a force-push, which changes every
later commit hash and moves release tags; that is an owner decision and has
not been made. The two reorganization receipts name the replaced bytes of
eight redacted records by digest only.
<!-- END retained catalogue record -->
