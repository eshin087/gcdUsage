const assert=require('node:assert/strict');
const fs=require('node:fs/promises');
const {chromium}=require(process.env.GCD_PLAYWRIGHT_MODULE || 'playwright');
async function main(){
 const browser=await chromium.connectOverCDP('http://127.0.0.1:9223');
 let page;
 for(let attempt=0;attempt<150&&!page;attempt++){page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url().startsWith('http://tauri.localhost'));if(!page)await new Promise(r=>setTimeout(r,100));}
 assert.ok(page);await page.waitForFunction(()=>!!window.__TAURI_INTERNALS__?.invoke);
 const invoke=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
 const overview=await invoke('get_overview');assert.ok(overview.settings.deviceId);
 for(const [cmd,args] of [['plugin:window|set_title',{label:'dashboard',title:'Forbidden'}],['plugin:fs|read_text_file',{path:'unneeded'}],['nonexistent_security_probe',{}]]) {
   let denied=false;try{await invoke(cmd,args);}catch(e){denied=true;}assert.ok(denied,cmd+' must reject');
 }
 const http=require('node:http');let received=0;
 const server=http.createServer((req,res)=>{received++;res.end('Unexpected navigation');});
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 const destination='http://127.0.0.1:'+server.address().port+'/security-probe';
 const before=page.url();
 await page.evaluate(url=>{location.href=url;},destination);
 await new Promise(r=>setTimeout(r,1000));
 const after=page.url();server.closeAllConnections();await new Promise(r=>server.close(r));
 assert.equal(after,before);assert.ok((await invoke('get_overview')).settings.deviceId);
 // WebView2 may send speculative GETs before NavigationStarting cancellation.
 // This verifies document isolation, not an outbound network firewall.
 const pagesBefore=browser.contexts().flatMap(c=>c.pages()).length;
 await page.evaluate(()=>window.open('https://example.invalid/gcd-security-popup-probe'));
 await new Promise(r=>setTimeout(r,500));assert.equal(browser.contexts().flatMap(c=>c.pages()).length,pagesBefore);assert.equal(page.url(),before);
 await fs.writeFile('.local-test/security-audit/native-security.json',JSON.stringify({allowedDashboard:true,ungrantedCommandsDenied:true,externalDocumentBlocked:true,speculativeRequestsObserved:received,popupsBlocked:true},null,2));
 await browser.close();console.log('Native permission, navigation, and popup checks passed');
}
main().catch(e=>{console.error(e);process.exit(1);});
