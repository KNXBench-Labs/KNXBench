# Limitierungen nach Kritikalität

Sortierung der 117 Einträge aus [`KNOWN_LIMITATIONS.md`](KNOWN_LIMITATIONS.md)
(`grep -c '^## [0-9]' docs/KNOWN_LIMITATIONS.md`), Stand 2026-09-20. Diese Datei
ordnet nur — sie ersetzt keinen Eintrag und enthält keine neuen Fakten.
Maßgeblich bleibt der Volltext dort. §105 ist absichtlich nicht eingestuft
(siehe unten); die restlichen 116 sind es.

Nummern sind die Abschnittsnummern der Quelldatei. §94 existiert nicht.

## Einstufung

| Stufe | Bedeutung |
| --- | --- |
| **K1 — kritisch** | Kann Daten verfälschen, Geräte falsch beschreiben oder das Projekt Fremden öffnen, ohne dass es auffällt. |
| **K2 — hoch** | Blockiert einen Kern-Workflow oder trägt eine Kompatibilitätsaussage nicht, die jemand erwarten würde. |
| **K3 — mittel** | Echte Lücke, aber enger Wirkungsbereich oder es gibt einen Weg drumherum. |
| **K4 — niedrig** | Kosmetik, Interna, Dokumentation. Kostet niemanden Daten. |
| **Erledigt** | Steht als Historie in der Datei, ist aber geschlossen. |

---

## K1 — kritisch (7)

| § | Thema | Warum K1 |
| --- | --- | --- |
| 92 | Inbetriebnahme nur gegen den eigenen Simulator geprüft | Kein einziger Download hat je ein reales Gerät adressiert. Jede Aussage über Phase 2 ruht auf Code, den dieses Projekt selbst geschrieben hat. |
| 22 | Web-/Docker-Ziel ohne jede Authentifizierung | Kein Login, keine Session, keine Autorisierung. Wer den Port erreicht, hat das Projekt. |
| 61 | DPT-Codec rät das Eingabeformat, mehrere Kodierungen sind Rulings statt Standard | Ein falsch kodierter Wert geht als gültiges Telegramm auf den Bus und sieht dort richtig aus. |
| 99 | Nibble-Reihenfolge im `MemoryControlBlock` ist abgeleitet, nicht spezifiziert | Eine geratene Byte-Anordnung in einem Schreibzugriff auf ein Gerät. |
| 8 | KNX Secure nicht implementiert | Eine gesicherte Installation fällt stillschweigend aus dem Funktionsumfang. Per Ruling zurückgestellt, nicht gelöst. |
| 5 | Exporte sind unsigniert, ETS-Annahme ungetestet | Eine Datei, von der niemand weiß, ob ETS sie zurücknimmt. |
| 1 | Single-Sample-Bias | Alles Gewusste über `.knxproj` stammt aus zwei Installationen. Färbt jede Import-Aussage. |

## K2 — hoch (30)

**Format und Import**

| § | Thema | Warum K2 |
| --- | --- | --- |
| 2 | Kein öffentliches XSD, Import ist tolerant statt validierend | Fehler fallen erst später auf. |
| 4 | Roundtrip semantisch, nicht byte-genau | Die Datei ist nicht die, die hereinkam. |
| 3 | Geräteparameter erhalten, aber nicht interpretiert | Integrität gewahrt, Nutzbarkeit nicht. |
| 11 | `.knxprod` mit Master-Data-Schema ≥ 12 nicht direkt importierbar | Neuere Herstellerdaten bleiben draußen. |
| 13 | AES-verschlüsselte Projekte (ETS6) werden abgelehnt | Ganze Projekte nicht zu öffnen. ZipCrypto (ETS4/5) geht. |
| 34 | Schema-≥21-Export verliert bekannte, nicht gemappte Attribute | Bekannter, benannter Verlust beim Export. |
| 87 | Ein Parser-Fix erreicht bereits eingelesene Zeilen nicht | Nur eine Migration holt sie zurück. |
| 12 | Herstellerdaten-Auflösung: drei Lücken bleiben | Betrifft die Zuordnung Produkt→Programm. |
| 117 | `read_on_init_flag` wird geparst, gespeichert und an der Core-Grenze fallen gelassen | Sechstes Kommunikationsobjekt-Flag existiert im Domänenmodell nicht; Datenintegritätslücke. |

**Inbetriebnahme und Bus**

| § | Thema | Warum K2 |
| --- | --- | --- |
| 7 | Geräte-Download nötig, aber blockiert | Ursache ist Hardware, nicht fehlendes Wissen. |
| 93 | `PID_PROGRAM_VERSION` wird bedingungslos geschrieben | Geparkt, verlagert nach C11. |
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
| 16 | Tauri v2 hängt unter Linux an archivierten GTK3-Bindings | `cargo deny` meldet es; Abhängigkeit ohne Wartung. |

## K3 — mittel (51)

| § | Thema | Warum K3 |
| --- | --- | --- |
| 9 | Projektdateien nicht diffbar (SQLite) |
| 14 | Default-Sprache des Projekts ist ein Platzhalter |
| 15 | Unparsbare Werte überleben nur auf `Override`-Feldern |
| 18 | `open_project` räumt den alten `store_path` nicht weg |
| 19 | Suchtreffer in zugeklapptem Baumast wird nicht aufgedeckt |
| 23 | `/api/project/download` puffert die ganze Datei im Speicher |
| 24 | `FsPicker` ohne Drag-and-Drop-Mehrfachauswahl |
| 29 | `knx bus monitor` formatiert Gruppenadressen immer dreistufig |
| 30 | `/api/project/download` hat keinen Aufrufer im Frontend |
| 33 | Loopback-Roundtrip beweist Korrektheit nicht in jeder Umgebung |
| 39 | CSV-Import adressiert nicht um, löscht nicht, verwaltet keine Bereiche |
| 40 | CSV-Nur-Export-Spalten werden beim Import nie angewandt |
| 41 | CSV aus Excel unter deutschem Gebietsschema kann überraschen |
| 45 | Doku-Export ohne natives PDF |
| 46 | Doku-Export löst Hersteller-, Produkt-, Programmnamen nicht auf |
| 47 | Doku-Export listet Parameterwerte und Modul-Argumente nicht |
| 48 | Doku-Export nur einsprachig |
| 49 | Doku-Export ohne Druckvorschau |
| 50 | Doku-Export ohne Abschnittsauswahl |
| 52 | Diff korreliert Geräte ohne Adresse und ohne `ets_id` nicht |
| 53 | Diff kollidiert bei zwei gleichnamigen Geschwister-Gebäudeteilen |
| 54 | Diff erkennt regenerierte `RefId`s eines Re-Imports nicht |
| 55 | Diff lässt sich nicht zurück auf ein Projekt anwenden |
| 56 | Diff kennt keinen Drei-Wege-Vergleich |
| 57 | Diff kann nicht gegen ein rohes `.knxproj` vergleichen |
| 58 | Diff hat kein CI-taugliches „Exit ≠ 0 bei Unterschied" |
| 59 | Diff zeigt welche Felder sich änderten, meist nicht die Werte |
| 60 | Diff-Webpanel zeigt nur gruppierte Zähler |
| 70 | Schreiben eines deklarierten, aber nicht gezeigten Parameters wird abgelehnt |
| 73 | Line-Scan lernt weder Produkt noch Hersteller noch Seriennummer |
| 75 | Kürzeres `--timeout-ms` möglich, aber nicht Default |
| 76 | Schnellpfad für negatives Layer-2-Confirm bewusst nicht gebaut |
| 77 | Line-Scan bleibt auf einer Linie, überquert keine Koppler |
| 81 | `new_project_impl` prüft „kann rückgängig", nicht „ist verändert" |
| 82 | Stale-Lock des Diagnose-Fensters sieht nur ein Browserprofil |
| 86 | Doppelte Bezeichner in einer Datei: DPT-Provenienz bleibt begrenzt |
| 88 | Anzeigename eines Herstellers ist Last-Writer-Wins (Absicht) |
| 89 | Fünf `Space/@Type`-Werte werden zu `BuildingPart` vergröbert |
| 91 | Laufende Bus-Session behält den Gruppenadressstil vom Start |
| 96 | Browser ohne Import-Antwort kommt nur per Reload ans Projekt |
| 97 | Fortschritt ist meist eine Phasenbezeichnung, keine Prozentzahl |
| 102 | Der Decode-Fehlerzweig des Write-Echos hat keinen bekannten Auslöser |
| 103 | „Ungespeichert" wird aus dem Undo-Stack erschlossen, kein echtes Dirty-Flag |
| 64 | `Languages`-Blöcke außerhalb eines Programms — Ingestion gelöst, Lesen teilweise |
| 104 | Gerät offline mitten in `LoadCompleting` kostet vollen Reconnect pro Poll | Latenz/Bustraffic, laut Eintrag ausdrücklich keine Korrektheitsfrage. |
| 106 | Debug-Report redigiert vier Musterklassen, sonst nichts | Bewusst begrenzt, offengelegt, Zip wird vor Versand angezeigt. |
| 107 | Keine Plugin-API — Dritte können nur forken, nicht nachladen | ADR-0025-Entscheidung; vier andere Erweiterungswege (Sprachpakete, Produktdatenbank, CSV, CLI) bleiben offen. |
| 108 | MP §2.3 widerspricht sich zur belegten `IA_new`; Ausnahmetext gewinnt | Ergebnis wird als Befund gemeldet, nicht stillschweigend erzwungen. |
| 113 | Eskalation lädt nur die Segmente neu, die der Plan tatsächlich trägt | Nur relevant, wenn der Aufrufer den Plan unvollständig baut; Regelfall betroffen es nicht. |
| 116 | `NM_IndividualAddress_Write` wiederholt nicht für den Bediener, liest eine Transport-Layer-Freigabe als MP §2.3 es nicht ausdrücklich sagt | Beide Abweichungen ändern keine Stopp/Weiter-Entscheidung des Standards. |
| 118 | Erfolgreicher Projekt-Load meldet sich Screenreadern nicht | Nur der Erfolgsfall fehlt; `failed` wird bereits angesagt. |

## K4 — niedrig (11)

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

## Erledigt, steht als Historie drin (17)

§10 Lizenz (AGPL) · §17 verwaiste `GroupLink`s · §20 Command-Palette-Overlay (teilweise) ·
§21 Export ohne Gruppenbereich · §25 Docker-Node-22 · §27 Heartbeat-Race ·
§28 Tunnel-Close-Signal · §31 Multicast-Override · §32 `ROUTING_BUSY` ·
§35 verworfene `EnrichmentIssue`s · §36 Log-Tab · §37 Übersetzungen (teilweise) ·
§66 servergenerierte Prosa (teilweise) · §67 Ablehnungsgrund eines Sprachpakets ·
§80 Projekt aus dem Nichts · §83 Launcher browser-verifiziert ·
§84 Gruppenadressstil sichtbar

---

## Nicht in dieser Zählung

- **§105** (Ctrl1-Priorität `SYSTEM` bei den vier verbindungsorientierten
  TL-Frames) ist gemerged und existiert, ist aber absichtlich nicht
  eingestuft: der Eintrag selbst sagt "Unknown on real hardware and
  untested" — Wirkung auf echtem Bus unbekannt, auf dem Simulator (der nicht
  arbitriert) unbeobachtbar. Eine Einstufung wäre hier geraten, nicht
  gelesen. Bleibt offen, bis eine Messung gegen echte Hardware oder ein
  Standard-Zitat zur Priorität-abhängigen Zustellung vorliegt.
