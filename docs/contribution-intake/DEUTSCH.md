# KNXBench verbessern – Schritt für Schritt

[English guide](README.md) · **Deutsch**

Ein Projekt oder Produkt funktioniert nicht so, wie du es erwartest? Melde es
uns. Du brauchst **kein Git, kein Terminal und keine XML- oder
KNX-Schema-Kenntnisse**. Eine kurze Beschreibung hilft bereits. Eine von KNXBench
erstellte Berichts-ZIP erleichtert die Untersuchung.

> **Vor dem Teilen:** Dieses Repository und seine Anhänge sind öffentlich.
> Keine originalen `.knxproj`-/`.knxprod`-Dateien, Kundendaten, Passwörter,
> Schlüsseldateien, Sicherheitsschlüssel oder vertraulichen Herstellerdateien
> anhängen. Starte mit dem **reduzierten Bericht**, nicht mit zusätzlichen
> XML-Dateien. Reduziert bedeutet nicht garantiert anonym: Prüfe den Inhalt.

## Der kurze Weg

**Datei auswählen → Analysieren → Vorschau prüfen → Freigeben → ZIP herunterladen
→ Im GitHub-Formular anhängen → Absenden.**

In der App erklärt ein Hinweis immer den nächsten Schritt. Im Analysedialog
findest du zusätzlich **So hilfst du mit — Schritt für Schritt**. Die App reicht
nichts automatisch ein.

[Beitragsformular öffnen](https://github.com/KNXBench-Labs/KNXBench/issues/new?template=analysis.yml)

**Keine ZIP oder kein Analyse-Menü?** Melde das Problem trotzdem im Formular.
Beschreibe, was du versucht hast und was passiert ist. Unbekannte optionale
Felder bleiben leer. Nicht stattdessen das Original anhängen. Du musst KNXBench
nicht selbst kompilieren, nur um einen Fehler zu melden.

## Was du brauchst

- Zum Absenden eines Issues brauchst du ein GitHub-Konto und musst angemeldet
  sein. Git oder eine Entwicklerumgebung brauchst du nicht.
- Für die Analyse brauchst du einen KNXBench-Build mit
  **Datei → Unterstützungslücken analysieren…**. In der englischen Oberfläche:
  **File → Analyze support gaps…**.
- Eine vorhandene `.knxproj`-Projektdatei oder ein `.knxprod`-Produktpaket,
  dessen Analyse erlaubt ist. Die Datei darf höchstens **32 MiB** groß sein.
  Sie bleibt gepackt; nichts vorher entpacken.
- Nutze für öffentliche Beiträge eigenes freigegebenes Material, keine
  vertrauliche Kundeninstallation. Ein Produkt nutzen zu dürfen bedeutet nicht
  automatisch, seine Dateien öffentlich weitergeben zu dürfen.

## 1. Datei auswählen

Öffne **Datei → Unterstützungslücken analysieren…** und wähle die Datei aus.

**Was passiert?** Noch nichts wird hochgeladen. Du kannst den Dialog weiterhin
schließen. Andere Dateiendungen, leere oder zu große Dateien werden hier nicht
analysiert. Nicht umbenennen, um die Prüfung zu umgehen.

## 2. Analyse starten

Klicke auf **Auf diese Instanz hochladen & analysieren** und warte auf die Ergebnisse.

**Wohin geht meine Datei?** An die KNXBench-Instanz, die du gerade benutzt.
Diese kann auch auf einem anderen Rechner laufen. Die Datei geht nicht an
GitHub. Die Analyse verändert weder dein geöffnetes Projekt noch deine
Produktdatenbank und stellt keine Verbindung zur KNX-Hardware her.

**Was bedeuten die Ergebnisse?**

- **Geprüft:** Diese Prüfung lief. Das heißt nicht, dass alles unterstützt wird.
- **Abgelehnt:** Der Importer nahm das Format nicht an. Auch das ist ein
  hilfreicher Befund – kein Fehler, den du vor dem Melden selbst beheben musst.
- **Teilweise:** Ein Teil der Arbeit konnte nicht abgeschlossen werden,
  beispielsweise wegen einer Analysegrenze.
- **Nicht verfügbar / Nicht geprüft:** Für diese Prüfung gibt es kein Ergebnis.
  Das bedeutet **nicht null Probleme**.
- **Analyse abgeschlossen:** Der angegebene Analyseumfang wurde bearbeitet,
  nicht vollständige ETS-Kompatibilität bestätigt.

Du musst weder XML noch Befund-IDs oder technische Zähler verstehen.
Offline-Planung beweist nicht, dass ein echtes Gerät funktioniert.

### Zusätzliche Offline-Verfahrensbeobachtungen

Lokale Source-Builds können **Offline-Verfahrensfolgen** für die erste moderne
Produktfamilie anzeigen (`MV-07B0`, AP1). **Nur Schrittfolge aufgelöst** bedeutet,
dass die deklarierte Reihenfolge rekonstruiert wurde — kein vollständiger
Download-Plan und kein Nachweis für ein funktionierendes Gerät. Unbekannte
Schritte und fehlende oder widersprüchliche Referenzen bleiben sichtbar.
Andere Masken/Varianten und vollständige Gerätesemantik gehören nicht dazu.

Du kannst solche Beobachtungen über dieselbe Vorschau, ZIP und das Issue melden.
Die lokalen Details enthalten Kennungen, Quellprüfsummen und Originalwerte:
Kopiere sie nicht ungeprüft in ein öffentliches Issue. Die reduzierte ZIP lässt
diese Details weg; wertfreie Befundcode-Zähler bleiben enthalten. Zusätzliche
XML erfordert bewusste Auswahl, Prüfung und Erlaubnis. Nichts wird automatisch
versendet.

## 3. Mit dem reduzierten Bericht beginnen

Lass **Öffentliches GitHub-Issue** ausgewählt. Lass **alle optionalen
Kontext-Samples abgewählt**. So erhältst du einen reduzierten Ergebnisbericht
mit Strukturbeobachtungen, ohne Originaldatei und ohne Quellwert-Samples.

**Warum nicht alle Samples hinzufügen?** Das sind vollständige, unveränderte
XML-Dateien. Sie können Namen, Adressen, Gerätekennungen und proprietäre Daten
enthalten und sind **nicht anonymisiert**. Starte ohne sie. Bei Bedarf fragt ein
Maintainer nach mehr Kontext. Zusätzliche Dateien nur auswählen, wenn sie nötig
sind, du sie geprüft hast und ihre Veröffentlichung erlaubt ist.

## 4. Vorschau prüfen und freigeben

Klicke auf **Ausgewählte Evidenz in Vorschau prüfen**. Sieh dir die tatsächlich
ausgehenden Dateien an:

| Datei | Einfach erklärt |
| --- | --- |
| `findings.json` | Analyseergebnisse; private Quellwerte und Detailangaben werden reduziert. |
| `manifest.json` | Dateiliste, Größen und Prüfsummen. Du musst nichts selbst berechnen. |
| `README.md` | Hinweise für die spätere Prüfung durch einen Maintainer. |
| `samples/member-N.xml`, falls gewählt | Unveränderte Quell-XML: besonders sorgfältig prüfen. |

Prüfe, ob du die Inhalte veröffentlichen darfst. Setze erst danach das
Freigabehäkchen. Siehst du vertrauliche Informationen, **teile sie nicht**.
Die Reduktion ist kein vollständiger Geheimnisfilter. Kein Original in ein
öffentliches Issue legen.

**Herunterladen gesperrt?** Du brauchst eine abgeschlossene Vorschau und das
Freigabehäkchen. Änderungen an Datei, Samples oder Beitragsweg löschen diese
Freigabe; Vorschau und Bestätigung müssen dann erneut erfolgen.

## 5. ZIP herunterladen

Klicke auf **Evidenz-ZIP herunterladen**. Prüfe die Downloads deines Browsers
und speichere **`knxbench-evidence.zip`** an einem Ort, an dem du sie wiederfindest.
Die ZIP bleibt gepackt.

**Was passiert?** Die App fordert einen Download an. Sie weiß nicht, ob du die
Datei wirklich gespeichert hast, und lädt sie nicht zu GitHub hoch. Öffentliche
ZIPs sind auf 24.000.000 Byte begrenzt – unter GitHubs dokumentierter Grenze von
25 MB für andere Anhänge. [1]

## 6. Auf GitHub anhängen und absenden

1. Klicke auf **Öffentliches Beitragsformular öffnen** oder benutze den Link oben.
2. Melde dich bei GitHub an, falls nötig. Ein normales Konto reicht.
3. Vergib einen kurzen Titel, zum Beispiel **„Produktanalyse wird abgelehnt“**.
4. Schreibe in **What did you do, and what happened?**, was du getan hast und was
   passiert ist. Deutsch ist willkommen. Beispiel: „Ich habe ein Produktpaket
   gewählt und Analysieren angeklickt. Der Import wird abgelehnt. Ich hätte
   erwartet, dass das Produkt verfügbar ist. Den Grund kenne ich nicht.“
   Das ist eine **Beispielbeschreibung**, kein gemessenes KNXBench-Ergebnis.
5. Ziehe die gespeicherte ZIP in **Report ZIP (optional)** oder nutze die
   Anhangsauswahl. **Warte, bis der Anhangslink erscheint.** [1], [3]
6. Lass andere optionale Felder leer, wenn du die Antwort nicht kennst. Der
   App-Link übergibt nach einer Analyse die Analyzer-Version an das Formular;
   GitHub unterstützt solche Feld-ID-Vorbelegungen. Prüfe den Wert, falls er
   angezeigt wird. [2], [3]
7. Bestätige die beiden Veröffentlichungshinweise und klicke auf **Create issue**. [2]

> **Wichtig:** Bereits beim Anhängen beginnt der Upload, noch vor dem Absenden
> des Issues. Öffentliche Anhänge können ohne Anmeldung geöffnet werden.
> Erlaubnis und Datenschutz deshalb **vor dem Anhängen** prüfen. [1]

**Woran erkenne ich, dass es geklappt hat?** GitHub zeigt dein erstelltes Issue
mit einer Issue-Nummer. Bewahre den Seitenlink auf; dort erscheinen Rückfragen.
Ein offenes Formular, ein Download oder ein Anhangslink allein bedeutet noch
nicht, dass das Issue abgesendet wurde. Ein erstelltes Issue verspricht weder
eine Lösung noch Kompatibilität oder eine bestimmte Antwortzeit.

## Wenn etwas nicht klappt

- **Analyse-Menü fehlt:** Melde das Problem ohne ZIP. Gib den installierten
  Build oder die Version an, wenn du sie kennst; sonst bleibt die Version leer.
- **Analyse abgelehnt oder teilweise:** Teile den reduzierten Bericht, falls ein
  Export möglich ist. Nicht Schema-Nummern, XML oder Dateiendungen ändern, damit
  das Ergebnis erfolgreich aussieht.
- **Kein Export möglich:** Beschreibe den Fehler in eigenen Worten. Keine
  Originalarchive, Passwörter, Kundendaten-Screenshots oder vertraulichen Logs posten.
- **Browser blockiert den Download:** Prüfe seinen Downloadhinweis und versuche
  es nach Kontrolle derselben Vorschau erneut. Nicht einfach annehmen, die ZIP sei gespeichert.
- **ZIP zu groß:** Entferne optionale XML-Samples und prüfe die Vorschau erneut.
  Nicht stattdessen das Original aufteilen oder veröffentlichen.
- **GitHub nimmt den Anhang nicht an:** Anmeldung prüfen, Upload abwarten und
  den reduzierten Bericht erneut versuchen. Eine Beschreibung ohne ZIP ist erlaubt.
- **Privates Original gesperrt:** Es gibt bekannte Schlüsselmerkmale oder
  ungeprüfte Inhalte. Diese Sperre lässt sich hier nicht übergehen. Die Datei
  nicht manuell öffentlich hochladen.

## Vertrauliche Dateien und E-Mail

**Der private Posteingang ist noch nicht bestätigt.** `contribute@knxbench.com`
ist vorgesehen, aber kein verifiziert funktionierendes Postfach. Frage zuerst
nach einem funktionierenden privaten Kontaktweg – **ohne vertrauliche Dateien
mitzusenden**. Nicht von einer erfolgreichen Zustellung ausgehen.

Erst nach Bestätigung des Wegs und der Umgangsbedingungen: **Privater Kontakt /
E-Mail-Entwurf** auswählen, Inhalte prüfen und gegebenenfalls das Original
zusätzlich freigeben. Originale können trotzdem gesperrt bleiben. Ein Entwurf
hängt keine ZIP an und sendet sie nicht. Datei selbst anhängen, Empfänger prüfen
und selbst absenden. Keine Garantie für Zustellung, Verschlüsselung, Aufbewahrung
oder Löschung. Keine Passwörter, Schlüsseldateien oder Schlüssel versenden.
Originale gehören nicht in öffentliche Issues oder deren Versionsgeschichte.

## Was passiert nach dem Beitrag?

Ein Maintainer prüft den Bericht, fragt bei Bedarf nach freigegebenem Kontext,
vergleicht echte Projekt-, Produkt- und Spezifikationsinformationen und schreibt
vor einer Korrektur einen Regressionstest. Samples laufen nicht als Plugins.
Eine neue Schema-Nummer allein bedeutet keine Unterstützung. Rückfragen und
Fortschritte erscheinen im Issue; automatische Korrekturen oder Antwortfristen
werden nicht versprochen. Öffentliche Testdateien brauchen eine separate
Erlaubnisprüfung. Öffentliche Beiträge können kopiert oder indexiert werden;
Löschen garantiert nicht, dass alle Kopien verschwinden. Sicherheitslücken sind
keine gewöhnlichen Sample-Beiträge: Geheimnisse oder vertrauliche Exploit-Originale
hier nicht veröffentlichen.

## Quellen zu den GitHub-Schritten

[1] https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/attaching-files

[2] https://docs.github.com/en/issues/tracking-your-work-with-issues/creating-an-issue

[3] https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-githubs-form-schema
