/** Universal support reporting at the real shell, with every API call intercepted. */
import {test,expect} from "@playwright/test";
import {join} from "node:path";
import {messages as en} from "../src/messages/en";
import {messages as de} from "../src/messages/de";
import {achievementsFixtureAnswer} from "./achievements-fixture";
import { openFileSubmenu } from "./file-menu";

for(const language of ["en","de"] as const)for(const width of [1440,400]){
 test(`${language} unknown device support preview and deliberate handoff ${width}px`,async({page})=>{
  const m=language==="de"?de:en;const unexpected:string[]=[];const calls:string[]=[];
  const report={kind:"knxbench-support-report",format:1,knxbenchVersion:"fixture",support:"unknown",written:false,facts:[],context:null,missingEvidence:["product-identity","hardware-verification"],withheld:["credentials","raw-memory-and-binaries"],priorRun:null};
  await page.setViewportSize({width,height:900});
  await page.route("**/api/**",async route=>{
   const path=new URL(route.request().url()).pathname;const method=route.request().method();calls.push(`${method} ${path}`);let body:unknown;let status=200;
   if(path==="/api/auth/status")body={required:false,authenticated:true};
   else if(path==="/api/settings"&&method==="GET")body={schemaVersion:1,status:"ok",settings:{uiLanguage:language,theme:"porcelain"}};
   else if(path==="/api/project"){status=400;body={error:"no project open"};}
   else if(path==="/api/project/load-progress")body=null;
   else if(path==="/api/log")body=[];
   else if(path==="/api/legacy-password")body={available:true,remembered:false,problem:null};
   else if(path==="/api/product-languages")body=[];
   else if(path==="/api/bus/discover")body={interfaces:[]};
   else if(["/api/device-download/status","/api/device-address/status"].includes(path)){status=404;body={error:"no operation"};}
   else if(path==="/api/support/preview"){expect(route.request().postDataJSON()).toMatchObject({deviceId:null,input:{includeContext:false,contextRights:false}});body={token:"a".repeat(64),digest:"b".repeat(64),report,summary:"Unknown device: product identity and hardware evidence unavailable. No download requested.",localAddress:null,mask:null,profile:"descriptor-only-v1",reads:["DeviceDescriptorRead type 0"],deadlineSeconds:30};}
   else if(achievementsFixtureAnswer(method,path)!==undefined)body=achievementsFixtureAnswer(method,path);
   else {unexpected.push(`${method} ${path}`);status=404;body={error:"strict support fixture"};}
   await route.fulfill({status,contentType:"application/json",body:JSON.stringify(body)});
  });
  await page.goto("/");await page.locator(".file-menu > summary").click();
  await openFileSubmenu(page,"help");
  await page.locator(".file-menu [data-support-open]").click();
  const dialog=page.getByRole("dialog",{name:m["support.title"],exact:true});await expect(dialog).toBeVisible();
  expect(calls.some(x=>x.includes("/api/support/"))).toBe(false);
  if(process.env.KNX_SUPPORT_CAPTURE_DIR)await page.screenshot({path:join(process.env.KNX_SUPPORT_CAPTURE_DIR,`support-${language}-${width}-setup.png`)});
  await dialog.locator("[data-support-preview]").click();await expect(dialog.locator("[data-support-preview-content]")).toContainText("Unknown device");
  await expect(dialog.locator("[data-support-save]")).toBeDisabled();
  await dialog.locator("[data-support-export-consent]").check();await expect(dialog.locator("[data-support-save]")).toBeEnabled();
  expect(await dialog.locator("a").evaluateAll(nodes=>nodes.map(n=>(n as HTMLAnchorElement).href))).toEqual(["https://github.com/KNXBench-Labs/KNXBench/issues/new?template=analysis.yml","mailto:issue@knxbench.com?subject=KNXBench%20device%20support%20report"]);
  await dialog.locator("[data-support-program]").fill("M-0001_A-9205-04-9270");await expect(dialog.locator("[data-support-preview-content]")).toHaveCount(0);
  await dialog.locator("[data-support-preview]").click();await expect(dialog.locator("[data-support-preview-content]")).toBeVisible();
  await expect(dialog.locator("[data-support-export-consent]")).not.toBeChecked();
  expect(await page.evaluate(()=>document.documentElement.scrollWidth-innerWidth)).toBeLessThanOrEqual(1);
  const box=await dialog.boundingBox();expect(box).not.toBeNull();expect(box!.x).toBeGreaterThanOrEqual(0);expect(box!.x+box!.width).toBeLessThanOrEqual(width+1);
  await dialog.locator("[data-support-preview-content]").scrollIntoViewIfNeeded();
  if(process.env.KNX_SUPPORT_CAPTURE_DIR)await page.screenshot({path:join(process.env.KNX_SUPPORT_CAPTURE_DIR,`support-${language}-${width}.png`)});
  await dialog.getByRole("button",{name:m["support.close"],exact:true}).click();await expect(dialog).toHaveCount(0);
  expect(calls.filter(x=>x.includes("/api/support/diagnose")||x.includes("/api/support/export"))).toEqual([]);expect(unexpected).toEqual([]);
 });
}
