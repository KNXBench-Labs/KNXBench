# WP-34 — One term for writing configuration to a device (#41)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **implemented 11 November 2026.** 130 German catalogue values now say
*programmieren*/*Programmierung*; 19 remaining *Download* values are browser file
downloads, the ETS *Download-Plug-in* name and one parenthetical English term.
The rule lives in [GLOSSARY](../GLOSSARY.md#german-terminology-for-device-writes).
English copy is unchanged.

## Owner decision (11 November 2026)

Follow established KNX/ETS usage: German **“Programmieren”** for writing to
the device (ETS: “Applikationsprogramm programmieren”, “Physikalische Adresse
programmieren”), English **“Download”**. “Parametrieren” only for editing
parameters in the project, never for the bus operation.

## Work

Audit `messages/de.ts`, `messages/en.ts` and `docs/manual/`; unify labels,
headings and running text; keep established technical identifiers. On
10 November `main` used “Download” 113 times, “Programmieren/programmieren”
13 times and “In Gerät laden” twice in the German catalogue.
