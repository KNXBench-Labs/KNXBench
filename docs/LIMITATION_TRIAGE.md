# Limitierungen nach Kritikalität

Sortierung der **109 verbleibenden nummerierten Grenzen** aus
[`KNOWN_LIMITATIONS.md`](KNOWN_LIMITATIONS.md), gezählt mit
`grep -cE '^## (§)?[0-9]' docs/KNOWN_LIMITATIONS.md` (Stand 2026-10-06, AR15, §158 aus AR17 ergänzt;
vorher 2026-10-01: 110 Überschriften/103 Grenzen).
Die Datei enthält 120 nummerierte Überschriften: 109 Grenzen und elf
gelöste/historische Wegweiser (§18/23/24/42/90/95/130-GATE/149/150/152/156).
108 Grenzen sind eingestuft;
§105 bleibt wegen fehlender Hardwareevidenz ohne Einstufung. Der Befehl oben
zählt Überschriften, nicht automatisch offene Defekte. Geschlossene oder zurückgezogene Einträge sind aus der aktiven
Liste entfernt; frühere Nummern und Fragment-Links werden nicht wiederverwendet.
Der datierte [Implementierungsverlauf](IMPLEMENTATION_STATUS.md) und Git
bewahren die Nachweise.

§94 existiert nicht; §130 ist zweimal vergeben (Gate-Binary und
Anwendungszoom); nur der offene Zoom-Eintrag steht noch in der Tabelle. Die unnummerierten
Produktdaten-, Geräteeditor- und Inbetriebnahmegrenzen sind nicht eingestuft;
ihre stabilen IDs stehen in [ALPHA_READINESS](ALPHA_READINESS.md).
Nur die tatsächlich verbliebene Grenze eines teilweise gelösten Eintrags
wird hier gewichtet, nicht seine historische Überschrift.

## Einstufung

| Stufe | Bedeutung |
| --- | --- |
| **K1 — kritisch** | Kann Daten verfälschen, Geräte falsch beschreiben oder das Projekt Fremden öffnen, ohne dass es auffällt. |
| **K2 — hoch** | Blockiert einen Kern-Workflow oder trägt eine Kompatibilitätsaussage nicht, die jemand erwarten würde. |
| **K3 — mittel** | Echte Lücke, aber enger Wirkungsbereich oder es gibt einen Weg drumherum. |
| **K4 — niedrig** | Kosmetik, Interna, Dokumentation. Kostet niemanden Daten. |

---

## K1 — kritisch (5)

| § | Thema | Warum K1 |
| --- | --- | --- |
| 92 | Inbetriebnahme fast nur gegen den eigenen Simulator geprüft | Real beschrieben wurde bisher ein einziges Gerät (`1.1.67`, Maske `0701h`); jede weitere Aussage über Phase 2 ruht auf Code, den dieses Projekt selbst geschrieben hat. |
| 61 | DPT-Codec: Haupttypen 1–30, explizite Eingabeformate, mehrere Kodierungen sind Rulings statt Standard | Nicht mehr geraten; ein falsches Ruling ginge aber weiterhin als gültiges Telegramm auf den Bus und sähe dort richtig aus. |
| 99 | Nibble-Reihenfolge im `MemoryControlBlock` ist abgeleitet, nicht spezifiziert | Eine geratene Byte-Anordnung in einem Schreibzugriff auf ein Gerät. |
| 8 | KNX Secure nicht implementiert | Eine gesicherte Installation fällt stillschweigend aus dem Funktionsumfang. Per Ruling zurückgestellt, nicht gelöst. |
| 1 | Single-Sample-Bias | Alles Gewusste über `.knxproj` stammt aus zwei Installationen. Färbt jede Import-Aussage. |

## K2 — hoch (30)

**Format und Import**

| § | Thema | Warum K2 |
| --- | --- | --- |
| 2 | Kein öffentliches XSD, Import ist tolerant statt validierend | Fehler fallen erst später auf. |
| 3 | Geräteparameter erhalten, aber nicht interpretiert | Integrität gewahrt, Nutzbarkeit nicht. |
| 11 | Standalone `.knxprod` für Master-Data-Schemata 10–14, 20 und exakt 21/23 | Schema 10 seit ADR-0083 über Vokabular-Evidenz, nicht über eine Spezifikation; 15–19, 22, 24 werden abgewiesen; eingebettete Produktdaten folgen einer anderen Importstrecke. |
| 13 | AES-verschlüsselte Projekte (ETS6) werden abgelehnt | Ganze Projekte nicht zu öffnen. ZipCrypto (ETS4/5) geht. |
| 87 | Ein Parser-Fix erreicht bereits eingelesene Zeilen nicht | Nur eine Migration holt sie zurück. |
| 12 | Herstellerdaten-Auflösung: eine von drei Lücken geschlossen | Betrifft die Zuordnung Produkt→Programm. |

**Inbetriebnahme und Bus**

| § | Thema | Warum K2 |
| --- | --- | --- |
| 7 | Geräte-Download: v1 ist der verifizierte Speicherpfad für Maske `070nh` | Alles andere wird mit Namen abgelehnt (ADR-0048); verifiziert an einem Gerät (`1.1.67`). |
| 142 | Teil-Download für `070nh` | Alle drei Modi liefen nur auf `1.1.67` mit demselben Image; andere Programme und `AppliesTo` sind unbestätigt. Die Web-Bedienung ist geliefert. |
| 145 | Aktive, unverbundene Gruppenobjekte | Die Instanz-Flags werden angewandt; ohne Verbindung bleibt ein Unterschied im Communication-Bit gegenüber ETS. |
| 93 | `PID_PROGRAM_VERSION` wird bedingungslos geschrieben | Geparkt, unbesetzt — C11 und C12 sind gelandet, ohne das anzufassen (korrigiert 2026-09-20). |
| 101 | Der „once more"-Versuch kann die Worst-Case-Wartezeit verdreifachen | Spec-konform, aber teuer. |
| 72 | Ein ungedrosselter Line-Scan kostet echte Buszeit | Zehn Minuten aufwärts mit offenem Tunnel an einer laufenden Anlage. |
| 74 | Belegt-aber-beschäftigt ist nicht von abwesend zu unterscheiden | Der Draht gibt die Unterscheidung nicht her. |
| 78 | Andere KNXnet/IP-Endpunkte erscheinen als belegte Geräte | Scan-Ergebnis enthält Nicht-Geräte. |
| 62 | Group Monitor nur Tunneling, eine Session, Filter clientseitig | Nur der passive Empfangspfad hat Evidenz von einem echten Gateway. |
| 26 | `BusConnection` ohne KNX IP Secure | Folgt aus §8. |
| 79 | Discovery braucht erreichbares Multicast und zugelassene Unicast-Antworten | Host-Firewall behoben (CLI/HTTP erfolgreich); Docker-Bridge bleibt ohne passenden Multicast-Pfad, native WebKitGTK-Suche ungeprüft. |
| 112 | Downloadplan mit `A_Key_Write`-Bedarf wird ganz abgelehnt | Schlüsseländerung während eines Downloads ist damit kein lauffähiger Kern-Workflow. |
| 114 | Download-Counter-Refusal ist eigenes Ruling, nicht System-B-Pflicht | Ein konformes System-B-Gerät ohne Download Counter bekommt nie einen Partial Download — der Kern-Workflow ist für den Regelfall faktisch abgeschaltet. |

**Anwendung und Interoperabilität**

| § | Thema | Warum K2 |
| --- | --- | --- |
| 63 | Kein Mehrbenutzerbetrieb: ein Projekt, ein Undo-Stack, keine Konflikterkennung | Zwei Bearbeiter überschreiben sich. |
| 22 | Ein Passwort oder Loopback für den Server | Schutz vorhanden (ADR-0026), aber keine benutzerspezifischen Rechte; der Ein-Passwort-Betrieb ist die verbleibende Grenze. |
| 6 | Geräte hinter Hersteller-Plug-in-DLLs nicht konfigurierbar | Ganze Produktfamilien außen vor. |
| 38 | Gruppenadress-CSV ohne verifizierte ETS-Interoperabilität | Eigenes Format, ungetestete Annahme. |
| 44 | Doku-Export ohne ETS-Report-Parität | Und Parität ist derzeit nicht messbar. |
| 51 | Projekt-Diff ohne ETS-Vergleichsparität | Dito. |
| 85 | `.signature` wird gespeichert, nie geprüft | Eine Signatur, die niemand liest. |
| 68 | Wiederholte Modul-Instanziierung wird abgelehnt | Bewusst, aber es fehlt Funktionalität. |
| 69 | `Module` ohne `@Id` nicht zuordenbar | Optionales Attribut, das wir brauchen. |
| 71 | Vor Store-Schema 6 importierte Projekte haben keine Modul-Instanz-Ids | Nicht beschreibbar ohne Neuimport. |
| 129 | Veralteter Id-Allokator-Snapshot konnte Ids duplizieren | Datenverlustpfad geschlossen (ADR-0039 Phasen 1–2, 2026-09-27); dass alles über `Command::apply` läuft, sichert weiter nur das Review — Phasen 3–5 offen. |

## K3 — mittel (59)

| § | Thema | Warum K3 |
| --- | --- | --- |
| 9 | Projektdateien nicht diffbar (SQLite) | Git-External-Diff mit `knx diff` geprüft und im Handbuch beschrieben (AR15); ein Textformat gibt es nicht. |
| 14 | Default-Sprache des Projekts ist ein Platzhalter |
| 15 | Unparsbare Werte überleben nur auf `Override`-Feldern |

| 20 | Overlay gemeinsam, native Screenreader-/WebKitGTK-Abnahme bleibt offen | Fokus und Tastatur getestet; keine vollständige native Barrierefreiheitsprüfung. |

| 31 | Routing-Multicast-Override | Bibliothek und CLI können ihn setzen; andere Oberflächen übernehmen ihn nicht automatisch. |
| 36 | Session-Log-Export ist nur ein behaltenes Fenster | Höchstens 1000 Einträge; ein Neustart oder Verdrängung verhindert einen vollständigen Audit-Trail. |
| 37 | Importierte Übersetzungen erreichen nur einen Teil der Oberflächen | Parameter und einige Produktdaten sind übersetzt; andere Quellen/Prosa bleiben originalsprachlich. |
| 39 | CSV kann explizit umadressieren/löschen, verwaltet aber keine Bereiche |
| 40 | CSV-Nur-Export-Spalten werden beim Import nie angewandt |
| 41 | CSV aus Excel unter deutschem Gebietsschema kann überraschen |
| 45 | Doku-Export ohne natives PDF |
| 46 | Doku-Export löst Namen nur mit passenden installierten Produktdaten auf |
| 47 | Parameterwerte/Modul-Argumente erscheinen; unbewiesene Semantik bleibt roh |
| 48 | Doku-Export hat deutsche/englische Grundelemente, keinen vollständigen Prosakatalog |
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
| 66 | Server-generierte Prosa und Berichtdetails nicht vollständig lokalisiert | Parameter-Diagnosen sind übersetzt; weitere Details und Logtexte bleiben sprachneutral oder serverseitig. |
| 104 | Gerät offline mitten in `LoadCompleting` kostet vollen Reconnect pro Poll | Latenz/Bustraffic, laut Eintrag ausdrücklich keine Korrektheitsfrage. |
| 106 | Debug-Report redigiert vier Musterklassen, sonst nichts | Bewusst begrenzt, offengelegt, Zip wird vor Versand angezeigt. |
| 107 | Keine Plugin-API — Dritte können nur forken, nicht nachladen | ADR-0025-Entscheidung; vier andere Erweiterungswege (Sprachpakete, Produktdatenbank, CSV, CLI) bleiben offen. |
| 108 | MP §2.3 widerspricht sich zur belegten `IA_new`; Ausnahmetext gewinnt | Ergebnis wird als Befund gemeldet, nicht stillschweigend erzwungen. |
| 113 | Eskalation lädt nur die Segmente neu, die der Plan tatsächlich trägt | Nur relevant, wenn der Aufrufer den Plan unvollständig baut; Regelfall betroffen es nicht. |
| 116 | K6: Bibliotheks-Primitive weicht eng von MP §2.3 ab; bestätigte CLI/HTTP-Schreibstarts derzeit gesperrt | Der frühere Live-Erfolg ist kein vollständiger Pre-Write-Recovery-Nachweis (ADR-0059); Web zeigt die Sperre vor einer Einwilligung an. |
| 16 | Tauri v2 bleibt unter Linux auf GTK3 | Advisories offline geprüft „ok“ (2026-10-06); die GTK4-Migrationen von Wry/Tauri sind weiter offen. |
| 125 | ETS-6-Objekt-Ids (geräte-lokale Form) aus einem einzigen Projekt belegt | Folgt aus §1; eine zweite unabhängige Probe fehlt. |
| 126 | Line-Scan-Abgleich handelt auf Belegungsevidenz, nicht Geräteidentität | Nur nach ausdrücklicher Auswahl; Schweigen gilt nicht als Abwesenheit. |
| 127 | Site/Property-UI vorhanden, `Ground`-Semantik nur aus Schematext und Tests | Zwei Gebäude unter einer Wurzel im UI getestet; unabhängiger ETS-Export mit `Ground` fehlt weiterhin. |
| 128 | Legacy-`.vd3`–`.vd5`/`.pr3`–`.pr5` werden abgelehnt | Atomar abgelehnt und seit AR06 korrekt benannt; ein Legacy-Import wartet auf Design-Review und Board-Entscheidungen B-1–B-6. |
| 133 | Ein toter Webview lässt sich nicht per Fensterknopf schließen | Folge des §132-Fixes; aus den Quellen gelesen, nicht reproduziert. |
| 134 | Baggage wird inventarisiert, nicht interpretiert | Nichts wird ausgeführt oder entpackt; unbekannte Medien bleiben `unknown`. |
| 135 | Paketidentität wird aufgezeichnet, nicht entschieden | Gespeichert bleibt die zuerst installierte Fassung; Abweichungen werden gezeigt, nicht aufgelöst. |
| 136 | Maske `0701h`: Anwendungsdownload verifiziert, der abschließende Restart bleibt unbestätigt | Daten und Ladezustände rückgelesen; nur der Neustart ist nicht bestätigt. |
| 138 | Geräteschlüssel aus Projekt oder Schlüsseldatei, live nie geprüft | Das Testgerät hat keinen Schlüssel; einen zu setzen wäre ein unnötiger Schreibzugriff. |
| 139 | Seriennummer-Adressschreiben am Testgerät ignoriert und öffentlich pre-tunnel gesperrt | Read-only Suche bleibt; ADR-0057 verlangt vollständige gerätespezifische Wiederherstellung. Bit-2-Debug hat nur ein eigenes Property-Backup. |
| 140 | Adress-Reset historisch live, bestätigter CLI-Lauf derzeit gesperrt | ADR-0058 verlangt ein vollständiges dauerhaftes Backup aller betroffenen Geräte; Neustart unbestätigt, keine HTTP/UI-Route. |
| 141 | Master Reset löscht nur im Simulator | Auf Hardware bleibt die Konfiguration; die Unterstützungsprüfung ist eine eigene Regel. |
| 143 | RF-Domänenadressen nur im Simulator, kein RF-Gerät vorhanden | Auf Hardware gesperrt, keine CLI-/HTTP-Route. |
| 144 | RF-Gerätekonfiguration nur im Simulator | Auf Hardware gesperrt, keine CLI-/HTTP-Route. |
| 146 | Kanallabel sichtbar; Aktivierung kann `Undetermined` und DPT mehrdeutig bleiben | `@Name`/`@Number` sind gespeichert und in der UI gezeigt; fehlende/mehrdeutige Produktdaten werden nicht geraten (ADR-0050/0052). |
| 151 | Große Herstellerpakete nur per CLI-Opt-in | `--allow-large-package` (256 MiB/4 GiB, ADR-0082); Web/HTTP bleiben bei 64/256 MiB und weisen ab. |
| 153 | Schema-10-Semantik aus Namensgleichheit mit Schema 11 abgeleitet | Kein Schema-10-XSD gefunden; Zulassung über Korpus-Vokabular (ADR-0083). |
| 154 | Telegrammfluss-Ansicht nur in Chromium geprüft; Bewegung bei großen Karten teuer | Motion Off ist für mehrere hundert Knoten der unterstützte Modus; kein WebKitGTK-/Orca-/Echtbus-Nachweis. |
| 155 | Tunnel aus einem Docker-Bridge-Container nur mit Route-Back-fähigem Gateway | Ein Gateway gemessen; kein Live-Tunnel aus der Bridge; Discovery bleibt Host-Netz. |
| 157 | Öffnen eines älteren Projekts aktualisiert die Datei an Ort und Stelle | Seit 2026-10-06 atomar; keine Kopie, ältere KNXBench-Versionen lehnen die Datei danach ab. |
| 158 | Das AppImage startet nur mit X-Server | Ohne funktionierendes Xwayland bricht es sofort ab; nativer Wayland-Start aus dem entpackten AppImage ist dokumentiert und auf einer Maschine gemessen. |

## K4 — niedrig (14)

| § | Thema | Warum K4 |
| --- | --- | --- |

| 43 | Animationsschalter und OS-Präferenz existieren; nicht jede Fläche ist abgedeckt |
| 29 | `knx bus route-send` adressiert immer dreistufig | Monitor/Write folgen seit AR14 dem Stil des `--project`; nur `route-send` hat keine Projektoption. |
| 65 | `--version` nennt in Entwicklungs-Builds einen Commit, keinen Arbeitsstand | Release-Builds mit `KNX_REQUIRE_CLEAN_TREE=1` verweigern einen veränderten Baum (AR13). |

| 98 | Die zweite Flavour-Zeile sieht fast niemand |
| 100 | Hilfetexte liegen im Message-Katalog, ein Absatz pro Schlüssel |
| 109 | Zwei Downloadteile derselben `PartKind` ohne definierte Reihenfolge werden beide abgelehnt | Kein einziger Fall bislang beobachtet, Ablehnung statt Rateversuch. |
| 110 | `PID_GROUP_RESPONSER_TABLE` bleibt auf jedem Medium unimplementiert | Bewusst — PL110-only laut RES, dieses Projekt zielt auf TP1/RF/IP. |
| 111 | CP §3.5.4 Schritt 07 (Individualadresse entladen) bleibt unimplementiert | Bewusste Weigerung: das Werkzeug soll das eigene Zielgerät nicht unadressierbar machen. |
| 115 | `MasterResetResponse::recovery_wait`/`SessionTiming::restart_basic_t1` berechnen Wartezeiten, die niemand abwartet | Toter Code ohne heutigen Aufrufer. |
| 60 | Diff-Webpanel: lange Tabellen virtualisiert, mit Suche/Filter | Paging ersetzt (AR11); offen sind Suche über Tabellen hinweg, Sprung in den Explorer und eine Screenreader-Prüfung. |
| 121 | Zwei offene Fenster sehen Einstellungsänderungen erst nach Reload | Kein Push-Kanal; ein Reload genügt. |
| 124 | Schnittstellensuche zeigt vier Fakten, das Protokoll trägt mehr | `knx-net` dekodiert alles, die Oberfläche zeigt einen Ausschnitt. |
| 130 | Anwendungszoom browser-, nicht WebKitGTK-verifiziert | Nur in Chromium geprüft. (Zweiter Eintrag mit Nummer 130.) |
| 137 | Bus-Monitor-JSON ist ein behaltenes Fenster, keine vollständige Aufzeichnung | Verluste werden gezählt und exportiert, nicht wiederhergestellt. |

## Nicht in dieser Zählung

- **§149/150/152/156** sind gelöst (2026-10-03 bis 2026-10-05): CLI-Endung
  ohne Groß-/Kleinschreibung, verschachtelte `ModuleDef`s, Evidenzbudget,
  gemeldete `Parameter`/`ParameterRef`-Attribute; Nachweise im jeweiligen
  Eintrag und im [Ledger](status/LEDGER.md).
- **§42** ist durch AR04 gelöst: dokumentierter vollständiger transaktionaler
  Speicher-Fallback statt erfolgreicher No-Op-Arme; Offline-Regressionen prüfen
  Wiederöffnen, Undo/Redo, Reihenfolge, SQL-Fehler und unveränderte opaque/Manifest-
  Daten. Kein inkrementeller Performance- oder neuer UI-Vertrag; siehe
  [Speichervertrag](STORAGE_COMMAND_CONTRACT.md).
- **§18/23/24** sind gegen aktuelle Implementierung und vorhandene
  Regressionen abgeglichen (AR00): ETS-Import veröffentlicht `store_path=None`,
  Download streamt begrenzte Blöcke und der Dateipicker unterstützt
  sequenzielle Mehrfach-Uploads und Datei-Drop. Temporäre SQLite-Serialisierung
  und die einzelne Projektwahl sind keine wiedereröffneten Defekte.
- **§90/95** sind historische Klarstellungen, keine Kosten: 46 war eine
  Anzahl, kein DPT-Haupttyp; §95 verweist auf sechs dokumentierte Text-Rulings.
  Diese fünf Überschriften/Fragment-Links bleiben erhalten.
- **§130-GATE** ist durch AR01 gelöst: Laufzeit-Root statt Build-Pfad,
  explizite Workspace- und Nichtleer-Prüfung, ausgewiesener Scanumfang,
  negative/positive Fixtures und verhaltenswirksame Guard-Mutationen.
  Der gleich nummerierte Zoom-Eintrag bleibt offen; siehe
  [Prüfzielvertrag](VERIFICATION.md).

- **§105** (Ctrl1-Priorität `SYSTEM` bei den vier verbindungsorientierten
  TL-Frames) ist gemerged und existiert, ist aber absichtlich nicht
  eingestuft: der Eintrag selbst sagt "Unknown on real hardware and
  untested" — Wirkung auf echtem Bus unbekannt, auf dem Simulator (der nicht
  arbitriert) unbeobachtbar. Eine Einstufung wäre hier geraten, nicht
  gelesen. Bleibt offen, bis eine Messung gegen echte Hardware oder ein
  Standard-Zitat zur Priorität-abhängigen Zustellung vorliegt.
