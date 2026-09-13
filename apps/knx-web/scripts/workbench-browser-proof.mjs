/** Real Chromium proof with intercepted HTTP data. Never connects to a KNX bus.
 * Run against a local Vite server with PLAYWRIGHT_MODULE pointing to an installed
 * playwright-core index.mjs. Output directory and URL are command arguments.
 */
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE);
const output = resolve(process.argv[2]);
const base = process.argv[3] ?? 'http://127.0.0.1:1427';
await mkdir(output, { recursive: true });
const devices = ['Lichtaktor', 'Jalousieaktor', 'Tastsensor', 'Raumregler', 'Präsenzmelder', 'Heizungsaktor'].map((name, i) => ({ id: i + 1, name, address: `1.1.${11+i}`, description: 'Beispieldaten · kein reales Gerät', com_object_count: 4 }));
const addresses = Array.from({length:8}, (_,i) => ({id:i+1,address:`1/0/${i+1}`,name:`Beispiel Licht ${i+1}`}));
const tree = { schema_version:11, errors:0, warnings:2, can_undo:false, can_redo:false, installations:[{id:1,name:'Beispielprojekt · Villa Linden',topology:[{id:1,name:'Haus',address:1,lines:[{id:1,name:'Erdgeschoss',address:1,devices}]}],buildings:[{id:1,name:'Villa Linden',kind:'Building',devices:[],children:[{id:2,name:'Erdgeschoss',kind:'Floor',devices:[],children:[{id:3,name:'Wohnzimmer',kind:'Room',devices,children:[]}]}]}],unassigned:[],group_addresses:addresses,group_ranges:[]}] };
const coms = ['Licht schalten','Status','Zentral schalten','Sperren'].map((name,i) => ({id:i+1,number:i,name,dpt:'DPST-1-1',dpt_layer:'Program',description:null,description_layer:null,is_active:true,read:true,write:true,transmit:false,update:false,communication:true,links:[{ga_id:i+1,address:`1/0/${i+1}`,name:addresses[i].name,direction:'Receive'}]}));
// T16: one `DeviceProductNode` per `ProductResolution` variant, cycled across
// the six example devices so a screenshot run exercises all four states. Every
// ref and catalogue value below is invented — no real product is named here.
const products = [
  {product_ref:'M-00FA_H-EX42-1_P-1',program_ref:'M-00FA_H-EX42-1_HP-1',resolution:'Resolved',catalog:{manufacturer_id:'M-00FA',manufacturer_name:'Beispiel Gerätebau',product_text:'Beispiel-Schaltaktor 4-fach',order_number:'EX-4210',hardware_name:'EX-HW-42',hardware_version:'1',hardware_serial_number:null,catalog_item_name:'Beispiel-Schaltaktor, 4-fach, REG',catalog_item_number:'EX-4210-4',application_program_id:'M-00FA_A-1234-2-0000',application_name:'Schalten 4f',application_number:'4660',application_version:'2',mask_version:'MV-0701'}},
  {product_ref:'M-00FA_H-EX43-1_P-1',program_ref:'M-00FA_H-EX43-1_HP-1',resolution:'NotInDatabase',catalog:null},
  {product_ref:'M-00FB_H-EX07-1_P-1',program_ref:'M-00FB_H-EX07-1_HP-1',resolution:'NoDatabase',catalog:null},
  {product_ref:null,program_ref:null,resolution:'NoReference',catalog:null},
];
const longTree = structuredClone(tree);
longTree.installations[0].name = 'Beispielprojekt · großer Datenbestand';
longTree.installations[0].group_addresses = Array.from({length:2000}, (_,i) => ({id:i+1,address:`1/${Math.floor(i/256)}/${i%256}`,name:`Beispiel ${i+1} · ${'Langer beschreibender Gruppenadressenname '.repeat(8)}`}));
const telegrams = Array.from({length:12}, (_,i) => ({seq:i+1,timestamp:`2026-09-13T14:32:${String(8+i).padStart(2,'0')}.124Z`,source:'1.1.13',destination:`1/0/${i%8+1}`,destinationName:`Beispiel Licht ${i%8+1}`,service:'GroupValueWrite',rawPayload:i%2?'00':'01',decoded:{kind:'value',dpt:'DPST-1-1',text:i%2?'Off':'On'}}));
const errors = [], checks = [];
const browser = await chromium.launch({headless:true,executablePath:process.env.CHROMIUM_PATH ?? '/usr/bin/chromium',args:['--no-sandbox']});
try {
  const page = await browser.newPage({viewport:{width:1920,height:1080}});
  await page.addInitScript(() => {localStorage.setItem('knx-desktop:ui-language','de');localStorage.setItem('knx-desktop:theme','porcelain');});
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/api/**', async route => {
    const url = new URL(route.request().url()), path = url.pathname;
    let data;
    if (path === '/api/fs/list') data = [{name:'Beispiel.knxproj',is_dir:false},{name:'Grosses-Beispiel.knxproj',is_dir:false}];
    else if (path === '/api/project/import') data = route.request().postDataJSON().path === 'Grosses-Beispiel.knxproj' ? longTree : tree;
    else if (/^\/api\/device\/\d+$/.test(path)) { const n = Number(path.split('/').at(-1)); data = {...devices[n-1],com_objects:coms,product:products[(n-1)%products.length]}; }
    else if (path.endsWith('/parameters')) data = {programId:null,sections:[],stale:[],diagnostics:[]};
    else if (path === '/api/bus/monitor/telegrams') data = {sessionId:1,status:'active',nextSince:13,droppedBefore:0,telegrams:Number(url.searchParams.get('since'))?[]:telegrams};
    else if (path === '/api/product-languages' || path === '/api/log') data = [];
    else { errors.push(`Unexpected API request: ${route.request().method()} ${path}`); return route.fulfill({status:400,contentType:'application/json',body:'{"error":"Unexpected simulated request"}'}); }
    return route.fulfill({contentType:'application/json',body:JSON.stringify(data)});
  });
  await page.goto(base);
  await page.locator('.welcome-workspace button').first().focus();
  await page.keyboard.press('Enter');
  await page.getByRole('button',{name:'Beispiel.knxproj',exact:true}).focus();
  await page.keyboard.press('Enter');
  await page.locator('.workbench-navigation').getByRole('button',{name:'Gebäude',exact:true}).click();
  await page.locator('.building-diagram-node > .diagram-heading').filter({hasText:'Wohnzimmer'}).click();
  await page.locator('.structure-workspace .table-select').first().focus();
  await page.keyboard.press('Enter');
  await page.getByRole('tab',{name:'Parameter',exact:true}).click();
  await page.keyboard.press('ArrowLeft');
  assert.equal(await page.getByRole('tab',{name:'Kommunikationsobjekte',exact:true}).getAttribute('aria-selected'),'true');
  checks.push('File open, device selection, parameter/com-object tab switching through real keyboard events');
  await page.screenshot({path:`${output}/01-porcelain.png`});
  const settings = page.locator('.workbench-toolbar').getByRole('button',{name:'Einstellungen',exact:true});
  async function appearance(theme, accent, density) {
    await settings.click();
    const dialog = page.getByRole('dialog');
    await dialog.locator('select').first().selectOption(theme);
    await dialog.getByLabel('Akzentfarbe',{exact:true}).selectOption(accent);
    await dialog.getByLabel('Dichte',{exact:true}).selectOption(density);
    await page.keyboard.press('Escape');
    // Wait beyond the configured 200 ms theme transition. Document-wide
    // Animation.finished can remain pending for animations in hidden panels.
    await page.waitForTimeout(250);
  }
  await appearance('graphite','violet','compact');
  await page.locator('.workbench-navigation').getByRole('button',{name:'Gruppenadressen',exact:true}).click();
  await page.locator('.table-select').first().click();
  await page.screenshot({path:`${output}/02-graphite.png`});
  await appearance('graphite','mint','comfortable');
  await page.locator('.diagnostic-navigation button').first().click();
  await page.locator('.bus-monitor-table tbody tr').first().focus();
  await page.keyboard.press('Enter');
  assert.match(await page.locator('.telegram-details').innerText(), /Beispiel Licht 1/);
  await page.screenshot({path:`${output}/03-busmonitor.png`});
  checks.push('Graphite group addresses; Mint/comfortable monitor; keyboard telegram detail without a send');
  for (const width of [1280,1920,960]) {
    await page.setViewportSize({width,height:1080});
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth),false,`Overflow at ${width}`);
    await page.screenshot({path:`${output}/monitor-${width}.png`});
  }
  checks.push('No document overflow at 1280, 1920 and 960 CSS px (960 models 200% desktop layout at 1920)');
  await appearance('system','mint','compact');
  await page.emulateMedia({colorScheme:'light'});
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'porcelain');
  await page.emulateMedia({colorScheme:'dark'});
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'graphite');
  await page.emulateMedia({reducedMotion:'reduce'});
  const duration = await settings.evaluate(el => getComputedStyle(el).transitionDuration);
  assert.equal(duration,'0s');
  checks.push('System mode follows live OS light/dark; reduced motion removes button transition');
  await page.emulateMedia({reducedMotion:'no-preference'});
  await page.mouse.move(0,0);
  await settings.evaluate(el => {
    window.proofTransitions = [];
    el.addEventListener('transitionend', e => window.proofTransitions.push({property:e.propertyName,seconds:e.elapsedTime}));
  });
  await settings.hover();
  await page.waitForTimeout(300);
  const transitions = await page.evaluate(() => window.proofTransitions);
  assert.ok(transitions.some(e => e.property === 'background-color' && e.seconds > 0 && e.seconds <= 0.14));
  checks.push(`Real hover transition completed: ${JSON.stringify(transitions)}`);
  function luminance(color) {
    const channels = color.match(/[\d.]+/g).slice(0,3).map(Number).map(v => v/255).map(v => v <= 0.04045 ? v/12.92 : ((v+0.055)/1.055)**2.4);
    return channels[0]*0.2126 + channels[1]*0.7152 + channels[2]*0.0722;
  }
  for (const theme of ['porcelain','graphite']) {
    let statusColor;
    for (const accent of ['violet','mint','blue','amber','rose']) {
      await appearance(theme,accent,'compact');
      const colors = await page.locator('.primary-action').evaluate(el => ({fg:getComputedStyle(el).color,bg:getComputedStyle(el).backgroundColor,warning:getComputedStyle(document.documentElement).getPropertyValue('--knx-warning-color')}));
      const [light,dark] = [luminance(colors.fg),luminance(colors.bg)].sort((a,b)=>b-a);
      assert.ok((light+0.05)/(dark+0.05) >= 4.5,`${theme}/${accent} primary-action contrast`);
      if (statusColor !== undefined) assert.equal(colors.warning,statusColor,'Accent changed warning color');
      statusColor = colors.warning;
    }
  }
  checks.push('All five accents in both themes: primary-action text contrast >=4.5 and warning color independent of accent');
  await page.setViewportSize({width:1280,height:1080});
  await page.locator('.file-menu summary').click();
  await page.locator('.file-menu').getByRole('button',{name:'Projekt öffnen…',exact:true}).click();
  await page.getByRole('button',{name:'Grosses-Beispiel.knxproj'}).click();
  await page.locator('.workbench-navigation').getByRole('button',{name:'Gruppenadressen',exact:true}).click();
  assert.equal(await page.locator('.workspace-table tbody tr').count(),2000);
  await page.locator('.workspace-table tbody tr').last().scrollIntoViewIfNeeded();
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth),false,'Long labels overflow');
  await page.screenshot({path:`${output}/04-large-list-1280.png`});
  checks.push('2000 address rows with long names: last row reachable, no page overflow at 1280 CSS px');
  assert.deepEqual(errors,[]);
  await writeFile(`${output}/checks.json`,JSON.stringify({timestamp:new Date().toISOString(),checks,errors,scope:'Simulated HTTP fixture, no backend or KNX hardware; not native Tauri or screen-reader proof'},null,2));
  console.log(checks.join('\n'));
} finally { await browser.close(); }
