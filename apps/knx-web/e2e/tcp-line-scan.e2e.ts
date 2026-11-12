/** Exercise explicit line-scan transport and server-owned identity through the offline Web app. */
import { expect, test, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";
import type { LineScanStartResponse, LineScanResultsResponse } from "../src/api";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
import { intercept, openProject, dismissToasts } from "./communication-fixture";

const estimate = { candidateCount:2, omittedAddresses:["1.1.6"], responseTimeoutMs:2000, vacantConfirmations:1, interProbePauseMs:0, worstCaseMs:4000 };
function captures(): { start:LineScanStartResponse; poll:LineScanResultsResponse; cancel:LineScanResultsResponse } {
  // A proving run supplies the actual socket-HTTP witness; ordinary offline runs stay self-contained.
  const path=process.env.KNXBENCH_SCAN_CAPTURE_DIR;
  if (path) return JSON.parse(readFileSync(`${path}/actual-backend.json`,"utf8"));
  const identity={sessionId:1,serverIncarnation:"scan-browser",transport:"tcp" as const,skippedFrames:2};
  const poll:LineScanResultsResponse={ ...identity,status:"running",error:null,cleanupError:null,disconnectConfirmed:null,nextSince:1,completedCount:1,totalCount:2,omittedAddresses:["1.1.6"],excludedAddresses:["1.1.6"],results:[{address:"1.1.5",outcome:{kind:"occupied",maskVersion:0x07b0}}] };
  return { start:{...identity,estimate},poll,cancel:{...poll,status:"cancelled",disconnectConfirmed:true} };
}
async function serve(page:Page,language:"en"|"de",options:{missing?:boolean; malformed?:boolean}={}) {
  const state=await intercept(page,language); const data=captures();
  const control={active:false,starts:[] as string[],cancels:[] as string[],polls:[] as string[]};
  const settings={uiLanguage:language,theme:"porcelain",preferredGateway:"192.0.2.10:3671",preferredIpTunnelTransport:"tcp",releaseChecksEnabled:false,lineScanExclusions:["1.1.6"]};
  await page.route("https://api.github.com/**",route=>{state.unexpected.push("release check");return route.abort();});
  await page.route("**/api/legacy-password",route=>route.fulfill({json:{available:true,remembered:false,problem:null}}));
  await page.route("**/api/settings",route=>route.fulfill({json:{schemaVersion:1,status:"ok",settings}}));
  await page.route("**/api/bus/monitor/telegrams**",route=>route.fulfill({status:404,json:{error:"no monitor"}}));
  await page.route("**/api/bus/scan/**",async route=>{
    const url=new URL(route.request().url()); const path=url.pathname;
    if (path.endsWith("/estimate")) return route.fulfill({json:estimate});
    if (path.endsWith("/start")||path.endsWith("/start-tcp")) {
      control.starts.push(path);
      if (options.missing) return route.fulfill({status:404,json:{error:"unsupported"}});
      expect(path).toBe("/api/bus/scan/start-tcp");
      expect(route.request().postDataJSON()).toEqual({gateway:"192.0.2.10:3671",area:1,line:1,firstDevice:5,lastDevice:7,excluded:["1.1.6"],responseTimeoutMs:2000,interProbePauseMs:0});
      control.active=true; return route.fulfill({json:data.start});
    }
    if (path.endsWith("/results")) {
      control.polls.push(url.search);
      if (options.malformed) { const invalid:Record<string,unknown>={...data.poll};delete invalid.transport; return route.fulfill({json:invalid}); }
      if (!control.active) return route.fulfill({status:404,json:{error:"no scan"}});
      const bound=url.searchParams.get("sessionId");
      if (bound!==null) {
        expect(bound).toBe(String(data.start.sessionId));expect(url.searchParams.get("serverIncarnation")).toBe(data.start.serverIncarnation);
      }
      return route.fulfill({json:{...data.poll,results:url.searchParams.get("since")==="0"?data.poll.results:[]}});
    }
    if (path.endsWith("/cancel-bound")) {
      control.cancels.push(url.search);
      expect(route.request().method()).toBe("POST"); expect([...url.searchParams.keys()].sort()).toEqual(["serverIncarnation","sessionId"]);
      expect(url.searchParams.get("sessionId")).toBe(String(data.start.sessionId)); expect(url.searchParams.get("serverIncarnation")).toBe(data.start.serverIncarnation);
      return route.fulfill({json:data.cancel});
    }
    state.unexpected.push(`${route.request().method()} ${path}`); return route.fulfill({status:404,json:{error:"unexpected scan action"}});
  });
  return {state,control,data,settings};
}
async function openScan(page:Page,language:"en"|"de") {
  const m=language==="en"?en:de;
  await openProject(page,language);await dismissToasts(page,language);
  await page.keyboard.press("Control+Shift+P");await page.getByRole("combobox").fill(m["toolbar.busMonitor"]);await page.getByRole("combobox").press("Enter");
  await page.locator(".bus-diagnostics-tabs").getByRole("button",{name:m["lineScan.title"],exact:true}).click();
  return page.locator(".line-scan-panel");
}
async function fillPlan(page:Page,language:"en"|"de") {
  const m=language==="en"?en:de;const panel=page.locator(".line-scan-panel");
  await panel.getByLabel(m["lineScan.firstDevice"],{exact:true}).fill("5");await panel.getByLabel(m["lineScan.lastDevice"],{exact:true}).fill("7");
  await panel.getByLabel(m["lineScan.timeout"],{exact:true}).fill("2000");await panel.getByLabel(m["lineScan.pause"],{exact:true}).fill("0");
}
for (const language of ["en","de"] as const) for (const width of [1440,400]) {
  const m=language==="en"?en:de;
  test(`${language} ${width}: scan-local TCP, captured provenance, skipped frames, remount and exact Cancel`,{tag:"@exclusive"},async({page},info)=>{
    await page.setViewportSize({width,height:1000});const fixture=await serve(page,language);const panel=await openScan(page,language);
    const selector=panel.getByRole("combobox",{name:m["lineScan.transport"],exact:true});
    await expect(selector).toHaveValue("udp");expect(fixture.control.starts).toEqual([]);
    await fillPlan(page,language);await panel.getByRole("button",{name:m["lineScan.estimate"],exact:true}).click();
    await expect(panel.locator(".line-scan-start")).toBeEnabled();await selector.selectOption("tcp");
    await expect(panel.locator(".line-scan-experimental")).toHaveText(m["transport.experimentalNote"]);
    await expect(panel.locator(".line-scan-start")).toBeDisabled();expect(fixture.control.starts).toEqual([]);
    await panel.getByRole("button",{name:m["lineScan.estimate"],exact:true}).click();await panel.locator(".line-scan-start").click();
    await expect(panel).toContainText(m["lineScan.actualTransport"].replace("{transport}","TCP"));
    await expect(panel).toContainText(m["lineScan.skippedFrames"].replace("{count}","2"));await expect(selector).toBeDisabled();
    await expect(panel.locator("[data-scan-outcome=occupied]")).toContainText("0x07b0");
    await page.locator(".bus-diagnostics-tabs").getByRole("button",{name:m["toolbar.busMonitor"],exact:true}).click();
    await page.locator(".bus-diagnostics-tabs").getByRole("button",{name:m["lineScan.title"],exact:true}).click();
    await expect(panel.locator("[data-scan-outcome]")).toHaveCount(1);expect(fixture.control.starts).toEqual(["/api/bus/scan/start-tcp"]);expect(fixture.control.cancels).toEqual([]);
    await panel.getByRole("button",{name:m["lineScan.cancel"],exact:true}).click();
    await expect(panel).toContainText(m["lineScan.status.cancelled"]);await expect(panel).toContainText(m["lineScan.skipSample"]);
    expect(fixture.control.cancels).toEqual([`?sessionId=${fixture.data.start.sessionId}&serverIncarnation=${fixture.data.start.serverIncarnation}`]);
    expect(fixture.settings.preferredIpTunnelTransport).toBe("tcp");expect(fixture.state.unexpected).toEqual([]);
    expect(await page.evaluate(()=>document.documentElement.scrollWidth-document.documentElement.clientWidth)).toBeLessThanOrEqual(1);
    await panel.locator(".line-scan-progress").scrollIntoViewIfNeeded();await panel.screenshot({path:info.outputPath("tcp-line-scan.png")});
    info.annotations.push({type:"backend-fixture",description:process.env.KNXBENCH_SCAN_CAPTURE_DIR?"actual socket-HTTP start/poll/cancel JSON":"authored offline control (not captured-backend proof)"});
  });
  test(`${language} ${width}: old TCP server refuses without UDP fallback`,{tag:"@exclusive"},async({page})=>{
    await page.setViewportSize({width,height:1000});const fixture=await serve(page,language,{missing:true});const panel=await openScan(page,language);await fillPlan(page,language);
    await panel.getByRole("combobox",{name:m["lineScan.transport"],exact:true}).selectOption("tcp");
    await panel.getByRole("button",{name:m["lineScan.estimate"],exact:true}).click();await panel.locator(".line-scan-start").click();
    await expect(panel.getByRole("alert")).toContainText(m["lineScan.tcpUnsupported"]);expect(fixture.control.starts).toEqual(["/api/bus/scan/start-tcp"]);expect(fixture.control.cancels).toEqual([]);expect(fixture.state.unexpected).toEqual([]);
  });
  for (const lateError of [false,true]) test(`${language} ${width}: stale first-poll ${lateError?"error":"success"} cannot replace the accepted successor`,{tag:"@exclusive"},async({page})=>{
    await page.setViewportSize({width,height:1000});const fixture=await serve(page,language);
    let release!:()=>void;let first=true;
    const delayed=new Promise<void>(resolve=>{release=resolve;});
    await page.route("**/api/bus/scan/results**",async route=>{
      if (!first) return route.fallback();first=false;await delayed;
      return route.fulfill(lateError?{status:500,headers:{"x-retired-scan":"yes"},json:{error:"retired first poll"}}:
        {headers:{"x-retired-scan":"yes"},json:{...fixture.data.cancel,status:"completed",serverIncarnation:"retired-process",results:[{address:"1.1.99",outcome:{kind:"occupiedBusy"}}]}});
    });
    const panel=await openScan(page,language);await fillPlan(page,language);
    await panel.getByRole("combobox",{name:m["lineScan.transport"],exact:true}).selectOption("tcp");
    await panel.getByRole("button",{name:m["lineScan.estimate"],exact:true}).click();await panel.locator(".line-scan-start").click();
    await expect(panel.locator("[data-scan-outcome]")).toHaveCount(1);await expect(panel).toContainText("1.1.5");
    const oldResponse=page.waitForResponse(response=>response.headers()["x-retired-scan"]==="yes");release();
    await (await oldResponse).finished();await page.evaluate(()=>new Promise<void>(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve()))));
    await expect(panel.locator("[data-scan-outcome]")).toHaveCount(1);await expect(panel).toContainText("1.1.5");
    await expect(panel).not.toContainText("1.1.99");await expect(panel).not.toContainText("retired first poll");
    expect(fixture.control.starts).toEqual(["/api/bus/scan/start-tcp"]);expect(fixture.control.cancels).toEqual([]);expect(fixture.state.unexpected).toEqual([]);
  });
  test(`${language} ${width}: malformed retained metadata locks actions`,{tag:"@exclusive"},async({page})=>{
    await page.setViewportSize({width,height:1000});const fixture=await serve(page,language,{malformed:true});const panel=await openScan(page,language);
    await expect(panel.getByRole("combobox",{name:m["lineScan.transport"],exact:true})).toBeDisabled();await expect(panel.locator(".line-scan-start")).toBeDisabled();
    expect(fixture.control.starts).toEqual([]);expect(fixture.control.cancels).toEqual([]);expect(fixture.state.unexpected).toEqual([]);
  });
}
