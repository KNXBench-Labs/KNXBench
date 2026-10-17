/** Project payload bytes download inertly after explicit current-bound user selection. */
import { openNavigationSections } from "./disclosure-fixture";
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
  test(`${language} retained payload download at ${width}px`,async({page})=>{
    const m=language==="en"?en:de;
    const tree: ProjectTree & {has_store_path:boolean}={...admitProjectHistory(structuredClone(HISTORY_FIXTURE)).project,
      schema_version:12,errors:0,warnings:7,import_report_available:true,import_report_historical:true,has_store_path:true,
      server_incarnation:"fictional-evidence",project_incarnation:3};
    const payload=Buffer.from("<script>window.FOREIGN_PAYLOAD_EXECUTED=true</script>\nprivate fictional attachment");
    const zip=Buffer.from([80,75,3,4,1,2,3,4]);
    const rows=[{id:0,source_path:"P-FICTION/UserFiles/a-long-fictional-attachment-name.html",kind:"ContainerEntry",role:"user-file-candidate",content_class:"unknown",bytes:payload.length,sha256:"c".repeat(64),downloadable:true},
      {id:1,source_path:"P-FICTION/AddinData/fictional-extension.etsapp",kind:"ContainerEntry",role:"ets-app-named-candidate",content_class:"zip",bytes:zip.length,sha256:"f".repeat(64),downloadable:true}];
    const unexpected:string[]=[];const errors:string[]=[];let reads=0;let lists=0;
    page.on("pageerror",error=>errors.push(error.name));
    await page.setViewportSize({width,height:900});
    await page.route("**/api/**",route=>{
      const path=new URL(route.request().url()).pathname;const method=route.request().method();
      let body:unknown;let status=200;
      if(path==="/api/auth/status"&&method==="GET")body={required:false,authenticated:true};
      else if(path==="/api/settings"&&method==="GET")body={schemaVersion:1,status:"ok",settings:{uiLanguage:language,theme:language==="en"?"graphite":"porcelain"}};
      else if(path==="/api/project"&&method==="GET")body=tree;
      else if(path==="/api/project/payloads"&&method==="GET") {lists++;body={server_incarnation:tree.server_incarnation,project_incarnation:tree.project_incarnation,snapshot_revision:tree.snapshot_revision,max_download_bytes:67108864,records:rows};}
      else if(/^\/api\/project\/payloads\/[01]\/download$/.test(path)&&method==="GET") {
        const query=new URL(route.request().url()).searchParams;
        expect(query.get("serverIncarnation")).toBe(tree.server_incarnation);
        expect(query.get("projectIncarnation")).toBe(String(tree.project_incarnation));
        expect(query.get("snapshotRevision")).toBe(String(tree.snapshot_revision));
        reads++;const selected=path.includes("/0/")?payload:zip;
        return route.fulfill({status:200,headers:{"content-type":"application/octet-stream","content-length":String(selected.length),"content-disposition":"attachment; filename=\"retained-payload.bin\"","x-content-type-options":"nosniff"},body:selected});
      }
      else if(path==="/api/project/load-progress"&&method==="GET")body=null;
      else if(path==="/api/product-languages"&&method==="GET")body=[];
      else if(path==="/api/bus/discover"&&method==="POST")body={interfaces:[]};
      else if(["/api/device-download/status","/api/device-address/status"].includes(path)&&method==="GET"){status=404;body={error:"no fixture operation"};}
      else if(achievementsFixtureAnswer(method,path)!==undefined)body=achievementsFixtureAnswer(method,path);
      else {unexpected.push(`${method} ${path}`);status=404;body={error:"strict fictional evidence fixture"};}
      return route.fulfill({status,contentType:"application/json",body:JSON.stringify(body)});
    });
    await page.goto("/?view=editor");
    await openNavigationSections(page);
    await page.locator(".workbench-navigation").getByRole("button",{name:m["workbench.overview"],exact:true}).click();
    const panel=page.locator(".project-payload-panel");
    await expect(panel).toBeVisible();expect(lists).toBe(0);expect(reads).toBe(0);
    await panel.getByRole("button",{name:m["payloads.show"],exact:true}).click();
    await expect(panel).toContainText(m["payloads.privacy"]);expect(lists).toBe(1);expect(reads).toBe(0);
    await expect(panel.locator(".payload-path").first()).toHaveText(rows[0].source_path);
    for(const [id,expected] of [[0,payload],[1,zip]] as const) {
      await panel.locator("select").selectOption(String(id));
      const event=page.waitForEvent("download");
      await panel.getByRole("button",{name:m["payloads.download"],exact:true}).click();
      const download=await event;const saved=await download.path();expect(saved).not.toBeNull();
      expect(await readFile(saved!)).toEqual(expected);expect(download.suggestedFilename()).toBe("retained-payload.bin");
    }
    expect(reads).toBe(2);
    expect(await page.evaluate(()=>Object.hasOwn(window,"FOREIGN_PAYLOAD_EXECUTED"))).toBe(false);
    await expect(panel).toContainText(m["payloads.role.ets-app-named-candidate"]);
    // Main's Import card always lists both counts: 0 errors, 7 historical warnings.
    await expect(page.locator('.dashboard-issues .dashboard-value')).toHaveText(["0", "7"]);
    const overflow=await page.evaluate(()=>document.documentElement.scrollWidth-innerWidth);
    expect(overflow).toBeLessThanOrEqual(1);
    if(process.env.KNX_EVIDENCE_CAPTURE_DIR)await page.screenshot({path:join(process.env.KNX_EVIDENCE_CAPTURE_DIR,`payload-${language}-${width}.png`)});
    expect(unexpected).toEqual([]);expect(errors).toEqual([]);
  });
}
