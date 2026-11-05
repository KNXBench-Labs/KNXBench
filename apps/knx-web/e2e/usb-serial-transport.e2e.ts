/** Exercise explicit serial sessions through the full app with strict offline HTTP fixtures. */
import { expect, test, type Page } from "@playwright/test";
import { isDeepStrictEqual } from "node:util";
import type { BusMonitorStartResponse, BusMonitorTelegramsResponse } from "../src/api";
import type { ActivitySnapshot } from "../src/liveActivity";
import { validSerialTarget, type SerialTarget } from "../src/serialTransport";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { dismissToasts, intercept, openProject } from "./communication-fixture";

const serial: SerialTarget = {path:"/synthetic/tpuart",source:"1.1.250",mode:"passive"};
async function serve(page: Page, language: "en"|"de") {
  const state = await intercept(page,language);
  const starts: unknown[] = [], stops: string[] = [], ipStarts: unknown[] = [], settings: Record<string,unknown> = {
    uiLanguage:language, theme:"porcelain", preferredGateway:"192.0.2.10:3671", releaseChecksEnabled:false,
  };
  const reads: unknown[] = [], writes: unknown[] = [], requests: string[] = [];
  page.on("request",request=>{const url=new URL(request.url());if(url.pathname.startsWith("/api/"))requests.push(`${request.method()} ${url.pathname}`);});
  let connected: SerialTarget | null = null;
  await page.route("https://api.github.com/**",route=>{state.unexpected.push("external release check"); return route.abort();});
  await page.route("**/api/settings",async route=>{
    if(route.request().method()==="PUT") {
      const patch: unknown=route.request().postDataJSON();
      if(patch && typeof patch === "object" && "settings" in patch && patch.settings && typeof patch.settings === "object") Object.assign(settings,patch.settings);
    } else if(route.request().method()!=="GET") {state.unexpected.push("settings method");}
    return route.fulfill({json:{schemaVersion:1,status:"ok",settings}});
  });
  await page.route("**/api/bus/activity",route=>{
    if(route.request().method()!=="GET") state.unexpected.push("activity method");
    const snapshot: ActivitySnapshot={serverIncarnation:"serial-fixture",coverage:"partial",historyState:"disabled",sessions:connected?[{kind:"busMonitor",id:17,state:"active",serial:connected}]:[],oneShot:[],oneShotDropped:0,busyLocks:[],untracked:[]};
    return route.fulfill({json:snapshot});
  });
  await page.route("**/api/bus/monitor/start-serial",route=>{
    const payload: unknown=route.request().postDataJSON(); starts.push(payload);
    if(route.request().method()!=="POST" || !validSerialTarget(payload)) {state.unexpected.push("invalid serial start"); return route.fulfill({status:400,json:{error:"invalid target"}});}
    connected=payload;
    const response: BusMonitorStartResponse={sessionId:17,serverIncarnation:"serial-fixture",assignedAddress:payload.source,serial:payload};
    return route.fulfill({json:response});
  });
  await page.route("**/api/bus/monitor/start",route=>{ipStarts.push(route.request().postDataJSON());return route.fulfill({status:502,json:{error:"synthetic IP unavailable"}});});
  await page.route("**/api/bus/monitor/telegrams**",route=>{
    if(route.request().method()!=="GET") state.unexpected.push("poll method");
    if(!connected) return route.fulfill({status:404,json:{error:"no session"}});
    const response: BusMonitorTelegramsResponse={sessionId:17,serverIncarnation:"serial-fixture",serial:connected,status:"active",contextStatus:"current",projectOpen:true,nextSince:0,droppedBefore:0,telegrams:[]};
    return route.fulfill({json:response});
  });
  await page.route("**/api/bus/monitor/stop-bound**",route=>{
    if(route.request().method()!=="POST") state.unexpected.push("stop method");
    stops.push(new URL(route.request().url()).search); connected=null;
    return route.fulfill({json:{sessionId:17,serverIncarnation:"serial-fixture",telegramCount:0,droppedCount:0}});
  });
  await page.route("**/api/bus/read",route=>{
    const payload: unknown=route.request().postDataJSON();reads.push(payload);
    const expected={sessionId:17,serverIncarnation:"serial-fixture",destination:"1/1/1"};
    if(route.request().method()!=="POST" || connected?.mode!=="active" || !isDeepStrictEqual(payload,expected)) {
      state.unexpected.push("invalid bound group read");return route.fulfill({status:400,json:{error:"invalid bound read"}});
    }
    return route.fulfill({json:{service:"GroupValueRead"}});
  });
  await page.route("**/api/bus/write-bound",route=>{
    const payload: unknown=route.request().postDataJSON();writes.push(payload);
    const expected={sessionId:17,serverIncarnation:"serial-fixture",destination:"1/1/1",dpt:"DPST-1-1",inputFormat:null,value:"on"};
    if(route.request().method()!=="POST" || connected?.mode!=="active" || !isDeepStrictEqual(payload,expected)) {
      state.unexpected.push("invalid bound group write");return route.fulfill({status:400,json:{error:"invalid bound write"}});
    }
    return route.fulfill({json:{service:"GroupValueWrite",encodedPayload:"0x01 (6-bit)",decodedEcho:{kind:"value",dpt:"DPST-1-1",text:"on"}}});
  });
  await page.route("**/api/bus/write",route=>{state.unexpected.push("legacy unbound serial write");return route.fulfill({status:409,json:{error:"serial requires bound write"}});});
  return {state,starts,stops,ipStarts,settings,reads,writes,requests};
}
async function command(page:Page,label:string) {
  await page.keyboard.press("Control+Shift+P"); await page.getByRole("combobox").fill(label); await page.getByRole("combobox").press("Enter");
}
for(const language of ["en","de"] as const) for(const width of [1440,400]) {
  test(`${language} ${width}: explicit passive footer and active monitor, exact targets and bound stop`,async({page})=>{
    const m=language==="en"?en:de;
    await page.setViewportSize({width,height:1000}); await page.emulateMedia({reducedMotion:"reduce"});
    const fixture=await serve(page,language); await openProject(page,language); await dismissToasts(page,language);
    await expect(page.locator(".bus-connection-state")).toContainText(m["busConnection.state.disconnected"]);
    await expect.poll(()=>fixture.requests.filter(request=>request==="POST /api/bus/discover").length).toBe(1);
    const bootstrapDiscovery=fixture.requests.filter(request=>request==="POST /api/bus/discover").length;
    await page.locator(".bus-connection-trigger").click();
    const panel=page.locator(".bus-connection-panel");
    await panel.getByRole("combobox",{name:m["busTransport.label"],exact:true}).selectOption("serial");
    await expect(panel.getByRole("button",{name:m["busConnection.connect"],exact:true})).toBeDisabled();
    await panel.getByLabel(m["busTransport.source"],{exact:true}).fill(serial.source);
    await expect(panel.getByRole("button",{name:m["busConnection.connect"],exact:true})).toBeDisabled(); // Empty path independently refuses.
    await panel.getByLabel(m["busTransport.path"],{exact:true}).fill(serial.path);
    await panel.getByLabel(m["busTransport.source"],{exact:true}).fill("0.0.0");
    await expect(panel.getByRole("button",{name:m["busConnection.connect"],exact:true})).toBeDisabled();
    await panel.getByLabel(m["busTransport.source"],{exact:true}).fill(serial.source);
    await expect(panel).toContainText(m["busTransport.passiveHint"]);
    expect(fixture.starts).toEqual([]); expect(fixture.ipStarts).toEqual([]);
    await panel.getByRole("button",{name:m["busConnection.connect"],exact:true}).click();
    await expect.poll(()=>fixture.starts).toEqual([serial]);
    await expect(page.locator(".bus-connection-state")).toContainText(serial.path);
    await expect(page.locator(".bus-connection-state")).not.toContainText("192.0.2.");
    await page.locator(".bus-connection").scrollIntoViewIfNeeded();
    await page.screenshot({path:test.info().outputPath("serial-footer.png")});
    await page.keyboard.press("Escape");
    await page.locator(".bus-connection-action").click();
    await expect.poll(()=>fixture.stops).toEqual(["?sessionId=17&serverIncarnation=serial-fixture"]);
    await command(page,m["toolbar.busMonitor"]); const monitor=page.locator(".bus-monitor-panel");
    await expect(monitor).toBeVisible(); await dismissToasts(page,language);
    await monitor.getByRole("combobox",{name:m["busTransport.mode"],exact:true}).selectOption("active");
    await expect(monitor).toContainText(m["busTransport.activeWarning"]);
    expect(fixture.starts).toHaveLength(1);
    await monitor.getByRole("button",{name:m["busMonitor.connect"],exact:true}).click();
    await expect.poll(()=>fixture.starts).toEqual([serial,{...serial,mode:"active"}]);
    await expect(monitor.locator(".bus-monitor-session")).toContainText(serial.path);
    const read=monitor.getByRole("button",{name:m["busTransport.read"],exact:true});
    await expect(read).toBeDisabled(); // No destination selected yet, never a spontaneous read.
    expect(fixture.reads).toEqual([]);expect(fixture.writes).toEqual([]);
    await monitor.locator(".bus-compose-destination").fill("1/1/1");
    await read.click();
    await expect.poll(()=>fixture.reads).toEqual([{sessionId:17,serverIncarnation:"serial-fixture",destination:"1/1/1"}]);
    await monitor.locator(".bus-compose-dpt").selectOption("DPST-1-1");
    await monitor.locator(".bus-compose-value").fill("on");
    await monitor.getByRole("button",{name:m["busCompose.send"],exact:true}).click();
    await expect.poll(()=>fixture.writes).toEqual([{sessionId:17,serverIncarnation:"serial-fixture",destination:"1/1/1",dpt:"DPST-1-1",inputFormat:null,value:"on"}]);
    await monitor.locator(".bus-transport-fields").scrollIntoViewIfNeeded();
    await monitor.locator(".bus-transport-fields").screenshot({path:test.info().outputPath("serial-monitor-fields.png")});
    expect(await page.evaluate(()=>document.documentElement.scrollWidth-document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
    await monitor.getByRole("button",{name:m["busMonitor.disconnect"],exact:true}).click();
    await expect.poll(()=>fixture.stops).toEqual(["?sessionId=17&serverIncarnation=serial-fixture","?sessionId=17&serverIncarnation=serial-fixture"]);
    await monitor.getByRole("combobox",{name:m["busTransport.label"],exact:true}).selectOption("ip");
    await expect(monitor.getByLabel(m["busMonitor.gatewayHost"],{exact:true})).toHaveValue("192.0.2.10");
    await expect(monitor.getByLabel(m["busMonitor.gatewayPort"],{exact:true})).toHaveValue("3671");
    await expect.poll(()=>fixture.settings.preferredBusTransport).toBe("ip");
    expect(fixture.settings.preferredGateway).toBe("192.0.2.10:3671");
    expect(fixture.requests.filter(request=>request==="POST /api/bus/discover")).toHaveLength(bootstrapDiscovery);
    expect(fixture.requests.filter(request=>request==="POST /api/bus/write")).toEqual([]);
    expect(fixture.starts).toHaveLength(2); expect(fixture.ipStarts).toEqual([]); expect(fixture.state.unexpected).toEqual([]);
  });
}
