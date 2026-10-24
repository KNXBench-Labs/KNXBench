# Session notice: compact restart snapshot

Andre explicitly authorised this change on 24 October 2026. Forward the following message to the ongoing implementation session; this file is not a live-delivery acknowledgement.

## Copy-ready message

Der Handover wurde auf Andres Wunsch verschlankt. Bitte vor deinem nächsten Dokumentations-Checkpoint das neue `AGENTS.md` und `.ai/CURRENT_STATE.md` von `docs/compact-handover-20261024` bzw. nach Integration von `main` lesen.

`CURRENT_STATE.md` ist jetzt eine überschreibbare, modellunabhängige Momentaufnahme mit Ziel höchstens 50 Zeilen, kein fortlaufendes Codex/Claude-Journal. Aktualisiere nur bei relevanten Änderungen an Auftrag/Stop-Go, Zuständigkeit/Branch, unfertiger Arbeit, Gates/Evidenz, Blockern oder nächstem Schritt; nicht für triviale Fragen. Größere Übergaben können separat in `.ai/logs/` liegen. Vor jeder Änderung frisch lesen und fremde aktive/geparkte Tracks bewahren.

Dein WP-06-Track bleibt erhalten. Deine Worktree- und Gate-Eingaben wurden nicht geändert. Bitte beim Integrieren dieser Dokumentationsänderung deinen dann aktuellen WP-06-Stand erhalten; der Snapshot enthält nur eine datierte Beobachtung, keine vorweggenommene Abnahme. Das alte Journal ist bytegenau in `.ai/archive/CURRENT_STATE_2026-10-24_pre-compact.md` gesichert.

Repo-Dokumente bleiben verbindlicher Projektstand, der Vault enthält langlebiges Wissen, Hermes-Sessions den Gesprächsverlauf. Das Profil-SOUL und die Vault-Notiz `git-regeln.md` sind angepasst. Diese Nachricht ändert weder deinen Auftrag noch Freigaben und verlangt keinen Stop oder Neustart laufender Tests.

## Verification and delivery boundary

- This is an instruction/documentation-only change, not product or hardware acceptance. Verification covers the documentation checker, heading anchors, local-info scan, whitespace, the snapshot line budget and byte-exact archive preservation; no product build or runtime test is claimed.
- The owner chose manual forwarding because the separate browser login was not accepted. No live delivery or recipient acknowledgement is claimed.
- The pre-change journal is preserved byte-for-byte from `57bdb354c01d0dd225408cd4f46f0cece59ff3cd` (SHA-256 `ca624c7a2280491522d7a1b788d87b5a0f6ffd4dbb5a6d2cd026ac0ce93122bc`). Historical links retain their original context; use that source revision when resolving them.
