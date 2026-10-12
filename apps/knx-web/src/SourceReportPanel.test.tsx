/** Selected-source evidence stays explicit, source-bound and separate from primary counts. */
// @vitest-environment happy-dom
import {act} from "react";
import {createRoot,type Root} from "react-dom/client";
import {afterEach,beforeEach,expect,it,vi} from "vitest";
import SourceReportPanel from "./SourceReportPanel";
import type {ProjectTree} from "./bindings/ProjectTree";
import {settingsStorage,resetSettingsForTests} from "./settingsStore";
import {resetUiLanguageForTests,saveUiLanguage} from "./uiLanguage";
const mocked=vi.hoisted(()=>({catalogue:vi.fn(),report:vi.fn(),download:vi.fn()}));
vi.mock("./api",()=>({getSourceReportCatalogue:mocked.catalogue,getSourceReport:mocked.report,errorMessage:(e:unknown)=>String(e)}));
vi.mock("./localJsonDownload",()=>({downloadLocalJson:mocked.download}));
const id="c".repeat(64);
const tree=():ProjectTree=>({schema_version:12,errors:0,warnings:7,can_undo:false,can_redo:false,is_modified:false,
  group_address_style:"ThreeLevel",installations:[],server_incarnation:"fictional",project_incarnation:1,snapshot_revision:1});
const catalogue=()=>({server_incarnation:"fictional",project_incarnation:1,snapshot_revision:1,records:[{id,status:"recorded",source_name:"fictional.knxproj",bytes:123,error_rows:0,warning_rows:2}]});
const exact=JSON.stringify({source:{file_name:"fictional.knxproj"},counts:{rows:[]},unknown:[],opaque:[],inferred:[],conflicts:[],unsupported:[],errors:[]});
const evidence=()=>({scope:"selectiveSource" as const,id,server_incarnation:"fictional",project_incarnation:1,snapshot_revision:1,report_json:exact});
let host:HTMLDivElement;let root:Root;
beforeEach(()=>{vi.clearAllMocks();vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT",true);resetSettingsForTests();resetUiLanguageForTests();saveUiLanguage(settingsStorage,"en");host=document.createElement("div");document.body.append(host);root=createRoot(host);});
afterEach(()=>{act(()=>root.unmount());host.remove();vi.unstubAllGlobals();resetSettingsForTests();resetUiLanguageForTests();});
function button(label:string){const result=[...host.querySelectorAll("button")].find(b=>b.textContent?.includes(label));expect(result,`missing action: ${label}`).toBeDefined();return result!;}
async function loadCatalogue(){await act(async()=>button("Show source report catalogue").click());}
it("requests no bodies automatically, then reads only the explicitly selected source",async()=>{
  mocked.catalogue.mockResolvedValue(catalogue());mocked.report.mockResolvedValue(evidence());
  act(()=>root.render(<SourceReportPanel tree={tree()}/>));expect(mocked.catalogue).not.toHaveBeenCalled();
  await loadCatalogue();expect(mocked.report).not.toHaveBeenCalled();
  await act(async()=>button("Read selected source report").click());
  expect(mocked.report).toHaveBeenCalledWith(id);expect(host.textContent).toContain("not merged-project compatibility");
  expect(host.querySelector("section,h3")).toBeNull();expect(host.querySelector("h4")?.textContent).toBe("Recorded source-specific import reports");
  expect(host.querySelector('.source-report-panel [role="note"]')?.textContent).toBe("Report recorded");
  expect(host.textContent).toContain("private source data");
  await act(async()=>button("Download source report").click());
  expect(JSON.parse(mocked.download.mock.calls[0][0]).scope).toBe("selectiveSource");
  expect(mocked.download.mock.calls[0][0]).toContain(exact);
});
it("does not turn an unavailable source report into measured zero or a download",async()=>{
  mocked.catalogue.mockResolvedValue({...catalogue(),records:[{id,status:"unavailable",source_name:null,bytes:null,error_rows:null,warning_rows:null}]});
  act(()=>root.render(<SourceReportPanel tree={tree()}/>));await loadCatalogue();
  expect(host.textContent).toContain("Report unavailable");expect(mocked.report).not.toHaveBeenCalled();
  expect(button("Read selected source report").disabled).toBe(true);
});
it("discards a late report after current-project revision changed",async()=>{
  mocked.catalogue.mockResolvedValue(catalogue());let finish!:(value:ReturnType<typeof evidence>)=>void;
  mocked.report.mockReturnValue(new Promise(resolve=>{finish=resolve;}));
  const initial=tree();act(()=>root.render(<SourceReportPanel tree={initial}/>));await loadCatalogue();
  await act(async()=>button("Read selected source report").click());
  act(()=>root.render(<SourceReportPanel tree={{...initial,snapshot_revision:2}}/>));
  await act(async()=>finish(evidence()));expect(host.textContent).not.toContain("Download source report");
});
it("refuses another-project catalogue without offering its source records",async()=>{
  mocked.catalogue.mockResolvedValue({...catalogue(),project_incarnation:7});
  act(()=>root.render(<SourceReportPanel tree={tree()}/>));await loadCatalogue();
  expect(host.querySelector('[role="alert"]')).not.toBeNull();expect(host.querySelector("select")).toBeNull();
});
