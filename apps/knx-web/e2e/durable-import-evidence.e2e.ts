/** Complete historical reports are explicit local downloads, with strict fictional APIs. */
import { expect, test } from "@playwright/test";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { HISTORY_FIXTURE } from "../src/projectHistory.fixture";
import { admitProjectHistory } from "../src/projectHistory";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";

for (const language of ["en","de"] as const) for (const width of [400,1440]) {
  test(`${language} full historical report at ${width}px`,async({page})=>{
    const m=language==="en"?en:de;
    const tree: ProjectTree & {has_store_path:boolean}={...admitProjectHistory(structuredClone(HISTORY_FIXTURE)).project,
      schema_version:12,errors:0,warnings:1001,import_report_available:true,import_report_historical:true,has_store_path:true,
      server_incarnation:"fictional-evidence",project_incarnation:3};
    const report={source:{file_name:"fictional.knxproj"},counts:{rows:[]},source_observations:null,
      unknown:Array.from({length:1001},(_,index)=>({name:`fictional-${index}`,sample:"x".repeat(100)})),
      opaque:[],inferred:[],conflicts:[],unsupported:[],errors:[]};
    const unexpected:string[]=[];const errors:string[]=[];let reportReads=0;
    page.on("pageerror",error=>errors.push(error.name));
    await page.setViewportSize({width,height:900});
    await page.route("**/api/**",route=>{
      const path=new URL(route.request().url()).pathname;const method=route.request().method();
      let body:unknown;let status=200;
      if(path==="/api/auth/status"&&method==="GET")body={required:false,authenticated:true};
      else if(path==="/api/settings"&&method==="GET")body={schemaVersion:1,status:"ok",settings:{uiLanguage:language,theme:language==="en"?"graphite":"porcelain"}};
      else if(path==="/api/project"&&method==="GET")body=tree;
      else if(path==="/api/project/import-report"&&method==="GET") {reportReads++;body={available:true,historical:true,server_incarnation:tree.server_incarnation,project_incarnation:tree.project_incarnation,report_json:JSON.stringify(report)};}
      else if(path==="/api/project/load-progress"&&method==="GET")body=null;
      else if(path==="/api/product-languages"&&method==="GET")body=[];
      else if(path==="/api/bus/discover"&&method==="POST")body={interfaces:[]};
      else if(["/api/device-download/status","/api/device-address/status"].includes(path)&&method==="GET"){status=404;body={error:"no fixture operation"};}
      else if(achievementsFixtureAnswer(method,path)!==undefined)body=achievementsFixtureAnswer(method,path);
      else {unexpected.push(`${method} ${path}`);status=404;body={error:"strict fictional evidence fixture"};}
      return route.fulfill({status,contentType:"application/json",body:JSON.stringify(body)});
    });
    await page.goto("/?view=editor");
    await page.locator(".workbench-navigation").getByRole("button",{name:m["workbench.overview"],exact:true}).click();
    const panel=page.locator(".import-report-panel");
    await expect(panel).toBeVisible();expect(reportReads).toBe(0);
    await expect(page.locator('.dashboard [role="note"]')).toContainText(m["importEvidence.explanation"]);
    await expect(page.locator('.project-explorer [role="note"]')).toContainText(m["importEvidence.historical"]);
    await panel.getByRole("button",{name:m["importEvidence.show"],exact:true}).click();
    await expect(panel).toContainText(m["importEvidence.historical"]);
    await expect(panel).toContainText(m["importEvidence.privacy"]);
    await panel.locator("summary").click();
    await expect(panel.locator("pre")).toBeVisible();
    expect((await panel.locator("pre").textContent())?.length).toBe(65536);
    const downloadEvent=page.waitForEvent("download");
    await panel.getByRole("button",{name:m["importEvidence.download"],exact:true}).click();
    const download=await downloadEvent;const path=await download.path();expect(path).not.toBeNull();
    const contents=JSON.parse(await readFile(path!,"utf8"));
    expect(contents.report).toEqual(report);expect(contents.historical).toBe(true);
    expect(contents.report.unknown).toHaveLength(1001);expect(reportReads).toBe(1);
    const overflow=await page.evaluate(()=>document.documentElement.scrollWidth-innerWidth);
    expect(overflow).toBeLessThanOrEqual(1);
    if(process.env.KNX_EVIDENCE_CAPTURE_DIR)await page.screenshot({path:join(process.env.KNX_EVIDENCE_CAPTURE_DIR,`report-${language}-${width}.png`)});
    expect(unexpected).toEqual([]);expect(errors).toEqual([]);
  });
}
