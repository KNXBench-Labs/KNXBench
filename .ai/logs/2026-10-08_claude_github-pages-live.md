# 2026-10-08 — Claude: knxbench.com live on GitHub Pages

Owner report: Pages "set up", TXT record present, nothing answers. Diagnosis
(read-only): only the `_github-pages-challenge-KNXBench-Labs` TXT existed (account
domain verification); no apex A/AAAA, no `www`; no Pages site in any repo
(API 404). Registrar per RDAP: Mesh Digital Limited (IANA 1390); owner: Host
Europe; nameservers `ns45/46.domaincontrol.com`.

DNS (owner, Host Europe): A 185.199.108-111.153, AAAA 2606:50c0:8000-8003::153,
`www` CNAME knxbench-labs.github.io (values from github/docs raw source);
verified record-by-record at both authoritative nameservers via raw DNS queries
(first pass missed one A and all AAAA; owner added them).

Implementation (`0f4963d1`): story `build --approval` published variant behind
`release.check_approval` (4 new tests, gate-bypass mutant caught by
`test_stale_or_missing_approval_writes_nothing`); website `--release` (no
banner/launch note/noindex, open robots, CNAME, privacy text citing GitHub's
"Data collection" doc), `--story-approval` override for the refusal test;
preview output byte-identical except the info pages' feedback link (diffed);
contact text no longer names the interim repo (caught by the new release test);
`.github/workflows/pages.yml` (starter-workflow action versions). Gates: website
20, story 67, Chromium offline 154 preview + 22 release, doc gates 5/5.

GitHub: `POST pages build_type=workflow`, `PUT cname=knxbench.com` (domain
`verified`), first run 37791564504 green (build + deploy). Certificate
`approved` for knxbench.com + www; `https_enforced=true` set. Live: IPv4 200 on
/, /de/, /en/privacy/, /story/; www 301 → https://knxbench.com/; all 34 files
of the live build-manifest hash-equal to the local release build; live Chromium
screenshot inspected, zero console errors, zero foreign requests. Not verified:
IPv6 (this host has no IPv6 route), http→https redirect still answered 200 at
16:22 right after enabling enforcement. Actions annotate Node 20 deprecation
for checkout@v4/configure-pages@v5/upload-artifact@v4.
