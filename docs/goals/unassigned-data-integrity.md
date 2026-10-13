# Prepared Goal A — data integrity, model and import

Prepared as planning only; the owner invoked it on 2026-10-13 (see *Session
status* below). Goal B follows this goal, not concurrently.

## Session status

**Owner:** Claude session `62a4cca1c10f`, worktree
`KNXBench.worktrees/data-integrity-20261013`, branch
`feature/data-integrity-20261013`, invoked 2026-10-13 ~17:15 CEST on fresh
`origin/main` `4c0bea47`. No web lock is taken: A contains no web UI.
Concurrent owner seen at start: codex `468712901140` restructures
`.ai/CURRENT_STATE.md`, `IMPLEMENTATION_STATUS.md` and two research topics
(docs only); A keeps its own entries in those files small and re-fetches
before every integration.

Finite plan, in order. Each package is delivered (tests + gate + push),
blocked (named missing evidence) or excluded; nothing else is added.

| Package | Scope of record | State |
| --- | --- | --- |
| A1 | Read-only CLI/comparison readers use `open_existing_read_only` (KL-157) | delivered (merge `a78c1010`) |
| A2 | Revision-bound parameter writes and installation rename; host contract for B | in progress (candidate `aabf4d2c`) |
| A3 | Shutdown admission + bounded drain of download/address workers (KL-163) | planned |
| A4 | Functions backend fresh on current schema (MODEL-07); Site ADR or blocked design (MODEL-05/KL-127) | planned |
| A5 | Legacy atomic types 3/5 against the offline oracle (KL-128) | planned |
| A6 | Evidence review PDB-04/KL-68, KL-171/172: new material only | planned |
| A7 | KL-110/GAP-T30-04 prerequisite reconciliation (docs) | planned |

```text
/goal KNXBench: Datenintegrität, Modell und evidenzgebundener Import

Arbeite die folgenden Pakete autonom und inkrementell ab. Dies ist keine
Freigabe für Buszugriffe, Hersteller-/ETS-Ausführung, Releases oder Deployments.
Priorität: Korrektheit und Datenerhalt vor Funktionsumfang.

START
- Workspace: <old-sources>/KNXBench.
- Lies zuerst .ai/CURRENT_STATE.md, AGENTS.md und den lokal vorbereiteten Audit
  docs/status/2026-10-13-unassigned-work-goal-audit.md.
- git fetch origin; prüfe frisches origin/main, neueste Locks/Reservierungen und
  reale Session-/Worktree-Aktivität. Der gemeinsame Root ist absichtlich alt und
  fremd verändert: nicht synchronisieren, resetten, stagen oder bereinigen.
- Erstelle einen eigenen Worktree von origin/main. Lies OPEN_WORK, LEDGER,
  ARCHITECTURE, DATA_MODEL, IMPORT_EXPORT und die betroffenen Detaildossiers und
  Verträge. Die Audit-Zahlen sind ein Snapshot, kein heutiger Statusersatz.
- Dokumentiere einen endlichen Paketplan und deine Eigentümerschaft. Prüfe jede
  Aufgabe erneut gegen aktuellen Code; bereits gelieferte Funktionen nicht bauen.
  Fremde Branches, Stash, Journale und vertrauliche Forschung bleiben unangetastet.

PAKETE, IN DIESER REIHENFOLGE
A1 — KL-157, wirklich lesende Native-Pfade:
  Stelle die lesenden CLI-Export/Diff-/Vergleichspfade auf den bereits vorhandenen
  open_existing_read_only-Pfad um. Keine eigene Copy-/Migration-Implementierung.
  Prüfe alte und aktuelle Schemas, Foreign/Missing/Future-Dateien, Opaque-Daten
  und Working History. Lesende Aufrufe dürfen die Originaldatei nicht migrieren.
  Writable Open und absichtliche Schreibbefehle nicht beiläufig umdefinieren.

A2 — konkurrierende Parameteränderungen und Installation-Rename:
  Verwende echte Server-/Projekt-Inkarnation und Snapshot-Revision als verbindliche
  Schreibbedingung. Binde die tatsächliche Installation und den exakten Zielwert;
  weder first-installation-Annahme noch UI-Zähler sind Autorität. Prüfung,
  Persistenz und Veröffentlichung müssen atomar sein. Stale Requests: typisierter
  Konflikt, kein Modell-/Datei-/Undo-Effekt. Bestehende Aufrufer ausdrücklich
  migrieren oder ihre Legacy-Grenze dokumentieren; kein stiller Bypass.
  Zwei-Client-Tests einschließlich Projektwechsel und Persistenzfehler. Erstelle
  einen konkreten Host-Vertrag für Goal B. Keine allgemeine Benutzerverwaltung.

A3 — KL-163, Server-Shutdown und Schreibworker:
  Verhindere neue Aufträge nach Shutdown-Beginn; ergänze ein konfigurierbares,
  begrenztes Warten auf vorhandene Download-/Address-Worker. Nur offline mit
  kontrollierten Worker-/Simulator-Fixtures testen. Keine erfundene sichere
  Geräte-Abbruchprozedur. Timeout, zweites Signal und Kill bleiben ausdrückliche
  Grenzen; Server- und Docker-Stop-Zeit müssen dokumentiert zusammenpassen.
  Weder laufenden Nutzer-Server noch Container/Gateway stoppen oder ersetzen.

A4 — MODEL-07 Functions und MODEL-05/Sites:
  Prüfe den geparkten WIP 1154b72f nur als Referenz: schema 12, ungeprüft,
  letzte Abnahme fehlgeschlagen. Nicht mergen, rebasen oder komplett übernehmen.
  Baue Functions frisch auf dem aktuellen Modell/Native-Schema: normalisierte
  Identitäten und Befehle, erhaltene unbekannte Daten, belegtes Importmapping,
  Root-/History-Migration, Diff/Report/Projection/API und Roundtrip-Tests.
  Prüfe unabhängige Fixtures, Versions-/History-Migration und Selective Import.
  Kein UI in diesem Paket: gib Goal B einen getesteten Vertrag; Backend allein
  ist nicht die abgeschlossene Gesamtfunktion.
  Für mehrere Installationen unter einem Site zuerst ADR, Samples und eindeutige
  Ownership/Referenzinvarianten. Nur belegte Offline-Operationen implementieren.
  Bei fehlender Grundlage liefere einen präzisen blockierten Entwurf statt
  erfundener Cross-Installation-Moves oder ETS-Kompatibilität.

A5 — KL-128, Legacy atomic types 3 und 5:
  Nutze vorhandene Originaldaten und unabhängige ETS-Konvertierungsartefakte
  ausschließlich lesend als Oracle. Belege string/long-enum Mapping, Bounds,
  Defaults und Enum-Identität vor Implementierung. Unbekanntes erhalten und
  melden. Keine ETS-/Hersteller-Ausführung, Live-Downloads oder geratenen
  Legacy-Prozeduren. Fehlt die entscheidende Oracle-Semantik, blockiere nur A5.

A6 — PDB-04/KL-68 und KL-171/172, Restverträge:
  Prüfe nur tatsächlich neue, autorisierte Primärquellen/Artefakte auf fehlende
  Modulzuordnung, Color/Picture-Werte, UTF-8-Payloadregeln und LoadedImage-Struktur.
  Keine Wiederholung abgeschlossener Recherche ohne neues Material; keine
  Login-/SDK-Umgehung, geratenen Codecs, Repeat-Aktivierungen oder Downloadbilder.
  Ohne vollständigen Vertrag: genaue Evidenzlücke, Testplan und bestehender
  Fallback. Kein künstliches DONE; Forschung ist keine Runtime-Unterstützung.

A7 — KL-110 und GAP-T30-04:
  Gleiche den bereits gelieferten Prerequisite-/Testplan-Stand ab. Dokumentiere
  verbleibende Medien-/Differential-/Recovery-Gates; implementiere weder neue
  Medien noch Differential-Downloads. Alte Research-Pakete nicht erneut liefern.

AUSGESCHLOSSEN
- KL-170 Encoder/Byte-Order-Probe und Testerberichte: bestehender Owner/Reporter.
- IMPORT-02 ohne autorisierte Revisionsexporte und geprüfte Baselines.
- Issue #4/GA7/GA8 und #6/#7 Reporterbestätigung; bereits gelieferte #8/#9/#10.
- Geparkter ETS6-/SmartSensor-Host, Datenübernahme dieses Owner-Tracks und private
  Herstellerforschung; KNX Secure und jeglicher echter Bus-/Gerätezugriff.
- Aufgeschobenes Version-Pinning und Allocator-Phasen; generisches Diff-Merge,
  native PDF, Plugin-API oder neue große Produktideen.
- Web-Oberfläche, Releases, CI-Aktivierung, Deployment, fremde Repositories.

QUALITÄT UND LIEFERUNG
- Bestehende Services/Adapters nutzen, Core ohne UI-/Formatabhängigkeit halten.
- Verhalten zuerst RED, dann GREEN; echte relevante Unit/Integration/HTTP/
  Migration/History/Native-/Roundtrip-/Concurrency-Tests. Opaque-Daten bytegenau.
- Vollständige erforderliche Rust/Web/Build/Clippy/Format/Bindings/Docs/xtask-
  Prüfkette auf eingefrorenen Quellen; xtask an den tatsächlichen Worktree binden.
  Gemeinsame Gate-Leases in etablierter Reihenfolge. Corpus-Tests müssen echte
  Inputs sehen; Worktree-Skips zählen nicht. Keine Subagents oder Quota-Abfragen.
- Dokumentiere Änderungen, Grenzen, ADRs, Ledger nur evidenzgerecht und Handover.
  Limitations-Dossiers/ catalogue.json pflegen und den Index regenerieren;
  erhaltene Historie und alte Anchors nicht löschen. Keine große Doku-Umsortierung.
- Mit Aufruf dieses Goals sind fokussierte KNXBench-Commits, normale Main-
  Integration und Push nach jedem vollständig grünen Paket freigegeben:
  KNXBench <github@knxbench.com>, kein Co-Authored-By. Quelle und Remote danach
  exakt verifizieren; bei laufzeitrelevanter Integration neu prüfen.
- Nach Lieferung eigene Artefakte aufräumen, Evidenz erhalten und direkt das
  nächste unabhängige Paket bearbeiten. Explizite Nutzerpausen gelten sofort.
- Fehlende externe Evidenz blockiert nur ihr Paket. Keine Endlossuche oder
  automatische Kompatibilitätsfreigabe; Ergebnisse als geliefert/blockiert/
  bewusst ausgeschlossen unterscheiden.

ENDE
Beende Goal A mit veröffentlichten Commits, echten Prüfergebnissen, Restgrenzen
und einem konkreten Vertrag/Handover für Goal B. Starte Goal B nicht automatisch.
```
