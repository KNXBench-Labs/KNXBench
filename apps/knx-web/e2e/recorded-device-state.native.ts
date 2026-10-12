/** Actual built Inspector and production server/native read proof, entirely synthetic. */
import {test,expect} from "@playwright/test";
import {readFile,mkdir} from "node:fs/promises";import {join} from "node:path";
import {messages as en} from "../src/messages/en";import {messages as de} from "../src/messages/de";
for(const language of ["en","de"] as const)for(const width of [400,1440]){
 test(`${language} actual recorded device state at ${width}px`,async({page,request})=>{
  const m=language==="en"?en:de;const errors:string[]=[];const writes:string[]=[];let reads=0;
  const imported=await request.post("/api/project/import",{data:{path:"device-state.knxproj"}});expect(imported.status(),await imported.text()).toBe(200);
  const tree=await (await request.get("/api/project")).json();const device=tree.installations[0].topology[0].lines[0].devices[0];const id=device.id;
  const before=await (await request.get(`/api/devices/${id}/recorded-state`)).json();
  expect(before.state.name).toBe("  Fictional <script>state()</script> device  ");
  expect(before.state.parts[0].intended_loaded).toBe(true);expect(before.state.parts[1].intended_loaded).toBe(false);expect(before.state.parts.every((part:{reported:unknown})=>part.reported===null)).toBe(true);
  const filename=`device-state-${language}-${width}.knxdb`;
  expect((await request.post("/api/project/save-as",{data:{path:filename}})).status()).toBe(200);
  expect((await request.post("/api/project/open",{data:{path:filename}})).status()).toBe(200);
  const reopened=await (await request.get(`/api/devices/${id}/recorded-state`)).json();expect(reopened.state).toEqual(before.state);
  const initial=await (await request.get("/api/project")).json();const native=join(process.env.KNX_METADATA_DATA_DIR!,filename);const original=await readFile(native);
  page.on("pageerror",error=>errors.push(error.message));page.on("request",req=>{const path=new URL(req.url()).pathname;if(path===`/api/devices/${id}/recorded-state`){expect(req.method()).toBe("GET");reads++;}if(path.startsWith("/api/")&&req.method()!=="GET")writes.push(path);});
  await page.route("**/api/bus/**",route=>{expect(new URL(route.request().url()).pathname).toBe("/api/bus/discover");return route.fulfill({status:200,contentType:"application/json",body:JSON.stringify({interfaces:[]})});});
  await page.route("**/api/settings",route=>{expect(route.request().method()).toBe("GET");return route.fulfill({status:200,contentType:"application/json",body:JSON.stringify({schemaVersion:1,status:"ok",settings:{uiLanguage:language,theme:language==="en"?"graphite":"porcelain",motionLevel:"off",achievementsEnabled:"false"}})});});
  await page.setViewportSize({width,height:1000});await page.goto("/?view=editor");
  await page.getByRole("button",{name:m["toolbar.commands"],exact:true}).click();const palette=page.getByRole("dialog").getByRole("combobox");await expect(palette).toBeVisible();await palette.fill(m["workbench.devices"]);await palette.press("Enter");
  const row=page.locator(`.devices-table [data-rename-id="${id}"]`);await expect(row).toBeVisible();await row.locator("td:nth-child(3) button").click();
  const controls=page.locator(".workbench-panel-controls");const properties=controls.getByRole("button",{name:m["workbench.properties"],exact:true});
  if(await properties.getAttribute("aria-expanded")!=="true")await properties.click();
  const panel=page.locator(".recorded-device-state-panel");await expect(panel).toBeVisible();expect(reads).toBe(0);
  await panel.getByRole("button",{name:m["recordedDevice.show"],exact:true}).click();
  await expect(panel).toContainText(m["recordedDevice.explanation"]);await expect(panel).toContainText(m["recordedDevice.dates"]);
  await expect(panel).toContainText(m["recordedDevice.belief"]);await expect(panel).toContainText(m["recordedDevice.report"]);
  expect(await panel.locator("dd").first().textContent()).toBe(before.state.name);expect(await panel.locator("li").count()).toBe(5);
  await expect(panel).toContainText(m["recordedDevice.absent"]);expect(reads).toBe(1);expect(await panel.locator("script,input,textarea").count()).toBe(0);
  const box=await panel.boundingBox();expect(box).not.toBeNull();expect(box!.x).toBeGreaterThanOrEqual(0);expect(box!.x+box!.width).toBeLessThanOrEqual(width+1);
  expect(await panel.evaluate(el=>el.scrollWidth-el.clientWidth)).toBeLessThanOrEqual(1);
  if(process.env.KNX_EVIDENCE_CAPTURE_DIR){await mkdir(process.env.KNX_EVIDENCE_CAPTURE_DIR,{recursive:true});await panel.screenshot({path:join(process.env.KNX_EVIDENCE_CAPTURE_DIR,`device-state-${language}-${width}.png`)});}
  expect(await (await request.get("/api/project")).json()).toEqual(initial);expect(await readFile(native)).toEqual(original);expect(errors).toEqual([]);expect(writes.filter(path=>path!=="/api/bus/discover")).toEqual([]);
 });
}
