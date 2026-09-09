/* Run against an isolated native app; see native-smoke.cjs for launch options.
 * Checks the cached dock totals against SQLite and exercises the settings UI.
 * Reports never contain prompt previews, paths, or account identifiers. */
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');
const path=require('node:path');
const {DatabaseSync}=require('node:sqlite');
const {chromium}=require(process.env.GCD_PLAYWRIGHT_MODULE || 'playwright');
async function main(){
 assert.ok(process.env.GCD_USAGE_DATA_DIR,'Use isolated app data');
 const browser=await chromium.connectOverCDP('http://127.0.0.1:9223');
 try {
  let page;for(let i=0;i<150&&!page;i++){page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url().startsWith('http://tauri.localhost'));if(!page)await new Promise(r=>setTimeout(r,100));}assert.ok(page);
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  const invoke=async(command,args={})=>{let timer;try{return await Promise.race([page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args}),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error(command+' timed out')),15000)})]);}finally{clearTimeout(timer)}};
  const waitDock=async minutes=>{for(let i=0;i<240;i++){const o=await invoke('get_overview');if(!o.importing&&o.dock?.updatedAt&&o.dock.minutes===minutes)return;await new Promise(r=>setTimeout(r,500));}throw Error('Dock interval did not finish loading');};
  await waitDock(60);const initial=await invoke('get_overview');
  assert.equal(initial.settings.dockMinutes,60);assert.equal(initial.settings.stripLocked,false);
  if(process.env.GCD_QA_EXPECT_HISTORY==='1'){assert.ok(initial.dock.prompts.length>0,'Expected populated history');assert.ok(initial.dock.tokens>0,'Expected recent measured activity');}
  await page.locator('nav').getByRole('button',{name:'Settings',exact:true}).click();
  const db=new DatabaseSync(path.join(process.env.GCD_USAGE_DATA_DIR,'usage.sqlite3'),{readOnly:true});
  const intervals=[];
  for(const minutes of [30,60,360,1440,10080,90,60]){
   await page.getByRole('spinbutton',{name:'Custom dock duration (minutes)',exact:true}).fill(String(minutes));
   await page.getByRole('button',{name:'Save changes',exact:true}).click();await waitDock(minutes);
   const o=await invoke('get_overview'),d=o.dock;assert.equal(d.minutes,minutes);assert.ok(d.updatedAt);
   const rows=db.prepare('SELECT data FROM requests WHERE timestamp>=? AND timestamp<?').all(d.updatedAt-minutes*60,d.updatedAt).map(r=>JSON.parse(r.data));
   const total=rows.reduce((s,r)=>s+(r.tokens.input??0)+(r.tokens.cacheRead??0)+(r.tokens.cacheWrite??0)+(r.tokens.output??0),0);
   assert.equal(d.tokens,total);assert.equal(d.models.reduce((s,m)=>s+m.tokens,0),total);
   for(const provider of ['claude','codex']){
    assert.ok(d.prompts.filter(p=>p.provider===provider).length<=10);
    assert.equal(d.providerTokens[provider]??0,rows.filter(r=>r.provider===provider).reduce((s,r)=>s+(r.tokens.input??0)+(r.tokens.cacheRead??0)+(r.tokens.cacheWrite??0)+(r.tokens.output??0),0));
   }
   assert.ok(d.prompts.every((p,i)=>!i||d.prompts[i-1].timestamp>=p.timestamp));
   assert.ok(d.prompts.filter(p=>p.browser).every(p=>p.tokens===null));
   intervals.push({minutes,tokens:total,requests:rows.length,prompts:d.prompts.length});
  }
  for(const minutes of [0,-1,43201,1.5]){await assert.rejects(()=>invoke('save_settings',{settings:{...initial.settings,dockMinutes:minutes}}));}
  const toggle=page.getByRole('checkbox',{name:/^Show prompt previews on hover/});
  await toggle.setChecked(false);await page.getByRole('button',{name:'Save changes',exact:true}).click();
  assert.equal((await invoke('get_overview')).settings.dockPreviews,false);
  await page.reload();await page.locator('nav').getByRole('button',{name:'Settings',exact:true}).click();
  assert.equal(await page.getByRole('checkbox',{name:/^Show prompt previews on hover/}).isChecked(),false);
  await invoke('save_settings',{settings:initial.settings});await waitDock(60);
  assert.deepEqual(errors,[]);db.close();
  const out=process.env.GCD_QA_OUTPUT_DIR || '.local-test/dock-qa';await fs.mkdir(out,{recursive:true});
  const report={intervals,invalidDurationsRejected:true,previewTogglePersisted:true,errors};await fs.writeFile(path.join(out,'dock-report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));
  await invoke('plugin:window|close',{label:'dashboard'}).catch(e=>{if(!/closed|crashed|destroyed/i.test(String(e)))throw e;});
 } finally {await browser.close().catch(()=>{});}
}
main().catch(e=>{console.error(e);process.exitCode=1});
