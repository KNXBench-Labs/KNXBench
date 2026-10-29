/** Admits fixtures the Rust serializer accepts and re-emits unchanged; renders them in EN/DE. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import fixtures from "./postDownloadCheck.fixtures.json";
import cases from "./postDownloadCheck.cases.json";
import PostDownloadCheckView from "./PostDownloadCheckView";
import { admitPostDownloadCheck, checkLines, NOT_CHECKED, runStateLines, type PostDownloadCheck } from "./postDownloadCheck";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const FIXTURES = fixtures as Record<string, PostDownloadCheck>;

interface TableCase { name: string; fixture: string; set: [string, unknown][] }
const CASES = cases as unknown as { accepted: TableCase[]; refused: TableCase[] };

/** One table case's `[pointer, value]` edits on a copy of its fixture, as the Rust test applies them. */
function edited(entry: TableCase): unknown {
  const value = JSON.parse(JSON.stringify(FIXTURES[entry.fixture])) as Record<string, unknown>;
  expect(value, entry.fixture).toBeTypeOf("object");
  for (const [pointer, replacement] of entry.set) {
    const keys = pointer.split("/").slice(1);
    const last = keys.pop()!;
    let parent: unknown = value;
    for (const key of keys) parent = (parent as Record<string, unknown>)[key];
    expect(parent, `${entry.name}: ${pointer}`).toBeTypeOf("object");
    (parent as Record<string, unknown>)[last] = replacement;
  }
  return value;
}
let host: HTMLDivElement | undefined;
let root: Root | undefined;

async function show(value: unknown, language = "en"): Promise<HTMLDivElement> {
  saveUiLanguage(settingsStorage, language);
  resetUiLanguageForTests();
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => root!.render(<PostDownloadCheckView value={value} />));
  return host;
}

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  host?.remove();
  host = undefined;
  root = undefined;
  resetSettingsForTests();
  resetUiLanguageForTests();
});

describe("admission", () => {
  it("admits every serializer fixture exactly and reads an absent field as not checked", () => {
    for (const [name, value] of Object.entries(FIXTURES)) {
      expect(admitPostDownloadCheck(value), name).toEqual(value);
    }
    expect(admitPostDownloadCheck(undefined)).toEqual(NOT_CHECKED);
  });

  it("refuses contradictions, runtime claims and raw identity fields instead of guessing", () => {
    const clone = (name: string) => JSON.parse(JSON.stringify(FIXTURES[name]));
    const running = clone("matched"); running.runtime = "running";
    const raw = clone("matched"); raw.observation.reach.identity.application = "0083002715";
    const contradiction = clone("matched"); contradiction.observation.reach.identity.planLoadVerdict = "differsFromPlan";
    const readAfterFailure = clone("partialReading");
    readAfterFailure.observation.reach.identity.loadStates[2].observed = { result: "observed", loadState: "loaded" };
    const unreachedIdentity = clone("notReached"); unreachedIdentity.observation.reach.identity = { result: "unavailable", part: { kind: "mask" } };
    for (const value of [running, raw, contradiction, readAfterFailure, unreachedIdentity, null, "matched", {}]) {
      expect(admitPostDownloadCheck(value)).toBeNull();
    }
  });

  it("admits and refuses the case table the Rust serializer runs too", () => {
    for (const entry of CASES.accepted) {
      const value = edited(entry);
      expect(admitPostDownloadCheck(value), entry.name).toEqual(value);
    }
    expect(CASES.refused.length).toBeGreaterThanOrEqual(40);
    for (const entry of CASES.refused) {
      expect(admitPostDownloadCheck(edited(entry)), entry.name).toBeNull();
    }
  });

  it("warns on a verdict other than a match", () => {
    expect(checkLines(FIXTURES.differsFromPlan).at(-1)).toEqual({ key: "postDownloadCheck.verdict.differsFromPlan", warning: true });
    expect(checkLines(FIXTURES.partialReading).at(-1)).toEqual({ key: "postDownloadCheck.verdict.unknown", warning: true });
  });

  it("states reach, identity and verdict as separate facts, warning only where it should", () => {
    expect(checkLines(FIXTURES.matched).map((line) => [line.key, !!line.warning])).toEqual([
      ["postDownloadCheck.reach.reached", false],
      ["postDownloadCheck.identity.match", false],
      ["postDownloadCheck.verdict.matchesPlan", false],
    ]);
    expect(checkLines(FIXTURES.identityMismatch).map((line) => line.key)).toEqual([
      "postDownloadCheck.reach.reached", "postDownloadCheck.identity.mismatch.application",
    ]);
    expect(checkLines(FIXTURES.notReached)[0]).toEqual({ key: "postDownloadCheck.reach.notReached", warning: true });
  });
});

describe("rendering", () => {
  it("shows expected next to observed, the exemplar caveat and an unknown runtime in English", async () => {
    const view = await show(FIXTURES.matched);
    const rows = [...view.querySelectorAll("tbody tr")].map((row) => [...row.querySelectorAll("td")].map((cell) => cell.textContent));
    expect(rows).toEqual([
      ["Address table", "loaded", "loaded"],
      ["Association table", "unloaded", "unloaded"],
      ["Application program", "loaded", "loaded"],
    ]);
    expect(view.textContent).toContain(en["postDownloadCheck.exemplar"]);
    expect(view.textContent).toContain(en["postDownloadCheck.runtimeUnknown"]);
    expect(view.querySelector("button")).toBeNull();
  });

  it("keeps the earlier reading when a later one failed, in German", async () => {
    const view = await show(FIXTURES.partialReading, "de");
    const rows = [...view.querySelectorAll("tbody tr")].map((row) => [...row.querySelectorAll("td")].map((cell) => cell.textContent));
    expect(rows).toEqual([
      ["Adresstabelle", "geladen", "geladen"],
      ["Zuordnungstabelle", "entladen", de["postDownloadCheck.observed.unavailable"]],
      ["Applikationsprogramm", "geladen", de["postDownloadCheck.observed.notRead"]],
    ]);
    expect(view.textContent).toContain(de["postDownloadCheck.verdict.unknown"]);
    expect(view.textContent).toContain(de["postDownloadCheck.runtimeUnknown"]);
  });

  it("says mismatch and unavailable loudly in both languages and offers no resend", async () => {
    let view = await show(FIXTURES.identityMismatch, "de");
    expect(view.querySelector("[role=status]")!.textContent).toBe(de["postDownloadCheck.identity.mismatch.application"]);
    expect(view.querySelector("table")).toBeNull();
    expect(view.querySelector("button")).toBeNull();
    await act(async () => root!.unmount()); host!.remove(); root = undefined;
    view = await show(FIXTURES.identityUnavailable);
    expect(view.textContent).toContain("Identity unavailable: property 0/78 could not be read.");
    expect(view.textContent).not.toContain(en["postDownloadCheck.identity.match"]);
  });

  it("says reached when the first read was only acknowledged and the mask then unavailable", async () => {
    const mask = JSON.parse(JSON.stringify(FIXTURES.identityUnavailable));
    mask.observation.reach.identity.part = { kind: "mask" };
    let view = await show(mask);
    expect(view.textContent).toContain("Device reached: it answered or acknowledged the first read.");
    expect(view.textContent).toContain("Identity unavailable: the mask version could not be read.");
    await act(async () => root!.unmount()); host!.remove(); root = undefined;
    view = await show(mask, "de");
    expect(view.textContent).toContain("Gerät erreicht: Es hat den ersten Lesezugriff beantwortet oder quittiert.");
    expect(view.textContent).toContain("Kennung nicht verfügbar: Die Maskenversion konnte nicht gelesen werden.");
  });

  it("names the plan's mask, not the device's, when nothing was sent", async () => {
    const unsupported = JSON.parse(JSON.stringify(FIXTURES.unsupported));
    unsupported.observation.reason = "unsupportedMask";
    let view = await show(unsupported);
    expect(view.textContent).toContain("for the plan's mask");
    expect(view.textContent).not.toContain("this device's mask");
    await act(async () => root!.unmount()); host!.remove(); root = undefined;
    view = await show(unsupported, "de");
    expect(view.textContent).toContain("Für die Maske des Plans");
  });

  it("scopes the check to one attempt after the transfer and a restart wait only if any", async () => {
    let view = await show(FIXTURES.matched);
    expect(view.textContent).toContain("after the transfer (and its restart wait, if the plan restarts)");
    expect(view.textContent).toContain("makes no second attempt");
    expect(view.textContent).not.toContain("retries");
    await act(async () => root!.unmount()); host!.remove(); root = undefined;
    view = await show(FIXTURES.matched, "de");
    expect(view.textContent).toContain("falls der Plan neu startet");
    expect(view.textContent).toContain("keinen zweiten Versuch");
  });

  it("shows a malformed record as an unreadable warning, never as a result", async () => {
    const view = await show({ observation: { status: "attempted", reach: { result: "reached" } }, runtime: "unknown" });
    const section = view.querySelector("section")!;
    expect(section.dataset.check).toBe("unreadable");
    expect(section.querySelector("[role=status]")!.className).toBe("form-warning");
    expect(view.textContent).toContain(en["postDownloadCheck.unreadable"]);
    expect(view.textContent).not.toContain(en["postDownloadCheck.notChecked"]);
    expect(view.querySelector("table")).toBeNull();
  });

  it("renders a legacy record without the field as not checked, never as loaded", async () => {
    const view = await show(undefined, "de");
    expect(view.dataset.check ?? view.querySelector("section")!.dataset.check).toBe("notChecked");
    expect(view.textContent).toContain(de["postDownloadCheck.notChecked"]);
    expect(view.textContent).not.toContain("geladen");
  });
});

describe("run-state presentation", () => {
  it("names each reported state with its scope and no health or function claim in both languages", async () => {
    for (const [language, messages, running, scope] of [
      ["en", en, "Run state: running", "validated plan-matching Applicationprogram object 3"],
      ["de", de, "Laufzustand: läuft", "validierten, zum Plan passenden Applikationsprogramm-Objekts 3"],
    ] as const) {
      const view = await show(FIXTURES.runStateRunning, language);
      expect(view.textContent, language).toContain(running);
      expect(view.textContent, language).toContain(scope);
      expect(view.textContent, language).toContain(messages["postDownloadCheck.runState.scope"]);
      expect(view.textContent, language).not.toContain(messages["postDownloadCheck.runtimeUnknown"]);
      expect(view.querySelector("button")).toBeNull();
      await act(async () => root!.unmount()); host!.remove(); root = undefined;
    }
    expect(en["postDownloadCheck.runState.scope"]).toMatch(/not device health.*not a successful download.*not a unique program instance.*not installation function/s);
    expect(de["postDownloadCheck.runState.scope"]).toMatch(/keine Gerätegesundheit.*kein erfolgreicher Download.*keine eindeutige Programminstanz.*keine Anlagenfunktion/s);
  });

  it("keeps the load table independent of the run state", async () => {
    const view = await show(FIXTURES.runStateRunning);
    const rows = [...view.querySelectorAll("tbody tr")].map((row) => [...row.querySelectorAll("td")].map((cell) => cell.textContent));
    expect(rows[2]).toEqual(["Application program", "loaded", "loaded"]);
    expect(runStateLines(FIXTURES.runStateRunning).map((line) => line.key)).toEqual([
      "postDownloadCheck.runState.state.running", "postDownloadCheck.runState.scope",
    ]);
  });

  it("states every non-observed result as its own fact while the summary stays unknown", () => {
    const keys = (name: string) => runStateLines(FIXTURES[name]).map((line) => [line.key, line.values ?? null]);
    expect(keys("runStateUndefined")).toEqual([["postDownloadCheck.runState.undefined", null]]);
    expect(keys("runStateUnsupportedMask")).toEqual([["postDownloadCheck.runState.notRead.unsupportedMask", null]]);
    expect(keys("runStateLoadFailed")).toEqual([["postDownloadCheck.runState.notRead.loadReadFailed", null]]);
    expect(keys("runStateCandidateMismatch")).toEqual([["postDownloadCheck.runState.candidateMismatch.objectType", null]]);
    expect(keys("runStateUnavailable")).toEqual([["postDownloadCheck.runState.unavailable", { step: "runState", cause: "noCorrelatedAnswer" }]]);
    expect(runStateLines(FIXTURES.matched)).toEqual([]);
    expect(runStateLines(FIXTURES.notChecked)).toEqual([]);
  });

  it("renders a legacy unknown record with the legacy wording and an unavailable read with its own line", async () => {
    let view = await show(FIXTURES.runStateUnavailable);
    expect(view.textContent).toContain("Run state unavailable: the run state read failed (no correlated answer).");
    expect(view.textContent).toContain("Whether the application runs: unknown (no defined run state observed).");
    expect(view.textContent).not.toContain("unknown (not observed)");
    await act(async () => root!.unmount()); host!.remove(); root = undefined;
    view = await show(FIXTURES.matched, "de");
    expect(view.textContent).toContain(de["postDownloadCheck.runtimeUnknown"]);
    expect(view.textContent).toContain("unbekannt (nicht beobachtet)");
    expect(view.textContent).not.toContain("kein definierter Laufzustand");
    expect(view.textContent).not.toContain("Laufzustand");
  });

  it("says no defined run state was observed whenever a run-state record exists without one, in both languages", async () => {
    for (const name of ["runStateUndefined", "runStateUnavailable", "runStateCandidateMismatch", "runStateUnsupportedMask", "runStateLoadFailed"]) {
      for (const [language, present, legacy] of [
        ["en", "unknown (no defined run state observed)", "unknown (not observed)"],
        ["de", "unbekannt (kein definierter Laufzustand beobachtet)", "unbekannt (nicht beobachtet)"],
      ] as const) {
        const view = await show(FIXTURES[name], language);
        expect(view.textContent, `${name} ${language}`).toContain(present);
        expect(view.textContent, `${name} ${language}`).not.toContain(legacy);
        // The load verdict and the load/transfer caveat stay their own facts.
        expect(view.textContent, `${name} ${language}`).toContain(language === "en" ? "not proof that the installation works" : "kein Beweis, dass die Anlage funktioniert");
        await act(async () => root!.unmount()); host!.remove(); root = undefined;
      }
    }
  });

  it("shows an observed state beside a differing machine: both facts, no unknown wording", async () => {
    const entry = CASES.accepted.find((candidate) => candidate.name === "differingLoadWithObservedState")!;
    expect(entry, "the shared table names the case").toBeDefined();
    expect(entry.fixture).toBe("runStateRunning");
    const value = edited(entry) as PostDownloadCheck;
    expect(admitPostDownloadCheck(value)).toEqual(value);
    expect(checkLines(value).at(-1)).toEqual({ key: "postDownloadCheck.verdict.differsFromPlan", warning: true });
    expect(runStateLines(value).map((line) => line.key)).toEqual([
      "postDownloadCheck.runState.state.running", "postDownloadCheck.runState.scope",
    ]);
    const view = await show(value);
    const rows = [...view.querySelectorAll("tbody tr")].map((row) => [...row.querySelectorAll("td")].map((cell) => cell.textContent));
    expect(rows[2]).toEqual(["Application program", "loaded", "unloaded"]);
    expect(view.textContent).toContain("Run state: running");
    expect(view.textContent).toContain(en["postDownloadCheck.verdict.differsFromPlan"]);
    expect(view.textContent).not.toContain("Whether the application runs");
  });
});
