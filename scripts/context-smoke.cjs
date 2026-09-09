// Native 0.4 regression. Run only against scripts/fixture-history.py data.
const assert=require('node:assert/strict');const fs=require('node:fs/promises');
const {chromium}=require(process.env.GCD_PLAYWRIGHT_MODULE||'playwright');
(async()=>{const browser=await chromium.connectOverCDP('http://127.0.0.1:9223');try{
 let page;for(let i=0;i<150&&!page;i++){page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url().startsWith('http://tauri.localhost'));if(!page)await new Promise(r=>setTimeout(r,100));}assert.ok(page);
 const invoke=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
 let o;for(let i=0;i<120;i++){o=await invoke('get_overview');if(!o.importing&&o.dock.prompts.length===20)break;await new Promise(r=>setTimeout(r,500));}assert.equal(o.settings.deviceName,'Synthetic QA');assert.equal(o.dock.prompts.length,20);assert.ok(o.dock.tokens>0);
 await invoke('import_history');await new Promise(r=>setTimeout(r,6000));
 const history=await invoke('get_history',{filter:{limit:50}});assert.equal(history.total,36);assert.ok(history.items.every(r=>r.prompt.project));assert.ok(history.items.every(r=>!r.prompt.preview.includes('<send_user_message')&&!r.prompt.preview.includes('<in-app-browser-context')&&!r.prompt.preview.includes('Internal browser source')));
 const errors=[];page.on('pageerror',e=>errors.push(e.message));await fs.mkdir('.local-test/v04/screens',{recursive:true});await page.locator('nav').getByRole('button',{name:'Overview',exact:true}).click();await page.screenshot({path:'.local-test/v04/screens/overview.png',fullPage:true});
 await page.locator('nav').getByRole('button',{name:/History/}).click();await page.locator('.prompt-link').first().click();await page.getByRole('dialog',{name:'Recorded prompt details'}).waitFor();assert.ok(await page.locator('.detail-preview').innerText());await page.screenshot({path:'.local-test/v04/screens/details.png'});await page.getByRole('button',{name:'Done',exact:true}).click();
 await page.screenshot({path:'.local-test/v04/screens/history.png',fullPage:false});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'History must fit the window');
 const original=await invoke('open_original_prompt',{id:'local:'+history.items[0].prompt.id});assert.equal(original,false);
 await assert.rejects(()=>invoke('open_original_prompt',{id:'https://example.invalid/'}));await assert.rejects(()=>invoke('get_prompt_detail',{id:"local:' OR 1=1 --"}));
 const detail=await invoke('get_prompt_detail',{id:'local:'+history.items[0].prompt.id});assert.equal(detail.conversationId,history.items[0].prompt.sessionId);assert.equal(detail.originalUrl,null);
 await page.locator('nav').getByRole('button',{name:'Settings',exact:true}).click();await page.screenshot({path:'.local-test/v04/screens/settings.png',fullPage:true});
 assert.ok(await page.evaluate(()=>parseFloat(getComputedStyle(document.documentElement).fontSize)>=18));
 await assert.rejects(()=>invoke('get_browser_history',{filter:{}}));await assert.rejects(()=>invoke('import_browser_history',{accountLabel:'Test'}));
 assert.equal(await page.getByRole('button',{name:'Browser chats'}).count(),0);
 await page.locator('nav').getByRole('button',{name:'Overview',exact:true}).click();assert.equal(await page.getByLabel('Remaining requests unavailable').textContent(),'—');
 assert.deepEqual(errors,[]);const report={prompts:history.total,contextLabels:true,wrapperCleanup:true,detailNavigation:true,untrustedLinksRejected:true,defaultFont:18,retiredBrowserCommandsDenied:true,proRemainingUnknown:true,errors};await fs.writeFile('.local-test/v04/context-report.json',JSON.stringify(report,null,2));console.log(JSON.stringify(report));
}finally{await browser.close()}})().catch(e=>{console.error(e);process.exitCode=1});
