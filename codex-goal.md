# Codex Goal: KNXBench UI vollständig neu gestalten

## Auftrag und Freigabe

Setze die vom Nutzer am 13.09.2026 ausdrücklich bestätigte UI-Neugestaltung vollständig um. Er hat den Entwurf mit „perfekt, genau so will ich das“ freigegeben. Arbeite selbstständig bis zu einem funktionierenden, geprüften Ergebnis; beende das Goal nicht nach Planung, einem Prototyp oder einer teilweise umgestalteten Oberfläche. Eine erneute Freigabe desselben Designs ist nicht erforderlich. Diese Datei ist die Arbeitsanweisung für einen anschließend gestarteten `/goal`-Lauf; ihre Erstellung allein startet keinen Lauf.

**Designreferenzen im Repository:**

- `docs/design/2026-09-13-codex-ui-concept/README.md`
- `docs/design/2026-09-13-codex-ui-concept/01-porcelain.png`
- `docs/design/2026-09-13-codex-ui-concept/02-graphite.png`
- `docs/design/2026-09-13-codex-ui-concept/03-busmonitor.png`

Öffne die drei Bilder vor der Umsetzung. Ihre Gestaltung, Informationshierarchie und Anmutung sind die visuelle Zielvorgabe. Sie sind generierte Mockups mit fiktiven Daten: keine Bildtexte, Zahlen, Geräte, Statusmeldungen oder vermeintlichen Schreiboperationen ungeprüft als fachliche Anforderungen übernehmen. Die historische Aussage „Design nicht freigegeben“ im damaligen Vorschlag ist durch die Nutzerfreigabe und diese Datei überholt.

**Ziel:** Eine moderne, Linux-first, themebare KNX-Engineering-App, deren Bedienung mit Maus und Tastatur Freude macht: präzise, übersichtlich, frisch und dynamisch, mit sanften Animationen im Apple-Stil und einem abgesicherten Diagnose-Zusatzfenster für einen zweiten Bildschirm, soweit die vorhandene Plattform dies verlässlich ermöglicht.

## Modell und Arbeitsweise

- Bevorzuge **GPT-6 Astra (`gpt-6-astra`)** für diesen komplexen Umbau. Nutze hohe Reasoning-Intensität für Architektur, Zustandsübergänge, Synchronisation und Fehleranalyse; normale Intensität für klare Routinearbeit. Qualität und verlässliche Fertigstellung haben Vorrang vor maximaler Geschwindigkeit.
- Prüfe das tatsächlich aktive Modell und die in der Laufzeit verfügbare Modellwahl. Wähle Astra, wenn die Laufzeit einen autorisierten Wechsel ermöglicht. Ist es nicht verfügbar, verwende das leistungsfähigste verfügbare geeignete Coding-Modell und dokumentiere die Abweichung. Behaupte keinen Modellwechsel, den du nicht ausgeführt hast. Die Datei selbst ändert kein Modell.
- Spare Aufwand durch gezielte Code-Lektüre, Wiederverwendung und passende Tests. Wiederhole erfolgreiche große Testläufe nur bei relevanten Änderungen oder ungeklärten Befunden. Kein Tokenbudget ist vorgegeben.
- Halte den Nutzer mit kurzen, konkreten Fortschrittsmeldungen auf dem Laufenden. Routineentscheidungen innerhalb dieses Designs selbst treffen. Wesentliche zusätzliche Produkt-, Daten- oder Sicherheitsentscheidungen bei Bedarf gezielt klären und unabhängige Arbeit fortsetzen.
- Zusätzliche Käufe, Credits, kostenpflichtige API-Ausweichwege oder externe Veröffentlichungen sind nicht Bestandteil dieses Auftrags.

## Branch und Zusammenarbeit mit Claude

Der Nutzer verlangt Arbeit auf **`main`** und hat ausdrücklich darauf hingewiesen, dass **Claude parallel in einer aktiven Session arbeitet**.

1. Zu Beginn und nach jeder Wiederaufnahme `.ai/CURRENT_STATE.md`, die einschlägigen `.ai/logs/`, `AGENTS.md`, den aktuellen Branch und `git status` lesen. Im direkten Hauptchat vorhandenes `MEMORY.md` nach Repository-Regel lesen. Dateizustand erneut prüfen; die ursprüngliche Bestandsaufnahme kann bereits veraltet sein.
2. Auf `main` bleiben. Keine neuen Branches oder Worktrees anlegen, keine Branchwechsel, Resets, Bereinigungen oder History-Rewrites durchführen. Nicht selbst committen oder pushen; dazu liegt hier kein ausdrücklicher Auftrag vor.
3. Claudes aktuelle Aufgaben und betroffene Dateien aus dem Handover feststellen. Bereits vorhandene Umsetzung wiederverwenden. Fremde oder noch nicht zuordenbare Änderungen niemals überschreiben, zurücksetzen oder als eigene Arbeit ausgeben.
4. Vor jedem Patch den aktuellen Inhalt der betroffenen Dateien lesen. Bei einer erkennbar gleichzeitig bearbeiteten Datei keine konkurrierenden Änderungen schreiben; zunächst eine unabhängige Aufgabe bearbeiten. Wenn ein echter Besitzkonflikt ungelöst bleibt, ihn konkret benennen und die notwendige Abstimmung anfordern. Keine Nachrichten an andere Personen oder externe Dienste senden.
5. Übergaben kollisionsarm ergänzen: unmittelbar zuvor erneut lesen, eigenen datierten Eintrag hinzufügen und Claudes Einträge erhalten. Nicht einen alten Gesamtsnapshot über eine inzwischen aktualisierte Datei schreiben.

## Aufgabenteilung mit Claude (festgelegt vom Nutzer am 13.09.2026)

Der Nutzer hat die Backlog-Aufteilung zwischen beiden Sessions ausdrücklich entschieden. Sie gilt zusätzlich zur oben beschriebenen Zusammenarbeit auf `main`.

**An Codex gehen genau zwei Backlog-Punkte aus `docs/GAP_ANALYSIS_ETS.md`, und nur diese beiden:**

- **T21 — grafische Topologie- und Gebäudeansichten.** Schließt die Lücken D1/D2.
- **T16 — Gerätekatalog-Browser für die Topologie.**

Beide sind reine Frontend-Arbeit und fallen damit in die UI-Neugestaltung, die dieses Goal beschreibt. Sie sind Teil des Pflichtumfangs, nicht optionale Zugabe: eine neue Oberfläche ohne Topologieansicht und ohne Katalog-Browser erfüllt dieses Goal nicht.

**Alles Übrige bleibt bei Claude.** Der Nutzer hat diesen Rest am 13.09.2026 zunächst gesperrt und noch am selben Tag die nicht-UI-Hälfte freigegeben („du kannst mit den Tasks weitermachen, die kein UI beinhalten“). Claude arbeitet also selbstständig an **A6** (passwortgeschützte Projekte), **E4** (weitere DPT-Haupttypen) und den Datenseiten von **D8**/**D10** weiter. Gesperrt bleiben die Punkte, die Oberfläche anfassen: **T28** (In-App-Hilfe, Tooltips, Handbuch) und die Chrome-Hälften von **D8** (Einstellungen über Theme/Motion/Sprache hinaus) und **D10** (Oberflächensprache). T28 ist ausdrücklich **kein** Codex-Auftrag, obwohl es Frontend berührt — der Nutzer hat nur T21 und T16 übertragen; es ist schlicht geparkt.

Daraus folgt praktisch: keine neuen Features außerhalb von T21, T16 und der beschlossenen Neugestaltung beginnen. Wird bei der Umsetzung eine Lücke sichtbar, die zu einem der gesperrten Punkte gehört, gehört sie in den Bericht und nicht in den Patch.

### Was Claude parallel tut und warum es das UI nicht berührt

Claude arbeitet derzeit **T17 (Linien-Scan)** ab, in einem eigenen Worktree auf dem Branch `t17-line-scan`. Die verbleibenden Teilaufgaben liegen in `crates/knx-net`, `apps/knx-cli` und `docs/` — **kein Pfad unter `apps/knx-web`**. Die Web-Gates laufen in dieser Arbeit folgerichtig als „nicht zutreffend“ mit. Ein Besitzkonflikt am Frontend besteht dadurch nicht.

Berührungspunkt ist trotzdem vorhanden: T17 erweitert `crates/knx-net/src/cemi.rs` um die Transportschicht (neues Feld `transport: Tpci` an `LDataFrame`, neue `ApplicationService`-Varianten). Wer den Bus-Monitor im neuen UI baut, sollte wissen, dass die gerenderten Telegrammzeilen aus `apps/knx-server/src/bus.rs` stammen und deren Feldbedeutung sich in diesem Zuge geschärft hat.

### Verbindliche Schnittstellen zwischen beiden Strängen

1. **Die Server-API ist der Vertrag.** `apps/knx-server` (axum) ist das, was die neue Oberfläche konsumiert. Claude hält sie stabil und dokumentiert Änderungen; Codex baut gegen sie und definiert sie nicht einseitig um. Ein benötigter Endpunkt, der fehlt, wird angefordert und nicht nebenbei erfunden.
2. **TypeScript-Typen kommen aus `crates/knx-projection`.** Die Bindings unter `crates/knx-projection/bindings/` sind generiert und entstehen auf einer Maschine erst, nachdem `cargo test -p knx-projection` dort einmal gelaufen ist — ein leeres Verzeichnis bedeutet also nicht, dass es die Typen nicht gibt. Die neue Oberfläche verwendet die generierten Typen; handgeschriebene Duplikate desselben Modells sind der Weg, auf dem Frontend und Domäne auseinanderlaufen.
3. **`apps/knx-desktop` (Tauri) hängt am Web-UI.** Die Neugestaltung muss die Desktop-Hülle mitdenken, nicht nur den Browser.
4. **Die bestehenden Web-Gates sind der Maßstab, den die neue Oberfläche erben muss:** aus `apps/knx-web` heraus `npm test -- --run` (zuletzt 340 bestandene Tests über 31 Dateien) und `./node_modules/.bin/tsc --noEmit` (der direkte Pfad ist nötig, weil ein schlichtes `npx tsc` in eine Namenskollision läuft). Die alte Oberfläche bleibt so lange stehen, bis die neue dieselben Gates besteht — sonst fällt ein Gate weg, statt ersetzt zu werden.

## Grenzen und fachliche Invarianten

Die geltenden Repository-Anweisungen und Architekturentscheidungen bleiben verbindlich. Vor betroffenen Änderungen insbesondere `docs/ARCHITECTURE.md`, `docs/DATA_MODEL.md`, `docs/IMPORT_EXPORT.md`, `docs/COMPATIBILITY.md`, `docs/RESEARCH.md`, `docs/KNOWN_LIMITATIONS.md`, `docs/ROADMAP.md` und einschlägige ADRs lesen.

- Frontend spricht ausschließlich mit `knx-server` über dessen HTTP-API. Neue Anzeigeformen gehören in Projektionen. Validierung und Mutationen bleiben in Domain-/Application-Commands; Undo/Redo und Persistenzinvarianten erhalten.
- Bestehende Funktionen vollständig erhalten, auch Funktionen, die Claude seit dem Entwurf ergänzt hat. Keine UI-Sackgassen und keine dauerhaft funktionslosen Platzhalter.
- Unterstützte Parameterbearbeitung und ihre aktuellen Schreibgrenzen erhalten; die tatsächliche aktuelle Implementierung ist maßgeblich. Importdiagnosen, unbekannte Daten, schreibgeschützte Zustände und Fehler nicht verdecken.
- Kein zusätzlicher Auftrag für Commissioning/Download, KNX Secure, unbestätigte proprietäre Formate oder ETS-Parität. Produktdaten nicht hart codieren; Originaldaten nicht ändern.
- Nur „KNX-compatible“ als Kompatibilitätsbezeichnung. Keine Zertifizierungsbehauptung, kein grüner Prüfstatus ohne echte Bedeutung. Keine erfundenen Speicher-, Verbindungs- oder Gerätestatus.
- Keine Live-Busaktionen zur Designprüfung auslösen. Senden mit simulierten Transporten prüfen; ein echter Hardwaretest benötigt einen ausdrücklich dafür autorisierten Kontext.

## Verbindliche Gestaltung und Bedienung

### Gemeinsame Shell

Baue die Dreiteilung aus Navigation links, Arbeitsbereich in der Mitte und kontextbezogenem Inspektor rechts. Bereichsbreiten sind verstellbar, Bereiche sinnvoll einklappbar. Bei weniger Platz bleiben alle Funktionen erreichbar; keine abgeschnittenen Pflichtaktionen. Projekt-/Dateimenü, Undo/Redo, Suche, Befehlseinstieg und Speichern stehen in einer aufgeräumten oberen Leiste. Seltene Aktionen werden verständlich gruppiert.

Nutze ruhige Flächen, dezente Tiefe, feine Rahmen, konsistente Outline-Icons, klare Typografie, großzügige Hierarchie und kompakte Datentabellen. Keine Neon-Effekte, dekorativen Riesenkarten oder unübersichtliche Buttonwand. Monospace für Adressen und Rohdaten. Visuelle Werte aus den Referenzbildern in ein gemeinsames Designsystem übersetzen, nicht jede Ansicht separat stylen.

### Themes und Bewegung

- Vollständiges helles **Porcelain** und dunkles **Graphite**, Systemmodus, wählbare Akzentfarbe einschließlich Violett und Mint. Alle Tabellen, Overlays, Formulare, Fehler, Fokuszustände und Diagramme, falls bereits vorhanden, müssen beide Themes unterstützen.
- Vorhandene CSS-Variablen und Theme-/Motion-Infrastruktur erweitern. Einstellungen persistieren; vorhandene gespeicherte Werte bewusst weiter unterstützen oder mit dokumentiertem Fallback überführen. Kein ungestylter Startblitz. Semantische Statusfarben bleiben von der Akzentfarbe unabhängig.
- Kompakte und komfortable Dichte mit klar bedienbaren Zielen. Kontrast und Fokus praktisch prüfen, keine unbelegte Accessibility-Konformität behaupten.
- Sanftes Hover-/Druckfeedback etwa 100–140 ms, Kontext-/Panel-/Dialogwechsel etwa 180–240 ms mit weichem Abbremsen. Animationen müssen unterbrechbar sein und dürfen Eingaben nicht verzögern.
- Bestehende Bewegungsoptionen erhalten; Off und `prefers-reduced-motion` zuverlässig respektieren. Keine Bewegung pro Bus-Telegramm, keine springenden Tabellen oder Animationswarteschlangen.

### Maus und Tastatur

Sichtbare Primäraktionen und konsistente Kontextmenüs, klare Trennung von Fokus, Einzel- und Mehrfachauswahl. Bestehende Mehrfachaktionen erhalten. Listen/Bäume mit Pfeiltasten, Dialoge mit korrektem Fokusmanagement und Escape, komplette Abläufe ohne Maus. Bestehende Suche `Strg+K`, Palette `Strg+Umschalt+P`, Undo/Redo `Strg+Z` / `Strg+Umschalt+Z` erhalten. Texteingaben nicht durch globale Kürzel stören. Neue Aktionen müssen über dieselben validierten Befehle erreichbar sein, unabhängig vom Eingabegerät.

## Funktionsabdeckung

Erstelle zum Start eine knappe Abdeckungsmatrix anhand des **aktuellen** Codes: bisheriger Einstieg, neuer Einstieg, Nachweis. Folgende Bereiche müssen mindestens erfasst und vollständig in das neue Design integriert werden:

- Projektübersicht, Importberichte und dauerhaft erreichbare Warnungen/Fehler.
- `.knxproj` öffnen/exportieren, `.knxdb` öffnen/speichern/speichern unter, Gruppenadress-CSV, Dokumentationsexport und Projektvergleich.
- Gebäude, Gebäudeteile, Topologie mit Bereichen/Linien; Geräte anlegen, bearbeiten, zuordnen und löschen.
- Produktkatalog, Herstellerfilter, Suche, Paketinstallation und Produkt-/Anwendungsmetadaten.
- Gruppenadressen/-bereiche, DPTs, Kommunikationsobjekte, Flags sowie Verknüpfungen und Richtungen.
- Parameter, bestehende Diagnosen, unterstützte Schreiboperationen und schreibgeschützte Felder.
- Mehrfachauswahl und vorhandene Sammelaktionen, Undo/Redo, Projektsuche, Befehlspalette.
- Protokoll und Busmonitor: Verbindung, Filter, Telegrammdetails, DPT-Dekodierung und explizites Senden.
- UI-/Produktdatensprache, Sprachpakete, Theme-, Motion- und neue Erscheinungsbild-Einstellungen.

Die zentralen bestehenden Einstiegspunkte liegen unter `apps/knx-web/src/`, insbesondere `App.tsx`, `styles.css`, `theme.ts`, `motion.ts`, `ProjectExplorer.tsx`, `Inspector.tsx`, `Dashboard.tsx`, `CatalogBrowser.tsx`, `ParameterPanel.tsx`, `BulkActionToolbar.tsx`, `Search.tsx`, `CommandPalette.tsx`, `commandRegistry.ts`, `Overlay.tsx`, `SettingsPanel.tsx`, `ProjectDiffPanel.tsx`, `LogPanel.tsx`, `BusMonitorPanel.tsx` und `BusComposeForm.tsx`. Diese Liste dient der Orientierung und ist keine Aufforderung, jede Datei ändern zu müssen.

## Zweiter Bildschirm

Prüfe einen gezielten separaten Busmonitor-/Diagnose-Arbeitsbereich in Browser und Tauri. Kein allgemeines freies Docking-System und kein zweiter unabhängig schreibender Projekteditor. Beachte insbesondere die aktuelle Fassung von `KNOWN_LIMITATIONS.md` §63.

- Genau ein bearbeitender Hauptarbeitsbereich; Zusatzfenster darf keine Projektmutationen oder eigenes Projekt-Undo anbieten. Die API bleibt die fachliche Grenze.
- Gemeinsame Bus-Sitzung, keine versehentlichen Doppelverbindungen. Schließen eines Fensters beendet nicht automatisch die gemeinsam genutzte Busverbindung.
- Projektwechsel, DPT-/Namensänderungen und Verbindungswechsel werden synchronisiert oder vor weiterer Nutzung ausdrücklich als veraltet gesperrt. Keine stillschweigend falsche Dekodierung oder Sendekonfiguration.
- Senden bleibt eine explizite, validierte Busaktion und ist nicht durch Projekt-Undo rückgängig zu machen.
- Fensterpositionen, mehrere Skalierungen, fehlender zweiter Monitor, blockierte Popups, Wiederöffnen und Rückkehr zum Hauptfenster behandeln; bei nicht verfügbarem Zusatzfenster bleibt der Monitor im Hauptfenster vollständig nutzbar.
- Falls belastbare Umsetzung eine neue, nicht freigegebene Mehrbenutzerarchitektur oder eine nicht verfügbare Plattformfähigkeit erfordert: die übrige Neugestaltung fertigstellen, den konkreten Blocker mit Evidenz dokumentieren und diese Teilanforderung offenlassen. Nicht bloß ein zweites unsynchronisiertes Fenster als fertig ausgeben.

## Ausführung in überprüfbaren Etappen

- [x] Aktuellen Stand, Claudes aktive Änderungen und Funktionsabdeckung aufnehmen; kurze ausführbare Teilplanung mit konkreten Tests und Datei-Zuständigkeiten dokumentieren.
- [x] Theme-/Motion-Grundlage, UI-Bausteine und Shell umsetzen; vorhandene Ansichten funktionsfähig einbinden.
- [ ] Gebäude-/Gerätearbeit einschließlich Kommunikationsobjekten, Parametern und Katalog in die neue Hierarchie überführen.
- [ ] Gruppenadressen, Verknüpfungen, Mehrfachaktionen und alle Datei-/Export-/Vergleichsfunktionen integrieren.
- [ ] Protokoll, Busmonitor, Einstellungen und Sprache konsistent gestalten; alle bisherigen Einstiege gegen die Matrix prüfen.
- [ ] Abgesicherten Diagnose-Zusatzfensterbetrieb implementieren und nachweisen, soweit wie oben beschrieben möglich.
- [ ] Maus-/Tastaturbedienung, Theme-/Motion-Wechsel, Fehlerzustände und reale Bildschirmdarstellung prüfen; Befunde beheben.
- [ ] Abschließende Gates ausführen, echte Screenshots ablegen und Dokumentation/Handover vervollständigen.

Für jede funktionale Änderung gezielte Regressionstests an der verantwortlichen Schicht schreiben oder erweitern. Vorhandene Hilfen, Komponenten und Tests bevorzugen. Keine Tests, die lediglich CSS-Zeilen oder Implementierungsdetails nachbauen. Große Umbauten nach Verantwortung zerlegen; keine parallele zweite App, kein vollständiger Austausch des bestehenden Stacks ohne konkrete Notwendigkeit.

## Nachweis und Fertigkriterien

Das Goal ist erst erfüllt, wenn alle verpflichtenden Funktionen in der neuen Oberfläche nutzbar sind, die Abdeckungsmatrix belegt ist und keine bekannte eigene Regression offenbleibt. Offene Zusatzfenster-Blocker ausdrücklich ausweisen; in diesem Fall nicht behaupten, auch diese Fähigkeit sei abgeschlossen.

Während der Arbeit die kleinsten relevanten Tests und Typprüfungen nutzen. Abschließend für den fertigen Stand die relevanten Repository-Gates nachweisen:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p xtask -- check-layering
```

Im Verzeichnis `apps/knx-web`:

```bash
npm test
npm run build
```

Aktuelle Handover-Hinweise zu Build-Nebenwirkungen beachten, insbesondere das bereinigte Vite-Ausgabeverzeichnis und die getrackte `dist/.gitkeep`. Fremde Build-Artefakte nicht überschreiben. Bei Konflikten mit Claudes aktivem Build zunächst unabhängige Prüfungen durchführen und den gemeinsamen Ausgabepfad abstimmen.

Zusätzlich im echten Browser prüfen: drei Referenzansichten, Hell/Dunkel/System, Akzentwechsel, beide Dichten, reduzierte Bewegung, vollständiger Tastaturablauf, lange Texte, leere/große Listen und Fehlerfälle. Mindestens Desktopbreiten 1280 und 1920 Pixel sowie erhöhte Skalierung prüfen. Zusatzfenster nach Möglichkeit auch in Tauri prüfen; fehlende Laufzeit ausdrücklich dokumentieren. DOM-Tests allein beweisen keine visuelle Qualität, Screenreader-Nutzbarkeit oder native Mehrfensterfähigkeit.

Liefere **mindestens drei echte Screenshots der implementierten App**, analog zu den Referenzen, und einen kurzen realen Bewegungsnachweis als Aufnahme oder nachvollziehbaren geprüften Ablauf. Verwende dafür Beispieldaten, die klar als solche erkennbar sind. Generierte Bilder gelten nicht als Implementierungsnachweis. Berichte tatsächliche Testresultate und nicht ausführbare Prüfungen getrennt. Ändere keine fachlichen Tests, um Regressionen zu verdecken.

## Kontinuität und Session-Limits

**Nutzerwunsch: Bei erreichtem Session-/Nutzungslimit automatisch fortfahren, sobald wieder verfügbar.**

- Das Ziel bleibt über automatische Fortsetzungen und Kontextkompaktierungen hinweg dasselbe. Nach Wiederaufnahme selbstständig am letzten verifizierten Zwischenstand fortsetzen, ohne erneute allgemeine Freigabe.
- Nach jeder abgeschlossenen Etappe und vor absehbarem Limit einen wiederaufnehmbaren Checkpoint in `.ai/logs/` und einen kurzen Eintrag in `.ai/CURRENT_STATE.md` sichern. Festhalten: Datum, Goal, freigegebenes Design, aktive Modellwahl, exakt geänderte Dateien, Claudes Abgrenzung, erledigte/offene Schritte, letzter Test mit Ergebnis, offene Fehler und nächste konkrete Aktion. Checkboxen in dieser Datei aktuell halten.
- Ist das Limit erreicht und die Laufzeit unterstützt wartende Goals oder eine automatische Wiederaufnahme, diese Möglichkeit nutzen und nach Freigabe der Kapazität automatisch weiterarbeiten. Vorhandene Limit-/Retry-Zeitangaben beachten; keine hektischen Wiederholungen, keine Limitumgehung und kein unautorisierter kostenpflichtiger Ausweichweg.
- Eine Markdown-Anweisung kann einen vom Anbieter gestoppten oder beendeten Prozess nicht selbst starten. Fehlt der Laufzeit ein automatischer Wiederanlauf, den Checkpoint sichern und diese technische Grenze ehrlich melden; keinen eingerichteten Scheduler oder garantierten Neustart behaupten. Beim nächsten tatsächlichen Resume ohne erneute Designabstimmung fortsetzen.
- Ein Limit, Kontextende oder Zeitablauf ist kein Fertignachweis. Goal nicht deshalb als abgeschlossen markieren. Für Goal-Status und Blockierung gelten die tatsächlichen Regeln der verfügbaren Goal-Werkzeuge.

## Abschlussbericht

Kurzer Bericht auf Deutsch: was umgesetzt wurde, Screenshots/Bewegungsnachweis mit Pfaden, geprüfte Abläufe und Gates, etwaige konkrete Restgrenzen. `.ai/CURRENT_STATE.md` im vorgeschriebenen Format aktualisieren; Tagesnotiz und technische Logs nach Repository-Regeln pflegen. Keine pauschale Behauptung „fertig“, solange Pflichtumfang oder erforderliche Beweise fehlen.

## Quellen für die Codex-Betriebsangaben

Am 13.09.2026 abgerufen:

- Modellwahl: https://developers.openai.com/codex/models/ — GPT-6 Astra als leistungsfähigstes Modell für komplexe Arbeit; Verfügbarkeit in der jeweiligen Laufzeit prüfen.
- Nutzungslimits: https://developers.openai.com/codex/pricing/ — nutzungsabhängige Limits. Daraus folgt keine Zusage eines automatischen Wiederanlaufs nach Limitende.
