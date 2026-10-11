/** Missing historical evidence must remain distinct from a measured clean import. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import Dashboard from "./Dashboard";
import { settingsStorage, resetSettingsForTests } from "./settingsStore";
import type { ProjectTree } from "./bindings/ProjectTree";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";

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
