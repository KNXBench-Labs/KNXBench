# Prepared Goal B — workbench, UX and platform

Planning only; no session owns these packages until the user invokes this prompt.
Start after Goal A ends. Read the root-local audit and A's published outcomes
before creating a fresh worktree; prepared files may not yet exist on origin/main.

## Session status

**Owner:** Claude session `ba673d45ff9d`, worktree `KNXBench.worktrees/workbench-b`,
branch `feature/workbench-b-20261013`, invoked 2026-10-13 ~21:25 CEST on fresh
`origin/main` `40aba5dc` after Goal A ended. Web lock taken per package (see
`.ai/CURRENT_STATE.md` *Coordination*). Concurrent owner seen at start: Claude
`bcee21b10872` in worktree `log-16k` (KL-036 session log: `LogPanel`,
`sessionLogExport`, `session_log.rs`, desktop log export and their catalogue
keys). B does not edit those files and rebases over them when they land.

Finite plan, in order. Each package is delivered (tests + gate + push),
blocked (named missing evidence) or excluded; nothing else is added.

| Package | Scope of record | State |
| --- | --- | --- |
| B1a | Revision-bound parameter writes + installation renames in the web client (A2); `serverStopping` worded (A3) | delivered (gate in the W42 entry) |
| B1b | Functions in the workbench over A4's API (read, create/update/move/delete, members, undo/redo/save/reopen); no Site UI (ADR-0125 blocked) | delivered (W42 entry) |
| B2 | Desktop webview reload/navigation guard for notes/instruction drafts; native/Orca/Firefox only as actually run | planned |
| B3 | Read-only stored BusAccess/IP view, redacted endpoints, no connect | planned |
| B4 | Instructions for natively created devices (no invented original) | planned |
| B5 | Catalogue backup deletion; web direct `.knxprod` install routed to preview/apply | planned |
| B6 | KL-164 idempotent achievement increments | planned |
| B7 | Naming recipe UI over the existing `knx-edit` service | planned |
| B8 | EN/DE report texts, KL-13/KL-29/KL-43 summaries, TOOLS-01–05 provenance, optional FUTURE-08 | planned |
| B9 | Issue #14: hover `transform` reset app-wide (`button:hover:not(:disabled)`), FsPicker geometry regression | delivered (W42 entry) |
| B10 | Issue #11: group-address tree starts collapsed (`expanded` set), Expand/Collapse all, selection ancestors and filter matches open, per-project view state in localStorage | delivered (W42 entry) |
| B11 | Issue #13: Log time column, even rows, one `Export…` button with scope in its dialog, plain link/unlink messages | blocked until `log-16k` (bcee21b10872) lands on main; touches its files |
| B12 | Issues #12 + #15: one `+ New` menu per header reusing existing forms (structure, topology, group addresses incl. a **Group address** entry with range preselection, grouped per installation); fix the three stale "first installation only" manual sentences | delivered (W42 entry) |
| B13 | Issue #16: collapsing a communication-object channel hides its child rows/editors while drafts, pending requests and results survive reopening ([triage](../status/2026-10-13-issue-16-triage.md)); contract + ADR-0102 update | delivered (W42 entry) |

B9–B12 were added on 13 October at the owner's request (triage `59b1ef8d0f72`, handover 22:10);
#15 joined B12 (triage `922727d3ed74`, 22:45) and #16 became B13 (same session, 22:16).
Order after B1a: B9, B13, B1b, B10, B12, then B2…B8; B11 once `log-16k` lands.
Each issue is updated or closed only after its fix is on `main`.

## Goal A outcome (handover, 2026-10-13)

Goal A ended on `e5eb1e8a` + docs `5ca02b2b` (session `62a4cca1c10f`); it did not start Goal B.
What B can build on, and what it must not:

| From A | State | What B uses | Must not |
| --- | --- | --- | --- |
| A1 KL-157 | delivered | Readers open `.knxdb` read-only; edits are writes | Upgrade a file outside a confirmed write |
| A2 | delivered | [Revision-bound writes](../contracts/revision-bound-writes.md): bind parameter writes and installation renames to `serverIncarnation`/`projectIncarnation`/`snapshotRevision` + expected value; on `409` keep the draft and offer reload/compare | Keep a second revision counter; send unbound writes from new UI |
| A3 KL-163 | delivered | Tunnel routes answer `503 serverStopping` while the server stops; show it as "server is stopping", not as a bus error | Retry automatically |
| A4 MODEL-07 | delivered (backend, projection, API) | [Functions contract](../contracts/project-functions.md): `GET /api/project/functions`, `POST /api/project/functions/edit` (bound; `409 functionStale`, `422 functionRefused`), TS `FunctionsView`; deletes of a member GA or a Function's room are refused | Cascade deletes; edit `Project.functions` outside commands; show schema-20/21 Functions (not modeled) |
| A4 MODEL-05 | blocked design ([ADR-0125](../adr/0125-multi-installation-site-is-blocked.md)) | Nothing | Build site UI or cross-installation moves |
| A5 KL-128 | delivered | Legacy string/long-enum parameters appear in the existing parameter panel like ETS4's conversion | Hide `binary-values-not-modeled` notes, or show them as import errors |
| A6 PDB-04, KL-68, KL-171, KL-172 | blocked (no new evidence) | [Review](../status/2026-10-13-goal-a6-a7-evidence-review.md) with gaps and fallbacks | Add Color/Picture/Repeat/UTF-8/LoadedImage semantics |
| A7 KL-110, GAP-T30-04 | reconciled, no implementation | Readiness contracts only | Any new medium or differential download |

Native schema is 15 after A4: files a current build upgrades are refused by
Alpha 11 and earlier (manual notes it). Delivery details: W42 history,
entries of 13 October.

```text
/goal KNXBench: Workbench-Restarbeiten, UX und Plattformhärtung

Schließe die folgenden klar begrenzten Workbench-Reste autonom in kleinen Paketen.
Kein Neubau bereits gelieferter Notes-/Instructions-/Parameter-/Katalog-Funktionen.
Dies ist keine Freigabe für Hardware, ETS-/Herstellerausführung, Releases oder
Deployments. Datenintegrität schlägt Komfort, auch wenn Komfort hübscher aussieht.

START UND VORAUSSETZUNG
- Workspace: <old-sources>/KNXBench.
- Lies zuerst .ai/CURRENT_STATE.md, AGENTS.md und den lokal vorbereiteten Audit
  docs/status/2026-10-13-unassigned-work-goal-audit.md.
- Goal A muss beendet sein. Prüfe seine veröffentlichten Verträge/Commits und
  blockierten Forschungsergebnisse; implementiere keine UI für nicht admittierte
  Backend-/Modellfunktionen. Ein blockiertes A-Paket blockiert nicht den Rest von B.
- git fetch origin; prüfe frisches origin/main, aktuelle Session-/Worktree-
  Eigentümerschaft und vollständige Web-Lock-/Release-Queue. Erstelle einen
  eigenen Worktree. Den alten fremd veränderten Root niemals synchronisieren,
  resetten, stagen oder bereinigen. Fremde Branches/Stash/Journale nicht anfassen.
- Lies OPEN_WORK/LEDGER, Host-Verträge für Notes/Instructions/Parameter,
  catalogue-update, offline-name-recipe sowie relevante Detaildossiers.
- Dokumentiere einen endlichen Plan. Nutze den vorhandenen Web-Lock pro Paket;
  bei neuer fremder Reservierung koordiniert warten, nicht deren Slot übernehmen.

PAKETE, IN DIESER REIHENFOLGE
B1 — Goal-A-Verträge in den Workbench bringen:
  Integriere dessen revisiongebundene Parameter-/Installation-Requests und,
  sofern tatsächlich geliefert, Functions und belegte Site-Operationen.
  Verwende reale Identitäten/Revisionen und bestehende Auswahl/Explorer/Inspector.
  Stale-Konflikt erhält den Entwurf und bietet bewusstes Reload/Vergleich; ein
  verspäteter Request darf kein anderes Projekt/Target treffen. Functions-Ansicht
  mit eindeutigen Mitgliedschaften, Roh-Provenienz und Undo/Redo/Save/Reopen.
  Keine zweite Domänenlogik, kein eigener Revisionszähler, keine WIP-Komplettmerge.

B2 — native Entwürfe und Plattformnachweis:
  Browser-beforeunload/Tab-Close ist bereits geliefert. Prüfe Desktop-Webview-
  Reload/Navigation und ergänze den fehlenden Schutz für Notes/Instructions-
  Entwürfe, ohne Save/Apply vorzutäuschen. Echte WebKitGTK-/Tauri-Tests und
  gezielte Orca-/Firefox-Prüfung auf isolierten fiktionalen Projekten, soweit die
  tatsächliche Umgebung das ermöglicht. Fehlende Interaktion/Umgebung konkret
  melden. Chromium ist kein nativer oder kompletter Accessibility-Nachweis.
  Keine Nutzerdateien, laufenden Nutzer-Server oder Container verändern.

B3 — gespeicherte BusAccess-/IP-Daten lesbar machen:
  Nutze vorhandene Retention-/Payload-/Application-Projections. Nur lesende,
  begrenzte Ansicht mit genauem Device-/Line-/Source-Bezug, unbekannten Feldern
  und sichtbarer Provenienz. Sensible Endpoints standardmäßig redigieren bzw.
  nur bewusst offenlegen. Keine automatische Gatewaywahl, Verbindung oder
  Busabfrage. Ambiguität verweigern, Originalbytes erhalten, Outgoing-Privacy testen.

B4 — Anleitungen für nativ angelegte Geräte:
  Erweitere den vorhandenen normalisierten Instruction-Overlay-Workflow um
  ausdrücklich eigene Geräte ohne importiertes Original. Kein ETS-Original oder
  Source-Digest erfinden; fehlend, leer und vorhanden unterscheiden. Vorhandene
  Preview/Bestätigung/Undo/History nutzen; Reset kehrt zum tatsächlich vorhandenen
  Zustand zurück. Persistenzfehler bleibt ohne Modell-/Datei-/History-Effekt.
  Kein allgemeiner RTF-Decoder, externe Ressource, ausführbares HTML oder ETS-Export.

B5 — bestehende Katalog-Updates fertig abrunden:
  Ergänze ausdrücklich bestätigte Backup-Löschung mit frischer allowlist-gebundener
  Dateidentität, sicherer Pfad-/Symlink-Behandlung und Fehlerfällen. Route den
  Web-Direktinstall für .knxprod zum vorhandenen Preview/Apply-Workflow bzw.
  deprekiere ihn kompatibel; Restore und Legacy-Import müssen weiter funktionieren.
  Keine Online-Catalog-Quelle, automatische Updates, Gerätereferenz-Umbindung,
  Version-Pinning oder eigenmächtige Löschung anderer/backups aktueller Nutzer.

B6 — KL-164 Achievement-Zähler:
  Behebe verlorene konkurrierende Inkremente mit begrenzten, idempotenten
  Ereignissen/Deltas und Request-IDs. Zwei Fenster müssen zählen; doppelte,
  wiederholte und umgeordnete Requests dürfen nicht doppelt zählen. Prüfe
  Persistenzfehler sowie Reset/Restart und die Begrenzung der Dedup-Evidenz.
  Gemeinsamer Installationsdatensatz bleibt; keine Benutzerverwaltung und keine
  erfundenen historischen Inkremente. Erfolgs-/Hardware-Achievements nicht aufweichen.

B7 — bounded naming recipe als UI:
  Nutze ausschließlich den vorhandenen knx-edit-Literal-Prefix/Suffix-Service.
  Explizite Stable-ID-Auswahl, bindende Vorschau/Revision, Bestätigung, ein Undo,
  Save/Reopen/Redo. Keine neue Macro-VM, Recorder, Scheduler, erzeugte Auswahl,
  MCP/LLM-Schreibrechte oder Buswirkung. Konflikte erfordern eine neue Vorschau.

B8 — aktuelle Texte, Reports und Statusaussagen:
  Schließe konkrete EN/DE-Lücken in Report-eigenen und neu betroffenen UI-Texten;
  technische Rohdetails und unbekannte Identifikatoren exakt erhalten.
  Prüfe KL-13/KL-29-Kurzfassungen gegen ihre letzten Updates und den Code:
  reale ZipCrypto-Samples und CLI-Adressstil/Namensauflösung sind bereits geliefert.
  Ebenso per-category Motion, Notes-/Catalogue-Hosts: alte Absenzbehauptungen in
  OPEN_WORK/ROADMAP/Limitations nicht als neue Features bauen. Ledger-Gesamtzeilen
  nur schließen, wenn ihr eigener Gesamtvertrag wirklich erfüllt ist.
  TOOLS-01–05: vorhandenen separaten ProjectStats-Collector und echte Messlücken
  dokumentieren; keine zweite Implementierung, keine Änderungen in fremdem
  Workspace, kein N/A als Null oder erfundene Messung. FUTURE-08: zusätzliche
  humorvolle Texte nur optional; die existierende Mechanik nicht neu erfinden.
  Die separat geparkte große Dokumentrestrukturierung bleibt ausgeschlossen.

AUSGESCHLOSSEN
- Bereits gelieferte #8/#9/#10, Notes-Targets/Browserguard/Reviewtext, vier GA-Views
  und ihre vorhandenen Hosts; Issue #4/GA7/GA8 bleibt beim Reporter-Owner.
- KL-170 Encoder/Tester-/Hardware-Follow-ups, #6/#7 Reporterbestätigungen,
  IMPORT-02 ohne autorisierte Inputs, Color/Picture-/Repeat-/UTF8-/LoadedImage-
  Semantik ohne vollständige neue Evidenz.
- SmartSensor-/ETS6-/FastDownload-Hosts oder private Herstellerforschung,
  KNX Secure und jeder echte Bus-/Gerätezugriff.
- Optionale MCP-Client-Abnahme/Clientwahl und jede neue Agent-Mutation.
- Neue Version-Selector-/Allocator-/Merge-/PDF-/Mobile-/OS-/Plugin-Projekte;
  CI-Aktivierung, Release/Tag/Assets/Image-Push, Deployment oder Containerwechsel.

QUALITÄT UND LIEFERUNG
- RED/GREEN für echte Verhaltensänderungen. Unit/HTTP/Native-/Migration-/Roundtrip-
  und Concurrent-Client-Negative-Tests. Bei UI reale Produktionsbuild-Browser-
  Abnahme EN/DE, hell/dunkel, 400/1440 und Keyboard/Stale/Refusal/Persistence.
- Vollständige erforderliche Rust/Web/Build/Clippy/Format/Bindings/Docs/xtask-
  Prüfkette auf eingefrorenen Quellen, xtask an den tatsächlichen Worktree binden;
  bestehende gemeinsame Gate-Leases nutzen. Root-only Corpus wirklich einbinden,
  nicht Worktree-Skips zählen. Native/Orca nur nach tatsächlicher Ausführung behaupten.
- Self-Review als solche benennen. Docs/Contracts/ADRs/Handbuch, Ledger nur
  evidenzgerecht, Limitations-Dossier + catalogue.json + regenerierten Index und
  .ai/CURRENT_STATE.md pflegen. Keine erhaltene Historie/Anchors löschen.
- Mit Aufruf dieses Goals sind fokussierte Commits, normale Main-Integration und
  Push nach jedem vollständig grünen Paket erlaubt: KNXBench
  <github@knxbench.com>, kein Co-Authored-By. Remote/Quellbytes verifizieren,
  bei runtime-relevanter Integration erneut prüfen; eigene Scratch-Artefakte
  aufräumen, Evidenz erhalten und Web-Lock wieder freigeben.
- Keine Subagents, keine Quota-Abfragen. Explizite Pausen sofort beachten;
  sonst direkt ins nächste unabhängige Paket weiterarbeiten. Einzelne echte
  Blocker benennen und isolieren, nicht den gesamten Goal künstlich blockieren.

ENDE
Abschluss nach Lieferung oder konkret belegter Blockierung aller B-Pakete.
Berichte veröffentlichte Commits, echte Tests und weiterhin geltende Grenzen;
kein neues Release, kein Deployment, keine vollständige ETS-/Native-/WCAG-Behauptung.
```
