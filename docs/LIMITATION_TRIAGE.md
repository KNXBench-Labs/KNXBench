# Limitierungen nach Kritikalität

Sortierung der 146 nummerierten Einträge aus [`KNOWN_LIMITATIONS.md`](KNOWN_LIMITATIONS.md)
(`grep -cE '^## (§)?[0-9]' docs/KNOWN_LIMITATIONS.md`), Stand 2026-09-30. Diese
Datei ordnet nur — sie ersetzt keinen Eintrag und enthält keine neuen Fakten.
Maßgeblich bleibt der Volltext dort. §105 ist absichtlich nicht eingestuft
(siehe unten); die restlichen 145 sind es.

Nummern sind die Abschnittsnummern der Quelldatei. §94 existiert nicht; §130
ist zweimal vergeben (Gate-Binary und Anwendungszoom) und steht deshalb
zweimal in der Tabelle. Die sieben unnummerierten Abschnitte am Ende der
Quelldatei (Korpus-, Katalog-, Geräteeditor- und Inbetriebnahmegrenzen) sind
nicht eingestuft.

**Stand 2026-09-30 (Neuzählung).** Die Fassung vom 2026-09-20 zählte 119
Einträge. Seitdem sind §121–§146 hinzugekommen, und etliche Einträge sind
laut ihrem eigenen Status geschlossen oder verengt; nur der Statustext der
Quelle hat entschieden, nicht der Titel (bei §19, §58 und §120 ist der Titel
älter als der Text).

## Einstufung

| Stufe | Bedeutung |
| --- | --- |
| **K1 — kritisch** | Kann Daten verfälschen, Geräte falsch beschreiben oder das Projekt Fremden öffnen, ohne dass es auffällt. |
| **K2 — hoch** | Blockiert einen Kern-Workflow oder trägt eine Kompatibilitätsaussage nicht, die jemand erwarten würde. |
| **K3 — mittel** | Echte Lücke, aber enger Wirkungsbereich oder es gibt einen Weg drumherum. |
| **K4 — niedrig** | Kosmetik, Interna, Dokumentation. Kostet niemanden Daten. |
| **Erledigt** | Steht als Historie in der Datei, ist aber geschlossen. |

---

## K1 — kritisch (5)

| § | Thema | Warum K1 |
| --- | --- | --- |
| 92 | Inbetriebnahme fast nur gegen den eigenen Simulator geprüft | Real beschrieben wurde bisher ein einziges Gerät (`1.1.67`, Maske `0701h`); jede weitere Aussage über Phase 2 ruht auf Code, den dieses Projekt selbst geschrieben hat. |
| 61 | DPT-Codec: Haupttypen 1–30, explizite Eingabeformate, mehrere Kodierungen sind Rulings statt Standard | Nicht mehr geraten; ein falsches Ruling ginge aber weiterhin als gültiges Telegramm auf den Bus und sähe dort richtig aus. |
| 99 | Nibble-Reihenfolge im `MemoryControlBlock` ist abgeleitet, nicht spezifiziert | Eine geratene Byte-Anordnung in einem Schreibzugriff auf ein Gerät. |
| 8 | KNX Secure nicht implementiert | Eine gesicherte Installation fällt stillschweigend aus dem Funktionsumfang. Per Ruling zurückgestellt, nicht gelöst. |
| 1 | Single-Sample-Bias | Alles Gewusste über `.knxproj` stammt aus zwei Installationen. Färbt jede Import-Aussage. |

## K2 — hoch (27)

**Format und Import**

| § | Thema | Warum K2 |
| --- | --- | --- |
| 2 | Kein öffentliches XSD, Import ist tolerant statt validierend | Fehler fallen erst später auf. |
| 3 | Geräteparameter erhalten, aber nicht interpretiert | Integrität gewahrt, Nutzbarkeit nicht. |
| 11 | `.knxprod` mit Master-Data-Schema ≥ 12 nicht direkt importierbar | Neuere Herstellerdaten bleiben draußen. |
| 13 | AES-verschlüsselte Projekte (ETS6) werden abgelehnt | Ganze Projekte nicht zu öffnen. ZipCrypto (ETS4/5) geht. |
| 87 | Ein Parser-Fix erreicht bereits eingelesene Zeilen nicht | Nur eine Migration holt sie zurück. |
| 12 | Herstellerdaten-Auflösung: eine von drei Lücken geschlossen | Betrifft die Zuordnung Produkt→Programm. |

**Inbetriebnahme und Bus**

| § | Thema | Warum K2 |
| --- | --- | --- |
| 7 | Geräte-Download: v1 ist der verifizierte Speicherpfad für Maske `070nh` | Alles andere wird mit Namen abgelehnt (ADR-0048); verifiziert an einem Gerät (`1.1.67`). |
| 93 | `PID_PROGRAM_VERSION` wird bedingungslos geschrieben | Geparkt, unbesetzt — C11 und C12 sind gelandet, ohne das anzufassen (korrigiert 2026-09-20). |
| 101 | Der „once more"-Versuch kann die Worst-Case-Wartezeit verdreifachen | Spec-konform, aber teuer. |
| 72 | Ein ungedrosselter Line-Scan kostet echte Buszeit | Zehn Minuten aufwärts mit offenem Tunnel an einer laufenden Anlage. |
| 74 | Belegt-aber-beschäftigt ist nicht von abwesend zu unterscheiden | Der Draht gibt die Unterscheidung nicht her. |
| 78 | Andere KNXnet/IP-Endpunkte erscheinen als belegte Geräte | Scan-Ergebnis enthält Nicht-Geräte. |
| 62 | Group Monitor nur Tunneling, eine Session, Filter clientseitig | Nur der passive Empfangspfad hat Evidenz von einem echten Gateway. |
| 26 | `BusConnection` ohne KNX IP Secure | Folgt aus §8. |
| 79 | Discovery braucht IP-Multicast, das Dockers Default-Bridge nicht führt | Im Container stumm. |
| 112 | Downloadplan mit `A_Key_Write`-Bedarf wird ganz abgelehnt | Schlüsseländerung während eines Downloads ist damit kein lauffähiger Kern-Workflow. |
| 114 | Download-Counter-Refusal ist eigenes Ruling, nicht System-B-Pflicht | Ein konformes System-B-Gerät ohne Download Counter bekommt nie einen Partial Download — der Kern-Workflow ist für den Regelfall faktisch abgeschaltet. |

**Anwendung und Interoperabilität**

| § | Thema | Warum K2 |
| --- | --- | --- |
| 63 | Kein Mehrbenutzerbetrieb: ein Projekt, ein Undo-Stack, keine Konflikterkennung | Zwei Bearbeiter überschreiben sich. |
| 6 | Geräte hinter Hersteller-Plug-in-DLLs nicht konfigurierbar | Ganze Produktfamilien außen vor. |
| 38 | Gruppenadress-CSV ohne verifizierte ETS-Interoperabilität | Eigenes Format, ungetestete Annahme. |
| 44 | Doku-Export ohne ETS-Report-Parität | Und Parität ist derzeit nicht messbar. |
| 51 | Projekt-Diff ohne ETS-Vergleichsparität | Dito. |
| 85 | `.signature` wird gespeichert, nie geprüft | Eine Signatur, die niemand liest. |
| 68 | Wiederholte Modul-Instanziierung wird abgelehnt | Bewusst, aber es fehlt Funktionalität. |
| 69 | `Module` ohne `@Id` nicht zuordenbar | Optionales Attribut, das wir brauchen. |
| 71 | Vor Store-Schema 6 importierte Projekte haben keine Modul-Instanz-Ids | Nicht beschreibbar ohne Neuimport. |
| 129 | Veralteter Id-Allokator-Snapshot konnte Ids duplizieren | Datenverlustpfad geschlossen (ADR-0039 Phasen 1–2, 2026-09-27); dass alles über `Command::apply` läuft, sichert weiter nur das Review — Phasen 3–5 offen. |

## K3 — mittel (53)

| § | Thema | Warum K3 |
| --- | --- | --- |
| 9 | Projektdateien nicht diffbar (SQLite) |
| 14 | Default-Sprache des Projekts ist ein Platzhalter |
| 15 | Unparsbare Werte überleben nur auf `Override`-Feldern |
| 18 | `open_project` räumt den alten `store_path` nicht weg |
| 23 | `/api/project/download` puffert die ganze Datei im Speicher |
| 24 | `FsPicker` ohne Drag-and-Drop-Mehrfachauswahl |
| 29 | `knx bus monitor` formatiert Gruppenadressen immer dreistufig |
| 39 | CSV-Import adressiert nicht um, löscht nicht, verwaltet keine Bereiche |
| 40 | CSV-Nur-Export-Spalten werden beim Import nie angewandt |
| 41 | CSV aus Excel unter deutschem Gebietsschema kann überraschen |
| 45 | Doku-Export ohne natives PDF |
| 46 | Doku-Export löst Hersteller-, Produkt-, Programmnamen nicht auf |
| 47 | Doku-Export listet Parameterwerte und Modul-Argumente nicht |
| 48 | Doku-Export nur einsprachig |
| 52 | Diff korreliert Geräte ohne Adresse und ohne `ets_id` nicht |
| 53 | Diff kollidiert bei zwei gleichnamigen Geschwister-Gebäudeteilen |
| 54 | Diff erkennt regenerierte `RefId`s eines Re-Imports nicht |
| 55 | Diff lässt sich nicht zurück auf ein Projekt anwenden |
| 56 | Diff kennt keinen Drei-Wege-Vergleich |
| 70 | Schreiben eines deklarierten, aber nicht gezeigten Parameters wird abgelehnt |
| 73 | Line-Scan lernt weder Produkt noch Hersteller noch Seriennummer |
| 75 | Kürzeres `--timeout-ms` möglich, aber nicht Default |
| 76 | Schnellpfad für negatives Layer-2-Confirm bewusst nicht gebaut |
| 77 | Line-Scan bleibt auf einer Linie, überquert keine Koppler |
| 82 | Stale-Lock des Diagnose-Fensters sieht nur ein Browserprofil |
| 86 | Doppelte Bezeichner in einer Datei: DPT-Provenienz bleibt begrenzt |
| 88 | Anzeigename eines Herstellers ist Last-Writer-Wins (Absicht) |
| 97 | Fortschritt ist meist eine Phasenbezeichnung, keine Prozentzahl |
| 102 | Der Decode-Fehlerzweig des Write-Echos hat keinen bekannten Auslöser |
| 64 | `Languages`-Blöcke außerhalb eines Programms — Ingestion gelöst, Lesen teilweise |
| 104 | Gerät offline mitten in `LoadCompleting` kostet vollen Reconnect pro Poll | Latenz/Bustraffic, laut Eintrag ausdrücklich keine Korrektheitsfrage. |
| 106 | Debug-Report redigiert vier Musterklassen, sonst nichts | Bewusst begrenzt, offengelegt, Zip wird vor Versand angezeigt. |
| 107 | Keine Plugin-API — Dritte können nur forken, nicht nachladen | ADR-0025-Entscheidung; vier andere Erweiterungswege (Sprachpakete, Produktdatenbank, CSV, CLI) bleiben offen. |
| 108 | MP §2.3 widerspricht sich zur belegten `IA_new`; Ausnahmetext gewinnt | Ergebnis wird als Befund gemeldet, nicht stillschweigend erzwungen. |
| 113 | Eskalation lädt nur die Segmente neu, die der Plan tatsächlich trägt | Nur relevant, wenn der Aufrufer den Plan unvollständig baut; Regelfall betroffen es nicht. |
| 116 | `NM_IndividualAddress_Write` wiederholt nicht für den Bediener, liest eine Transport-Layer-Freigabe als MP §2.3 es nicht ausdrücklich sagt | Beide Abweichungen ändern keine Stopp/Weiter-Entscheidung des Standards. |
| 16 | Tauri v2 bleibt unter Linux auf GTK3 | Die früheren Advisories sind zurückgezogen (verifiziert 2026-09-22); offen ist nur der Wechsel auf das GTK4-Backend. |
| 125 | ETS-6-Objekt-Ids (geräte-lokale Form) aus einem einzigen Projekt belegt | Folgt aus §1; eine zweite unabhängige Probe fehlt. |
| 126 | Line-Scan-Abgleich handelt auf Belegungsevidenz, nicht Geräteidentität | Nur nach ausdrücklicher Auswahl; Schweigen gilt nicht als Abwesenheit. |
| 127 | Liegenschaft über mehrere Gebäude nur aus Schematext und synthetischen Tests | Kein ETS-Beispiel mit `Ground`-Wurzel vorhanden. |
| 128 | Legacy-`.vd3`–`.vd5`/`.pr3`–`.pr5` werden abgelehnt, unter falschem Namen | Atomar abgelehnt, 0 Zeilen geschrieben; nur die Meldung stimmt nicht. |
| 130 | Ein Gate-Binary kann ein nicht mehr existierendes Verzeichnis „prüfen" | Meldet Erfolg über null Dateien; Ergebnisgröße statt Exit-Code prüfen. (Nummer doppelt vergeben, siehe K4.) |
| 133 | Ein toter Webview lässt sich nicht per Fensterknopf schließen | Folge des §132-Fixes; aus den Quellen gelesen, nicht reproduziert. |
| 134 | Baggage wird inventarisiert, nicht interpretiert | Nichts wird ausgeführt oder entpackt; unbekannte Medien bleiben `unknown`. |
| 135 | Paketidentität wird aufgezeichnet, nicht entschieden | Gespeichert bleibt die zuerst installierte Fassung; Abweichungen werden gezeigt, nicht aufgelöst. |
| 136 | Maske `0701h`: Anwendungsdownload verifiziert, der abschließende Restart bleibt unbestätigt | Daten und Ladezustände rückgelesen; nur der Neustart ist nicht bestätigt. |
| 138 | Geräteschlüssel aus Projekt oder Schlüsseldatei, live nie geprüft | Das Testgerät hat keinen Schlüssel; einen zu setzen wäre ein unnötiger Schreibzugriff. |
| 139 | Adressierung über Seriennummer: Lesen live verifiziert, Schreiben greift am Testgerät nicht | Wird ehrlich als „gesendet, aber NICHT bestätigt" gemeldet. |
| 140 | Individualadress-Reset nur im Simulator, auf Hardware gesperrt | Trifft jedes Gerät mit gedrückter Taste; ohne Bedieneranforderung bewusst gesperrt. |
| 141 | Master Reset löscht nur im Simulator | Auf Hardware bleibt die Konfiguration; die Unterstützungsprüfung ist eine eigene Regel. |
| 143 | RF-Domänenadressen nur im Simulator, kein RF-Gerät vorhanden | Auf Hardware gesperrt, keine CLI-/HTTP-Route. |
| 144 | RF-Gerätekonfiguration nur im Simulator | Auf Hardware gesperrt, keine CLI-/HTTP-Route. |
| 146 | Kanal ohne `@Text` hat keinen eigenen Namen; manche Aktivierungen bleiben `Undetermined` | `@Name`/`@Number` werden nicht gespeichert; Unsicheres wird nicht geraten (ADR-0050). |

## K4 — niedrig (16)

| § | Thema | Warum K4 |
| --- | --- | --- |
| 42 | `command_sync.rs`' Moduldoku überzeichnet die eigene Rolle |
| 43 | Animationen ohne In-App-Schalter, nur OS-Präferenz |
| 65 | `--version` nennt einen Commit, nie einen Arbeitsstand |
| 90 | DPT-Haupttyp 46 gibt es nicht — 46 war eine Anzahl |
| 95 | Sieben Stellen, an denen der Standardtext nicht wörtlich gilt (Wegweiser, keine Kosten) |
| 98 | Die zweite Flavour-Zeile sieht fast niemand |
| 100 | Hilfetexte liegen im Message-Katalog, ein Absatz pro Schlüssel |
| 109 | Zwei Downloadteile derselben `PartKind` ohne definierte Reihenfolge werden beide abgelehnt | Kein einziger Fall bislang beobachtet, Ablehnung statt Rateversuch. |
| 110 | `PID_GROUP_RESPONSER_TABLE` bleibt auf jedem Medium unimplementiert | Bewusst — PL110-only laut RES, dieses Projekt zielt auf TP1/RF/IP. |
| 111 | CP §3.5.4 Schritt 07 (Individualadresse entladen) bleibt unimplementiert | Bewusste Weigerung: das Werkzeug soll das eigene Zielgerät nicht unadressierbar machen. |
| 115 | `MasterResetResponse::recovery_wait`/`SessionTiming::restart_basic_t1` berechnen Wartezeiten, die niemand abwartet | Toter Code ohne heutigen Aufrufer. |
| 60 | Diff-Webpanel blättert große Tabellen | Weitgehend gelöst (CT-1); Restgrenze ist das Paging. |
| 121 | Zwei offene Fenster sehen Einstellungsänderungen erst nach Reload | Kein Push-Kanal; ein Reload genügt. |
| 124 | Schnittstellensuche zeigt vier Fakten, das Protokoll trägt mehr | `knx-net` dekodiert alles, die Oberfläche zeigt einen Ausschnitt. |
| 130 | Anwendungszoom browser-, nicht WebKitGTK-verifiziert | Nur in Chromium geprüft. (Zweiter Eintrag mit Nummer 130.) |
| 137 | Bus-Monitor-JSON ist ein behaltenes Fenster, keine vollständige Aufzeichnung | Verluste werden gezählt und exportiert, nicht wiederhergestellt. |

## Erledigt, steht als Historie drin (44)

§10 Lizenz (AGPL) · §17 verwaiste `GroupLink`s · §20 Command-Palette-Overlay (teilweise) ·
§21 Export ohne Gruppenbereich · §25 Docker-Node-22 · §27 Heartbeat-Race ·
§28 Tunnel-Close-Signal · §31 Multicast-Override · §32 `ROUTING_BUSY` ·
§35 verworfene `EnrichmentIssue`s · §36 Log-Tab · §37 Übersetzungen (teilweise) ·
§66 servergenerierte Prosa (teilweise) · §67 Ablehnungsgrund eines Sprachpakets ·
§80 Projekt aus dem Nichts · §83 Launcher browser-verifiziert ·
§84 Gruppenadressstil sichtbar ·
§119 `ntfs3`-Fingerprint (Arbeitskopie seit 2026-09-28 auf ext4) ·
§4 byte-genauer Roundtrip (Export zurückgezogen) ·
§5 unsignierte Exporte (Export zurückgezogen) ·
§19 Suchtreffer im zugeklappten Ast (Titel veraltet, Text beschreibt die Lösung) ·
§22 Server-Authentifizierung (ADR-0026) ·
§30 Download ohne Frontend-Aufrufer ·
§33 Routing-Roundtrip auf dem LAN ·
§34 Schema-≥21-Export (zurückgezogen) ·
§49 Doku-Vorschau (CT-2) ·
§50 Doku-Abschnittsauswahl (CT-2) ·
§57 roher `.knxproj`-Vergleich (CT-6) ·
§58 Diff-Exit-Code (CLI) ·
§59 Diff-Vorher/Nachher (CT-1) ·
§81 `new_project_impl` prüft Dirty ·
§89 `Space/@Type` erhalten ·
§91 GA-Stil der Bus-Session ·
§96 verlorene Import-Antwort ·
§103 echtes Dirty-Flag ·
§117 `read_on_init` (T02) ·
§118 Load-Ansage ·
§120 Theme-Kontrast (Gate; Titel veraltet) ·
§122 Einstellungsdiagnosen folgen der UI-Sprache ·
§123 keine gepunktete GA-Notation mehr ·
§131 ehrliche Korpus-Gates ·
§132 Schließen-Knopf mit Ungespeichert-Abfrage ·
§142 Teil-Download `070nh` live ·
§145 Flag-Overrides in der Gruppenobjekttabelle

---

## Nicht in dieser Zählung

- **§105** (Ctrl1-Priorität `SYSTEM` bei den vier verbindungsorientierten
  TL-Frames) ist gemerged und existiert, ist aber absichtlich nicht
  eingestuft: der Eintrag selbst sagt "Unknown on real hardware and
  untested" — Wirkung auf echtem Bus unbekannt, auf dem Simulator (der nicht
  arbitriert) unbeobachtbar. Eine Einstufung wäre hier geraten, nicht
  gelesen. Bleibt offen, bis eine Messung gegen echte Hardware oder ein
  Standard-Zitat zur Priorität-abhängigen Zustellung vorliegt.
