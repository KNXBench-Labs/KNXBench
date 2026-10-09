/* Run through playwright-cli run-code after substituting DEMO_CONFIG.
 * Real UI actions and real server reads only; no route mocks or bus actions.
 */
async (page) => {
  const config = /* DEMO_CONFIG */ null;
  if (!config) throw new Error("Supply the verified candidate and evidence paths");
  const assertions = [];
  const result = { assertions, projects: [], externalRequests: [], pageErrors: [], httpErrors: [] };
  const check = (condition, name) => {
    if (!condition) throw new Error(name);
    assertions.push(name);
  };
  const responseListener = response => {
    if (response.status() >= 400) result.httpErrors.push({path: new URL(response.url()).pathname, status: response.status()});
  };
  const requestListener = request => {
    const url = new URL(request.url());
    if (url.origin !== config.origin) result.externalRequests.push(request.url());
    if (url.pathname.startsWith("/api/bus/") && url.pathname !== "/api/bus/discover") {
      result.pageErrors.push(`Unexpected bus request: ${url.pathname}`);
    }
  };
  const errorListener = error => result.pageErrors.push(String(error));
  page.on("response", responseListener);
  page.on("request", requestListener);
  page.on("pageerror", errorListener);
  async function get(path) {
    const response = await page.request.get(config.origin + path);
    check(response.ok(), `GET ${path} succeeds`);
    return response.json();
  }
  const nav = name => page.getByRole("navigation", {name:"Navigation", exact:true}).getByRole("button", {name, exact:true}).click();
  async function openNative(filename) {
    await page.locator("summary", {hasText:"File"}).first().click();
    await page.getByRole("button", {name:"Open (.knxdb)…", exact:true}).click();
    const picker = page.getByRole("dialog", {name:"Open", exact:true});
    await picker.getByRole("button", {name:filename, exact:true}).click();
    await picker.waitFor({state:"hidden"});
    await page.getByRole("button", {name:"Save", exact:true}).waitFor();
  }
  async function selectDevice(target) {
    await nav("Topology");
    await page.getByRole("button", {name:`${target.deviceAddress} ${target.deviceName}`,exact:true}).first().click();
    await page.getByRole("tablist", {name:target.deviceName,exact:true}).waitFor();
  }
  async function acknowledgement(path, action) {
    const pending = page.waitForResponse(r => r.url().includes(path) && r.request().method() === "POST");
    await action();
    const response = await pending;
    check(response.ok(), `${path} UI write acknowledged`);
    // Save/save-as are Promise<void> routes, not JSON read-model responses.
    return path === "save" ? null : response.json();
  }
  function devices(tree) { return tree.installations.flatMap(i => i.topology.flatMap(a => a.lines.flatMap(l => l.devices))); }
  function groups(tree) { return tree.installations.flatMap(i => i.group_addresses); }
  try {
    await page.addInitScript(() => {
      localStorage.setItem("knx-desktop:ui-language", "en");
      localStorage.setItem("knx-desktop:theme", "porcelain");
    });
    await page.goto(config.origin);
    const skip = page.getByRole("button", {name:"Skip introduction", exact:true});
    // First-run guide opens asynchronously after settings + version hydrate.
    // A fresh acceptance runtime must exercise its actual dismissal.
    await skip.waitFor();
    await skip.click();
    await skip.waitFor({state:"hidden"});
    await nav("Product catalog");
    await page.getByText("No matches.", {exact:true}).waitFor();
    const catalogRegion = page.getByRole("region", {name:"Device catalog"});
    check(await catalogRegion.locator("select option").count() === 1
          && await catalogRegion.locator("select").textContent() === "All manufacturers",
          "Fresh catalogue has only All manufacturers");
    const catalogue = await acknowledgement("/api/catalog/install", () =>
      page.getByRole("region", {name:"Device catalog"}).locator("input[type=file]").setInputFiles(config.candidate + "/fictional-demo-devices.knxprod"));
    check(catalogue.unknown === 0 && catalogue.conflicts === 0, "Fictional catalogue installs with zero unknown/conflicts");
    await page.getByRole("region", {name:"Device catalog"}).locator("select option", {hasText:"KNXBench Demo Devices (fictional)"}).waitFor({state:"attached"});
    result.catalogue = {scheme:catalogue.scheme, members:catalogue.members.length, unknown:catalogue.unknown,
                        conflicts:catalogue.conflicts, sha256:catalogue.sha256};
    const repeat = await acknowledgement("/api/catalog/install", () =>
      page.getByRole("region", {name:"Device catalog"}).locator("input[type=file]").setInputFiles(config.candidate + "/fictional-demo-devices.knxprod"));
    check(repeat.skipped === true, "Identical catalogue re-import is deduplicated");
    for (const target of config.targets) {
      await openNative(target.key + ".knxdb");
      const baseline = await get("/api/project");
      check(devices(baseline).length === target.devices, `${target.key}: exact device count`);
      check(groups(baseline).length === target.groups, `${target.key}: exact group count`);
      check(baseline.schema_version === 10, `${target.key}: native schema 10`);
      check(groups(baseline).every(g => g.dpt_detail.outcome === "Declared" && g.links.some(l=>l.direction==="Send") && g.links.some(l=>l.direction==="Receive")),
            `${target.key}: all baseline groups typed with sender and receiver`);
      check(new Set(devices(baseline).map(d=>d.address)).size === target.devices, `${target.key}: unique individual addresses`);
      const device = devices(baseline).find(d => d.address === target.deviceAddress);
      check(device?.name === target.deviceName, `${target.key}: practice device identity`);
      // Every programme/parameter must resolve, not merely the one photographed.
      let evaluatedDevices = 0;
      for (const d of devices(baseline)) {
        const panel = await get(`/api/device/${d.id}/parameters`);
        check(panel.programId !== null && panel.stale.length === 0 && panel.diagnostics.length === 0,
              `${target.key}: device ${d.id} catalogue resolves without diagnostics`);
        check(panel.sections.flatMap(s=>s.fields).some(f=>f.name==="StartupDelay" && f.value==="2" && f.editable),
              `${target.key}: device ${d.id} original editable parameter`);
        evaluatedDevices++;
      }
      await nav("Buildings");
      await page.getByRole("button", {name:target.name, exact:true}).first().waitFor();
      await page.screenshot({path:`${config.evidence}/${target.key}-buildings.png`});
      await nav("Group addresses");
      await page.getByRole("button", {name:`${target.traceAddress} ${target.traceName}`,exact:true}).first().click();
      const links = page.getByRole("region", {name:"Links",exact:true});
      await links.getByRole("cell", {name:"Send",exact:true}).waitFor();
      await links.getByRole("cell", {name:"Receive",exact:true}).waitFor();
      check((await links.innerText()).includes(target.traceName.split(" / ")[0]), `${target.key}: UI exercise 1 traces participants`);
      await page.screenshot({path:`${config.evidence}/${target.key}-groups.png`});
      await selectDevice(target);
      const note = " Practice note: reviewed offline.";
      const description = page.getByRole("textbox", {name:"Description",exact:true});
      await description.waitFor();
      await description.fill(device.description + note);
      await acknowledgement("description", () => description.press("Tab"));
      check((await get(`/api/device/${device.id}`)).description === device.description + note,
            `${target.key}: UI exercise 2 description saved`);
      await page.getByRole("tab", {name:"Parameters",exact:true}).click();
      const parameter = page.getByRole("spinbutton", {name:/Delay after bus voltage recovery/});
      await parameter.waitFor();
      check(await parameter.inputValue() === "2", `${target.key}: parameter before edit`);
      await parameter.fill("5");
      const changed = await acknowledgement("parameters", () => parameter.press("Tab"));
      check(changed.sections.flatMap(s=>s.fields).some(f=>f.name==="StartupDelay" && f.value==="5"),
            `${target.key}: UI exercise 3 parameter acknowledged`);
      // Existing group creation has no declared-DPT input. Linking infers 1.001.
      await nav("Group addresses");
      const row = page.locator("li.tree-new-row").filter({has:page.getByPlaceholder("New group address",{exact:true})});
      await row.locator("input").nth(0).fill("0/0/200");
      await row.getByPlaceholder("New group address",{exact:true}).fill("Practice light / Switch command");
      const rangeOption = await row.locator("select option").evaluateAll(xs => xs.map(x=>({value:x.value,text:x.textContent})))
        .then(xs=>xs.find(x=>x.text.includes("0/0/0–0/0/255")));
      check(!!rangeOption, `${target.key}: practice middle range exists`);
      await row.locator("select").selectOption(rangeOption.value);
      await acknowledgement("group-address", () => row.getByRole("button", {name:"Add",exact:true}).click());
      await selectDevice(target);
      await page.getByRole("tab", {name:"Communication objects",exact:true}).click();
      const object = page.locator("details.com-object-detail").filter({has:page.getByText(target.objectText,{exact:true})});
      // Object summaries are inside a collapsed channel. Open the real channel first.
      const channel = object.locator("xpath=ancestor::details[contains(@class,'com-object-channel')]");
      if (await channel.count() && !await channel.evaluate(x=>x.open)) await channel.locator("summary").first().click();
      if (!await object.evaluate(x=>x.open)) await object.locator("summary").click();
      const linkRow = object.locator("li.tree-new-row");
      const gaOption = await linkRow.locator("select").nth(0).locator("option").evaluateAll(xs=>xs.map(x=>({value:x.value,text:x.textContent})))
        .then(xs=>xs.find(x=>x.text.startsWith("0/0/200 ")));
      check(!!gaOption, `${target.key}: practice GA in object picker`);
      await linkRow.locator("select").nth(0).selectOption(gaOption.value);
      await linkRow.locator("select").nth(1).selectOption("Receive");
      await acknowledgement("link", () => linkRow.getByRole("button",{name:"Link",exact:true}).click());
      const edited = await get("/api/project");
      const newGroup = groups(edited).find(g=>g.address==="0/0/200");
      check(newGroup?.dpt_detail.outcome === "Inferred" && newGroup.dpts[0] === "DPST-1-1" && newGroup.links.length === 1
            && newGroup.links[0].direction === "Receive", `${target.key}: UI exercise 4 reserve receiver and inferred DPT`);
      await page.screenshot({path:`${config.evidence}/${target.key}-device.png`});
      await page.locator("summary",{hasText:"File"}).first().click();
      await page.getByRole("button",{name:"Save As…",exact:true}).click();
      const save = page.getByRole("dialog",{name:"Save as",exact:true});
      const filename = target.key + "-practice.knxdb";
      await save.getByRole("textbox",{name:"filename",exact:true}).fill(filename);
      await acknowledgement("save", () => save.getByRole("button",{name:"Save",exact:true}).click());
      await openNative(filename);
      const reopened = await get("/api/project");
      check(devices(reopened).length === target.devices && groups(reopened).length === target.groups + 1,
            `${target.key}: reopened counts`);
      check(devices(reopened).find(d=>d.id===device.id).description === device.description + note,
            `${target.key}: description survives reopen`);
      const panel = await get(`/api/device/${device.id}/parameters`);
      check(panel.sections.flatMap(s=>s.fields).some(f=>f.name==="StartupDelay" && f.value==="5"),
            `${target.key}: parameter survives reopen`);
      const detail = await get(`/api/device/${device.id}`);
      check(detail.com_objects.find(c=>c.number===target.objectNumber).links.some(l=>l.address==="0/0/200" && l.direction==="Receive"),
            `${target.key}: UI exercise 5 receiver link survives reopen`);
      check(groups(reopened).find(g=>g.address==="0/0/200").dpts[0] === "DPST-1-1", `${target.key}: practice DPT survives reopen`);
      result.projects.push({key:target.key, devices:target.devices, baselineGroups:target.groups, evaluatedDevices,
                            exercises:5, practiceFilename:filename, practiceObject:target.objectNumber,
                            practiceDeviceId:device.id, descriptionSaved:true, parameterSaved:true, linkSaved:true});
    }
    check(result.projects.length === 3, "All three demos exercised");
    check(result.externalRequests.length === 0, "No external browser requests");
    check(result.pageErrors.length === 0, "No page exceptions or operational bus requests");
    check(result.httpErrors.every(e => e.path === "/favicon.ico" || e.path === "/api/bus/discover"),
          "Only pre-existing favicon/offline-discovery HTTP errors allowed");
    result.success = true;
    await page.evaluate(value => { window.__communityDemoVerification = value; }, result);
    return {success:true, projects:result.projects, assertionCount:assertions.length, httpErrors:result.httpErrors};
  } catch (error) {
    result.success = false;
    result.failure = String(error);
    await page.evaluate(value => { window.__communityDemoVerification = value; }, result);
    throw error;
  } finally {
    page.off("response", responseListener);
    page.off("request", requestListener);
    page.off("pageerror", errorListener);
  }
}
