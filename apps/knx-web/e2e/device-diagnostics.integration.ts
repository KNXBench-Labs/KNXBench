/** Real production UI → real HTTP handlers → regular KNX simulator → actual ZIP download. */
import {test,expect} from "@playwright/test";
import {join} from "node:path";
import {mkdirSync} from "node:fs";
import {messages as en} from "../src/messages/en";
import {messages as de} from "../src/messages/de";
import { openFileSubmenu } from "./file-menu";
const evidence=process.env.KNX_DIAG_EVIDENCE;
for(const language of ["en","de"] as const)for(const width of [1440,400]){
 test(`${language} simulated device diagnostic ZIP ${width}px`,async({page})=>{
  const m=language==="de"?de:en;await page.setViewportSize({width,height:900});
  await page.addInitScript(lang=>localStorage.setItem("uiLanguage",lang),language);
  // Only presentation preference is overridden; diagnostic responses are real.
  await page.route("**/api/settings",async route=>{if(route.request().method()!=="GET"){await route.continue();return;}const response=await route.fetch();const body=await response.json();body.settings.uiLanguage=language;await route.fulfill({response,json:body});});
  const diagnosticRequests:string[]=[];page.on("request",r=>{if(r.url().includes("/api/device-diagnostics/"))diagnosticRequests.push(new URL(r.url()).pathname);});
  await page.goto("/");await page.locator(".file-menu > summary").click();await openFileSubmenu(page,"help");await page.locator(".file-menu [data-diagnostics-open]").click();
  const dialog=page.getByRole("dialog",{name:m["diagnostics.title"],exact:true});await expect(dialog).toBeVisible();
  expect(diagnosticRequests).toEqual([]);
  await dialog.locator("[data-diagnostics-device]").selectOption("1");await dialog.locator("[data-diagnostics-gateway]").fill("192.0.2.10:3671");
  await dialog.locator("[data-diagnostics-preview]").click();await expect(dialog.locator("[data-diagnostics-review]")).toContainText("1.1.25");
  expect(diagnosticRequests).toEqual(["/api/device-diagnostics/preview"]);await expect(dialog.locator("[data-diagnostics-start]")).toBeDisabled();
  if(evidence)await page.screenshot({path:join(evidence,`${language}-${width}-preview.png`)});
  await dialog.locator("[data-diagnostics-read-consent]").check();await dialog.locator("[data-diagnostics-start]").click();
  await expect(dialog.locator("[data-diagnostics-result]")).toBeVisible();await expect(dialog.locator("[data-diagnostics-review]")).toContainText(m["diagnostics.simulated"]);
  // A clean run must not show the failure banner (in-flight progress was once refused).
  await expect(dialog.getByRole("alert")).toHaveCount(0);
  await expect(dialog.locator("[data-diagnostics-save]")).toBeDisabled();await dialog.locator("[data-diagnostics-share-consent]").check();
  const download=page.waitForEvent("download");await dialog.locator("[data-diagnostics-save]").click();const zip=await download;
  expect(zip.suggestedFilename()).toMatch(/^knxbench-device-diagnostics-[a-f0-9]{12}\.zip$/);
  if(evidence){mkdirSync(join(evidence,"exports"),{recursive:true});await zip.saveAs(join(evidence,"exports",`${language}-${width}.zip`));await page.screenshot({path:join(evidence,`${language}-${width}-result.png`)});}
  const box=await dialog.boundingBox();expect(box!.x).toBeGreaterThanOrEqual(0);expect(box!.x+box!.width).toBeLessThanOrEqual(width+1);
  expect(await page.evaluate(()=>document.documentElement.scrollWidth-innerWidth)).toBeLessThanOrEqual(1);
  expect(diagnosticRequests.filter(p=>p.endsWith("/start"))).toHaveLength(1);expect(diagnosticRequests.filter(p=>p.endsWith("/export"))).toHaveLength(1);
 });
}
test("cooperative stop after a real simulator response retains a downloadable partial",async({page})=>{
 await page.goto("/");await page.locator(".file-menu > summary").click();await openFileSubmenu(page,"help");await page.locator(".file-menu [data-diagnostics-open]").click();
 const dialog=page.locator(".diagnostic-export");await dialog.locator("[data-diagnostics-device]").selectOption("1");await dialog.locator("[data-diagnostics-gateway]").fill("192.0.2.10:3671");
 const previewResponse=page.waitForResponse(r=>r.url().endsWith("/api/device-diagnostics/preview"));await dialog.locator("[data-diagnostics-preview]").click();const preview=await(await previewResponse).json();
 await dialog.locator("[data-diagnostics-read-consent]").check();await dialog.locator("[data-diagnostics-start]").click();
 await expect.poll(async()=>{const r=await page.request.get(`/api/device-diagnostics/${preview.token}`);return(await r.json()).report.items[0].status;},{intervals:[50],timeout:10000}).toBe("read");
 await dialog.locator("[data-diagnostics-cancel]").click();await expect(dialog.locator("[data-diagnostics-result]")).toContainText("cancelled");
 const final=await(await page.request.get(`/api/device-diagnostics/${preview.token}`)).json();expect(final.report.items[0].bytes).toEqual([7,1]);expect(final.report.items.some((i:{status:string})=>i.status!=="read")).toBe(true);expect(final.report.simulated).toBe(true);
 await dialog.locator("[data-diagnostics-share-consent]").check();const promise=page.waitForEvent("download");await dialog.locator("[data-diagnostics-save]").click();const zip=await promise;
 if(evidence){await zip.saveAs(join(evidence,"exports","cancelled-partial.zip"));await page.screenshot({path:join(evidence,"cancelled-partial.png")});}
});
