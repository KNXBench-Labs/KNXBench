/** Pins the report wizard's pure rules: O1 defaults, requests, sizes, errors, mail draft. */
import { describe, expect, it } from "vitest";
import { PROBLEM_TYPES, SECTION_KINDS, type SectionKind } from "./diagnosticBundleApi";
import {
  buildQuery, boundedClientFacts, bytesByKind, defaultOffer, defaultSelection, formatBytes, initialChoice, mailDraftFor, markerKeys,
  noteKey, reasonKey, recentErrors, RECENT_ERROR_LIMIT, toggleEarlier, toggleSection, withProblemType,
} from "./reportProblem";
import { messages as de } from "./messages/de";
import { messages as en } from "./messages/en";
import type { ToastNotification } from "./toast";

// Owner table O1 of docs/open-work/DIAG-03, transcribed independently of the module:
// F fixed, P preselected, O offered, - not offered; columns programming, reading,
// import, catalogue, UI/other.
const O1_TABLE: Record<SectionKind, string> = {
  environment: "FFFFF",
  sessionLog: "PPPPP",
  operation: "PP--O",
  earlierOperation: "OO--O",
  device: "PP-OO",
  legacyReports: "PP---",
  importReport: "--P-O",
  importFile: "--O--",
  catalogue: "OO-P-",
  project: "OOO-O",
  busMonitor: "OO--O",
  clientState: "----P",
};
const LETTER = { fixed: "F", preselected: "P", offered: "O", notOffered: "-" } as const;

describe("owner table O1", () => {
  it("is mirrored cell by cell", () => {
    expect(PROBLEM_TYPES).toEqual(["deviceProgramming", "deviceReading", "projectImport", "catalogue", "userInterface"]);
    for (const kind of SECTION_KINDS) {
      expect(PROBLEM_TYPES.map((type) => LETTER[defaultOffer(type, kind)]).join(""), kind).toBe(O1_TABLE[kind]);
    }
  });

  it("preselects exactly the fixed and ticked rows per problem type", () => {
    expect(defaultSelection("deviceProgramming")).toEqual(["environment", "sessionLog", "operation", "device", "legacyReports"]);
    expect(defaultSelection("deviceReading")).toEqual(["environment", "sessionLog", "operation", "device", "legacyReports"]);
    expect(defaultSelection("projectImport")).toEqual(["environment", "sessionLog", "importReport"]);
    expect(defaultSelection("catalogue")).toEqual(["environment", "sessionLog", "catalogue"]);
    expect(defaultSelection("userInterface")).toEqual(["environment", "sessionLog", "clientState"]);
  });
});

describe("wizard choice", () => {
  it("starts with a shortcut's type, device and operation, else user interface", () => {
    expect(initialChoice()).toMatchObject({ problemType: "userInterface", operation: null, deviceId: null, earlier: [] });
    const choice = initialChoice({ problemType: "deviceProgramming", deviceId: 4, operation: { relatedKey: "downloadId", relatedValue: "7" } });
    expect([...choice.sections]).toEqual(defaultSelection("deviceProgramming"));
    expect(choice).toMatchObject({ deviceId: 4, operation: { relatedKey: "downloadId", relatedValue: "7" } });
  });

  it("takes the new type's defaults on a type change and keeps operation and device", () => {
    const start = toggleSection(initialChoice({ problemType: "deviceReading", deviceId: 4, operation: { recordingId: "r" } }), "project", true);
    const changed = withProblemType(toggleEarlier(start, "older", true), "projectImport");
    expect([...changed.sections]).toEqual(defaultSelection("projectImport"));
    expect(changed).toMatchObject({ deviceId: 4, operation: { recordingId: "r" }, earlier: [] });
    expect(withProblemType(start, "deviceReading")).toBe(start);
  });

  it("never unticks the fixed row or ticks an unoffered one", () => {
    const choice = initialChoice({ problemType: "catalogue" });
    expect(toggleSection(choice, "environment", false)).toBe(choice);
    expect(toggleSection(choice, "operation", true)).toBe(choice);
    expect([...toggleSection(choice, "device", true).sections]).toContain("device");
    expect([...toggleSection(choice, "sessionLog", false).sections]).not.toContain("sessionLog");
  });

  it("toggles earlier recordings without duplicates", () => {
    let choice = toggleEarlier(initialChoice({ problemType: "deviceReading" }), "a", true);
    choice = toggleEarlier(toggleEarlier(choice, "b", true), "a", true);
    expect(choice.earlier).toEqual(["b", "a"]);
    expect(toggleEarlier(choice, "b", false).earlier).toEqual(["a"]);
  });
});

describe("buildQuery", () => {
  const client = { uiLanguage: "de" };

  it("sends the defaults explicitly, with the shortcut's operation and device", () => {
    const query = buildQuery(initialChoice({ problemType: "deviceProgramming", deviceId: 4, operation: { relatedKey: "operationId", relatedValue: "x" } }), client);
    expect(query).toEqual({
      problemType: "deviceProgramming",
      sections: ["environment", "sessionLog", "operation", "device", "legacyReports"],
      operation: { relatedKey: "operationId", relatedValue: "x" },
      deviceId: 4,
    });
  });

  it("counts earlier operations exactly when some are picked", () => {
    const choice = initialChoice({ problemType: "deviceReading" });
    expect(buildQuery(choice, null).sections).not.toContain("earlierOperation");
    const picked = buildQuery(toggleEarlier(choice, "r-1", true), null);
    expect(picked.sections).toContain("earlierOperation");
    expect(picked.earlierRecordingIds).toEqual(["r-1"]);
  });

  it("sends no operation for types that do not offer one, and the client only for user interface", () => {
    const imported = buildQuery(initialChoice({ problemType: "projectImport", operation: { recordingId: "r" }, deviceId: 2 }), client);
    expect(imported).toEqual({ problemType: "projectImport", sections: ["environment", "sessionLog", "importReport"], deviceId: 2 });
    const ui = buildQuery(initialChoice({ problemType: "userInterface" }), client);
    expect(ui.client).toBe(client);
    expect(ui.sections).toEqual(["environment", "sessionLog", "clientState"]);
    expect(buildQuery(initialChoice({ problemType: "userInterface" }), null).client).toBeUndefined();
  });

  it("omits client data when its checkbox is unchecked and permits reselecting", () => {
    const choice = initialChoice({ problemType: "userInterface" });
    expect(buildQuery(toggleSection(choice, "clientState", false), client).client).toBeUndefined();
    expect(buildQuery(toggleSection(choice, "clientState", true), client).client).toBe(client);
  });

  it("never sends a section the type does not offer", () => {
    for (const type of PROBLEM_TYPES) {
      let choice = initialChoice({ problemType: type });
      for (const kind of SECTION_KINDS) choice = toggleSection(choice, kind, true);
      for (const kind of buildQuery(choice, client).sections!) expect(defaultOffer(type, kind)).not.toBe("notOffered");
    }
  });
});

describe("sizes and markers", () => {
  it("formats binary units in the UI language", () => {
    expect(formatBytes(0, "en")).toBe("0 B");
    expect(formatBytes(1023, "en")).toBe("1,023 B");
    expect(formatBytes(1536, "en")).toBe("1.5 KiB");
    expect(formatBytes(10 * 1024 * 1024 + 1, "en")).toBe("10 MiB");
    expect(formatBytes(1536, "de")).toBe("1,5 KiB");
    expect(formatBytes(3 * 1024 ** 3, "en")).toBe("3 GiB");
  });

  it("adds several sections of one kind", () => {
    const section = (kind: SectionKind, bytes: number) => ({ key: kind, kind, label: null, bytes, entries: 0,
      markers: { addresses: false, names: false, parameterValues: false, networkAddresses: false }, notes: [], files: [] });
    const sizes = bytesByKind([section("earlierOperation", 10), section("earlierOperation", 5), section("device", 3)]);
    expect([...sizes]).toEqual([["earlierOperation", 15], ["device", 3]]);
  });

  it("lists content markers in a fixed order", () => {
    expect(markerKeys({ addresses: true, names: false, parameterValues: true, networkAddresses: true })).toEqual([
      "reportProblem.marker.addresses", "reportProblem.marker.parameterValues", "reportProblem.marker.networkAddresses",
    ]);
    expect(markerKeys({ addresses: false, names: false, parameterValues: false, networkAddresses: false })).toEqual([]);
  });

  it("translates only the server's known reasons", () => {
    expect(reasonKey("no device is chosen")).toBe("reportProblem.reason.noDevice");
    expect(reasonKey("this section is withheld until its content is admitted as free of credentials")).toBe("reportProblem.reason.withheld");
    expect(reasonKey("a secret-free copy of the project could not be made: disk")).toBeNull();
  });

  it("translates the withheld, no-device and keep-private texts into German", () => {
    const withheld = "this section is withheld until its content is admitted as free of credentials";
    const noDevice = "the selected operation identifies no single device of this project";
    const privateRaw = "contains device configuration, possibly passwords; recognised KNX keys are redacted; keep private";
    const keys = [reasonKey(withheld), reasonKey(noDevice), reasonKey("the selected diagnostic source is unavailable"), noteKey(privateRaw)];
    expect(keys).toEqual(["reportProblem.reason.withheld", "reportProblem.reason.sourceNamesNoDevice",
      "reportProblem.reason.source", "reportProblem.note.privateRaw"]);
    for (const key of keys) {
      expect(en[key!]).toBeTruthy();
      expect(de[key!]).toBeTruthy();
      expect(de[key!]).not.toBe(en[key!]);
    }
    expect(de["reportProblem.note.privateRaw"]).toMatch(/Passwörter/);
    expect(noteKey("removed BCU key at device 1.1.1")).toBeNull();
  });
});

describe("client state and mail draft", () => {
  const note = (id: number, severity: "error" | "info"): ToastNotification => ({ id, entry: {
    timestamp: `2026-10-29T10:00:${String(id % 60).padStart(2, "0")}Z`, severity, source: `ui:toast:${severity === "error" ? "error" : "fun"}`,
    message: `message ${id}`, detail: id % 2 ? null : `detail ${id}`, location: null,
  } });

  it("keeps the last error notifications only, bounded", () => {
    const history = Array.from({ length: 120 }, (_, i) => note(i, i % 3 === 0 ? "info" : "error"));
    const errors = recentErrors(history);
    expect(errors).toHaveLength(RECENT_ERROR_LIMIT);
    expect(errors.at(-1)).toEqual({ timestamp: history[119].entry.timestamp, source: "ui:toast:error", message: "message 119", detail: null });
    expect(errors.every((e) => e.source === "ui:toast:error")).toBe(true);
    expect(recentErrors([note(1, "info")])).toEqual([]);
  });

  it("never emits lone UTF16 surrogates when bounding emoji or malformed error text", () => {
    const facts = { appVersion: "0.1", shell: "browser" as const, uiLanguage: "en", theme: "x".repeat(127) + "🙂", viewport: { width: 400, height: 900 }, uiScale: 1 };
    const history: ToastNotification[] = [{ id: 1, entry: { timestamp: "2026-10-29T10:00:00Z", severity: "error", source: "ui:toast:error", message: "e" + "🙂".repeat(2000), detail: "bad\ud800text", location: null } }];
    const result = boundedClientFacts(facts, history);
    // Unicode-mode matching ignores genuine surrogate pairs but catches unpaired units,
    // which JSON.stringify escapes and the server's UTF8 JSON parser cannot admit.
    for (const value of [result.theme, result.recentErrors[0].message, result.recentErrors[0].detail!]) expect(value).not.toMatch(/[\uD800-\uDFFF]/u);
    expect(result).toMatchObject({ truncated: true, metadataTruncated: true, truncatedErrorCount: 1 });
    expect(result.recentErrors[0].message).toMatch(/^e🙂/u);
    expect(new TextEncoder().encode(JSON.stringify(result)).byteLength).toBeLessThanOrEqual(65536);
  });

  it("discloses shortened metadata even when there are no recent errors", () => {
    const payload = boundedClientFacts({ appVersion: "v", shell: "browser", uiLanguage: "en", theme: "theme".repeat(1000), viewport: { width: 400, height: 900 }, uiScale: 1 }, []);
    expect(payload).toMatchObject({ truncated: true, metadataTruncated: true, omittedErrorCount: 0 });
    expect(new TextEncoder().encode(JSON.stringify(payload)).byteLength).toBeLessThanOrEqual(65536);
  });

  it("admits exactly 65536 serialized UTF8 bytes including escaping and omits the next byte", () => {
    const facts = { appVersion: "0.1", shell: "browser" as const, uiLanguage: "de", theme: "porcelain", viewport: { width: 400, height: 900 }, uiScale: 1 };
    const errors = Array.from({ length: 31 }, (_, i) => ({ timestamp: "2026-10-29T10:00:00Z", source: "ui:toast:error", message: i === 0 ? "" : "🙂\n\"" + "x".repeat(2041), detail: null }));
    const expected = { ...facts, recentErrors: errors, truncated: false, metadataTruncated: false, omittedErrorCount: 0, truncatedErrorCount: 0 };
    const bytes = (value: unknown) => new TextEncoder().encode(JSON.stringify(value)).byteLength;
    const fill = 65536 - bytes(expected);
    expect(fill).toBeGreaterThan(0); expect(fill).toBeLessThanOrEqual(2048);
    errors[0].message = "x".repeat(fill);
    const history = () => errors.map((entry, id): ToastNotification => ({ id, entry: { ...entry, severity: "error", location: null } }));
    expect(bytes(expected)).toBe(65536);
    expect(boundedClientFacts(facts, history())).toEqual(expected);
    errors[0].message += "x";
    expect(bytes(expected)).toBe(65537);
    const bounded = boundedClientFacts(facts, history());
    expect(bounded.omittedErrorCount).toBe(1); expect(bounded.recentErrors).toEqual(errors.slice(1));
    expect(bytes(bounded)).toBeLessThanOrEqual(65536);
  });

  it("names the saved file in the mail draft and nothing else changes", () => {
    const draft = "mailto:issue@knxbench.com?subject=KNXBench&body=Please%20attach%20the%20saved%20file%20knxbench-diagnostics-1-20261029100000.zip%20to%20this%20mail.";
    expect(mailDraftFor(draft, "knxbench-diagnostics-1-20261029100000.zip", "knxbench-diagnostics-1-20261029100000.zip")).toBe(draft);
    expect(mailDraftFor(draft, "knxbench-diagnostics-1-20261029100000.zip", "knxbench-diagnostics-1-20261029100005.zip"))
      .toBe(draft.replace("100000.zip", "100005.zip"));
  });
});
