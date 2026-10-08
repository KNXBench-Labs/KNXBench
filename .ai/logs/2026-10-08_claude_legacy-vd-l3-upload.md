# 2026-10-08 — Claude — Legacy VD L3: web upload and one remembered password

## Scope

ADR-0094 L3 covers:

- a server route and web upload for `.vd3`–`.vd5`;
- a password dialog;
- one remembered password (0600, XDG config), which can be set and
  forgotten from the CLI and the web app;
- a visual check of a real legacy device in the web app.

Download stays L4.

## Decisions

- **Password policy in `knx-app`, not in the server or the client.** The
  given password wins, then the remembered one, then a refusal. Nothing else
  is tried (no candidate list; GPL firewall and grilling Q3/Q12 unchanged).
- **One plain 0600 file, no keyring.**
  - No new dependency, and it works headless on a server.
  - The cost is a plain-text secret readable by the server user. This is
    documented in KNOWN_LIMITATIONS §128, and the user approved a local
    file explicitly.
- **Remember only after success.** A wrong password is never kept.
- **A separate route** (`/api/catalog/install-legacy`) instead of
  overloading `/api/catalog/install`. The package route names a legacy
  file as `422 legacyProductDatabase`, so the client can move on. Its old
  400 text stays because four crates pin it.
- **Settings access is injected** (`legacyPassword` prop). Theme and panel
  tests pin exact fetch sequences, and an unconditional fetch on mount
  would have shifted them.

## Findings during the work

- The nested `Overlay` renders inside `.device-wizard-step`. A broad
  `input { width: 100% }` rule there stretched checkboxes in any nested
  dialog. Excluded checkboxes; the e2e spec pins it (mutant
  `css-checkbox-stretched`).
- Real-data UI check: parameter selects commit on blur, so a Playwright
  probe must focus, select, then press Tab. Without the blur, no POST is
  sent, which looks like "visibility does not react".
- Visibility in N000520 swaps same-named fields ("Objektwert für EIN" as
  on/off vs. percent). Counting visible labels therefore cannot show it;
  compare `etsId` sets or control contents instead.
- `TypeNone` spacer parameters are reported `editable: true`. The server
  refuses writes by name. This is queued as its own fix (KNOWN_LIMITATIONS
  §128).

## Evidence

- Mutation sweep 19/19 named, sources restored. The sweep script lives in
  scratch only.
- Review findings: three minors, all fixed test-first (temp-file race red
  4/4, then green).
- Gate (head `ec4a2b07`): web 147/2,402 + Chromium 175; Rust 3,697/0/182; corpus 4/4; xtask x5. Details in IMPLEMENTATION_STATUS.
