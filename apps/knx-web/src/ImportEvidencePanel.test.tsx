/** Report reads are explicit, source-bound and invalidated by project replacement. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import ImportEvidencePanel from "./ImportEvidencePanel";
import type { ProjectTree } from "./bindings/ProjectTree";
import { settingsStorage, resetSettingsForTests } from "./settingsStore";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";

const mocks = vi.hoisted(() => ({ getImportEvidence: vi.fn(), download:vi.fn() }));
vi.mock("./api", () => ({getImportEvidence:mocks.getImportEvidence, errorMessage:(e:unknown)=>String(e)}));
vi.mock("./localJsonDownload", () => ({downloadLocalJson:mocks.download}));
const tree = (): ProjectTree => ({ schema_version:12, errors:0, warnings:2, can_undo:false,
  can_redo:false,is_modified:false,group_address_style:"ThreeLevel",installations:[],
  import_report_available:true,server_incarnation:"fictional-server",project_incarnation:2 });
const evidenceBase = () => ({ available:true, historical:true, server_incarnation:"fictional-server",project_incarnation:2,
  report:{source:{file_name:"fictional.knxproj"}, counts:{rows:[]}, unknown:[{name:"fictional attribute"}],
    opaque:[],inferred:[],conflicts:[],unsupported:[],errors:[]},
});
const evidence = () => ({...evidenceBase(),report_json:JSON.stringify(evidenceBase().report)});
let host: HTMLDivElement; let root: Root;
beforeEach(() => {vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);vi.clearAllMocks();resetSettingsForTests();resetUiLanguageForTests();saveUiLanguage(settingsStorage,"en");
  host=document.createElement("div");document.body.append(host);root=createRoot(host);});
afterEach(() => {act(()=>root.unmount());vi.unstubAllGlobals();host.remove();resetSettingsForTests();resetUiLanguageForTests();});
function button(label:string) {const result=Array.from(host.querySelectorAll("button")).find(b=>b.textContent?.includes(label));
  expect(result,`missing button: ${label}`).toBeDefined();return result!;}
async function load() {await act(async()=>button("Show full import report").click());}

it("makes no automatic request and preserves the full report in explicit local download", async()=>{
  mocks.getImportEvidence.mockResolvedValue(evidence());
  act(()=>root.render(<ImportEvidencePanel tree={tree()} />));
  expect(mocks.getImportEvidence).not.toHaveBeenCalled();
  await load();
  expect(host.textContent).toContain("Historical import findings");
  expect(host.textContent).toContain("not reassessed");
  await act(async()=>button("Download full report").click());
  expect(JSON.parse(mocks.download.mock.calls[0][0]).report).toEqual(evidence().report);
  expect(host.textContent).toContain("private source data");
});
it("retains immutable original evidence across an unrelated same-project refresh", async()=>{
  mocks.getImportEvidence.mockResolvedValue(evidence());
  const initial=tree();act(()=>root.render(<ImportEvidencePanel tree={initial} />));await load();
  act(()=>root.render(<ImportEvidencePanel tree={{...initial,snapshot_revision:99,is_modified:true}} />));
  expect(host.textContent).toContain("Download full report");
  expect(mocks.getImportEvidence).toHaveBeenCalledTimes(1);
});
it("does not publish a late response after the project has changed",async()=>{
  let finish!: (value: ReturnType<typeof evidence>) => void;
  mocks.getImportEvidence.mockReturnValue(new Promise(resolve=>{finish=resolve;}));
  act(()=>root.render(<ImportEvidencePanel tree={tree()} />));
  await load();
  act(()=>root.render(<ImportEvidencePanel tree={{...tree(),project_incarnation:3}} />));
  await act(async()=>finish(evidence()));
  expect(host.querySelector("pre")).toBeNull();
  expect(host.textContent).not.toContain("Download full report");
});
it("refuses evidence bound to another current project",async()=>{
  mocks.getImportEvidence.mockResolvedValue({...evidence(),project_incarnation:7});
  act(()=>root.render(<ImportEvidencePanel tree={tree()} />));await load();
  expect(host.querySelector('[role="alert"]')).not.toBeNull();
  expect(host.querySelector("pre")).toBeNull();
});
it("does not offer unavailable legacy evidence as a measured report",()=>{
  act(()=>root.render(<ImportEvidencePanel tree={{...tree(),import_report_available:false}} />));
  expect(host.querySelector("button")).toBeNull();
  expect(mocks.getImportEvidence).not.toHaveBeenCalled();
});
