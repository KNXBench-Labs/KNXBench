/** Missing historical evidence must remain distinct from a measured clean import. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Dashboard from "./Dashboard";
import { settingsStorage, resetSettingsForTests } from "./settingsStore";
import type { ProjectTree } from "./bindings/ProjectTree";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";

let host: HTMLDivElement;
let root: Root;
const tree = (): ProjectTree => ({schema_version:12,errors:0,warnings:0,can_undo:false,can_redo:false,is_modified:false,group_address_style:"ThreeLevel",installations:[]});
beforeEach(() => { resetSettingsForTests(); resetUiLanguageForTests(); saveUiLanguage(settingsStorage, "en"); host=document.createElement("div"); document.body.append(host); root=createRoot(host); });
afterEach(() => { act(() => root.unmount()); host.remove(); resetSettingsForTests(); resetUiLanguageForTests(); });

describe("native import evidence availability", () => {
  it.each([ ["en","Historical import report unavailable","not reassessed"], ["de","Historischer Importbericht nicht verfügbar","nicht erneut geprüft"] ] as const)("shows missing native evidence explicitly in %s", (language,label,explanation) => {
    saveUiLanguage(settingsStorage, language);
    resetUiLanguageForTests();
    act(() => root.render(<Dashboard tree={{...tree(),import_report_available:false}} />));
    expect(host.querySelector('[role="note"]')?.textContent).toContain(label);
    expect(host.textContent).toContain(explanation);
  });
  it("does not label a measured zero as unavailable", () => {
    act(() => root.render(<Dashboard tree={{...tree(),import_report_available:true}} />));
    expect(host.querySelector('[role="note"]')).toBeNull();
  });
  it("retains known current-session warnings alongside missing historical evidence", () => {
    act(() => root.render(<Dashboard tree={{...tree(),warnings:7,import_report_available:false}} />));
    expect(host.querySelector('[role="note"]')).not.toBeNull();
    expect(host.querySelector('.dashboard-issues')?.textContent).toContain("7");
  });
  it("does not invent availability for older responses that omit it", () => {
    act(() => root.render(<Dashboard tree={{...tree(),warnings:2}} />));
    expect(host.querySelector('[role="note"]')).toBeNull();
    expect(host.querySelector('.dashboard-issues')?.textContent).toContain("2");
  });
});

describe("project status import findings", () => {
  const links = () => [...host.querySelectorAll<HTMLButtonElement>(".dashboard-finding-link")];
  it("opens the matching severity from each non-zero counter", () => {
    const open = vi.fn();
    act(() => root.render(<Dashboard tree={{...tree(),errors:2,warnings:7,import_report_available:true}} onOpenImportFindings={open} />));
    const [errors, warnings] = links();
    expect(errors.textContent).toBe(`${en["dashboard.showInLog"]}2`);
    expect(warnings.textContent).toBe(`${en["dashboard.showInLog"]}7`);
    expect(warnings.getAttribute("aria-describedby")).toBe("dashboard-import-warnings-label");
    expect(host.querySelector("#dashboard-import-warnings-label")?.textContent).toBe(en["dashboard.importWarnings"]);
    act(() => warnings.click());
    act(() => errors.click());
    expect(open.mock.calls).toEqual([["warning"], ["error"]]);
    expect(host.querySelector(".dashboard-import-state")?.textContent).toBe(en["dashboard.importState.errors"]);
  });
  it("keeps a zero as plain text with an explicit clean statement, not a dead control", () => {
    const open = vi.fn();
    act(() => root.render(<Dashboard tree={{...tree(),import_report_available:true}} onOpenImportFindings={open} />));
    expect(links()).toHaveLength(0);
    expect(host.querySelector(".dashboard-issues")?.textContent).toBe(`${en["dashboard.importErrors"]}0${en["dashboard.importWarnings"]}0`);
    expect(host.querySelector(".dashboard-import-state")?.textContent).toBe(en["dashboard.importState.clean"]);
  });
  it("says 'not assessed' when the report is missing and nothing is counted", () => {
    act(() => root.render(<Dashboard tree={{...tree(),import_report_available:false}} onOpenImportFindings={vi.fn()} />));
    expect(links()).toHaveLength(0);
    expect(host.querySelector(".dashboard-import-state")?.textContent).toBe(en["dashboard.importState.unavailable"]);
    expect(host.querySelector('[role="note"]')?.textContent).toContain("Historical import report unavailable");
  });
  it("still opens counted current-session warnings next to a missing historical report", () => {
    const open = vi.fn();
    act(() => root.render(<Dashboard tree={{...tree(),warnings:3,import_report_available:false}} onOpenImportFindings={open} />));
    expect(host.querySelector('[role="note"]')).not.toBeNull();
    act(() => links()[0].click());
    expect(open).toHaveBeenCalledWith("warning");
  });
  it("shows the warning count only as text when no navigation is wired", () => {
    act(() => root.render(<Dashboard tree={{...tree(),warnings:4}} />));
    expect(links()).toHaveLength(0);
    expect(host.querySelector(".dashboard-issues")?.textContent).toContain("4");
  });
  it("keeps every counter and the schema version under labelled sections", () => {
    act(() => root.render(<Dashboard tree={{...tree(),schema_version:13,errors:1,warnings:2}} />));
    const headings = [...host.querySelectorAll("h3")].map((h) => h.textContent);
    expect(headings).toEqual([en["dashboard.section.contents"], en["dashboard.section.import"], en["dashboard.section.file"]]);
    const labels = [...host.querySelectorAll("dt")].map((d) => d.textContent);
    expect(labels).toEqual([en["dashboard.installations"], en["dashboard.areas"], en["dashboard.lines"], en["dashboard.devices"],
      en["dashboard.buildingParts"], en["dashboard.groupAddresses"], en["dashboard.comObjects"],
      en["dashboard.importErrors"], en["dashboard.importWarnings"], en["dashboard.schemaVersion"]]);
    expect([...host.querySelectorAll("dd")].map((d) => d.textContent)).toEqual(["0","0","0","0","0","0","0","1","2","13"]);
    for (const section of host.querySelectorAll("section")) {
      expect(host.querySelector(`#${section.getAttribute("aria-labelledby")}`)).not.toBeNull();
    }
  });
  it("words the status in German", () => {
    saveUiLanguage(settingsStorage, "de");
    resetUiLanguageForTests();
    act(() => root.render(<Dashboard tree={{...tree(),warnings:7}} onOpenImportFindings={vi.fn()} />));
    expect(host.querySelector(".dashboard-import-state")?.textContent).toBe(de["dashboard.importState.warnings"]);
    expect(links()[0].textContent).toBe(`${de["dashboard.showInLog"]}7`);
  });
});
