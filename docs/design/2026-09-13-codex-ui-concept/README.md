# KNXBench — Vorschlag zur UI-Neugestaltung

Stand: 13.09.2026. Designvorschlag, keine implementierte Oberfläche und keine freigegebene Implementierungsspezifikation. Claude arbeitet parallel auf `main`; die Bestandsaufnahme ist eine Momentaufnahme der gelesenen Arbeitsdateien. Keine App-Dateien geändert.

## Empfehlung

Eine fokussierte Arbeitsoberfläche mit drei Bereichen: Navigation links, Listen und Detailarbeit in der Mitte, Eigenschaften rechts. Ruhige Flächen, klare Typografie, präzise Auswahlzustände und sparsame Akzentfarbe geben der technischen Arbeit Leichtigkeit. Häufige Aktionen bleiben sichtbar; seltene Dateioperationen wandern in ein klar beschriftetes Dateimenü. Warnungen bleiben erreichbar und werden niemals nur durch kurzlebige Meldungen vermittelt.

Abgewogene Richtungen:

- **Fokussierter Arbeitsbereich — empfohlen:** stabile Navigation und kontextbezogene Details; gut für Maus, Tastatur und tägliche Projektarbeit.
- **Freies Docking wie in einer IDE:** flexibel für viele Bildschirme, aber mehr Layoutzustände und Bedienaufwand. Vorerst nur verstellbare Bereiche und ein gezieltes Zusatzfenster vorsehen.
- **Grafische Gebäude-/Topologiefläche:** anschaulich, benötigt jedoch neue Interaktionen und eigene fachliche Projektionen. Für diesen ersten Entwurf sind Listen und Bäume die verlässlichere Grundlage.

## Drei Konzeptbilder

Die PNGs wurden mit dem eingebauten Bildgenerator erstellt. Sie zeigen fiktive Beispieldaten und vorgeschlagene Layouts, keine Screenshots einer laufenden Implementierung. Die vollständigen Generierungsprompts stehen in `prompts.md`.

1. **Porcelain — Gebäude und Geräte:** [01-porcelain.png](01-porcelain.png). Heller Arbeitsplatz; Raum, Geräte und Kommunikationsobjekte bleiben zusammen sichtbar.
2. **Graphite — Gruppenadressen:** [02-graphite.png](02-graphite.png). Dunkles Theme, tabellarische Adressarbeit und zugehörige Sender/Empfänger. Die Erscheinungsbild-Einstellungen sind hier zur Demonstration eingeblendet; im normalen Betrieb liegen sie im Einstellungsdialog.
3. **Graphite / Mint — Busmonitor:** [03-busmonitor.png](03-busmonitor.png). Separater Diagnosebereich als Konzept für einen zweiten Bildschirm. Verbindungszustand und explizites Senden bleiben klar sichtbar.

Bilddetails sind illustrativ: etwa Checkboxen im Kommunikationsobjektbereich, Beschreibungsfelder, zusätzliche Filter und Auswahlfelder sind kein Nachweis vorhandener Schreiboperationen. Der grüne Punkt neben „KNX-compatible“ im ersten Bild ist kein Prüf- oder Verbindungsnachweis und sollte in der Umsetzung entfallen. Die DPT-Auswahl im Busmonitor darf die bestehende Auflösung und Konfliktprüfung nicht umgehen.

## Tatsächlich im UI vorgefunden

| Bereich | Vorhandene Funktionen | Platz im neuen Design |
|---|---|---|
| Dateien | `.knxproj` öffnen/exportieren, `.knxdb` öffnen/speichern/speichern unter; Gruppenadress-CSV; Dokumentationsexport | Dateimenü, sichtbares Speichern, CSV direkt bei Gruppenadressen |
| Projektübersicht | Projektzahlen, Schema-/Importinformationen, Fehler und Warnungen | Übersicht und dauerhaft erreichbare Importhinweise |
| Struktur | Gebäudeteile, Bereiche und Linien anlegen/bearbeiten/löschen; Geräte nach Linie oder Gebäudeteil zuordnen | Navigation, strukturbezogene Listen, Inspektor |
| Geräte | Produktkatalog mit Suche/Herstellerfilter, Paketinstallation und Geräteanlage; Namen und Individualadressen bearbeiten | Produktkatalog und Geräteinspektor |
| Gruppenadressen | Adressen und Bereiche anlegen/bearbeiten/löschen; DPTs; Kommunikationsobjekte verknüpfen/entknüpfen und Flags bearbeiten | Adresstabelle plus kontextbezogene Verknüpfungen |
| Parameter | Parameteransicht, unterstützte Werte bearbeiten, Diagnosen und schreibgeschützte Zustände | Eigener Geräte-Tab; vorhandene Schreibgrenzen beibehalten |
| Mehrfachaktionen | Mehrfachauswahl für Geräte/Gruppenadressen; gesammelt löschen, Geräte verschieben | Kontextbezogene Aktionsleiste |
| Navigation | Projektsuche `Strg+K`, Befehlspalette `Strg+Umschalt+P`, Undo/Redo `Strg+Z` / `Strg+Umschalt+Z` | Sichtbare Suche und Befehlseinstieg, gleiche Kürzel |
| Diagnose | Protokoll mit Filtern; Projektvergleich mit nativer Datei; Busmonitor über Tunnelling, Telegrammfilter/-details, DPT-Dekodierung und Werte senden | Eigene Arbeitsansichten; optional Monitor als Zusatzfenster |
| Einstellungen | Theme-Auswahl mit aktuell einem Theme; Bewegung Off/Subtle/Standard und Smooth/Glitch; UI-/Produktdatensprache und Sprachpakete | Zusammenhängender Einstellungsdialog mit Vorschau |

Quellen: `apps/knx-web/src/App.tsx`, `ProjectExplorer.tsx`, `Inspector.tsx`, `CatalogBrowser.tsx`, `BulkActionToolbar.tsx`, `ParameterPanel.tsx`, `Dashboard.tsx`, `ProjectDiffPanel.tsx`, `LogPanel.tsx`, `BusMonitorPanel.tsx`, `BusComposeForm.tsx`, `SettingsPanel.tsx`, `commandRegistry.ts`, `theme.ts`, `motion.ts`, `styles.css`; außerdem `docs/ARCHITECTURE.md` und `docs/KNOWN_LIMITATIONS.md`. Die derzeitige Shell stellt viele globale Aktionen nebeneinander; ihr flexibles 40/60-Layout teilt hauptsächlich Explorer und Inspektor auf. Eine neue Informationshierarchie bringt daher mehr als ein Farbwechsel.

## Bediengefühl

- **Maus:** klare Hoverzustände, sichtbare Primäraktionen, Kontextmenüs mit denselben Befehlen, verstellbare Bereichsbreiten. Auswahl und Mehrfachauswahl unterscheiden sich visuell. Verschieben bleibt auch über beschriftete Aktionen möglich.
- **Tastatur:** durchgängige Fokusreihenfolge, deutlich sichtbarer Fokus, Pfeiltastennavigation in Listen/Bäumen, Enter zum Öffnen/Bestätigen, Escape zum Schließen/Abbrechen. Bestehende Kürzel erhalten. Neue Kürzel zentral erfassen und nicht gegen Texteingaben oder Betriebssystemfunktionen arbeiten lassen.
- **Animation:** direktes Hover-/Druckfeedback etwa 100–140 ms; Paneel- und Dialogübergänge etwa 180–240 ms mit weichem Abbremsen. Neue Auswahl und Kontextwechsel räumlich nachvollziehbar machen. Keine wandernden Tabellenzeilen oder Animation pro eingehendem Telegramm. Eingaben warten niemals auf Animationen.
- **Weniger Bewegung:** bestehende Off/Subtle/Standard-Einstellungen und `prefers-reduced-motion` respektieren; Fokus und Zustand auch ohne Bewegung klar vermitteln. Statische Bilder demonstrieren keine Animation; ein späterer interaktiver Prototyp muss dieses Verhalten beweisen.
- **Themes:** bestehende CSS-Variablen als Ausgangspunkt verwenden, semantische Rollen für Fläche, Text, Rahmen, Auswahl, Fokus, Erfolg/Warnung/Fehler vervollständigen. Porcelain und Graphite teilen Komponenten und Layout. Hell/Dunkel/System, Akzentfarbe und Kompakt/Komfortabel sind vorgeschlagene Erweiterungen. Fachliche Statusfarben bleiben eindeutig und unabhängig von der Akzentwahl.

## Mehrere Bildschirme

Zunächst genau ein bearbeitender Hauptarbeitsbereich. Ein Zusatzfenster für Busmonitor/Protokoll kann Diagnosen sichtbar halten. Das ist **noch nicht implementiert**: `KNOWN_LIMITATIONS.md` §63 beschreibt gemeinsamen Projektzustand und Undo-Stack ohne Konflikterkennung oder Benachrichtigung anderer Clients. Einfach eine zweite schreibende Editorinstanz zu öffnen wäre deshalb keine belastbare Lösung.

Für ein Zusatzfenster müssen Projektwechsel, Fenster-Schließen, Fokus, Anzeige-Skalierung, wiederherstellbare Positionen und ein nicht mehr angeschlossener Monitor behandelt werden. Es teilt eine Busmonitor-Sitzung; ein zusätzliches Fenster darf nicht eine konkurrierende Verbindung starten oder beim Schließen die gemeinsame Sitzung unbeabsichtigt beenden. Projektwechsel müssen Namen/DPTs im Monitor aktualisieren oder den Bezug explizit als veraltet kennzeichnen. Senden bleibt eine explizite Busaktion und ist nicht Teil des Projekt-Undo. Mehrfensterbetrieb in Browser und Tauri muss separat geprüft werden.

## Umsetzung nach Designentscheidung

1. Theme-Rollen, Typografie, Fokus und neue Shell auf vorhandene Ansichten anwenden; alle bisherigen Aktionen erhalten.
2. Geräte-/Adressarbeit in Tabellen und kontextbezogene Details überführen; notwendige Anzeigeprojektionen über `knx-server` bereitstellen. Fachliche Validierung, Mutationen, Undo/Redo und Persistenz bleiben in ihren bisherigen Schichten.
3. Bewegungsverhalten im interaktiven Prototyp prüfen; Maus-/Tastaturabläufe, lange Namen, große Listen, Fehlermeldungen, Sprachwechsel und reduzierte Bewegung gezielt testen.
4. Zusatzfenster als getrennte Erweiterung mit abgesichertem Sitzungs- und Projektbezug.

Kein neuer Anspruch auf Geräteprogrammierung, KNX Secure, ETS-Reimport oder Hardwareverifikation. Nicht unterstützte Importdaten und eingeschränkte Parameter bleiben sichtbar. Für diese reine Designarbeit wurden keine Anwendungs-Tests ausgeführt; geprüft wurden Quellenbezug, Bilder und die neu abgelegten Dokumente.
