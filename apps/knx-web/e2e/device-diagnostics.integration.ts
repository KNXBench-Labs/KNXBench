/** Real production UI → real HTTP handlers → regular KNX simulator → report-wizard ZIP download. */
import {test,expect,type Download,type Page} from "@playwright/test";
import {join} from "node:path";
import {mkdirSync,readFileSync} from "node:fs";
import {messages as en} from "../src/messages/en";
import {messages as de} from "../src/messages/de";
import { openFileSubmenu } from "./file-menu";
const evidence=process.env.KNX_DIAG_EVIDENCE;
/** DIAG-03: a finished read is reported through the one wizard; returns the saved bundle's bytes. */
async function saveThroughWizard(page:Page,m:Record<keyof typeof en,string>,token:string|null):Promise<{name:string;bytes:Buffer;zip:Download}>{
 const bundles:{operation?:unknown}[]=[];page.on("request",r=>{if(r.url().endsWith("/api/diagnostics/bundle/preview"))bundles.push(r.postDataJSON());});
 await page.locator("[data-report-problem-open=diagnostics]").click();
 const wizard=page.getByRole("dialog",{name:m["reportProblem.title"],exact:true});await expect(wizard).toBeVisible();
 await expect(wizard.locator("[data-report-section=operation] input")).toBeChecked();
 if(token)expect(bundles[0]?.operation).toEqual({relatedKey:"diagnosticsToken",relatedValue:token});
 await wizard.locator("[data-report-next]").click();await wizard.locator("[data-report-next]").click();
 await expect(wizard.locator("[data-report-save]")).toBeDisabled();await wizard.locator("[data-report-consent]").check();
 const download=page.waitForEvent("download");await wizard.locator("[data-report-save]").click();const zip=await download;
 const bytes=readFileSync(await zip.path());
 // Central-directory names are stored uncompressed: the bundle carries the read's recording.
 for(const name of ["manifest.json","recording.jsonl","environment.json"])expect(bytes.includes(Buffer.from(name))).toBe(true);
 await expect(wizard.locator("a[data-report-mail]")).toBeFocused();
 return {name:zip.suggestedFilename(),bytes,zip};
}
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
  const box=await dialog.boundingBox();expect(box!.x).toBeGreaterThanOrEqual(0);expect(box!.x+box!.width).toBeLessThanOrEqual(width+1);
  if(evidence)await page.screenshot({path:join(evidence,`${language}-${width}-result.png`)});
  const {name,zip}=await saveThroughWizard(page,m,null);
  expect(name).toMatch(/^knxbench-diagnostics-.+\.zip$/);
  if(evidence){mkdirSync(join(evidence,"exports"),{recursive:true});await zip.saveAs(join(evidence,"exports",`${language}-${width}.zip`));}
  expect(await page.evaluate(()=>document.documentElement.scrollWidth-innerWidth)).toBeLessThanOrEqual(1);
  expect(diagnosticRequests.filter(p=>p.endsWith("/start"))).toHaveLength(1);expect(diagnosticRequests.filter(p=>p.endsWith("/export"))).toHaveLength(0);
 });
}
test("cooperative stop after a real simulator response is reported through the wizard",async({page})=>{
 await page.goto("/");await page.locator(".file-menu > summary").click();await openFileSubmenu(page,"help");await page.locator(".file-menu [data-diagnostics-open]").click();
 const dialog=page.locator(".diagnostic-export");await dialog.locator("[data-diagnostics-device]").selectOption("1");await dialog.locator("[data-diagnostics-gateway]").fill("192.0.2.10:3671");
 const previewResponse=page.waitForResponse(r=>r.url().endsWith("/api/device-diagnostics/preview"));await dialog.locator("[data-diagnostics-preview]").click();const preview=await(await previewResponse).json();
 await dialog.locator("[data-diagnostics-read-consent]").check();await dialog.locator("[data-diagnostics-start]").click();
 await expect.poll(async()=>{const r=await page.request.get(`/api/device-diagnostics/${preview.token}`);return(await r.json()).report.items[0].status;},{intervals:[50],timeout:10000}).toBe("read");
 await dialog.locator("[data-diagnostics-cancel]").click();await expect(dialog.locator("[data-diagnostics-result]")).toContainText("cancelled");
 const final=await(await page.request.get(`/api/device-diagnostics/${preview.token}`)).json();expect(final.report.items[0].bytes).toEqual([7,1]);expect(final.report.items.some((i:{status:string})=>i.status!=="read")).toBe(true);expect(final.report.simulated).toBe(true);
 if(evidence)await page.screenshot({path:join(evidence,"cancelled-partial.png")});
 const {zip}=await saveThroughWizard(page,en,preview.token);
 if(evidence){mkdirSync(join(evidence,"exports"),{recursive:true});await zip.saveAs(join(evidence,"exports","cancelled-partial.zip"));}
});
// System-2 memory profile (ADR-0143): device 2 maps to the synthetic product database's
// program; the example routes gateway 192.0.2.11 to its simulator. Only declared ranges are read.
for(const [language,width] of [["en",1440],["de",400]] as const){
 test(`${language} memory profile reads declared ranges and reports them through the wizard ${width}px`,async({page})=>{
  const m=language==="de"?de:en;await page.setViewportSize({width,height:900});
  await page.addInitScript(lang=>localStorage.setItem("uiLanguage",lang),language);
  await page.route("**/api/settings",async route=>{if(route.request().method()!=="GET"){await route.continue();return;}const response=await route.fetch();const body=await response.json();body.settings.uiLanguage=language;await route.fulfill({response,json:body});});
  const previews:unknown[]=[];page.on("request",r=>{if(r.url().endsWith("/api/device-diagnostics/preview"))previews.push(r.postDataJSON());});
  await page.goto("/");await page.locator(".file-menu > summary").click();await openFileSubmenu(page,"help");await page.locator(".file-menu [data-diagnostics-open]").click();
  const dialog=page.getByRole("dialog",{name:m["diagnostics.title"],exact:true});await expect(dialog).toBeVisible();
  await dialog.locator("[data-diagnostics-device]").selectOption("1");await dialog.locator("[data-diagnostics-gateway]").fill("192.0.2.11:3671");
  await dialog.locator("[data-diagnostics-profile]").selectOption("system2-declared-segments-v1");
  await expect(dialog.locator("[data-diagnostics-profile-hint]")).toHaveText(m["diagnostics.profileHint.memory"]);
  // Device 1 names no application: the real server refuses before any tunnel, with translated copy.
  await dialog.locator("[data-diagnostics-preview]").click();
  await expect(dialog.locator("[data-diagnostics-error]")).toContainText(m["diagnostics.refusal.programMissing"]);
  await expect(dialog.locator("[data-diagnostics-review]")).toHaveCount(0);
  await dialog.locator("[data-diagnostics-device]").selectOption("2");await expect(dialog.locator("[data-diagnostics-error]")).toHaveCount(0);
  await dialog.locator("[data-diagnostics-preview]").click();
  const review=dialog.locator("[data-diagnostics-review]");await expect(review).toContainText("1.1.26");
  await expect(dialog.locator("[data-diagnostics-ranges] li")).toHaveText([/^0116h · 30 /,/^3C00h · 12 /]);
  await expect(dialog.locator("[data-diagnostics-total]")).toContainText("42");
  await expect(review).toContainText("M-0001_A-0001-01-0000");
  expect(previews.map(p=>(p as {profile:string;deviceId:number})).map(p=>[p.deviceId,p.profile])).toEqual([[1,"system2-declared-segments-v1"],[2,"system2-declared-segments-v1"]]);
  if(evidence)await page.screenshot({path:join(evidence,`memory-${language}-${width}-preview.png`),fullPage:true});
  await expect(dialog.locator("[data-diagnostics-start]")).toBeDisabled();
  await dialog.locator("[data-diagnostics-read-consent]").check();await dialog.locator("[data-diagnostics-start]").click();
  const result=dialog.locator("[data-diagnostics-result]");await expect(result).toBeVisible({timeout:20000});
  await expect(dialog.getByRole("alert")).toHaveCount(0);
  await expect(dialog.locator("[data-diagnostics-coverage] li")).toHaveCount(2);
  const coverage=await dialog.locator("[data-diagnostics-coverage] li").allTextContents();
  expect(coverage[0]).toMatch(/^0116h · 30 \S+: 30 /);expect(coverage[1]).toMatch(/^3C00h · 12 \S+: 12 /);
  await expect(review).toContainText(m["diagnostics.simulated"]);
  const box=await dialog.boundingBox();expect(box!.x).toBeGreaterThanOrEqual(0);expect(box!.x+box!.width).toBeLessThanOrEqual(width+1);
  if(evidence){await result.scrollIntoViewIfNeeded();await page.screenshot({path:join(evidence,`memory-${language}-${width}-result.png`),fullPage:true});}
  const {zip}=await saveThroughWizard(page,m,null);
  if(evidence){mkdirSync(join(evidence,"exports"),{recursive:true});await zip.saveAs(join(evidence,"exports",`memory-${language}-${width}.zip`));}
  expect(await page.evaluate(()=>document.documentElement.scrollWidth-innerWidth)).toBeLessThanOrEqual(1);
 });
}
test("memory profile stop after the first chunk keeps a translated reason and reports the partial",async({page})=>{
 await page.goto("/");await page.locator(".file-menu > summary").click();await openFileSubmenu(page,"help");await page.locator(".file-menu [data-diagnostics-open]").click();
 const dialog=page.locator(".diagnostic-export");await dialog.locator("[data-diagnostics-device]").selectOption("2");await dialog.locator("[data-diagnostics-gateway]").fill("192.0.2.11:3671");
 await dialog.locator("[data-diagnostics-profile]").selectOption("system2-declared-segments-v1");
 const previewResponse=page.waitForResponse(r=>r.url().endsWith("/api/device-diagnostics/preview"));await dialog.locator("[data-diagnostics-preview]").click();const preview=await(await previewResponse).json();
 await dialog.locator("[data-diagnostics-read-consent]").check();await dialog.locator("[data-diagnostics-start]").click();
 // The example answers each frame after 250 ms: Stop lands while a later chunk is still pending.
 await expect.poll(async()=>{const r=await page.request.get(`/api/device-diagnostics/${preview.token}`);return(await r.json()).memory.chunks[0].status;},{intervals:[50],timeout:15000}).toBe("read");
 await dialog.locator("[data-diagnostics-cancel]").click();
 const reason=dialog.locator("[data-diagnostics-stop-reason]");await expect(reason).toContainText(en["diagnostics.stopReason.cancelled"],{timeout:15000});
 await expect(reason.locator("code")).toHaveText("cancelled");
 const final=await(await page.request.get(`/api/device-diagnostics/${preview.token}`)).json();
 expect(final.report.stopReason).toBe("cancelled");expect(final.exportable).toBe(true);
 expect(final.memory.chunks[0].bytes).toBe("0102030405060708090a0b0c");
 expect(final.memory.chunks.some((c:{status:string})=>c.status!=="read")).toBe(true);
 if(evidence)await page.screenshot({path:join(evidence,"memory-cancelled-partial.png"),fullPage:true});
 const {zip}=await saveThroughWizard(page,en,preview.token);
 if(evidence){mkdirSync(join(evidence,"exports"),{recursive:true});await zip.saveAs(join(evidence,"exports","memory-cancelled-partial.zip"));}
});
