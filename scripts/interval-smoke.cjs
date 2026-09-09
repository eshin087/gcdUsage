/* Exercises interval metrics against the isolated app's SQLite data. No previews,
 * credentials, or account identifiers are written into the report. Launch as
 * documented in native-smoke.cjs, and set GCD_USAGE_DATA_DIR to that test folder. */
const assert=require('node:assert/strict');
const fs=require('node:fs/promises');
const path=require('node:path');
const {DatabaseSync}=require('node:sqlite');
const {chromium}=require(process.env.GCD_PLAYWRIGHT_MODULE || 'playwright');
async function main(){
 const out=path.resolve(process.env.GCD_QA_OUTPUT_DIR || '.local-test/interval-qa');
 assert.ok(process.env.GCD_USAGE_DATA_DIR,'Use an isolated app data directory');
 await fs.mkdir(out,{recursive:true});
 const browser=await chromium.connectOverCDP('http://127.0.0.1:9223');
 let page;
 for(let i=0;i<150&&!page;i++) {page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url().startsWith('http://tauri.localhost'));if(!page)await new Promise(r=>setTimeout(r,100));}
 assert.ok(page);const errors=[];page.on('pageerror',e=>errors.push(e.message));
 const invoke=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
 await page.waitForFunction(async()=>{const o=await window.__TAURI_INTERNALS__.invoke('get_overview');return !o.importing && o.stats.requestCount>0;},null,{timeout:120000});
 const initial=await invoke('get_overview');
 assert.equal(initial.settings.fontScale,120);assert.equal(initial.settings.stripLocked,false);
 const db=new DatabaseSync(path.join(process.env.GCD_USAGE_DATA_DIR,'usage.sqlite3'),{readOnly:true});
 const now=Math.floor(Date.now()/1000); const report=[];
 for(const [name,seconds] of [['30m',1800],['1h',3600],['6h',21600],['24h',86400],['7d',604800],['all',null]]){
  const range={from:seconds==null?null:now-seconds,to:now};const begin=performance.now();
  const result=await invoke('get_usage_metrics',{range});
  const elapsedMs=Math.round(performance.now()-begin);
  const rows=db.prepare('SELECT data FROM requests WHERE timestamp>=? AND timestamp<?').all(range.from??0,range.to).map(r=>JSON.parse(r.data));
  const total=rows.reduce((sum,r)=>sum+(r.tokens.input??0)+(r.tokens.cacheRead??0)+(r.tokens.cacheWrite??0)+(r.tokens.output??0),0);
  const prompts=db.prepare("SELECT COUNT(*) n FROM prompts WHERE kind='user' AND timestamp>=? AND timestamp<?").get(range.from??0,range.to).n;
  assert.equal(result.stats.requestCount,rows.length,name);assert.equal(result.stats.totalTokens,total,name);assert.equal(result.stats.promptCount,prompts,name);
  assert.equal(result.activity.reduce((s,b)=>s+b.tokens,0),total,name);
  assert.equal(result.activity.reduce((s,b)=>s+b.requests,0),rows.length,name);
  assert.equal(result.stats.modelStats.reduce((s,m)=>s+m.totalTokens,0),total,name);
  assert.equal(result.stats.modelStats.reduce((s,m)=>s+m.requestCount,0),rows.length,name);
  assert.ok(result.activity.length<=96);report.push({name,prompts,requests:rows.length,tokens:total,elapsedMs,buckets:result.activity.length});
 }
 db.close();
 const waitMetrics=()=>page.waitForFunction(()=>document.querySelector('.stats-grid')?.getAttribute('aria-busy')==='false');
 for(const preset of ['30m','1h','6h','24h','7d','week','30d','month','all']){
  await page.getByRole('combobox',{name:'Statistics time range'}).selectOption(preset);
  await waitMetrics();assert.equal(await page.getByRole('alert').count(),0,preset);
 }
 await page.getByRole('combobox',{name:'Statistics time range'}).selectOption('custom');
 await page.getByLabel('Statistics start',{exact:true}).fill('2035-01-01T00:00');
 await page.getByLabel('Statistics end',{exact:true}).fill('2035-01-01T01:00');
 await waitMetrics();await page.getByText('No recorded activity in this interval.').waitFor();
 assert.equal(await page.locator('.stat-item strong').nth(1).innerText(),'0');
 await page.getByLabel('Statistics end',{exact:true}).fill('2035-01-01T00:00');
 await page.getByRole('alert').filter({hasText:'end time must be after'}).waitFor();
 assert.equal(await page.locator('.stat-item strong').nth(1).innerText(),'—');
 await page.getByRole('combobox',{name:'Statistics time range'}).selectOption('24h');await waitMetrics();
 await page.getByRole('button',{name:'Explore history'}).click();
 const historyFrom=await page.getByLabel('From',{exact:true}).inputValue();assert.match(historyFrom,/T/);
 await page.locator('nav').getByRole('button',{name:'Overview',exact:true}).click();await waitMetrics();
 await page.screenshot({path:path.join(out,'native-interval-overview.png'),fullPage:true});
 const sizes=[];
 for(const size of [90,160,120]){
  await page.locator('nav').getByRole('button',{name:'Settings',exact:true}).click();
  await page.getByRole('slider',{name:'Font size'}).fill(String(size));
  await page.getByRole('button',{name:/^(Save changes|Finish setup)$/}).click();
  await page.getByText(/^(Settings saved\.|All set\. Your meters keep running when you close this window\.)$/).waitFor();
  assert.equal((await invoke('get_overview')).settings.fontScale,size);
  const px=await page.evaluate(()=>parseFloat(getComputedStyle(document.documentElement).fontSize));
  assert.ok(Math.abs(px-13*size/100)<0.01); sizes.push({size,px});
 }
 await page.screenshot({path:path.join(out,'native-size-settings.png'),fullPage:false});
 await page.reload();await waitMetrics();
 assert.equal(await page.getByRole('combobox',{name:'Statistics time range'}).inputValue(),'24h');
 assert.equal((await invoke('get_overview')).settings.fontScale,120);
 assert.equal((await invoke('get_overview')).settings.stripLocked,false);
 assert.deepEqual(errors,[]);
 const result={intervals:report,fontSizes:sizes,rangePersisted:true,invalidAndEmptyRanges:true,browserErrors:errors};
 await fs.writeFile(path.join(out,'interval-report.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result,null,2));
 const closed=page.waitForEvent('close',{timeout:10000});
 await invoke('plugin:window|close',{label:'dashboard'}).catch(e=>{if(!/closed|crashed|destroyed/i.test(String(e)))throw e;});
 await closed;await browser.close().catch(()=>{});
}
main().catch(error=>{console.error(error);process.exitCode=1;});
