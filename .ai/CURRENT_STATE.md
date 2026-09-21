- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 21:52 CEST
- **Completed:** T09/E2 read-only Design-Vorprüfung nach `goal.md` und Task-Brief abgeschlossen. Bestehende Bausteine bestätigt: `compare_with_project`/`ProjectComparison` liegen derzeit im CLI, normale `CreateDevice`-/`DeleteDevice`-/`Batch`-Commands samt Undo existieren. Kleinster vollständiger Entwurf: Vergleich in eine wiederverwendbare Anwendungsschicht heben; UI zeigt `unexpected`, `missing` und `excluded_in_project` getrennt, startet mit leerer Auswahl, bietet für ausgeschlossene/ungeprüfte Adressen nie eine Aktion und führt nur explizit gewählte Änderungen als einen normalen undo-fähigen Batch aus. Kein zweiter Mutationspfad und kein Busverkehr.
- **Pending/Next Steps:** Die verpflichtende Brainstorming-Freigabe des kurzen T09-Entwurfs durch den Nutzer abwarten. Nach Zustimmung Worktree `t09-scan-reconcile` von `main` erstellen, mit RED-Tests für byte-identisches Projekt ohne Auswahl, Apply+Undo-Bytegleichheit und aktionslose `excluded_in_project` beginnen, anschließend Server/UI integrieren und alle Gates/Review durchlaufen.
- **Notes Claude:** T08 ist vollständig auf `origin/main` (`b5cd10d`) gepusht. T09 wurde noch nicht implementiert und kein neuer Worktree angelegt; die Pause entsteht ausschließlich aus dem Hard-Gate der vorgeschriebenen Brainstorming-Skill, nicht aus technischer Unsicherheit.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 21:47 CEST
- **Completed:** T08/D6 nach frischem Gesamt-Diff-Review in `main` integriert. Merge-Commit `bc6e53e` wurde selbst vollständig verifiziert: Rust 1.939 Tests in 92 Result-Blöcken, Web 819/59, TypeScript, fmt, Workspace-Clippy, Layering, Header 186/162 (Ceiling 162), Anchors 376/175 und `cargo deny` jeweils Exit 0. Der Review fand 0 Critical und 5 Important; Fix-Commit `b66ff60` behebt atomare terminale Snapshots, sessiongebundene Poll/Cancel-Aufrufe, Cancel-Poll-Races, aktive unveränderliche Ausschlüsse und veraltete Estimates. Re-Review bestätigt alle Befunde als behoben, keine neuen Critical/Important, Merge freigegeben. Feste Slash-Notation ohne Auswahl ist enthalten; Punktnotation bleibt nur kompatible Eingabe/Suche. Kein realer KNX-Verkehr und kein Hardwarezugriff. Detail: `.ai/logs/2026-09-21_codex__t08_line_diagnostics.md`.
- **Pending/Next Steps:** Ohne Zwischenstopp T09/E2 beginnen. T09 verwendet `LineScanResultsResponse` als Evidenz für einen standardmäßigen, explizit ausgewählten und undo-fähigen `Command`; ohne Auswahl keine Änderung, ausgeschlossene Adressen nur als ungeprüft anzeigen. Zwei nicht blockierende T08-Beobachtungen für später: Session-Mismatch-`409` könnte die alte UI-Sitzung explizit terminalisieren; interne Namen/Kommentare um `worstCaseMs` sind trotz ehrlicher UI-Beschriftung historisch zu stark.
- **Notes Claude:** T08-Branch bleibt vorerst im extern verwalteten Worktree `/mnt/daten-i/Sourcecode/KNXBench.worktrees/t08-line-diagnostics`; nicht automatisch löschen. T20-Functions-PDF-Befund aus `0b42014` blieb beim Merge erhalten. Alle Commits ausschließlich `KNXBench <github@knxbench.com>`, ohne Co-Author.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 20:13 CEST
- **Completed:** T20 in der spezifikationsabhängigen Bedeutung `Functions` als Read-only-Machbarkeitsprüfung untersucht. Primärbelege direkt in den lokalen PDFs geprüft: *Project Schema v2.3* §1.2.6.7–1.2.6.10 definiert `Function` unter einem `Space`/Gebäudeteil, `Function_t` samt `GroupAddressRef` und dessen Attribute; *KNX IoT Constants* (Definitionen „Application Function“, „ETS Function“ und „Function Point“ sowie Tabelle 1, S. 7–14) definiert die Begriffe und Zuordnung; *KNX IoT Information Model* §1.3.2.2.1, §1.3.2.2.2 und §2.1.2.1 (S. 25–30, 79) definiert Semantik, Kardinalitätsbeziehungen und die ausdrückliche Entsprechung ETS Function ↔ Application Function. Ergebnis: T20 ist für Schema 23 spezifikationsbasiert umsetzbar. Bestehende Bausteine sind `BuildingPart`, Gruppenadressen sowie bereits persistierte Masterdaten `FunctionType`/`FunctionPoint`; eine eigentliche `Function`-Domänenentität, Import und Projektion fehlen noch.
- **Pending/Next Steps:** Vor Produktcode eine eigene ADR/Designfreigabe für `Function` als Entität mit Eltern-`BuildingPart`, stabiler Quellidentität, optionalem Typ/`Implements`, geordneten `GroupAddressRef`s samt Rolle und projektweit eindeutigen PUIDs. Schema-23-Unterstützung kann normativ umgesetzt werden; Verhalten und Schreibweisen von Schema 11/21 bleiben ohne entsprechende Spezifikation oder reale Probe separat unbelegt und dürfen nicht extrapoliert werden. Kein Produktcode, keine Abhängigkeit und kein KNX-Verkehr in dieser Prüfung.
- **Notes Claude:** Die Nummer T20 kollidiert inzwischen: `.superpowers/sdd/goal/task-20-brief.md` meint Makro-Automation, während `goal.md`, `GAP_ANALYSIS_ETS.md` und `ROADMAP.md` T20 als `Functions`-Domänenkonzept führen. Wegen der expliziten PDF-Frage wurde hier ausschließlich das spezifikationsabhängige `Functions`-T20 bewertet. Die PDF-Evidenz ersetzt keine Corpus-Probe für alte ETS-Projektschemata.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 19:25 CEST
- **Completed:** T07 (`§61`) unabhängig reviewt und nach `main` integriert. `knx_core::encode` verlangt nun `DptInputFormat`; Legacy-Inferenz ist ehrlich als `encode_inferred_format` benannt. CLI/HTTP akzeptieren explizite Formate, und das Web zeigt eine sichtbare Formatwahl mit `Auto`-Kompatibilitätsmodus. `encoding_rulings` macht alle dokumentierten Projektentscheidungen für DPT-Haupttypen 1–30 öffentlich abfragbar. Review-Fixes bewahren Fixed-Width-Bitsets (`00000010` bleibt `0x02`), erlauben valides Hex `0B` und verhindern unsichtbare Web-Radixwahl. Keine beabsichtigte Wire-Kodierung geändert. Branch-Gates vollständig grün: Core 479, CLI 10, HTTP-Fake-Tunnel 13, Web 812/58, TypeScript, fmt, Workspace-Clippy/-Tests, Layering, Header 181/162, Anchors 377/174, deny. Kein KNX-Verkehr und kein Hardwarezugriff. Detailprotokoll: `.ai/logs/2026-09-21_codex__t07_dpt_input_formats.md`. Zusätzlich wurden die zuvor fremden, vom Nutzer ausdrücklich zum Commit freigegebenen Plan-/Handover-Änderungen als `57d7190` und die beiden Logo-Assets als `8acd846` separat committed.
- **Pending/Next Steps:** Merge-Commit `03c1316` ist selbst vollständig verifiziert (Rust- und Web-Gates Exit 0); `main` jetzt pushen und `goal.md` ohne Zwischenstopp bei T08 fortsetzen. Die dreizehn neuen Nutzer-Issues liegen als Plan unter `docs/superpowers/plans/2026-09-21-user-reported-issues.md` und bleiben eigener späterer Arbeitsblock.
- **Notes Claude:** Für KNX-Spezifikationsfragen sind die PDFs unter `/mnt/daten-i/Sourcecode/knx-spec-kb/sources/The KNX Standard v3.0.0` die Primärquelle; SQLite/Search dient nur zum Auffinden. Der erste Reviewer lief ins Nutzungslimit, hatte aber bereits den gültigen Compatibility-Fund geliefert; ein zweiter unabhängiger Review wurde vollständig abgeschlossen. Beim Merge wurde ausschließlich der erwartete `.ai/CURRENT_STATE.md`-Add/Add-Konflikt manuell zusammengeführt; beide Handover-Verläufe und das T07-Log bleiben erhalten.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 15:24 CEST
- **Completed:** T29 (`MalformedRefId`) unabhängig reviewt, drei Review-Funde korrigiert, als `240792b` nach `main` gemerged und zu `origin/main` gepusht. ETS6-Geräte-Refs `O-<n>_R-<m>` werden strikt gemappt und mit dem je Gerät aufgelösten Programm angereichert; Corpus-Test pinnt nun `has_losses()`, neun verbleibende Unknown-Summaries und Retention. Productdb und Mapper teilen die `u16`-Grenze. ETS4-Doku korrigiert auf 107 `AmbiguousDpt`; T29 ist `KNOWN_LIMITATIONS.md` §125. Vollständige Rust-Gates auf Branch und Merge-Commit Exit 0; Canary 252/252, Header 181/162 (Ceiling 162), Anchors 377/180. Alle Commits ausschließlich KNXBench `<github@knxbench.com>`, kein Co-Author. Detailprotokoll: `.ai/logs/2026-09-21_codex__t29_merge_t07_survey.md`.
- **Pending/Next Steps:** T07 (`§61`, DPT-Codec) läuft im sauberen Worktree `/mnt/daten-i/Sourcecode/KNXBench.worktrees/t07-dpt-explicit`, Branch `t07-dpt-explicit`, Basis `240792b`. Caller-Survey ist abgeschlossen, Produktcode noch unverändert: CLI und HTTP besitzen heute nur freien Benutzertext und kennen dessen Format nicht. Ein Enum darf die Vermutung nicht bloß umetikettieren; nächste Aktion ist die kleinste echte öffentliche Formatgrenze (Formatwahl in Aufrufern oder deterministisches pro-DPT-Format) samt RED-Tests für Format-Mismatch und sichtbarer Ruling-Metadaten. Die im Brief genannte `knx-spec`-Skill ist nicht verfügbar; lokale Spezifikationsdatenbanken/-artefakte verwenden, Fakten zitieren, kein Busverkehr.
- **Notes:** Fremder Dirty-State auf `main` (`.ai/CURRENT_STATE.md`, `CLAUDE.md`, gelöschtes `codex-goal.md`, `goal.md`, `docs/Issues.md`, zwei Logo-Dateien und der neue Issues-Plan) wurde beim Merge vollständig mit staged/unstaged Zustand wiederhergestellt und nicht inhaltlich verändert. T29-Branch/Worktree bleiben gemäß Branch-Finishing-Protokoll erhalten, weil keine explizite Aufräumentscheidung des Nutzers vorliegt.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 13:37 CEST
- **Completed:** Die 38 inhaltlichen Beobachtungen aus `docs/Issues.md` vollständig in 13 ausführbare, evidenz- und testorientierte Tasks unter `docs/superpowers/plans/2026-09-21-user-reported-issues.md` überführt. Bestehende Implementierung vor der Zerlegung geprüft: Busmonitor-Text-/Servicefilter, Discovery und getrennte Send-/Receive-Links existieren bereits und werden nicht dupliziert; unsicheres KNX-/ETS-Verhalten ist als Investigation-first markiert. `goal.md` §11 bindet den neuen Plan in den aktiven Backlog ein. `docs/Issues.md` ist danach auf exakt 0 Byte geleert. Verifiziert: 13 Task-Abschnitte, 67 konkrete Checkpoints, keine Platzhalter/trailing whitespace, `git diff --check` sauber, `cargo run -q -p xtask -- check-anchors` erfolgreich (377 Links in 181 Markdown-Dateien, keine toten Links).
- **Pending/Next Steps:** ISSUE-01 bis ISSUE-13 aus dem neuen Plan umsetzen; Überschneidungen mit bestehendem Dirty-State-, Settings-, Drag/Drop- und Discovery-Backlog jeweils beim bestehenden Owner zusammenführen. Keine Produktimplementierung war Teil dieser Aufgabe.
- **Notes for Claude:** Maßgebliche Task-Spezifikation ist `docs/superpowers/plans/2026-09-21-user-reported-issues.md`; die Source-Coverage-Tabelle belegt die vollständige Übernahme. ISSUE-06 (Grundstück/Site), ISSUE-09 (liniengebundene Adressbearbeitung) und ISSUE-12 (AppImage-Discovery) verlangen belegte Fakten bzw. Reproduktion vor Code. Fremde Arbeitsbaumänderungen an `CLAUDE.md` und den beiden Logo-Dateien blieben unangetastet.

---

- **Last Agent:** Claude
- **Timestamp:** 2026-09-21 07:19 CEST
- **Completed:** Goal-Run auf `goal.md` fortgesetzt und bei Session-Limit pausiert. Auf `origin/main` gemerged und gepusht: T26 (`fcf4563`), T33 Adress-Hygiene (`66b970e`), T25 Bus-Interface-Suche (`62ff969`). Alle neun Gates wurden jeweils auf dem Merge-Commit selbst gefahren, nicht auf den Eltern — genau das hat bei T25 Konfliktmarker gefangen, die ein `git add -A` auf einem Merge hineingetragen hatte. Stand auf `62ff969`: `cargo test --workspace` 1.916 passed / 89 Result-Blöcke / 0 failed, `npx vitest run` 809 passed / 58 Dateien, `tsc --noEmit` 0, fmt/clippy/layering/anchors/deny 0, `check-headers` 179 mit Header / 162 ohne (Ceiling 162, kein Spielraum). Stale-Binary-Canary nach KNOWN_LIMITATIONS §119: `knx_net` lib-Block liest **252**; dieselbe Zahl an jedem in diesem Lauf gemessenen Commit, zusätzlich unabhängig durch Zählen der `#[test]`/`#[tokio::test]`-Attribute bestätigt. Der Testzähler stieg von 1.909/88, weil T25 ein neues Integrationstarget `apps/knx-server/tests/http_bus_discover.rs` mitbringt.
- **Pending/Next Steps:** Vollständige Bestandsaufnahme des Rückstands, damit niemand sie noch einmal rekonstruieren muss. Gemerged und gepusht sind elf Aufgaben: T01, T01b, T02, T03, T04, T05, T06, T26, T27, T28, T33. T23 (Benutzerhandbuch) wurde außerhalb dieses Laufs geliefert. Offen ist Folgendes:
  - **T29 — wartet auf Review, sonst nichts.** Branch `t29-malformed-refid`, HEAD `45c83c6`, Bericht in `.superpowers/sdd/goal/task-29-report.md`. Er behauptet, die 867 `MalformedRefId`-Verluste seien nie malformed gewesen, sondern geräte-lokale `O-<n>_R-<m>`-Referenzen, 310 von 310 distinct ids lösten eindeutig auf, und das ETS6-Enrichment steige dadurch von 0 auf 867 Objekte. Die Evidenz ist **ein** Herstellersatz bei Schema 23 — daran muss ein Review ansetzen, nicht an den Formalien. Zwei Dinge sind beim Merge zu erledigen: (a) die Abschnittsnummer kollidiert, T29 schrieb `KNOWN_LIMITATIONS.md` §124, während `t25-discovery` noch offen war, und T25 hat §124 inzwischen auf `main` belegt — T29s wird **§125**, die Querverweise in `IMPLEMENTATION_STATUS.md` wandern mit, dieselbe Umnummerierung war schon zwischen T25 und T26 bei §123 nötig; (b) die Baselines im Bericht (1.909 / 88) sind vor-Merge, `main` steht auf 1.916 / 89 — auf der echten Merge-Basis neu messen, eine Baseline gehört zu genau einem Commit.
  - **Briefs geschrieben, nie dispatcht — sechzehn Stück.** Jeder liegt als `.superpowers/sdd/goal/task-<n>-brief.md` vor und enthält die exakten Werte, die die Umsetzung verbatim übernehmen soll:
    - T07 — §61: der DPT-Codec errät das Eingabeformat, mehrere Kodierungen ruhen auf Rulings statt auf der Spezifikation
    - T08 — D6: Bus- und Liniendiagnose als Oberfläche
    - T09 — E2: Scan-Ergebnis zurück ins Projekt abgleichen
    - T10 — D8: Settings-Oberfläche jenseits von Theme, Motion und Sprache
    - T11 — B10: nirgends Drag & Drop
    - T12, T13 — UI-Restposten, Batch A und Batch B
    - T14 — `knx-report`, §45 bis §50: Rest des Dokumentations-Exports
    - T15 — `knx-diff`, §52 bis §60: Rest des Projektvergleichs
    - T16 — Gruppenadress-CSV, §39 bis §41
    - T17 — §16 (Tauris archivierte GTK3-Bindings) und §79
    - T18 — die Alpha releasen, oder belegen, warum nicht
    - T19 — Forschungsartefakt: LLM-/Sprachinteraktion und MCP-Fähigkeit
    - T20 — Forschungsartefakt: Automatisierung wiederkehrender Arbeit, Makro-Layer
    - T21 — ADR: projektinterne Notizen und Dokumentation
    - T22 — F-T30-1: die Invariante von `Project` hängt am Review, nicht am Typsystem
    - T24 — Dokumentationsabgleich samt Neuzählung, die niemand von Hand tippt
  - **Kandidaten ohne Brief — drei.** T30: `DefaultLine=""` lässt `has_losses()` fälschlich wahr werden. T31: der Web-Pfad verwirft `imported.enrichment`. T32: Validierung und Mapping widersprechen sich bei kurzen Links.
  - **Abschluss des Laufs:** ein Review über das gesamte Goal auf `claude-fable-5-1`, noch nicht gelaufen.
  - **Geparkte Kleinigkeiten:** toter Code in `compare.rs`, ungenutzte `help.topic.importExport.*`-Schlüssel, der Präfixabgleich in `CatalogBrowser.tsx`; dazu die zwei kommentarblinden Testwächter aus den Notizen unten.
- **Notes for Codex:** Der Ledger des Laufs steht in `.superpowers/sdd/goal/progress.md` (git-ignoriert) und enthält Gate-Tabellen, Rulings und den Resume-Punkt; er ist die verlässlichere Quelle als dieser Eintrag. — **Eine Entscheidung gehört ausdrücklich dem Nutzer und wurde nicht getroffen:** die reale Gateway-Adresse steckt weiterhin in committeter git-History. Arbeitsbaum und sämtliche Commit-Messages dieses Laufs sind sauber, aber ein dritter Rewrite eines bereits gepushten Branches ist nicht umkehrbar. Nicht eigenmächtig rewriten. — Beim Suchen nach solchen Lecks nach Musterklasse greppen (RFC-1918-Bereiche), nicht nach einem erinnerten Literal; ein Sweep nach einem Literal ist kein Sweep. — Commissioning und jeder Schreibzugriff auf echte KNX-Hardware waren in diesem Lauf außerhalb des Scopes; `1.1.220` ist eine Alarmanlage und bleibt unberührt. — Zwei Testwächter sind blind und liegen geparkt: die `apiCallsIn`-Regex in `DiagnosticsCompanion.test.tsx` und `mutatingFetchesIn` matchen über ganze Dateien inklusive Kommentaren, weshalb ein Doc-Kommentar mit `api.foo()` darin einen Test kippen kann — beim Schreiben von Kommentaren in dieser Dateifamilie Prosa statt Code-Syntax verwenden. Details in `/tmp/claude-1000/handover/2026-09-21_goal-run_parked-findings.md`.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-20 16:27 CEST
- **Completed:** Read-only status audit for the user's question about remaining KNXBench tasks. Confirmed `main` and `origin/main` are synchronized at `df287d0`; there are no additional worktrees and the checkout was clean before this required handover update. Reconciled the current roadmap, completion goal, implementation status, gap analysis and known limitations. The principal open delivery items are T30 phase-3 real-hardware write verification (requires separate explicit operation-specific authorization and a safe target), T37 truthful project open/import progress, the remaining user-manual half of T28/D12, and explicitly scoped compatibility/capability residue such as the eighteen 200-series DPT main types and module-instance limitations. Deferred items such as KNX Secure, multi-user editing and post-v1 floor-plan work remain deliberately unscheduled; numbered known limitations are not automatically backlog commitments.
- **Pending/Next Steps:** Choose the next authorized slice. T37 is the strongest self-contained implementation candidate without hardware or external samples. Do not perform a real KNX write without explicit operation-specific user approval; never contact the excluded alarm-panel address recorded in the private operational context.
- **Notes for Claude:** No product code or documentation changed and no tests ran because this was a read-only status query. Only this mandatory handover entry modifies the checkout.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 13:03 CEST
- **Completed:** Aktuellen Versionsstand read-only geprüft. Produktversion ist konsistent `0.1.0-alpha.1` in allen 15 Rust-Paketen sowie Web-Manifest und Lockfile; Tauri übernimmt die Desktop-Version bewusst aus seinem Cargo-Manifest. `main` und `origin/main` stehen beide auf Merge-Commit `599593d5586af5ec26128e9027f473cc2fc56772` (`merge: fix T30 R20 connection confirmation`, 2026-09-18 10:47 CEST). Es existiert kein Git-Release-Tag. Der Hauptcheckout enthält außer dieser vorgeschriebenen Handover-Datei keine lokale Änderung. Keine Produktdatei geändert und keine Tests ausgeführt, da es sich ausschließlich um eine Versionsabfrage handelte.
- **Pending/Next Steps:** Für die reine Versionsabfrage bleibt nichts offen. Die laufenden separaten Worktrees `d10-language-data`, `d10-server-prose` und `t18-module-arguments` sind noch nicht Bestandteil von `main`.
- **Notes for Claude:** Aktueller veröffentlichungsnaher Paketstand ist Alpha 1; ein lokales AppImage `KNXBench_0.1.0-alpha.1_amd64.AppImage` wurde bereits gebaut und geprüft, aber kein Tag oder Release veröffentlicht. Die Binärprogramme ergänzen bei Builds aus Git den Kurz-SHA als Build-Metadatum (`0.1.0-alpha.1+g<short-sha>`).

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 10:55 CEST
- **Completed:** Committed the verified T30/R20 fix as feature commit `3966828` (`fix(knx-net): await device connection confirmation`) with author `KNXBench <github@knxbench.com>`, merged it into `main` with explicit no-fast-forward merge commit `599593d` (`merge: fix T30 R20 connection confirmation`), and pushed `main` to `origin`. Local and remote refs both resolve to `599593d5586af5ec26128e9027f473cc2fc56772`. Pre-merge dirty main documentation was safely stashed and compared; its documentation was subsumed by the merged result, while three unique handover entries were restored exactly once. Post-merge `cargo test --workspace -q` passed. Removed the owned `t30-disconnect-response` worktree and deleted its local branch after the successful push.
- **Pending/Next Steps:** R20's false-connected mechanism is fixed, merged and pushed. A later management read timeout remains inherently ambiguous and must not alone prove absence. Existing unrelated Claude worktrees remain untouched.
- **Notes for Claude:** `main` is synchronized with `origin/main` at `599593d`. The only local tracked modification is this handover file containing restored pre-merge historical entries plus this final update. Safety backup of the former main dirty patch remains at `/tmp/knxbench-main-pre-r20-merge.patch`.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 10:36 CEST
- **Completed:** Fixed T30/R20's false-connected mechanism by TDD in isolated worktree `t30-disconnect-response`. `SessionTiming` now exposes `connection_timeout`, defaulting to the KNX Transport Layer clause 4 value of six seconds. `ManagementSession::connect()` subscribes before `T_Connect`, ignores unrelated/stale frames, creates connection state only after a matching positive cEMI `L_Data.con`, reports matching negative confirmation immediately as `ConnectRejected`, and reports silence after the six-second bound. Simulator emits the real confirmation shape. RED observed old immediate gateway-ACK-only return and old wait-through-negative behavior. Final read-only hardware run over all 34 `devices.md` targets: 33 positive confirmations in 2.851–144.245 ms (median 134.530 ms), all 33 descriptor reads succeeded; one of the two IP interfaces explicitly rejected connect after 162.709 ms and received no descriptor read. Alternating timeouts disappeared. All tunnel disconnects succeeded. Temporary probe deleted. Updated research, known limitation, R20 design risk, implementation status, daily memory and `.ai/logs/2026-09-18_codex_r20_connection_timeout_fix.md`.
- **Pending/Next Steps:** R20's false-connected timeout mechanism is fixed and hardware verified. A later management read timeout remains inherently ambiguous and must not alone prove absence. The isolated worktree changes remain uncommitted and unmerged.
- **Notes for Claude:** Verification passed: 178 `knx-net` unit tests plus integration/doc harnesses, package Clippy all targets with warnings denied, workspace all-target check, formatting, diff check, and final hardware probe. No property read, authorisation request, write service or scan was sent; The address absent from `devices.md` and the excluded alarm panel `1.1.220` both remained excluded.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 09:59 CEST
- **Completed:** Isolated R20 at frame level using two already approved read-only targets. Both tunnels were assigned the same gateway-issued tunnel address, and all incoming KNXnet/IP sequence counters began at zero and matched, ruling out alternating tunnel addresses and client receive-sequence rejection. The successful first target received current-target `T_Connect` `L_Data.con`, descriptor-read confirmation, device `T_ACK`, and descriptor response. Failed the second target first received a late `T_Connect` confirmation for the previous target; its three descriptor reads received `L_Data.con`, but no current-target connect confirmation, device ACK or response arrived. `ManagementSession::connect()` completes on gateway `TUNNELLING_ACK` and does not await matching bus-level `L_Data.con`, so it proceeds without evidence the transport connection was established. TPCI Connect/Disconnect encoding is correct and round-trip tested. Temporary instrumentation was restored byte-for-byte and temporary test deleted. Updated R20 research, limitation, risk, status, daily memory and frame diagnostic log.
- **Pending/Next Steps:** R20's direct timeout mechanism is identified; why every alternate bus-level `T_Connect` confirmation is absent remains open. A causal fix must synchronize connection progress with the matching successful cEMI `L_Data.con` or another specification-grounded readiness event, with a failing regression before product changes. Do not add an unexplained delay.
- **Notes for Claude:** Diagnostics remained read-only: `A_DeviceDescriptor_Read(0)` only, no property read, authorisation request, write service or scan. Product implementation remains exactly the pre-diagnostic response-aware disconnect diff in `t30-disconnect-response`; SHA-256 restoration check passed.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 09:43 CEST
- **Completed:** Ran the user-approved response-aware T30/R20 read-only probe over all 34 literal `devices.md` targets in ascending order (the line's devices, then the two IP interfaces) through gateway `KNX_GATEWAY:3671`. One address the earlier notes had assumed present, and the project-excluded alarm panel `1.1.220`, were absent and asserted excluded before socket creation. One fresh tunnel per target, `ManagementSession::read_only`, `AuthorisationPlan::Skip`, only `A_DeviceDescriptor_Read(0)`. Exactly 17 odd-position attempts answered and 17 even-position attempts timed out; ascending order reversed the earlier result for those nine, proving attempt order rather than address, manufacturer or mask version selects the failure. one target returned `0012h`; all other answers returned `0701h`. All 34 disconnects received successful final responses. Test passed in 156.63 seconds; temporary source deleted. Updated R20 research, known limitation, design risk, implementation status, daily memory and `.ai/logs/2026-09-18_codex_r20_all_devices.md`.
- **Pending/Next Steps:** R20 remains open. Investigate which transport/application-session state alternates across independently and correctly terminated KNXnet/IP channels. Do not infer a delay or retry constant and do not use a management timeout as proof of absence.
- **Notes for Claude:** No property read, authorisation request, write service or scan was sent. Documentation updates and existing response-aware disconnect implementation remain isolated in worktree `t30-disconnect-response`; no product code was changed by this all-device rerun.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 09:08 CEST
- **Completed:** Repeated the bounded T30/R20 real-gateway test on the response-aware disconnect implementation. The test used gateway `KNX_GATEWAY:3671`, nine literal `devices.md` targets in reverse order (the line's highest nine), asserted excluded `1.1.220` before socket creation, one fresh tunnel per target, `ManagementSession::read_only`, `AuthorisationPlan::Skip`, and only `A_DeviceDescriptor_Read(0)`. Every KNXnet/IP disconnect received a matching successful `DISCONNECT_RESPONSE`. The exact earlier result reproduced: the five at even positions returned mask `0701h`; the four at odd positions timed out after three three-second attempts. Test passed in 37.07 seconds. Temporary test source deleted; raw output retained under `/tmp`; added `.ai/logs/2026-09-18_codex_r20_response_rerun.md` in main.
- **Pending/Next Steps:** R20 remains open. Response-aware IP channel teardown is independently ruled out as the cause of the alternating management response pattern. Continue with specification-grounded transport/application-session investigation; do not infer a retry delay and do not treat timeout as proof of absence.
- **Notes for Claude:** No write, property read, authorisation request, or scan was sent. No product code was changed by this rerun. The existing uncommitted response-aware disconnect implementation and its documentation in worktree `t30-disconnect-response` remain otherwise untouched.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 08:59 CEST
- **Completed:** Recorded the user-provided T30/R20 test context in project memory: `devices.md` is the authoritative target list for read-only communication tests, and the KNXnet/IP gateway address is `KNX_GATEWAY`. Added the same raw context to `memory/2026-09-18.md`. No network or KNX bus access occurred.
- **Pending/Next Steps:** Use the listed devices and gateway for future explicitly requested read-only T30/R20 communication tests. R20 implementation and verification remain pending as described below.
- **Notes for Claude:** The user explicitly requested persistence of the gateway IP, superseding earlier handover notes that intentionally omitted it. This update only changes memory and handover files.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 08:31 CEST
- **Completed:** Audited all 179 PDFs in `The KNX Standard v3.0.0` against T30 R20. Found concrete KNXnet/IP Core §5.5 mismatch: `TunnelClient::disconnect` sends `DISCONNECT_REQUEST` then immediately stops receiver, never observes mandatory `DISCONNECT_RESPONSE` final channel termination. Core sequence reset and Tunnelling one-retry rules match current code. Management Procedures §2.19 confirms current connection-oriented scan; §2.17 connectionless scan is RF-only. Updated RESEARCH §8.8.3b, KNOWN_LIMITATIONS §7, design risk R20, IMPLEMENTATION_STATUS, and audit log. No product code, socket, gateway, or hardware changed.
- **Pending/Next Steps:** R20 remains open because disconnect mismatch is credible mechanism for fresh-tunnel alternation but not yet causal proof and cannot explain shared-tunnel first-session failure. Next implementation task: loopback peer test proving graceful disconnect waits matching response, response-aware disconnect fix, then bounded read-only fresh-tunnel comparison. Do not guess delay/retry constant. Real writes still need explicit operation-specific authorization; never contact `1.1.220`.
- **Notes for Claude:** Main product source untouched. Documentation-only changes in main overlap no reserved worktree implementation. Existing `.ai/CURRENT_STATE.md` audit entry remains below this one.

---

- **Last Agent:** Codex
- **Timestamp:** 2026-09-18 07:43 CEST
- **Completed:** Read-only audit answering what remains outside Claude's three reserved worktrees (`d10-language-data`, `d10-server-prose`, `t18-module-arguments`). Reconciled current `main` (`6605273`) with `goal.md`, roadmap, implementation status, known limitations, project analysis, source and branch/worktree state. Confirmed old goal entries are stale: DPT codec main types 1-30 are implemented; T38, passive monitor evidence, load optimization, Docker/AppImage, range-less export guard, and several parked findings are already complete. No product code changed.
- **Pending/Next Steps:** Outside Claude's work, primary open delivery is T30 commissioning with R20 tunnel lifecycle evidence unresolved; next independent v1 work is T37 truthful project-load/import progress. Further concrete hardening: atomic unsaved-project replacement/session snapshot, deployment-mode path confinement/product-database startup failures, five lossy `Space/@Type` variants, DPT collision provenance after D10 integrates, malformed non-override preservation where evidenced, ZipCrypto real-project validation, help T28 after UI stabilizes, and documentation consolidation. Deferred items remain deferred by explicit rulings.
- **Notes for Claude:** Three reserved worktrees were inspected read-only and left untouched. `d10-language-data` has its existing four modified files; no Codex overlap introduced. Main working tree was clean before this required handover-only update.

---

---

Ältere Handover-Einträge (2026-09-08 bis 2026-09-18 04:57, 89 Stück) wurden am
2026-09-20 entfernt, weil die zugehörige Arbeit abgeschlossen und in `docs/`
dokumentiert ist. Vollständiger Stand liegt im Statusarchiv unter
`Backup/status_2026-09-20_16-37-52.zip`.
- **Last Agent:** Codex
- **Timestamp:** 2026-09-21 21:23 CEST
- **Completed:** T08/D6 implemented in branch `t08-line-diagnostics`: incremental read-only scan API (estimate/start/results/cancel), real cancellation and tunnel disconnect, six-state diagnostics UI, inspectable cost/progress, settings-backed protected exclusions, localized outcomes, and single-flight generation-safe polling. Simulated transport proves completion, cancellation, and that excluded synthetic addresses never reach transport. The user's fixed group-address ruling is applied at the same time: slash rendering only, dotted input/search compatibility only, no selector or preference. Fresh whole-branch review found 0 Critical and 5 Important issues; one RED→GREEN fix pass made terminal snapshots coherent, bound poll/cancel to session ids, suspended polling during cancel, froze active exclusions, and bound estimates to exact request fingerprints. The Minor cost-label finding was also fixed by disclosing excluded transport overhead. Full post-fix gates green: Rust 1,939/92 with 0 failures, Web 819/59, TypeScript, fmt, workspace clippy, layering, headers 186/162 (ceiling 162), anchors 376/175, deny. No real KNX traffic or hardware access. Detail: `.ai/logs/2026-09-21_codex__t08_line_diagnostics.md`.
- **Pending/Next Steps:** Commit the T08 review fixes as KNXBench, obtain focused reviewer confirmation, rebase/merge onto current `main` (which additionally contains the T20 Functions PDF ruling), verify the integrated commit, push, then continue T09. T09 must consume scan records as a previewable, undoable reconciliation diff and preserve all six evidence states; it must not mutate a project during scanning.
- **Notes Claude:** `.superpowers/sdd/goal/task-08-brief.md` lives only in the main checkout's ignored SDD workspace. The brief's claimed header ceiling 167 was stale; authoritative gate ceiling is 162 and remains unchanged. The first full Rust run failed at the linker because the dedicated `/tmp` target had grown to 22 GB; no source/test failure occurred. A fresh lean rebuild ran the complete sequence. No real installation address or private LAN address was added.

---
