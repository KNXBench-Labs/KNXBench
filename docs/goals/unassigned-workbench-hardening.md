# Prepared Goal B — workbench, UX and platform

Planning only; no session owns these packages until the user invokes this prompt.
Start after Goal A ends. Read the root-local audit and A's published outcomes
before creating a fresh worktree; prepared files may not yet exist on origin/main.

```text
/goal KNXBench: Workbench-Restarbeiten, UX und Plattformhärtung

Schließe die folgenden klar begrenzten Workbench-Reste autonom in kleinen Paketen.
Kein Neubau bereits gelieferter Notes-/Instructions-/Parameter-/Katalog-Funktionen.
Dies ist keine Freigabe für Hardware, ETS-/Herstellerausführung, Releases oder
Deployments. Datenintegrität schlägt Komfort, auch wenn Komfort hübscher aussieht.

START UND VORAUSSETZUNG
- Workspace: /mnt/daten-i/Sourcecode/KNXBench.
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
