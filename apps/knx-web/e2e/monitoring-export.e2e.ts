/** File-menu monitoring consent with intercepted API responses, not receiver certification. */
import {expect,test,type Page} from "@playwright/test";
import {join} from "node:path";
import {tree,preview,request} from "../src/monitorExport.test-fixtures";
import {messages as en} from "../src/messages/en";
import {messages as de} from "../src/messages/de";
import {achievementsFixtureAnswer} from "./achievements-fixture";
import {statusMetricsFixtureBody} from "./status-metrics-fixture";
import {openFileSubmenu} from "./file-menu";
const CAPTURE=process.env.KNX_MONITOR_EXPORT_CAPTURE_DIR;
async function serve(page:Page,language:"en"|"de") {
  const unexpected:string[]=[];const posts:{path:string;body:unknown}[]=[];let open=false;
  await page.route("https://api.github.com/**",route=>{unexpected.push(route.request().url());return route.abort();});
  await page.route("**/api/**",route=>{
    const path=new URL(route.request().url()).pathname;const method=route.request().method();
    const ok=(body:unknown)=>route.fulfill({status:200,contentType:"application/json",body:JSON.stringify(body)});
    const achievement=achievementsFixtureAnswer(method,path);if(achievement!==undefined)return ok(achievement);
    if(path==="/api/auth/status"&&method==="GET")return ok({required:false,authenticated:true});
    if(path==="/api/settings"&&method==="GET")return ok({schemaVersion:1,status:"ok",settings:{uiLanguage:language,theme:"porcelain",motionLevel:"off",releaseChecksEnabled:false,achievementsEnabled:"false",onboardingGuide:{seenStage:"alpha",version:"0.1.0-alpha.14"}}});
    if(path==="/api/settings"&&method==="PUT")return ok({schemaVersion:1,status:"ok",settings:route.request().postDataJSON().settings});
    if(path==="/api/project"&&method==="GET")return open?ok(tree()):route.fulfill({status:400,contentType:"application/json",body:'{"error":"no project open"}'});
    if(path==="/api/project/new"&&method==="POST"){open=true;return ok(tree());}
    if(path==="/api/project/status-metrics"&&method==="GET")return ok(statusMetricsFixtureBody(tree()));
    if(path==="/api/project/load-progress"&&method==="GET")return ok(null);
    if(path==="/api/log"&&method==="GET")return ok([]);
    if(path==="/api/legacy-password"&&method==="GET")return ok({available:true,remembered:false,problem:null});
    if(path==="/api/bus/discover"&&method==="POST")return ok({interfaces:[]});
    if(method==="GET"&&["/api/product-languages","/api/catalog/manufacturers","/api/catalog/items"].includes(path))return ok([]);
    if(method==="GET"&&["/api/device-download/status","/api/device-address/status"].includes(path))return route.fulfill({status:404,contentType:"application/json",body:'{"error":"no fixture operation"}'});
    if(method==="POST"&&path==="/api/project/monitor-export/preview"){posts.push({path,body:route.request().postDataJSON()});return ok(preview());}
    if(method==="POST"&&path==="/api/project/monitor-export/download"){posts.push({path,body:route.request().postDataJSON()});return route.fulfill({status:409,contentType:"application/json",body:'{"error":"Preview no longer current"}'});}
    unexpected.push(`${method} ${path}`);return route.fulfill({status:404,contentType:"application/json",body:'{"error":"strict monitoring export fixture"}'});
  });
  return {unexpected,posts};
}
for(const language of ["en","de"] as const)for(const width of [1440,400]) {
  test(`${language}: File monitoring preview and visible stale refusal at ${width}px`,async({page})=>{
    const m=language==="en"?en:de;await page.setViewportSize({width,height:900});const state=await serve(page,language);
    await page.goto("/");await page.getByRole("button",{name:m["toolbar.newProject"],exact:true}).first().click();
    await page.getByRole("dialog",{name:m["newProject.title"],exact:true}).getByRole("button",{name:m["newProject.create"],exact:true}).click();
    await page.getByRole("dialog",{name:m["projectWizard.done.title"],exact:true}).getByRole("button",{name:m["projectWizard.done.close"],exact:true}).click();
    await expect(page.locator("html")).toHaveAttribute("lang",language);await expect(page.locator("html")).toHaveAttribute("data-theme","porcelain");
    await page.locator(".file-menu > summary").click();await openFileSubmenu(page,"exchange");
    await page.getByRole("menuitem",{name:m["monitorExport.button"],exact:true}).click();
    const dialog=page.getByRole("dialog",{name:m["monitorExport.title"],exact:true});await expect(dialog).toBeVisible();
    await expect(dialog.getByRole("button",{name:m["monitorExport.cancel"],exact:true})).toBeFocused();
    expect(state.posts).toEqual([]);await expect(dialog.getByRole("button",{name:m["monitorExport.download"],exact:true})).toBeDisabled();
    const box=(await dialog.boundingBox())!;expect(box.x).toBeGreaterThanOrEqual(0);expect(box.x+box.width).toBeLessThanOrEqual(width+1);expect(box.y+box.height).toBeLessThanOrEqual(901);
    if(width===1440)expect(box.height,"consent should size to its content, not stretch to viewport height").toBeLessThan(650);
    expect(await dialog.evaluate(node=>node.scrollWidth-node.clientWidth)).toBeLessThanOrEqual(1);
    if(CAPTURE)await dialog.screenshot({path:join(CAPTURE,`${language}-${width}-consent.png`)});
    await dialog.getByRole("button",{name:m["monitorExport.preview"],exact:true}).click();
    await expect(dialog.locator("tbody tr")).toHaveCount(5);await expect(dialog.locator(".monitoring-export-issues")).toContainText("data_secure");
    // A real notification can otherwise cover the narrow modal's action footer.
    await page.evaluate(()=>{const toast=document.createElement("div");toast.className="toast-stack";toast.dataset.monitorTest="notification";
      toast.innerHTML='<div class="toast toast--fun">Synthetic notification: a long existing project message must not cover the monitoring export actions on a narrow screen.</div>';document.body.append(toast);});
    const footerClear=await dialog.locator("footer button:last-child").evaluate(node=>{const b=node.getBoundingClientRect();const target=document.elementFromPoint(b.x+b.width/2,b.y+b.height/2);return !!target&&node.contains(target);});
    expect(footerClear,"notification must not cover modal actions").toBe(true);
    const layers=await dialog.evaluate(node=>({modal:Number(getComputedStyle(node.parentElement!).zIndex),notification:Number(getComputedStyle(document.querySelector('[data-monitor-test="notification"]')!).zIndex)}));
    expect(layers.modal,"modal paint layer must be above background notifications, even when they are inert").toBeGreaterThan(layers.notification);
    if(language==="de"&&width===400) {
      const counts=dialog.locator(".monitoring-export-counts");await expect(counts).toHaveCount(1);
      expect(await counts.evaluate(node=>node.scrollWidth>node.clientWidth)).toBe(true);
      await expect(counts.locator("th").first()).toHaveCSS("white-space","nowrap");
    }
    expect(state.posts).toEqual([{path:"/api/project/monitor-export/preview",body:request}]);
    const download=dialog.getByRole("button",{name:m["monitorExport.download"],exact:true});await expect(download).toBeEnabled();await download.scrollIntoViewIfNeeded();
    await dialog.locator("section h3").scrollIntoViewIfNeeded();
    if(CAPTURE)await dialog.screenshot({path:join(CAPTURE,`${language}-${width}-report.png`)});
    await dialog.locator(".monitoring-export-issues li").last().scrollIntoViewIfNeeded();
    if(CAPTURE)await dialog.screenshot({path:join(CAPTURE,`${language}-${width}-diagnostics.png`)});
    await download.click();await expect(dialog.getByRole("alert").filter({hasText:"Preview no longer current"})).toBeVisible();await expect(download).toBeDisabled();
    expect(state.posts).toEqual([{path:"/api/project/monitor-export/preview",body:request},{path:"/api/project/monitor-export/download",body:{...request,bundleSha256:preview().bundleSha256}}]);
    await dialog.getByRole("button",{name:m["monitorExport.cancel"],exact:true}).click();await expect(dialog).toHaveCount(0);await expect(page.locator(".file-menu > summary")).toBeFocused();
    expect(state.unexpected).toEqual([]);
  });
}
