/** Actual modeled metadata read stays explicit and survives native Save As/reopen byte-exactly. */
import {test,expect} from "@playwright/test";
import {readFile,mkdir} from "node:fs/promises";
import {join} from "node:path";
import {messages as en} from "../src/messages/en";
import {messages as de} from "../src/messages/de";

for(const language of ["en","de"] as const)for(const width of [400,1440]){
 test(`${language} actual modeled project metadata at ${width}px`,async({page,request})=>{
  const m=language==="de"?de:en;const errors:string[]=[];let reads=0;
  const name="  Fictional <script>metadata()</script> project  ";
  const created=await request.post("/api/project/new",{data:{name,installationName:"Fictional installation"}});expect(created.status()).toBe(200);
  const before=(await (await request.get("/api/project/metadata")).json()).metadata;
  expect(before.name).toBe(name);expect(before.project_number).toBeNull();
  const filename=`metadata-${language}-${width}.knxdb`;
  expect((await request.post("/api/project/save-as",{data:{path:filename}})).status()).toBe(200);
  expect((await request.post("/api/project/open",{data:{path:filename}})).status()).toBe(200);
  const reopened=await (await request.get("/api/project/metadata")).json();expect(reopened.metadata).toEqual(before);
  const initial=await (await request.get("/api/project")).json();
  const nativePath=join(process.env.KNX_METADATA_DATA_DIR!,filename);const original=await readFile(nativePath);
  page.on("pageerror",error=>errors.push(error.message));
  page.on("request",req=>{if(new URL(req.url()).pathname==="/api/project/metadata") {expect(req.method()).toBe("GET");reads++;}});
  await page.route("**/api/bus/**",route=>{expect(new URL(route.request().url()).pathname).toBe("/api/bus/discover");return route.fulfill({status:200,contentType:"application/json",body:JSON.stringify({interfaces:[]})});});
  await page.route("**/api/settings",route=>{expect(route.request().method()).toBe("GET");return route.fulfill({status:200,contentType:"application/json",body:JSON.stringify({schemaVersion:1,status:"ok",settings:{uiLanguage:language,theme:language==="en"?"graphite":"porcelain"}})});});
  await page.setViewportSize({width,height:900});await page.goto("/?view=editor");
  const panel=page.locator(".project-metadata-panel");await expect(panel).toBeVisible();expect(reads).toBe(0);
  await panel.getByRole("button",{name:m["projectMetadata.show"],exact:true}).click();
  await expect(panel).toContainText(m["projectMetadata.explanation"]);await expect(panel).toContainText(m["projectMetadata.dates"]);
  await expect(panel.locator("dd").nth(1)).toHaveText(name);expect(await panel.locator("dd").nth(1).textContent()).toBe(name);await expect(panel.locator("dd").nth(2)).toHaveText(m["projectMetadata.absent"]);
  await expect(panel).toContainText(m["projectMetadata.completion.Undefined"]);expect(reads).toBe(1);
  expect(await panel.locator("script,input,textarea").count()).toBe(0);
  const box=await panel.boundingBox();expect(box).not.toBeNull();expect(box!.x).toBeGreaterThanOrEqual(0);expect(box!.x+box!.width).toBeLessThanOrEqual(width+1);
  expect(await page.evaluate(()=>document.documentElement.scrollWidth-document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
  if(process.env.KNX_EVIDENCE_CAPTURE_DIR){await mkdir(process.env.KNX_EVIDENCE_CAPTURE_DIR,{recursive:true});await panel.screenshot({path:join(process.env.KNX_EVIDENCE_CAPTURE_DIR,`metadata-${language}-${width}.png`)});}
  expect(await (await request.get("/api/project")).json()).toEqual(initial);expect(await readFile(nativePath)).toEqual(original);expect(errors).toEqual([]);
 });
}
