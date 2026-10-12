/** Actual production server exports retained fictional data only after local privacy review. */
import {test,expect} from "@playwright/test";
import {readFile,mkdir} from "node:fs/promises";
import {join} from "node:path";
import {execFileSync} from "node:child_process";
import {messages as en} from "../src/messages/en";
import {messages as de} from "../src/messages/de";

for(const language of ["en","de"] as const)for(const width of [400,1440]){
 test(`${language} reviewed real native export at ${width}px`,async({page,request})=>{
  const m=language==="de"?de:en;const errors:string[]=[];const writes:string[]=[];
  const imported=await request.post("/api/project/import",{data:{path:"fictional.knxproj"}});expect(imported.status()).toBe(200);
  const initial=(await (await request.get("/api/project")).json());
  page.on("pageerror",error=>errors.push(error.message));
  page.on("request",req=>{const url=new URL(req.url());if(url.pathname.startsWith("/api/")&&req.method()!=="GET")writes.push(url.pathname);});
  // Chromium native downloads need not emit page-scoped request events. Observe
  // the actual clicked link without rewriting it; the real download still runs.
  await page.addInitScript(()=>{
   const links:string[]=[];
   (window as unknown as {__nativeExportLinks:string[]}).__nativeExportLinks=links;
   const click=HTMLAnchorElement.prototype.click;
   HTMLAnchorElement.prototype.click=function(){if(new URL(this.href).pathname==="/api/project/download")links.push(this.href);return click.call(this);};
  });
  const links=()=>page.evaluate(()=>(window as unknown as {__nativeExportLinks:string[]}).__nativeExportLinks);
  // No actual KNX discovery/connection request reaches the backend.
  await page.route("**/api/bus/**",route=>{
   expect(new URL(route.request().url()).pathname).toBe("/api/bus/discover");
   return route.fulfill({status:200,contentType:"application/json",body:JSON.stringify({interfaces:[]})});
  });
  await page.route("**/api/settings",route=>{
   expect(route.request().method()).toBe("GET");
   return route.fulfill({status:200,contentType:"application/json",body:JSON.stringify({schemaVersion:1,status:"ok",settings:{uiLanguage:language,theme:language==="en"?"graphite":"porcelain"}})});
  });
  await page.setViewportSize({width,height:900});await page.goto("/?view=editor");
  const summary=page.locator(".file-menu > summary");await summary.click();
  await page.getByRole("button",{name:m["toolbar.exportProject"],exact:true}).click();
  const dialog=page.getByRole("dialog",{name:m["nativeExport.title"],exact:true});
  await expect(dialog).toBeVisible();await expect(dialog).toContainText(m["nativeExport.privacy"]);await expect(dialog).toContainText(m["nativeExport.scope"]);
  await expect(dialog.getByRole("button",{name:m["nativeExport.cancel"],exact:true})).toBeFocused();expect(await links()).toEqual([]);
  // Cancel loses nothing, so it stays quiet next to the primary download (main's dialog convention).
  const fill=(name:string)=>dialog.getByRole("button",{name,exact:true}).evaluate(el=>{const style=getComputedStyle(el);return `${style.backgroundColor} ${style.backgroundImage}`;});
  expect(await fill(m["nativeExport.cancel"])).not.toBe(await fill(m["nativeExport.download"]));
  await dialog.getByRole("button",{name:m["nativeExport.cancel"],exact:true}).click();await expect(dialog).toBeHidden();expect(await links()).toEqual([]);
  await summary.click();await page.getByRole("button",{name:m["toolbar.exportProject"],exact:true}).click();await expect(dialog).toBeVisible();
  const box=await dialog.boundingBox();expect(box).not.toBeNull();expect(box!.x).toBeGreaterThanOrEqual(0);expect(box!.x+box!.width).toBeLessThanOrEqual(width+1);
  const overflow=await dialog.evaluate(el=>el.scrollWidth-el.clientWidth);expect(overflow).toBeLessThanOrEqual(1);
  const footer=await dialog.locator("footer").boundingBox();expect(footer).not.toBeNull();
  expect(box!.y+box!.height-(footer!.y+footer!.height),"confirmation must fit its content instead of stretching like a search-results list").toBeLessThanOrEqual(32);
  if(process.env.KNX_EVIDENCE_CAPTURE_DIR){await mkdir(process.env.KNX_EVIDENCE_CAPTURE_DIR,{recursive:true});await page.screenshot({path:join(process.env.KNX_EVIDENCE_CAPTURE_DIR,`native-export-${language}-${width}.png`)});}
  const event=page.waitForEvent("download");await dialog.getByRole("button",{name:m["nativeExport.download"],exact:true}).click();
  const download=await event;expect(download.suggestedFilename()).toBe("project.knxdb");expect(await download.failure()).toBeNull();const saved=await download.path();expect(saved).not.toBeNull();
  const bytes=await readFile(saved!);expect(bytes.subarray(0,16).toString()).toBe("SQLite format 3\u0000");
  const inspected=JSON.parse(execFileSync("python3",["-c",`import sqlite3,json,sys,pathlib
p=pathlib.Path(sys.argv[1]);c=sqlite3.connect(p.as_uri()+'?mode=ro&immutable=1',uri=True)
r=c.execute("SELECT bytes FROM opaque_entry WHERE source_path=? AND xpath=''",('P-FICTION/UserFiles/attachment.txt',)).fetchall()
assert len(r)==1 and r[0][0]==b'fictional private source attachment'
print(json.dumps({'retained_attachment_equal':True,'source_rows':c.execute('SELECT count(*) FROM opaque_entry').fetchone()[0]}))`,saved!],{encoding:"utf8"}));
  expect(inspected.retained_attachment_equal).toBe(true);expect(inspected.source_rows).toBeGreaterThan(0);
  const clicked=await links();expect(clicked).toHaveLength(1);const url=new URL(clicked[0]);
  expect(url.searchParams.get("serverIncarnation")).toBe(initial.server_incarnation);expect(url.searchParams.get("projectIncarnation")).toBe(String(initial.project_incarnation));expect(url.searchParams.get("snapshotRevision")).toBe(String(initial.snapshot_revision));
  await expect(dialog).toBeHidden();const after=await (await request.get("/api/project")).json();expect(after).toEqual(initial);
  expect(writes.filter(path=>path!=="/api/bus/discover")).toEqual([]);expect(errors).toEqual([]);
 });
}
