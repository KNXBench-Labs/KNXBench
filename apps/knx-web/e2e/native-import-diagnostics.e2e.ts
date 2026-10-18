/** Missing native import evidence in the built workbench, fictional APIs only. */
import { openNavigationSections } from "./disclosure-fixture";
import { expect, test } from "@playwright/test";
import { join } from "node:path";
import { HISTORY_FIXTURE } from "../src/projectHistory.fixture";
import { admitProjectHistory } from "../src/projectHistory";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { achievementsFixtureAnswer } from "./achievements-fixture";
import { statusMetricsFixtureBody } from "./status-metrics-fixture";

for (const language of ["en", "de"] as const) for (const width of [400,1440]) for (const available of [false,true]) {
  test(`${language} native import evidence ${available ? "available" : "unavailable"} at ${width}px`, async ({ page }) => {
    const m = language === "en" ? en : de;
    const tree: ProjectTree & { has_store_path: boolean } = {...admitProjectHistory(structuredClone(HISTORY_FIXTURE)).project, schema_version: 12, errors:0, warnings:0, import_report_available:available, has_store_path:true};
    const unexpected: string[]=[];
    const errors:string[]=[];
    page.on("pageerror", error=>errors.push(error.name));
    await page.setViewportSize({width,height:900});
    await page.route("**/api/**", route=>{
      const path=new URL(route.request().url()).pathname;
      const method=route.request().method();
      let body:unknown; let status=200;
      if(path==="/api/auth/status" && method==="GET") body={required:false,authenticated:true};
      else if(path==="/api/settings" && method==="GET") body={schemaVersion:1,status:"ok",settings:{uiLanguage:language,theme:language==="en"?"graphite":"porcelain"}};
      else if(path==="/api/project" && method==="GET") body=tree;
      else if(path==="/api/project/load-progress" && method==="GET") body=null;
      else if(path==="/api/project/status-metrics" && method==="GET") body=statusMetricsFixtureBody(tree);
      else if(path==="/api/product-languages" && method==="GET") body=[];
      else if(path==="/api/bus/discover" && method==="POST") body={interfaces:[]};
      else if(["/api/device-download/status","/api/device-address/status"].includes(path) && method==="GET") {status=404;body={error:"no fixture operation"};}
      else if(achievementsFixtureAnswer(method,path)!==undefined) body=achievementsFixtureAnswer(method,path);
      else {unexpected.push(`${method} ${path}`);status=404;body={error:"strict offline diagnostics fixture"};}
      return route.fulfill({status,contentType:"application/json",body:JSON.stringify(body)});
    });
    await page.goto("/?view=editor");
    const notice=page.locator(".workbench-panel-controls").getByRole("button",{name:m["diagnostics.importReportUnavailable"],exact:true});
    if(!available) {
      await expect(notice).toBeVisible();
      await notice.click();
      const note=page.locator('.dashboard [role="note"]');
      await expect(note).toContainText(m["diagnostics.nativeImportReportUnavailableExplanation"]);
      await expect(page.locator('.project-explorer [role="note"]')).toContainText(m["diagnostics.importReportUnavailable"]);
      expect(await note.evaluate(node=>node.scrollWidth-node.clientWidth)).toBeLessThanOrEqual(1);
      if(process.env.KNX_DIAGNOSTICS_CAPTURE_DIR) await page.screenshot({path:join(process.env.KNX_DIAGNOSTICS_CAPTURE_DIR,`native-diagnostics-${language}-${width}.png`)});
    } else {
      await openNavigationSections(page);
      await page.locator('.workbench-navigation').getByRole('button',{name:m["workbench.overview"],exact:true}).click();
      await expect(page.locator('.dashboard')).toBeVisible();
      await expect(notice).toHaveCount(0);
      await expect(page.locator('.dashboard [role="note"]')).toHaveCount(0);
    }
    const overflow = await page.evaluate(() => ({ width: document.documentElement.scrollWidth-innerWidth, nodes: [...document.querySelectorAll('body *')].filter(node => node.getBoundingClientRect().right > innerWidth+1 && getComputedStyle(node).visibility === 'visible').slice(0,30).map(node => ({className:node.className,right:node.getBoundingClientRect().right,transform:getComputedStyle(node).transform,whiteSpace:getComputedStyle(node).whiteSpace})) }));
    expect(overflow.width, JSON.stringify(overflow.nodes)).toBeLessThanOrEqual(1);
    expect(errors).toEqual([]);
    expect(unexpected).toEqual([]);
  });
}
