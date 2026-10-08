# 2026-10-08 — Claude: public launch (history purge + visibility switch)

User request: "go live … setze das Repo auf Public".

## Audit (before any change)

- Mirror clone of every public ref (`main`, tag `v0.1.0-alpha.5`; no other
  branches, no PR refs). Scanner over every reachable object
  (`git rev-list --objects --all` → `git cat-file --batch`): GitHub/AWS/Slack/
  OpenAI/Anthropic/Google keys, JWTs, private-key headers, password
  assignments, home path, personal names, old account, RFC1918 addresses,
  suspicious file types, blobs > 5 MB.
- Result: no secret, no token, no personal identity (0 hits for every identity
  pattern). Password/token hits are test fixtures (`OWN_CLIENT_TOKEN =
  "1111…"`, synthetic legacy password `marvin-synthetic`, `0x12345678` key
  sentinel); the private-key hits are a test that checks such a header is
  refused. Private IPs are documentation/test literals (gateway address of the
  owner's test bus included; RFC1918, accepted).
- **Blockers found in history only:** `az-and-sensor-data/` (66 files,
  3.6 MB, added 2026-09-02, removed 2026-09-06 "vendor dump": ETS-plugin
  AutoSave data of an Elka alarm panel and a sensor plugin, 28 manufacturer
  firmware `.hex`/`.S19`, plugin help, compiler files) and
  `KV v2.5 - demo.knxproj` (third-party KNX Virtual demo project with a
  third-party user name in its project traces).

## Decision and execution

- User: rewrite after Codex pushed everything (answer to the follow-up
  question). Delete-and-recreate chosen over force-push because the older
  commit maps in `docs/history/` name full SHAs of commits that contained the
  payload; after a force-push GitHub can still serve such commits by SHA.
- Backup (local, never publish): `/mnt/daten-i/Sourcecode/KNXBench.backups/2026-10-08-pre-golive-purge/`
  (`public-refs.bundle`, verified; pre-refs; release assets + notes).
- `git filter-repo --invert-paths --path az-and-sensor-data/ --path 'KV v2.5 - demo.knxproj'`
  on a fresh mirror: 2,311 commits → 30 unchanged, 2,279 rewritten, 2 dropped
  as empty (`b79a1c40` removal, `a25bbad6` demo add). Every commit whose tree
  did not hold one of the paths kept a byte-identical tree; final `main` tree
  `cd8fc1db` and tag tree `42c64555` unchanged. Rescan of the purged history:
  0 objects under the removed paths, no new finding.
- Docs commit `be179214` (map `docs/history/COMMIT_MAP_2026-10-08-public.txt`,
  KL §162 addendum, status entry); `check-anchors` (fresh target, baked root
  verified) 442 links ok; `git diff --cached --check` clean.
- Old repository renamed to `KNXBench-Labs/KNXBench-prepublic-20261008`
  (stays **private**, rollback; holds the star). New `KNXBench-Labs/KNXBench`
  created with the same description, homepage, topics, wiki off, issues on.
- Push `main`, then CI + Linux AppImage workflows disabled before the tag push
  (the tag workflow uploads with `--clobber`); no run was triggered. Tag
  pushed (`0f792fc0` → `a0ff1f55`). Pre-release recreated with the original
  AppImage + SHA256SUMS; notes: "private repository" wording removed, purge
  note added. Downloaded back: `sha256sum -c` OK, `cmp` identical. AppImage
  workflow re-enabled, CI stays disabled (previous state).
- Final mirror of the new remote: refs `main be179214`, tag `0f792fc0`; 0
  objects under removed paths; old commit `60ed50b8` not present.
- Visibility switched to **public**; anonymous checks: web 200, API 200,
  release asset 200, credential-less clone ok.
- Security: Dependabot alerts + security updates re-enabled (previous state);
  secret scanning and push protection newly enabled.
- Root checkout: local `main` moved from `e99a94e9` to its tree-identical
  counterpart `9fd91108` (`update-ref` with old-value guard); porcelain status
  byte-identical before/after; root now `behind 61` on the new history. Tags
  force-fetched.

## Not done / follow-ups

- Older local branches, stashes and the scratch worktree `devnav-ro` still
  carry the old history (incl. the payload). Never push or merge them as they
  are; rebase with `--onto` using the map.
- Story companion commit links cite pre-rewrite hashes (owner remap).
- ADR-0091 launch consolidation (issue forms/guides into main repo, app and
  contact links), README/website wording that still says "private", and the
  community intake repo's links: separate packages.
- Old private repo `KNXBench-prepublic-20261008`: user deletes it when happy
  (needs a token with `delete_repo`).
