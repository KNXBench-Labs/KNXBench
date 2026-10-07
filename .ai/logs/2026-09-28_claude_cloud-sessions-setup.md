# 2026-09-28 — Claude: cloud-session preparation

**Branch:** `cloud-sessions-setup`, fast-forward onto `main`. This is tooling
and documentation only; no product code changed.

## Why

The user received a one-time $250 credit for Claude Code cloud sessions:

- claim by 2026-10-07;
- it expires on 2026-11-04;
- it applies only to cloud sessions, not to local Claude Code or Hermes.

The goal was to prepare the repository so that self-contained work can run
there safely.

## What

- **`docs/CLOUD_SESSIONS.md`:**
  - boundaries: no corpus, no bus, local integration;
  - the one-time environment configuration;
  - the operating procedure and the first-session check;
  - task briefs CT-1 … CT-5.
- **`tools/cloud/SESSION_RULES.md`:** the agent rules. They are printed into
  every cloud session by the hook.
- **`tools/cloud/session-start.sh`:** the SessionStart hook. It returns
  immediately unless `CLAUDE_CODE_REMOTE=true`. It sets the git identity,
  puts cargo on `PATH` via `CLAUDE_ENV_FILE`, runs `npm ci`, and prints the
  environment report.
- **`tools/cloud/setup-env.sh`:** the content for the claude.ai environment's
  setup-script field. It installs the CI's Tauri/WebKit dev packages and Rust
  1.98.0 through rustup from `static.rust-lang.org`, which is on the Trusted
  allowlist. It always exits 0.
- **`.claude/settings.json`:**
  - `attribution.commit = ""`, `attribution.pr = ""`,
    `attribution.sessionUrl = false`;
  - the hook registration.

  Empty strings are used instead of `attribution: false` because the docs say
  pre-2.1.281 versions reject `false` and skip the whole file.
- **`.gitignore`:** `.claude/` becomes `.claude/*` plus
  `!.claude/settings.json`. `settings.local.json`, which holds
  bypassPermissions and the Headroom hook, stays ignored.

## Evidence

- Docs were read on 2026-09-28 (`cloud-environments.md`,
  `claude-code-on-the-web.md`, `settings-reference.md`, `hooks.md`):
  - the VM runs Ubuntu 24.04 as root, with 4 vCPU / 16 GB / 30 GB;
  - the setup script has a cache budget of about 5 minutes;
  - Trusted includes `static.rust-lang.org` and `rustup.rs`, but not
    `sh.rustup.rs`;
  - cloud commits get `Claude-Session:` trailers unless
    `attribution.sessionUrl=false`;
  - SessionStart stdout becomes agent context.
- **Probe:** `docker run ubuntu:24.04` as root, 4 CPUs / 16 GB, on a clean
  export of the branch.
  - Run 1 found a real bug. `rustup-init` saved under a `mktemp` name fails
    with "unknown proxy name: 'tmp'", because it dispatches on argv[0]. The fix
    is to save it as `<tmpdir>/rustup-init`.
  - Run 2:
    - `setup_exit=0` after 55 s;
    - hook exit 0, reporting rustc 1.98.0, webkit present, `OriginalData`
      absent, and the identity set;
    - `CLAUDE_ENV_FILE` received the PATH line;
    - `cargo check -p knx-desktop` exited 0 after 63 s.
- **Local no-op:** with `CLAUDE_CODE_REMOTE` unset, the hook prints 0 bytes.
- **Gates:** `check-anchors` ok, `check-headers` ok (233/161, ceiling 161),
  `git diff --check` clean.

## Not verified / open

- Behaviour on the real cloud VM, which has Node 22 and a pre-installed Rust.
  The probe container had Node 18 from Ubuntu. The first-session check in
  CLOUD_SESSIONS §3.1 covers this.
- Whether the empty attribution strings really suppress both trailers in
  cloud commits. The first PR must be inspected.
- The cost of one brief in credit. It is unknown; measure it after CT-1.
